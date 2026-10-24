//! Purchase/click logic — port of the big-feature half of js/utils.js.

use super::Game;
use crate::defs::Currency;
use crate::format::Decimal;
use crate::player::PVal;

impl Game {
    /// canAffordPurchase: check the right currency without spending it.
    pub fn can_afford_purchase(&self, layer: &str, currency: Option<&Currency>, cost: Decimal) -> bool {
        match currency {
            None => !(self.lstate(layer).points < cost),
            Some(cur) => {
                let amount = match cur {
                    Currency::LayerPoints => self.lstate(layer).points,
                    other => self.currency_amount(other),
                };
                !(amount < cost)
            }
        }
    }

    /// Spend from the right currency (buyUpg payment path).
    fn pay_purchase(&mut self, layer: &str, currency: Option<&Currency>, cost: Decimal) {
        match currency {
            None => {
                let s = self.lstate_mut(layer);
                s.points = s.points - cost;
            }
            Some(cur) => match cur {
                Currency::LayerPoints => {
                    let s = self.lstate_mut(layer);
                    s.points = s.points - cost;
                }
                Currency::PlayerField(name) => match *name {
                    "points" => self.player.points = self.player.points - cost,
                    other => {
                        let v = self.player.extra_dec(other);
                        self.player.extra.insert(other.to_string(), PVal::Dec(v - cost));
                    }
                },
                Currency::LayerField { layer, name } => {
                    if *name == "points" {
                        let s = self.lstate_mut(layer);
                        s.points = s.points - cost;
                    } else {
                        let v = self.lstate(layer).extra_dec(name);
                        self.lstate_mut(layer)
                            .extra
                            .insert(name.to_string(), PVal::Dec(v - cost));
                    }
                }
                Currency::Buyable { layer, id } => {
                    let amount = self.get_buyable_amount(layer, id);
                    self.set_buyable_amount(layer, id, amount - cost);
                }
            },
        }
    }

    pub fn can_afford_upgrade(&self, layer: &str, id: i64) -> bool {
        let lt = self.ltmp(layer);
        if lt.deactivated {
            return false;
        }
        let upg = match lt.upgrades.get(&id) {
            Some(u) => u,
            None => return false,
        };
        let def = self
            .layer_def(layer)
            .upgrades
            .iter()
            .find(|u| u.id == id)
            .expect("upgrade def missing");
        if let Some(f) = &def.can_afford {
            if !f(self) {
                return false;
            }
        }
        self.can_afford_purchase(layer, def.currency.as_ref(), upg.cost)
    }

    /// buyUpg.
    pub fn buy_upg(&mut self, layer: &str, id: i64) {
        let lt = self.ltmp(layer);
        if !lt.upgrades.contains_key(&id) {
            return;
        }
        if !self.lstate(layer).unlocked || lt.deactivated {
            return;
        }
        if !lt.upgrades.get(&id).map(|u| u.unlocked).unwrap_or(false) {
            return;
        }
        if self.lstate(layer).upgrades.contains(&id) {
            return;
        }
        let (can_afford, pay, on_purchase, currency) = {
            let def = self
                .layer_def(layer)
                .upgrades
                .iter()
                .find(|u| u.id == id)
                .expect("upgrade def missing");
            (
                def.can_afford.clone(),
                def.pay.clone(),
                def.on_purchase.clone(),
                def.currency.clone(),
            )
        };
        if let Some(f) = &can_afford {
            if !f(self) {
                return;
            }
        }
        if let Some(pay) = pay {
            pay(self);
        } else {
            let cost = self.ltmp(layer).upgrades.get(&id).unwrap().cost;
            if !self.can_afford_purchase(layer, currency.as_ref(), cost) {
                return;
            }
            self.pay_purchase(layer, currency.as_ref(), cost);
        }
        self.lstate_mut(layer).upgrades.push(id);
        if let Some(f) = on_purchase {
            f(self);
        }
        self.out.canvas_dirty = true;
    }

    /// buyBuyable.
    pub fn buy_buyable(&mut self, layer: &str, id: &str) {
        if !self.lstate(layer).unlocked {
            return;
        }
        let bt = self.ltmp(layer).buyables.get(id).cloned();
        let bt = match bt {
            Some(b) if b.unlocked && b.can_buy => b,
            _ => return,
        };
        let buy = self
            .layer_def(layer)
            .buyables
            .items
            .iter()
            .find(|b| b.id == id)
            .and_then(|b| b.buy.clone());
        if let Some(buy) = buy {
            buy(self);
        } else {
            // Default buy: pay the current cost, gain one.
            let s = self.lstate_mut(layer);
            s.points = s.points - bt.cost;
            let amount = s.buyables.get(id).copied().unwrap_or_else(Decimal::zero);
            s.buyables.insert(id.to_string(), amount + Decimal::one());
            s.spent_on_buyables = s.spent_on_buyables + bt.cost;
        }
    }

    /// buyMaxBuyable (only when the def supplies its own buyMax).
    pub fn buy_max_buyable(&mut self, layer: &str, id: &str) {
        if !self.lstate(layer).unlocked {
            return;
        }
        let bt = self.ltmp(layer).buyables.get(id).cloned();
        if !matches!(bt, Some(b) if b.unlocked && b.can_buy) {
            return;
        }
        let buy_max = self
            .layer_def(layer)
            .buyables
            .items
            .iter()
            .find(|b| b.id == id)
            .and_then(|b| b.buy_max.clone());
        if let Some(f) = buy_max {
            f(self);
        }
    }

    pub fn can_buy_buyable(&self, layer: &str, id: &str) -> bool {
        match self.ltmp(layer).buyables.get(id) {
            Some(b) => b.unlocked && b.can_buy && !b.at_limit,
            None => false,
        }
    }

    /// clickClickable.
    pub fn click_clickable(&mut self, layer: &str, id: &str) {
        if !self.lstate(layer).unlocked || self.ltmp(layer).deactivated {
            return;
        }
        let ct = self.ltmp(layer).clickables.get(id).cloned();
        if !matches!(ct, Some(c) if c.unlocked && c.can_click) {
            return;
        }
        let on_click = self
            .layer_def(layer)
            .clickables
            .items
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.on_click.clone());
        if let Some(f) = on_click {
            f(self);
        }
    }

    /// Hold-to-click support (mousedown repeat) — same guard as a click.
    pub fn hold_clickable(&mut self, layer: &str, id: &str) {
        let has_hold = self
            .layer_def(layer)
            .clickables
            .items
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.on_hold.clone());
        if let Some(f) = has_hold {
            if let Some(c) = self.ltmp(layer).clickables.get(id) {
                if c.unlocked && c.can_click {
                    f(self);
                }
            }
        }
    }

    pub fn hold_buyable(&mut self, layer: &str, id: &str) {
        // TMT repeats buyBuyable while held; there is no separate onHold.
        self.buy_buyable(layer, id);
    }

    /// clickGrid.
    pub fn click_grid(&mut self, layer: &str, id: i64) {
        if !self.lstate(layer).unlocked || self.ltmp(layer).deactivated {
            return;
        }
        let (get_unlocked, get_can_click, on_click) = {
            let grid = match &self.layer_def(layer).grid {
                Some(g) => g,
                None => return,
            };
            (
                grid.get_unlocked.clone(),
                grid.get_can_click.clone(),
                grid.on_click.clone(),
            )
        };
        if !(get_unlocked)(self, id) {
            return;
        }
        if !(get_can_click)(self, id) {
            return;
        }
        if let Some(f) = on_click {
            f(self, crate::defs::Arg::Num(id as f64));
        }
    }

    pub fn hold_grid(&mut self, layer: &str, id: i64) {
        if !self.lstate(layer).unlocked || self.ltmp(layer).deactivated {
            return;
        }
        let (get_unlocked, get_can_click, on_hold) = {
            let grid = match &self.layer_def(layer).grid {
                Some(g) => g,
                None => return,
            };
            (
                grid.get_unlocked.clone(),
                grid.get_can_click.clone(),
                grid.on_hold.clone(),
            )
        };
        if let Some(f) = on_hold {
            if (get_unlocked)(self, id) && (get_can_click)(self, id) {
                f(self, crate::defs::Arg::Num(id as f64));
            }
        }
    }

    /// respecBuyables.
    pub fn respec_buyables(&mut self, layer: &str) {
        let respec = self.layer_def(layer).buyables.respec.clone();
        if let Some(f) = respec {
            f(self);
        }
    }

    /// sellOne / sellAll buttons.
    pub fn sell_one(&mut self, layer: &str, id: &str) {
        let f = self
            .layer_def(layer)
            .buyables
            .items
            .iter()
            .find(|b| b.id == id)
            .and_then(|b| b.sell_one.clone());
        if let Some(f) = f {
            f(self);
        }
    }

    pub fn sell_all(&mut self, layer: &str, id: &str) {
        let f = self
            .layer_def(layer)
            .buyables
            .items
            .iter()
            .find(|b| b.id == id)
            .and_then(|b| b.sell_all.clone());
        if let Some(f) = f {
            f(self);
        }
    }

    /// autobuyUpgrades.
    pub fn autobuy_upgrades(&mut self, layer: &str) {
        let ids: Vec<i64> = self.layer_def(layer).upgrades.iter().map(|u| u.id).collect();
        for id in ids {
            let custom = self
                .layer_def(layer)
                .upgrades
                .iter()
                .find(|u| u.id == id)
                .and_then(|u| u.can_afford.clone());
            let allowed = match custom {
                Some(f) => f(self),
                None => true,
            };
            if allowed {
                self.buy_upg(layer, id);
            }
        }
    }

    /// toggleAuto (milestone toggles and the `toggle` component).
    pub fn toggle_auto(&mut self, layer: &str, field: &str) {
        if let Some(s) = self.player.layers.get_mut(layer) {
            if let Some(v) = s.extra.get_mut(field) {
                *v = PVal::Bool(!v.as_bool());
            } else {
                s.extra.insert(field.to_string(), PVal::Bool(true));
            }
        }
        self.out.canvas_dirty = true;
    }

    /// text-input / slider / drop-down value setting (toValue semantics).
    pub fn set_player_value(&mut self, layer: &str, field: &str, text: &str) {
        let old = self
            .player
            .layers
            .get(layer)
            .and_then(|s| s.extra.get(field))
            .cloned();
        let new_val = match old {
            Some(PVal::Dec(_)) => PVal::Dec(crate::format::dec(text)),
            Some(PVal::Num(_)) | Some(PVal::Bool(_)) | Some(PVal::Str(_)) | None => {
                if let Ok(n) = text.trim().parse::<f64>() {
                    PVal::Num(n)
                } else {
                    PVal::Str(text.to_string())
                }
            }
        };
        if let Some(s) = self.player.layers.get_mut(layer) {
            s.extra.insert(field.to_string(), new_val);
        }
    }
}
