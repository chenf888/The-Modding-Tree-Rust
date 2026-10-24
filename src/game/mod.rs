//! The engine: game state container and the per-tick tmp cache
//! (Rust equivalent of js/technical/temp.js).

pub mod buy;
pub mod events;
pub mod reset;
pub mod tick;

use crate::defs::*;
use crate::format::Decimal;
use crate::player::{get_start_player, LayerState, Player};

pub use crate::player::PVal;
use std::collections::HashMap;

/// Options (TMT getStartOptions + toggles).
#[derive(Clone, Debug)]
pub struct Options {
    pub autosave: bool,
    pub ms_display: MsDisplay,
    pub theme: &'static str,
    pub hq_tree: bool,
    pub offline_prod: bool,
    pub hide_challenges: bool,
    pub force_one_tab: bool,
    pub force_tooltips: bool,
    pub hide_milestone_popups: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            autosave: true,
            ms_display: MsDisplay::Always,
            theme: "default",
            hq_tree: false,
            offline_prod: true,
            hide_challenges: false,
            force_one_tab: false,
            force_tooltips: true,
            hide_milestone_popups: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MsDisplay {
    Always,
    Last,
    Automation,
    Incomplete,
    Never,
}

impl MsDisplay {
    pub fn next(self) -> MsDisplay {
        match self {
            MsDisplay::Always => MsDisplay::Last,
            MsDisplay::Last => MsDisplay::Automation,
            MsDisplay::Automation => MsDisplay::Incomplete,
            MsDisplay::Incomplete => MsDisplay::Never,
            MsDisplay::Never => MsDisplay::Always,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            MsDisplay::Always => "always",
            MsDisplay::Last => "last",
            MsDisplay::Automation => "automation",
            MsDisplay::Incomplete => "incomplete",
            MsDisplay::Never => "never",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            MsDisplay::Always => "ALL",
            MsDisplay::Last => "LAST, AUTO, INCOMPLETE",
            MsDisplay::Automation => "AUTOMATION, INCOMPLETE",
            MsDisplay::Incomplete => "INCOMPLETE",
            MsDisplay::Never => "NONE",
        }
    }
}

/// One resolved upgrade/challenge/achievement/milestone entry in tmp.
#[derive(Clone, Debug, Default)]
pub struct CompTmp {
    pub unlocked: bool,
    pub cost: Decimal,
    pub effect: Option<EffectVal>,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ChallengeTmp {
    pub unlocked: bool,
    pub goal: Decimal,
    pub reward_effect: Option<EffectVal>,
    pub counts_as: Vec<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct BuyableTmp {
    pub unlocked: bool,
    pub cost: Decimal,
    pub effect: EffectVal,
    pub can_buy: bool,
    pub at_limit: bool,
    pub marked: bool,
    pub has_sell_one: bool,
    pub has_sell_all: bool,
}

#[derive(Clone, Debug)]
pub struct ClickableTmp {
    pub unlocked: bool,
    pub can_click: bool,
    pub marked: bool,
}

#[derive(Clone, Debug)]
pub struct BarTmp {
    pub unlocked: bool,
    pub progress: f64,
}

#[derive(Clone, Debug)]
pub struct MilestoneTmp {
    pub unlocked: bool,
}

/// tmp[layer] — everything the original computes by reflection each tick.
#[derive(Clone, Debug)]
pub struct LayerTmp {
    pub is_layer: bool,
    pub name: String,
    pub symbol: String,
    pub color: String,
    pub row: Row,
    pub display_row: Row,

    pub prestige_type: PrestigeType,
    pub resource: String,
    pub base_resource: String,
    pub base_amount: Decimal,
    pub requires: Decimal,
    pub exponent: Decimal,
    pub base: Decimal,
    pub round_up_cost: bool,
    pub gain_mult: Decimal,
    pub gain_exp: Decimal,
    pub direct_mult: Decimal,
    pub softcap: Decimal,
    pub softcap_power: Decimal,
    pub passive_generation: Option<Decimal>,
    pub can_buy_max: Option<bool>,
    pub reset_gain: Decimal,
    pub next_at: Decimal,
    pub next_at_disp: Decimal,
    pub can_reset: bool,
    pub resets_nothing: bool,
    pub auto_prestige: bool,
    pub auto_upgrade: bool,
    pub deactivated: bool,

    pub layer_shown: LayerShown,
    pub node_style: String,
    pub image: Option<String>,
    pub tooltip: Option<String>,
    pub tooltip_locked: Option<String>,
    pub marked: Option<String>,
    pub glow_color: String,
    pub true_glow: String,
    pub notify: bool,
    pub prestige_notify: bool,
    pub can_click: bool,
    pub style: String,
    pub show_best: bool,
    pub show_total: bool,
    pub effect: Option<EffectVal>,
    pub effect_description: String,

    pub upgrade_rows: i64,
    pub upgrade_cols: i64,
    pub challenge_rows: i64,
    pub challenge_cols: i64,
    pub achievement_rows: i64,
    pub achievement_cols: i64,
    pub buyable_rows: i64,
    pub buyable_cols: i64,
    pub clickable_rows: i64,
    pub clickable_cols: i64,

    pub upgrades: HashMap<i64, CompTmp>,
    pub challenges: HashMap<i64, ChallengeTmp>,
    pub achievements: HashMap<i64, CompTmp>,
    pub milestones: HashMap<i64, MilestoneTmp>,
    pub buyables: HashMap<String, BuyableTmp>,
    pub clickables: HashMap<String, ClickableTmp>,
    pub bars: HashMap<String, BarTmp>,
    pub infoboxes: HashMap<String, bool>,
}

impl Default for LayerTmp {
    fn default() -> Self {
        LayerTmp {
            is_layer: true,
            name: String::new(),
            symbol: String::new(),
            color: "#ffffff".to_string(),
            row: Row::Tree(0),
            display_row: Row::Tree(0),
            prestige_type: PrestigeType::None,
            resource: String::new(),
            base_resource: String::new(),
            base_amount: Decimal::zero(),
            requires: Decimal::zero(),
            exponent: Decimal::one(),
            base: Decimal::from(2),
            round_up_cost: false,
            gain_mult: Decimal::one(),
            gain_exp: Decimal::one(),
            direct_mult: Decimal::one(),
            softcap: Decimal::zero(),
            softcap_power: Decimal::one(),
            passive_generation: None,
            can_buy_max: None,
            reset_gain: Decimal::zero(),
            next_at: Decimal::zero(),
            next_at_disp: Decimal::zero(),
            can_reset: false,
            resets_nothing: false,
            auto_prestige: false,
            auto_upgrade: false,
            deactivated: false,
            layer_shown: LayerShown::Shown,
            node_style: String::new(),
            image: None,
            tooltip: None,
            tooltip_locked: None,
            marked: None,
            glow_color: "#ff0000".to_string(),
            true_glow: "#ff0000".to_string(),
            notify: false,
            prestige_notify: false,
            can_click: false,
            style: String::new(),
            show_best: false,
            show_total: false,
            effect: None,
            effect_description: String::new(),
            upgrade_rows: 0,
            upgrade_cols: 0,
            challenge_rows: 0,
            challenge_cols: 0,
            achievement_rows: 0,
            achievement_cols: 0,
            buyable_rows: 0,
            buyable_cols: 0,
            clickable_rows: 0,
            clickable_cols: 0,
            upgrades: HashMap::new(),
            challenges: HashMap::new(),
            achievements: HashMap::new(),
            milestones: HashMap::new(),
            buyables: HashMap::new(),
            clickables: HashMap::new(),
            bars: HashMap::new(),
            infoboxes: HashMap::new(),
        }
    }
}

/// The global tmp object.
pub struct Tmp {
    pub point_gen: Decimal,
    pub game_ended: bool,
    pub display_things: Vec<String>,
    pub background_style: String,
    pub oomps: Decimal,
    pub oomps_mag: i32,
    pub last_points: Decimal,
    pub split_screen: bool,
    pub scrolled: bool,
    pub screen_width: f64,
    pub screen_height: f64,
    pub layers: HashMap<String, LayerTmp>,
}

impl Default for Tmp {
    fn default() -> Self {
        Tmp {
            point_gen: Decimal::zero(),
            game_ended: false,
            display_things: Vec::new(),
            background_style: String::new(),
            oomps: Decimal::zero(),
            oomps_mag: 0,
            last_points: Decimal::zero(),
            split_screen: true,
            scrolled: false,
            screen_width: 1024.0,
            screen_height: 768.0,
            layers: HashMap::new(),
        }
    }
}

/// The whole game.
pub struct Game {
    pub defs: GameDefs,
    pub player: Player,
    pub options: Options,
    pub tmp: Tmp,
    /// Output queue consumed by the FFI layer each call.
    pub out: OutQueue,
    /// Milliseconds at the last tick.
    pub last_time: f64,
    pub last_saved: f64,
    pub rng: u64,
    /// Whether the game has run its first updateTemp.
    pub warmed_up: bool,
    /// Mouse position, fed by the page for particle spawns.
    pub mouse_x: f64,
    pub mouse_y: f64,
    /// Previous UI tree, kept for the DOM diff.
    pub ui_tree: Option<crate::render::UiNode>,
}

/// Things the engine wants to tell the page between calls.
#[derive(Default)]
pub struct OutQueue {
    pub popups: Vec<(String, String, &'static str, String)>,
    /// Particle spawn requests: (kind, text-or-image, x, y, count, spec json).
    pub particles: Vec<ParticleSpawn>,
    pub canvas_dirty: bool,
    pub full_rerender: bool,
    /// Serialized player JSON when a save is due (autosave).
    pub pending_save: Option<String>,
    pub title: Option<String>,
    pub nan_alert: Option<String>,
}

pub struct ParticleSpawn {
    pub kind: &'static str,
    pub content: String,
    pub x: f64,
    pub y: f64,
    pub count: i64,
    pub spec: String,
}

impl Game {
    pub fn new(defs: GameDefs) -> Game {
        let player = get_start_player(&defs);
        Game {
            defs,
            player,
            options: Options::default(),
            tmp: Tmp::default(),
            out: OutQueue::default(),
            last_time: 0.0,
            last_saved: 0.0,
            rng: 0x2545F4914F6CDD1D,
            warmed_up: false,
            mouse_x: 0.0,
            mouse_y: 0.0,
            ui_tree: None,
        }
    }

    // ----- small random generator (wasm has no rand crate here) -----

    pub fn rand_f64(&mut self) -> f64 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        let v = x.wrapping_mul(0x2545F4914F6CDD1D);
        ((v >> 11) as f64) / ((1u64 << 53) as f64)
    }

    // ----- accessors -----

    pub fn layer_def(&self, id: &str) -> &LayerDef {
        self.defs.layer(id)
    }

    pub fn lstate(&self, id: &str) -> &LayerState {
        self.player.layers.get(id).expect("layer state missing")
    }

    pub fn lstate_mut(&mut self, id: &str) -> &mut LayerState {
        self.player.layers.get_mut(id).expect("layer state missing")
    }

    pub fn ltmp(&self, id: &str) -> &LayerTmp {
        self.tmp.layers.get(id).expect("layer tmp missing")
    }

    pub fn get_start_points(&self) -> Decimal {
        self.defs.mod_info.initial_start_points
    }

    // ----- TMT-style helpers the game code calls -----

    pub fn has_upgrade(&self, layer: &str, id: i64) -> bool {
        (self.lstate(layer).upgrades.contains(&id)) && !self.ltmp(layer).deactivated
    }

    pub fn has_milestone(&self, layer: &str, id: i64) -> bool {
        (self.lstate(layer).milestones.contains(&id)) && !self.ltmp(layer).deactivated
    }

    pub fn has_achievement(&self, layer: &str, id: i64) -> bool {
        (self.lstate(layer).achievements.contains(&id)) && !self.ltmp(layer).deactivated
    }

    pub fn has_challenge(&self, layer: &str, id: i64) -> bool {
        (self.lstate(layer).challenges.get(&id.to_string()).copied().unwrap_or(0) > 0)
            && !self.ltmp(layer).deactivated
    }

    pub fn maxed_challenge(&self, layer: &str, id: i64) -> bool {
        let completions = self.challenge_completions(layer, id);
        completions >= self.layer_def(layer).challenges.iter().find(|c| c.id == id).map(|c| c.completion_limit).unwrap_or(1)
            && !self.ltmp(layer).deactivated
    }

    pub fn challenge_completions(&self, layer: &str, id: i64) -> i64 {
        self.lstate(layer).challenges.get(&id.to_string()).copied().unwrap_or(0)
    }

    pub fn in_challenge(&self, layer: &str, id: i64) -> bool {
        let challenge = self.lstate(layer).active_challenge;
        let current = match challenge {
            Some(c) => c,
            None => return false,
        };
        if current == id {
            return true;
        }
        self.ltmp(layer)
            .challenges
            .get(&current)
            .map(|c| c.counts_as.contains(&id))
            .unwrap_or(false)
    }

    pub fn get_buyable_amount(&self, layer: &str, id: &str) -> Decimal {
        self.lstate(layer).buyables.get(id).copied().unwrap_or_else(Decimal::zero)
    }

    pub fn set_buyable_amount(&mut self, layer: &str, id: &str, amount: Decimal) {
        self.lstate_mut(layer).buyables.insert(id.to_string(), amount);
    }

    pub fn get_clickable_state(&self, layer: &str, id: &str) -> crate::player::PVal {
        self.lstate(layer)
            .clickables
            .get(id)
            .cloned()
            .unwrap_or(crate::player::PVal::Str(String::new()))
    }

    pub fn set_clickable_state(&mut self, layer: &str, id: &str, value: crate::player::PVal) {
        self.lstate_mut(layer).clickables.insert(id.to_string(), value);
    }

    pub fn get_grid_data(&self, layer: &str, id: i64) -> crate::player::PVal {
        self.lstate(layer).grid.get(&id).cloned().unwrap_or(crate::player::PVal::Num(0.0))
    }

    pub fn set_grid_data(&mut self, layer: &str, id: i64, value: crate::player::PVal) {
        self.lstate_mut(layer).grid.insert(id, value);
    }

    pub fn upgrade_effect(&self, layer: &str, id: i64) -> EffectVal {
        self.ltmp(layer)
            .upgrades
            .get(&id)
            .and_then(|u| u.effect.clone())
            .unwrap_or(EffectVal::Dec(Decimal::one()))
    }

    pub fn buyable_effect(&self, layer: &str, id: &str) -> EffectVal {
        self.ltmp(layer)
            .buyables
            .get(id)
            .map(|b| b.effect.clone())
            .unwrap_or(EffectVal::Dec(Decimal::one()))
    }

    pub fn challenge_effect(&self, layer: &str, id: i64) -> EffectVal {
        self.ltmp(layer)
            .challenges
            .get(&id)
            .and_then(|c| c.reward_effect.clone())
            .unwrap_or(EffectVal::Dec(Decimal::one()))
    }

    pub fn achievement_effect(&self, layer: &str, id: i64) -> EffectVal {
        self.ltmp(layer)
            .achievements
            .get(&id)
            .and_then(|a| a.effect.clone())
            .unwrap_or(EffectVal::Dec(Decimal::one()))
    }

    /// TMT layerunlocked.
    pub fn layer_unlocked(&self, id: &str) -> bool {
        let lt = self.ltmp(id);
        if lt.prestige_type == PrestigeType::None {
            return self.lstate(id).unlocked;
        }
        self.defs.layers.iter().any(|l| l.id == id)
            && (self.lstate(id).unlocked || (lt.can_reset && lt.layer_shown == LayerShown::Shown))
    }

    /// TMT softcap helper.
    pub fn softcap(&self, value: Decimal, cap: Decimal, power: Decimal) -> Decimal {
        if value <= cap {
            value
        } else {
            value.pow(power) * cap.pow(Decimal::one() - power)
        }
    }

    pub fn do_popup(&mut self, kind: &'static str, text: String, title: String, color: String) {
        self.out.popups.push((title, text, kind, color));
    }

    /// Spawn a particle burst (minimal port of TMT makeParticles).
    pub fn make_particles(&mut self, kind: &'static str, content: String, count: i64, spec: String) {
        let (x, y) = self.last_mouse();
        self.out.particles.push(ParticleSpawn {
            kind,
            content,
            x,
            y,
            count,
            spec,
        });
    }

    /// Particle spawn position; JS passes the mouse position with each tick.
    pub fn last_mouse(&self) -> (f64, f64) {
        (self.mouse_x, self.mouse_y)
    }

    pub fn set_mouse(&mut self, x: f64, y: f64) {
        self.mouse_x = x;
        self.mouse_y = y;
    }
}

// ---------------------------------------------------------------------------
// updateTemp — resolve every view function into the tmp cache
// ---------------------------------------------------------------------------

impl Game {
    pub fn update_tmp(&mut self) {
        // TMT's setupTemp: make sure every layer has a tmp slot before any
        // callback reads another layer's tmp (stale-but-present beats missing).
        let layer_ids: Vec<String> = self.defs.layers.iter().map(|l| l.id.to_string()).collect();
        for id in &layer_ids {
            self.tmp.layers.entry(id.clone()).or_default();
        }

        // Layer-level views first (each callback sees fully updated player and
        // the *previous* tick's tmp for other layers, same as TMT).
        for i in 0..self.defs.layers.len() {
            let lt = self.resolve_layer_tmp(i);
            let id = self.defs.layers[i].id.to_string();
            self.tmp.layers.insert(id, lt);
        }

        // Derived per-layer values (resetGain, nextAt, canReset, notify...).
        let ids: Vec<String> = self.defs.layers.iter().map(|l| l.id.to_string()).collect();
        for id in &ids {
            let (reset_gain, next_at, next_at_disp) = {
                let this = &*self;
                (
                    this.get_reset_gain(id),
                    this.get_next_at(id, false),
                    this.get_next_at(id, true),
                )
            };
            {
                let lt = self.tmp.layers.get_mut(id).unwrap();
                lt.reset_gain = reset_gain;
                lt.next_at = next_at;
                lt.next_at_disp = next_at_disp;
            }
            let can_reset = self.get_can_reset(id);
            let notify = self.should_notify_layer(id);
            let prestige_notify = self.get_prestige_notify(id);
            let lt = self.tmp.layers.get_mut(id).unwrap();
            lt.can_reset = can_reset;
            lt.notify = notify;
            lt.prestige_notify = prestige_notify;
        }

        // Subtab notify flags ripple up to layer/subtab glow.
        for id in &ids {
            let glow = self.resolve_subtab_glow(id);
            if let Some(color) = glow {
                self.tmp.layers.get_mut(id).unwrap().true_glow = color;
            }
        }

        // Global values.
        let point_gen = (self.defs.get_point_gen)(self);
        let display_things: Vec<String> = self
            .defs
            .display_things
            .iter()
            .map(|f| f(self))
            .collect();
        let background_style = (self.defs.background_style)(self);
        self.tmp.point_gen = point_gen;
        self.tmp.display_things = display_things;
        self.tmp.background_style = background_style;
    }

    /// Evaluate one layer's view functions into a fresh LayerTmp.
    fn resolve_layer_tmp(&mut self, index: usize) -> LayerTmp {
        let def = &self.defs.layers[index];
        let id = def.id;
        let mut lt = LayerTmp::default();

        lt.is_layer = def.is_layer;
        lt.name = def.name.unwrap_or(id).to_string();
        lt.symbol = def
            .symbol
            .map(|s| s.to_string())
            .unwrap_or_else(|| capitalize_first(id));
        lt.color = def.color.to_string();
        lt.row = def.row;
        lt.display_row = def.display_row.unwrap_or(def.row);
        lt.prestige_type = def.prestige_type;
        lt.resource = def.resource.to_string();
        lt.base_resource = def.base_resource.to_string();
        lt.round_up_cost = def.round_up_cost;
        lt.resets_nothing = (def.resets_nothing)(self);
        lt.can_click = match &def.node_can_click {
            Some(f) => f(self),
            None => false,
        };
        lt.auto_prestige = (def.auto_prestige)(self);
        lt.auto_upgrade = (def.auto_upgrade)(self);
        lt.deactivated = (def.deactivated)(self);
        lt.layer_shown = (def.layer_shown)(self);
        lt.node_style = (def.node_style)(self);
        lt.image = def.image.clone();
        lt.tooltip = (def.tooltip)(self);
        lt.tooltip_locked = (def.tooltip_locked)(self);
        lt.marked = (def.marked)(self);
        lt.glow_color = def.glow_color.unwrap_or("#ff0000").to_string();
        lt.style = (def.style)(self);
        lt.effect_description = def
            .effect_description
            .as_ref()
            .map(|f| f(self))
            .unwrap_or_default();
        lt.effect = def.effect.as_ref().map(|f| f(self));
        lt.passive_generation = (def.passive_generation)(self);
        lt.can_buy_max = (def.can_buy_max)(self);
        lt.base_amount = (def.base_amount)(self);
        lt.requires = def.requires.resolve(self);
        lt.exponent = (def.exponent)(self);
        lt.base = (def.base)(self);
        lt.gain_mult = (def.gain_mult)(self);
        lt.gain_exp = (def.gain_exp)(self);
        lt.direct_mult = (def.direct_mult)(self);
        lt.softcap = (def.softcap)(self);
        lt.softcap_power = (def.softcap_power)(self);

        // showBest/showTotal: on if startData mentions them (TMT setupLayer).
        lt.show_best = def.start_data.iter().any(|(k, _)| *k == "best");
        lt.show_total = def.start_data.iter().any(|(k, _)| *k == "total");

        // Upgrades.
        let upg_ids: Vec<i64> = def.upgrades.iter().map(|u| u.id).collect();
        let (rows, cols) = GameDefs::upgrade_rows_cols(&upg_ids);
        lt.upgrade_rows = rows;
        lt.upgrade_cols = cols;
        for u in &def.upgrades {
            let unlocked = (u.unlocked)(self);
            let cost = match &u.cost {
                DecOrFn::Dec(d) => *d,
                DecOrFn::Fn(f) => f(self),
            };
            let effect = u.effect.as_ref().map(|f| f(self));
            let tooltip = (u.tooltip)(self);
            lt.upgrades.insert(
                u.id,
                CompTmp {
                    unlocked,
                    cost,
                    effect,
                    tooltip,
                },
            );
        }

        // Challenges.
        let ch_ids: Vec<i64> = def.challenges.iter().map(|c| c.id).collect();
        let (rows, cols) = GameDefs::upgrade_rows_cols(&ch_ids);
        lt.challenge_rows = rows;
        lt.challenge_cols = cols;
        for c in &def.challenges {
            let unlocked = (c.unlocked)(self);
            let reward_effect = c.reward_effect.as_ref().map(|f| f(self));
            lt.challenges.insert(
                c.id,
                ChallengeTmp {
                    unlocked,
                    goal: c.goal,
                    reward_effect,
                    counts_as: c.counts_as.clone(),
                },
            );
        }

        // Achievements.
        let ach_ids: Vec<i64> = def.achievements.iter().map(|a| a.id).collect();
        let (rows, cols) = GameDefs::upgrade_rows_cols(&ach_ids);
        lt.achievement_rows = rows;
        lt.achievement_cols = cols;
        for a in &def.achievements {
            let unlocked = true; // TMT achievements have no per-item unlocked default fn
            let effect = a.effect.as_ref().map(|f| f(self));
            lt.achievements.insert(
                a.id,
                CompTmp {
                    unlocked,
                    cost: Decimal::zero(),
                    effect,
                    tooltip: None,
                },
            );
        }

        // Milestones.
        for m in &def.milestones {
            let unlocked = (m.unlocked)(self);
            lt.milestones.insert(m.id, MilestoneTmp { unlocked });
        }

        // Buyables: cost/effect at the current amount, canBuy, limit.
        let buy_ids: Vec<&'static str> = def.buyables.items.iter().map(|b| b.id).collect();
        let ids_num: Vec<i64> = buy_ids.iter().filter_map(|s| s.parse().ok()).collect();
        let (rows, cols) = GameDefs::upgrade_rows_cols(&ids_num);
        lt.buyable_rows = rows;
        lt.buyable_cols = cols;
        for b in &def.buyables.items {
            let unlocked = (b.unlocked)(self);
            let amount = self.get_buyable_amount(id, b.id);
            let cost = (b.cost)(self);
            let effect = (b.effect)(self);
            let can_afford = (b.can_afford)(self);
            let at_limit = amount >= b.purchase_limit;
            let marked = (b.marked)(self);
            lt.buyables.insert(
                b.id.to_string(),
                BuyableTmp {
                    unlocked,
                    cost,
                    effect,
                    can_buy: can_afford && !at_limit && !lt.deactivated,
                    at_limit,
                    marked,
                    has_sell_one: b.sell_one.is_some(),
                    has_sell_all: b.sell_all.is_some(),
                },
            );
        }

        // Clickables.
        let click_ids: Vec<&'static str> = def.clickables.items.iter().map(|c| c.id).collect();
        let ids_num: Vec<i64> = click_ids.iter().filter_map(|s| s.parse().ok()).collect();
        let (rows, cols) = GameDefs::upgrade_rows_cols(&ids_num);
        lt.clickable_rows = rows;
        lt.clickable_cols = cols;
        for c in &def.clickables.items {
            let unlocked = (c.unlocked)(self);
            let can_click = (c.can_click)(self) && !lt.deactivated;
            let marked = (c.marked)(self);
            lt.clickables.insert(
                c.id.to_string(),
                ClickableTmp {
                    unlocked,
                    can_click,
                    marked,
                },
            );
        }

        // Bars.
        for bar in &def.bars {
            let unlocked = (bar.unlocked)(self);
            let progress = (bar.progress)(self);
            lt.bars.insert(
                bar.id.to_string(),
                BarTmp { unlocked, progress },
            );
        }

        // Infoboxes (unlocked flags only; open state lives in player).
        for infobox in &def.infoboxes {
            let unlocked = (infobox.unlocked)(self);
            lt.infoboxes.insert(infobox.id.to_string(), unlocked);
        }

        lt
    }

    /// shouldNotify: upgrades affordable, challenge completable, def flag, or
    /// any notifying subtab (TMT shouldNotify).
    fn should_notify_layer(&mut self, id: &str) -> bool {
        for u in &self.layer_def(id).upgrades {
            let ut = self.ltmp(id).upgrades.get(&u.id);
            let unlocked = ut.map(|t| t.unlocked).unwrap_or(true);
            if unlocked && self.can_afford_upgrade(id, u.id) && !self.has_upgrade(id, u.id) {
                return true;
            }
        }
        if let Some(active) = self.lstate(id).active_challenge {
            if self.can_complete_challenge(id, active) {
                return true;
            }
        }
        if (self.layer_def(id).should_notify)(self) {
            return true;
        }
        // Subtab notifications.
        if let Some(TabFormat::Tabs(tabs)) = &self.layer_def(id).tab_format {
            for tab in tabs {
                if self.subtab_should_notify(id, "mainTabs", tab.name) {
                    return true;
                }
            }
        }
        for family in &self.layer_def(id).microtabs {
            for tab in &family.tabs {
                if self.subtab_should_notify(id, family.family, tab.name) {
                    return true;
                }
            }
        }
        false
    }

    /// subtabShouldNotify.
    pub fn subtab_should_notify(&self, layer: &str, family: &str, subtab: &str) -> bool {
        let sub = self.find_subtab(layer, family, subtab);
        let sub = match sub {
            Some(s) => s,
            None => return false,
        };
        if !(sub.unlocked)(self) {
            return false;
        }
        match sub.embed_layer {
            Some(embed) => self.ltmp(embed).notify,
            None => (sub.should_notify)(self),
        }
    }

    /// subtabResetNotify.
    pub fn subtab_reset_notify(&self, layer: &str, family: &str, subtab: &str) -> bool {
        let sub = self.find_subtab(layer, family, subtab);
        let sub = match sub {
            Some(s) => s,
            None => return false,
        };
        match sub.embed_layer {
            Some(embed) => self.ltmp(embed).prestige_notify,
            None => (sub.prestige_notify)(self),
        }
    }

    pub fn find_subtab(&self, layer: &str, family: &str, subtab: &str) -> Option<&SubtabDef> {
        let def = self.layer_def(layer);
        if family == "mainTabs" {
            if let Some(TabFormat::Tabs(tabs)) = &def.tab_format {
                return tabs.iter().find(|t| t.name == subtab);
            }
        } else if let Some(f) = def.microtabs.iter().find(|f| f.family == family) {
            return f.tabs.iter().find(|t| t.name == subtab);
        }
        None
    }

    /// If a subtab is notifying, its glow color becomes the layer's trueGlow.
    fn resolve_subtab_glow(&self, id: &str) -> Option<String> {
        if let Some(TabFormat::Tabs(tabs)) = &self.layer_def(id).tab_format {
            for tab in tabs {
                if self.subtab_should_notify(id, "mainTabs", tab.name) {
                    return Some(tab.glow_color.unwrap_or("#ff0000").to_string());
                }
            }
        }
        for family in &self.layer_def(id).microtabs {
            for tab in &family.tabs {
                if self.subtab_should_notify(id, family.family, tab.name) {
                    return Some(tab.glow_color.unwrap_or("#ff0000").to_string());
                }
            }
        }
        None
    }

    /// prestigeNotify (TMT layerSupport.js).
    fn get_prestige_notify(&mut self, id: &str) -> bool {
        if let Some(f) = &self.layer_def(id).prestige_notify {
            return f(self);
        }
        if let Some(TabFormat::Tabs(tabs)) = &self.layer_def(id).tab_format {
            for tab in tabs {
                if self.subtab_reset_notify(id, "mainTabs", tab.name) {
                    return true;
                }
            }
        }
        for family in &self.layer_def(id).microtabs {
            for tab in &family.tabs {
                if self.subtab_reset_notify(id, family.family, tab.name) {
                    return true;
                }
            }
        }
        let lt = self.ltmp(id);
        if lt.auto_prestige || lt.passive_generation.is_some() {
            return false;
        }
        match lt.prestige_type {
            PrestigeType::Static => lt.can_reset,
            PrestigeType::Normal => {
                lt.can_reset && lt.reset_gain >= self.lstate(id).points / Decimal::from(10)
            }
            _ => false,
        }
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
