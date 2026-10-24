//! Reset/prestige logic — a port of the reset half of js/game.js and the
//! challenge functions in js/utils.js.

use super::Game;
use crate::defs::{Currency, PrestigeType, Row};
use crate::format::Decimal;
use crate::player::{get_start_player, layer_data_reset};

impl Game {
    // -----------------------------------------------------------------------
    // Gain formulas (getResetGain / getNextAt)
    // -----------------------------------------------------------------------

    pub fn get_reset_gain(&self, layer: &str) -> Decimal {
        let def = self.layer_def(layer);
        let lt = self.ltmp(layer);
        if let Some(f) = &def.get_reset_gain {
            return f(self);
        }
        if lt.prestige_type == PrestigeType::None {
            return Decimal::zero();
        }
        if lt.gain_exp == Decimal::zero() {
            return Decimal::zero();
        }
        let one = Decimal::one();
        match lt.prestige_type {
            PrestigeType::Static => {
                let can_buy_max = lt.can_buy_max.unwrap_or(false);
                if !can_buy_max || lt.base_amount < lt.requires {
                    return one;
                }
                let mut gain = lt
                    .base_amount
                    / lt.requires
                    / lt.gain_mult;
                gain = gain.max(one).log(lt.base) * lt.gain_exp;
                gain = gain.pow(lt.exponent.pow(-one));
                gain = gain * lt.direct_mult;
                (gain.floor() - self.lstate(layer).points + one).max(one)
            }
            PrestigeType::Normal => {
                if lt.base_amount < lt.requires {
                    return Decimal::zero();
                }
                let mut gain = (lt.base_amount / lt.requires)
                    .pow(lt.exponent)
                    * lt.gain_mult;
                gain = gain.pow(lt.gain_exp);
                if gain >= lt.softcap {
                    gain = gain.pow(lt.softcap_power)
                        * lt.softcap.pow(one - lt.softcap_power);
                }
                gain = gain * lt.direct_mult;
                gain.floor().max(Decimal::zero())
            }
            PrestigeType::Custom => Decimal::zero(),
            PrestigeType::None => Decimal::zero(),
        }
    }

    pub fn get_next_at(&self, layer: &str, can_max: bool) -> Decimal {
        let def = self.layer_def(layer);
        let lt = self.ltmp(layer);
        let custom_next = def.get_next_at.clone();
        if let Some(f) = custom_next {
            return f(self, if can_max { 1 } else { 0 });
        }
        if lt.prestige_type == PrestigeType::None {
            return Decimal::inf();
        }
        if lt.gain_mult <= Decimal::zero() {
            return Decimal::inf();
        }
        if lt.gain_exp <= Decimal::zero() {
            return Decimal::inf();
        }
        let one = Decimal::one();
        match lt.prestige_type {
            PrestigeType::Static => {
                let can_max = can_max && lt.can_buy_max.unwrap_or(false);
                let mut amount = self.lstate(layer).points;
                if can_max && lt.base_amount >= lt.next_at {
                    amount = amount + lt.reset_gain;
                }
                amount = amount / lt.direct_mult;
                let extra_cost = lt.base.pow(amount.pow(lt.exponent) / lt.gain_exp)
                    * lt.gain_mult;
                let mut cost = (extra_cost * lt.requires).max(lt.requires);
                if lt.round_up_cost {
                    cost = cost.ceil();
                }
                cost
            }
            PrestigeType::Normal => {
                let mut next = (lt.reset_gain + one) / lt.direct_mult;
                if next >= lt.softcap {
                    next = (next / lt.softcap.pow(one - lt.softcap_power))
                        .pow(one / lt.softcap_power);
                }
                next = next
                    .root(lt.gain_exp)
                    / lt.gain_mult;
                next = next.root(lt.exponent) * lt.requires;
                next = next.max(lt.requires);
                if lt.round_up_cost {
                    next = next.ceil();
                }
                next
            }
            PrestigeType::Custom => match &def.get_next_at {
                Some(f) => f(self, if can_max { 1 } else { 0 }),
                None => Decimal::zero(),
            },
            PrestigeType::None => Decimal::zero(),
        }
    }

    /// canReset: def override, or the default per-type checks.
    pub fn get_can_reset(&self, layer: &str) -> bool {
        let def = self.layer_def(layer);
        let lt = self.ltmp(layer);
        if let Some(f) = &def.can_reset {
            return f(self);
        }
        match lt.prestige_type {
            PrestigeType::Normal => lt.base_amount >= lt.requires,
            PrestigeType::Static => lt.base_amount >= lt.next_at,
            _ => false,
        }
    }

    // -----------------------------------------------------------------------
    // Reset mechanics
    // -----------------------------------------------------------------------

    pub fn add_points(&mut self, layer: &str, gain: Decimal) {
        let s = self.lstate_mut(layer);
        s.points = (s.points + gain).max(Decimal::zero());
        s.best = s.best.max(s.points);
        s.total = s.total + gain;
    }

    pub fn generate_points(&mut self, layer: &str, diff: f64) {
        let gain = self.ltmp(layer).reset_gain * Decimal::from_finite(diff);
        self.add_points(layer, gain);
    }

    /// doReset — the big one. `force` skips the gain checks (challenge exit).
    pub fn do_reset(&mut self, layer: &str, force: bool) {
        if self.ltmp(layer).prestige_type == PrestigeType::None && !force {
            return;
        }
        let row = self.ltmp(layer).row;
        if !force {
            let lt = self.ltmp(layer);
            if !lt.can_reset {
                return;
            }
            if lt.base_amount < lt.requires {
                return;
            }
            let mut gain = lt.reset_gain;
            if lt.prestige_type == PrestigeType::Static {
                if lt.base_amount < lt.next_at {
                    return;
                }
                gain = if lt.can_buy_max.unwrap_or(false) { gain } else { Decimal::one() };
            }

            let on_prestige = self.layer_def(layer).on_prestige.clone();
            if let Some(f) = on_prestige {
                self.update_milestones(layer);
                f(self, crate::defs::Arg::Dec(gain));
            }

            self.add_points(layer, gain);
            self.update_milestones(layer);
            self.update_achievements(layer);

            if !self.lstate(layer).unlocked {
                self.lstate_mut(layer).unlocked = true;
                self.out.canvas_dirty = true;
                let order = self.layer_def(layer).increase_unlock_order.clone();
                for other in order {
                    if let Some(s) = self.player.layers.get_mut(other) {
                        if !s.unlocked {
                            s.unlock_order += 1;
                        }
                    }
                }
            }
        }

        if (self.layer_def(layer).resets_nothing)(self) {
            return;
        }
        // (TMT's quick fix: baseAmount is recomputed from scratch next tick
        // because the base resource is reset below.)

        // Exit challenges in every layer at or below the resetting row.
        let layer_ids: Vec<String> = self.defs.layers.iter().map(|l| l.id.to_string()).collect();
        for other in &layer_ids {
            let other_row = self.ltmp(other).row;
            if row.is_number()
                && other_row.is_number()
                && row.number() >= other_row.number()
                && (!force || other != layer)
            {
                self.complete_challenge(other);
            }
        }

        self.player.points = if row == Row::Tree(0) {
            Decimal::zero()
        } else {
            self.get_start_points()
        };

        // rowReset from this row down to 0.
        for x in (0..=row.number()).rev() {
            self.row_reset(x, layer);
        }
        for other in &layer_ids {
            let r = self.ltmp(other).row;
            if !r.is_number() {
                self.row_reset_special(other, layer);
            }
        }

        self.lstate_mut(layer).reset_time = 0.0;

        self.update_tmp();
        self.update_tmp();
    }

    /// rowReset for a numeric row.
    fn row_reset(&mut self, row: i64, resetting_layer: &str) {
        let layers_in_row: Vec<String> = self
            .defs
            .layers
            .iter()
            .filter(|l| l.row == Row::Tree(row))
            .map(|l| l.id.to_string())
            .collect();
        for lr in layers_in_row {
            self.reset_one_layer(&lr, resetting_layer, row);
        }
    }

    fn row_reset_special(&mut self, lr: &str, resetting_layer: &str) {
        // TMT compares with isNaN(row) here, so side rows never clear
        // activeChallenge and never pass a numeric row.
        self.reset_one_layer(lr, resetting_layer, -1);
    }

    fn reset_one_layer(&mut self, lr: &str, resetting_layer: &str, row: i64) {
        let has_do_reset = self.layer_def(lr).do_reset.is_some();
        if has_do_reset {
            // Exit any challenge in that layer on an equal-or-higher row reset.
            if row >= 0 {
                self.lstate_mut(lr).active_challenge = None;
            }
            let do_reset = self.layer_def(lr).do_reset.clone();
            if let Some(f) = do_reset {
                f(self, crate::defs::Arg::Str(resetting_layer.to_string()));
            }
        } else if row >= 0 && self.ltmp(resetting_layer).row.number() > self.ltmp(lr).row.number() {
            // TMT guards this with `!isNaN(row)`: side layers (row "side"/
            // "otherside") are never layerDataReset by a prestige cascade.
            layer_data_reset(&mut self.player, &self.defs, lr, &[]);
        }
    }

    // -----------------------------------------------------------------------
    // Challenges
    // -----------------------------------------------------------------------

    pub fn start_challenge(&mut self, layer: &str, x: i64) {
        let mut enter = false;
        {
            let lt = self.ltmp(layer);
            let challenge_unlocked = lt.challenges.get(&x).map(|c| c.unlocked).unwrap_or(false);
            if !self.lstate(layer).unlocked || !challenge_unlocked || !self.can_enter_challenge(layer, x) {
                return;
            }
        }

        if self.lstate(layer).active_challenge == Some(x) {
            if self.can_exit_challenge(layer, x) {
                self.complete_challenge(layer);
                self.lstate_mut(layer).active_challenge = None;
            }
        } else {
            enter = true;
        }
        if enter || self.can_exit_challenge(layer, x) {
            self.do_reset(layer, true);
        }
        if enter {
            self.lstate_mut(layer).active_challenge = Some(x);
            let on_enter = self.layer_def(layer).challenges.iter().find(|c| c.id == x).unwrap().on_enter.clone();
            if let Some(f) = on_enter {
                f(self);
            }
        }
        // updateChallengeTemp equivalent: challenges are resolved every tick.
    }

    pub fn can_enter_challenge(&self, layer: &str, x: i64) -> bool {
        self.layer_def(layer)
            .challenges
            .iter()
            .find(|c| c.id == x)
            .map(|c| (c.can_enter)(self))
            .unwrap_or(true)
    }

    pub fn can_exit_challenge(&self, layer: &str, x: i64) -> bool {
        self.layer_def(layer)
            .challenges
            .iter()
            .find(|c| c.id == x)
            .map(|c| (c.can_exit)(self))
            .unwrap_or(true)
    }

    pub fn can_complete_challenge(&self, layer: &str, x: i64) -> bool {
        if self.lstate(layer).active_challenge != Some(x) {
            return false;
        }
        let def = self
            .layer_def(layer)
            .challenges
            .iter()
            .find(|c| c.id == x)
            .expect("unknown challenge");
        if let Some(f) = &def.can_complete {
            return f(self);
        }
        let goal = def.goal;
        match &def.goal_currency {
            Some(cur) => self.currency_amount(cur) >= goal,
            None => self.player.points >= goal,
        }
    }

    /// Read the amount held in a Currency (player side only).
    pub fn currency_amount(&self, currency: &Currency) -> Decimal {
        match currency {
            Currency::LayerPoints => Decimal::zero(), // caller resolves with layer
            Currency::PlayerField(name) => match *name {
                "points" => self.player.points,
                _ => self.player.extra_dec(name),
            },
            Currency::LayerField { layer, name } => {
                if *name == "points" {
                    self.lstate(layer).points
                } else {
                    self.lstate(layer).extra_dec(name)
                }
            }
            Currency::Buyable { layer, id } => self.get_buyable_amount(layer, id),
        }
    }

    pub fn complete_challenge(&mut self, layer: &str) {
        let x = self.lstate(layer).active_challenge;
        let x = match x {
            Some(x) => x,
            None => return,
        };
        let completions = self.can_complete_challenge(layer, x);
        if !completions {
            self.lstate_mut(layer).active_challenge = None;
            let on_exit = self.layer_def(layer).challenges.iter().find(|c| c.id == x).unwrap().on_exit.clone();
            if let Some(f) = on_exit {
                f(self);
            }
            return;
        }
        let limit = self
            .layer_def(layer)
            .challenges
            .iter()
            .find(|c| c.id == x)
            .map(|c| c.completion_limit)
            .unwrap_or(1);
        let current = self.challenge_completions(layer, x);
        if current < limit {
            self.out.canvas_dirty = true;
            let new_amount = (current + 1).min(limit);
            self.lstate_mut(layer)
                .challenges
                .insert(x.to_string(), new_amount);
            let on_complete = self.layer_def(layer).challenges.iter().find(|c| c.id == x).unwrap().on_complete.clone();
            if let Some(f) = on_complete {
                f(self);
            }
        }
        self.lstate_mut(layer).active_challenge = None;
        let on_exit = self.layer_def(layer).challenges.iter().find(|c| c.id == x).unwrap().on_exit.clone();
        if let Some(f) = on_exit {
            f(self);
        }
    }

    // -----------------------------------------------------------------------
    // Milestones & achievements
    // -----------------------------------------------------------------------

    pub fn update_milestones(&mut self, layer: &str) {
        if self.ltmp(layer).deactivated {
            return;
        }
        let ids: Vec<i64> = self.layer_def(layer).milestones.iter().map(|m| m.id).collect();
        for id in ids {
            if !self.has_milestone(layer, id) {
                let done = (self.layer_def(layer).milestones.iter().find(|m| m.id == id).unwrap().done)(self);
                if done {
                    self.lstate_mut(layer).milestones.push(id);
                    let (on_complete, requirement) = {
                        let def = self.layer_def(layer).milestones.iter().find(|m| m.id == id).unwrap();
                        (def.on_complete.clone(), def.requirement_description.clone())
                    };
                    if let Some(f) = on_complete {
                        f(self);
                    }
                    let popups_on = self.layer_def(layer).milestone_popups && !self.options.hide_milestone_popups;
                    if popups_on {
                        let color = self.ltmp(layer).color.clone();
                        self.do_popup("milestone", requirement, "Milestone Gotten!".to_string(), color);
                    }
                    self.lstate_mut(layer).last_milestone = Some(id);
                }
            }
        }
    }

    pub fn update_achievements(&mut self, layer: &str) {
        if self.ltmp(layer).deactivated {
            return;
        }
        let ids: Vec<i64> = self.layer_def(layer).achievements.iter().map(|a| a.id).collect();
        for id in ids {
            if !self.has_achievement(layer, id) {
                let done = (self.layer_def(layer).achievements.iter().find(|a| a.id == id).unwrap().done)(self);
                if done {
                    self.lstate_mut(layer).achievements.push(id);
                    let (on_complete, name) = {
                        let def = self.layer_def(layer).achievements.iter().find(|a| a.id == id).unwrap();
                        (def.on_complete.clone(), def.name.clone())
                    };
                    if let Some(f) = on_complete {
                        f(self);
                    }
                    if self.layer_def(layer).achievement_popups {
                        let color = self.ltmp(layer).color.clone();
                        self.do_popup("achievement", name, "Achievement Unlocked!".to_string(), color);
                    }
                }
            }
        }
    }

    /// milestoneShown (options.msDisplay filter).
    pub fn milestone_shown(&self, layer: &str, id: i64) -> bool {
        let complete = self.has_milestone(layer, id);
        let auto = !self
            .layer_def(layer)
            .milestones
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.toggles.is_empty())
            .unwrap_or(true);
        match self.options.ms_display {
            crate::game::MsDisplay::Always => true,
            crate::game::MsDisplay::Last => auto || !complete || self.lstate(layer).last_milestone == Some(id),
            crate::game::MsDisplay::Automation => auto || !complete,
            crate::game::MsDisplay::Incomplete => !complete,
            crate::game::MsDisplay::Never => false,
        }
    }

    // -----------------------------------------------------------------------
    // Hard reset
    // -----------------------------------------------------------------------

    pub fn hard_reset(&mut self) {
        self.player = get_start_player(&self.defs);
        self.player.time = self.last_time;
        self.tmp = crate::game::Tmp::default();
        self.update_tmp();
        self.update_tmp();
        self.out.full_rerender = true;
        self.out.canvas_dirty = true;
    }
}
