//! Game-definition API — the Rust equivalent of The Modding Tree's layers.js.
//!
//! A mod builds a `GameDefs` out of `LayerDef`s and registers it with the
//! engine. Every field mirrors the same-named TMT property; where TMT accepts
//! "a value or a function", the `_fn` variant of a field holds the function
//! and the plain field the constant.

use crate::format::Decimal;
use std::collections::HashMap;
use std::rc::Rc;

/// A callback that only reads game state (runs every tick, like TMT's tmp).
/// Rc (not Box) so the engine can cheaply clone a callback out of the defs
/// and then call it with `&mut Game`.
pub type View<T> = Rc<dyn Fn(&crate::game::Game) -> T>;
/// A callback with side effects (buy, reset, click...).
pub type Act = Rc<dyn Fn(&mut crate::game::Game)>;
/// CSS declarations, pre-serialized, e.g. "background-color:#4BDC13;font-size:14px".
pub type Style = String;

pub fn view<T>(f: impl Fn(&crate::game::Game) -> T + 'static) -> View<T> {
    Rc::new(f)
}

pub fn act(f: impl Fn(&mut crate::game::Game) + 'static) -> Act {
    Rc::new(f)
}

// ---------------------------------------------------------------------------
// Shared small types
// ---------------------------------------------------------------------------

/// Prestige layer type: how the reset-gain formula behaves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrestigeType {
    None,
    Normal,
    Static,
    Custom,
}

/// A layer's row: a numbered tree row, or one of the special side rows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    Tree(i64),
    Side,
    Otherside,
}

impl Row {
    pub fn is_number(&self) -> bool {
        matches!(self, Row::Tree(_))
    }
    pub fn number(&self) -> i64 {
        match self {
            Row::Tree(n) => *n,
            _ => -1,
        }
    }
}

/// Where the currency for a purchase comes from (TMT `currencyLocation` /
/// `currencyLayer` / `currencyInternalName`).
#[derive(Clone)]
pub enum Currency {
    /// Default: the layer's own points.
    LayerPoints,
    /// A field of the global player object, e.g. "points".
    PlayerField(&'static str),
    /// A field of another layer's data, e.g. ("c", "otherThingy").
    LayerField { layer: &'static str, name: &'static str },
    /// A buyable amount, e.g. ("c", "11").
    Buyable { layer: &'static str, id: &'static str },
}

/// Values that TMT effects can return (a Decimal, or a named map of them).
#[derive(Clone, Debug)]
pub enum EffectVal {
    Dec(Decimal),
    Map(Vec<(&'static str, Decimal)>),
}

impl EffectVal {
    pub fn dec(&self) -> Decimal {
        match self {
            EffectVal::Dec(d) => *d,
            EffectVal::Map(m) => m.first().map(|(_, d)| *d).unwrap_or_else(Decimal::zero),
        }
    }
    pub fn get(&self, key: &str) -> Decimal {
        match self {
            EffectVal::Dec(d) => *d,
            EffectVal::Map(m) => m
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, d)| *d)
                .unwrap_or_else(Decimal::zero),
        }
    }
}

/// Build a `View2` (game + numeric arg callback).
pub fn view2<T>(f: impl Fn(&crate::game::Game, i64) -> T + 'static) -> View2<T> {
    Rc::new(f)
}

impl Default for EffectVal {
    fn default() -> Self {
        EffectVal::Dec(Decimal::zero())
    }
}

/// A Decimal that may come from a per-tick function (e.g. `requires`).
pub enum DecOrFn {
    Dec(Decimal),
    Fn(View<Decimal>),
}

impl DecOrFn {
    pub fn resolve(&self, g: &crate::game::Game) -> Decimal {
        match self {
            DecOrFn::Dec(d) => *d,
            DecOrFn::Fn(f) => f(g),
        }
    }
}

/// Bar fill direction (TMT UP/DOWN/LEFT/RIGHT constants).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// A branch line on the tree: target layer + optional color and width.
#[derive(Clone)]
pub enum Branch {
    Plain(&'static str),
    Colored(&'static str, &'static str, Option<f64>),
}

// ---------------------------------------------------------------------------
// Tab layout ("tabFormat" content)
// ---------------------------------------------------------------------------

/// A number of pixels, or a (width, height) pair (for `blank`).
#[derive(Clone)]
pub enum Size {
    Height(f64),
    WH(f64, f64),
}

/// One item in a tabFormat content list. TMT writes these as
/// `"upgrades"`, `["display-text", fn, style]`, etc.
pub enum Content {
    MainDisplay,
    PrestigeButton,
    ResourceDisplay,
    Milestones,
    Upgrades,
    Challenges,
    Buyables,
    Clickables,
    Achievements,
    Grid,
    Blank(Option<Size>),
    RawHtml(View<String>),
    DisplayText(View<String>),
    DisplayImage(String),
    TextInput(&'static str),
    Slider(&'static str, f64, f64),
    DropDown(&'static str, Vec<String>),
    Toggle(&'static str, &'static str),
    Row(Vec<ContentItem>),
    Column(Vec<ContentItem>),
    HLine(Option<f64>),
    VLine(Option<f64>),
    Upgrade(i64),
    Challenge(i64),
    Bar(&'static str),
    Infobox(&'static str),
    Microtabs(&'static str),
    /// An embedded mini-tree of layer nodes (`["tree", layout]`).
    Tree(Vec<Vec<TreeNode>>),
    /// `upgrade-tree` / `buyable-tree` / `clickable-tree`.
    ThingTree(&'static str, Vec<Vec<i64>>),
    /// Engine-built tabs (info / options / changelog).
    InfoTab,
    OptionsTab,
    Changelog,
}

/// A tree node reference: a real layer id, or a literal node.
#[derive(Clone)]
pub enum TreeNode {
    Layer(&'static str),
}

pub struct ContentItem {
    pub content: Content,
    pub style: Option<Style>,
}

/// Build a bare content item, `item("upgrades")`-style.
pub fn item(content: Content) -> ContentItem {
    ContentItem { content, style: None }
}

/// Content plus a CSS style string.
pub fn styled(content: Content, style: &str) -> ContentItem {
    ContentItem { content, style: Some(style.to_string()) }
}

/// A named subtab of a layer (object-style tabFormat entry or a microtab).
pub struct SubtabDef {
    pub name: &'static str,
    pub content: Vec<ContentItem>,
    pub style: View<Style>,
    pub button_style: View<Style>,
    pub unlocked: View<bool>,
    pub should_notify: View<bool>,
    pub prestige_notify: View<bool>,
    pub embed_layer: Option<&'static str>,
    pub glow_color: Option<&'static str>,
}

impl Default for SubtabDef {
    fn default() -> Self {
        SubtabDef {
            name: "",
            content: Vec::new(),
            style: view(|_| String::new()),
            button_style: view(|_| String::new()),
            unlocked: view(|_| true),
            should_notify: view(|_| false),
            prestige_notify: view(|_| false),
            embed_layer: None,
            glow_color: None,
        }
    }
}

pub enum TabFormat {
    /// Array-style tabFormat (a plain content list).
    Array(Vec<ContentItem>),
    /// Object-style tabFormat (named subtabs).
    Tabs(Vec<SubtabDef>),
}

// ---------------------------------------------------------------------------
// Component definitions
// ---------------------------------------------------------------------------

pub struct UpgradeDef {
    pub id: i64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub cost: DecOrFn,
    pub currency: Option<Currency>,
    pub pay: Option<Act>,
    pub effect: Option<View<EffectVal>>,
    pub effect_display: Option<View<String>>,
    pub unlocked: View<bool>,
    pub style: View<Style>,
    pub tooltip: View<Option<String>>,
    pub full_display: Option<View<String>>,
    pub can_afford: Option<View<bool>>,
    pub on_purchase: Option<Act>,
    pub branches: Vec<Branch>,
}

impl Default for UpgradeDef {
    fn default() -> Self {
        UpgradeDef {
            id: 0,
            title: None,
            description: None,
            cost: DecOrFn::Dec(Decimal::zero()),
            currency: None,
            pay: None,
            effect: None,
            effect_display: None,
            unlocked: view(|_| true),
            style: view(|_| String::new()),
            tooltip: view(|_| None),
            full_display: None,
            can_afford: None,
            on_purchase: None,
            branches: Vec::new(),
        }
    }
}

pub struct MilestoneDef {
    pub id: i64,
    pub requirement_description: String,
    pub effect_description: View<String>,
    pub done: View<bool>,
    pub unlocked: View<bool>,
    pub toggles: Vec<(&'static str, &'static str)>,
    pub style: View<Style>,
    pub tooltip: View<Option<String>>,
    pub on_complete: Option<Act>,
}

impl Default for MilestoneDef {
    fn default() -> Self {
        MilestoneDef {
            id: 0,
            requirement_description: String::new(),
            effect_description: view(|_| String::new()),
            done: view(|_| false),
            unlocked: view(|_| true),
            toggles: Vec::new(),
            style: view(|_| String::new()),
            tooltip: view(|_| None),
            on_complete: None,
        }
    }
}

pub struct AchievementDef {
    pub id: i64,
    pub name: String,
    pub image: Option<String>,
    pub done: View<bool>,
    pub goal_tooltip: Option<String>,
    pub done_tooltip: Option<String>,
    pub tooltip: Option<String>,
    pub text_style: Style,
    pub style: View<Style>,
    pub on_complete: Option<Act>,
    pub effect: Option<View<EffectVal>>,
}

impl Default for AchievementDef {
    fn default() -> Self {
        AchievementDef {
            id: 0,
            name: String::new(),
            image: None,
            done: view(|_| false),
            goal_tooltip: None,
            done_tooltip: None,
            tooltip: None,
            text_style: String::new(),
            style: view(|_| String::new()),
            on_complete: None,
            effect: None,
        }
    }
}

pub struct ChallengeDef {
    pub id: i64,
    pub name: String,
    pub completion_limit: i64,
    pub challenge_description: View<String>,
    pub unlocked: View<bool>,
    pub goal: Decimal,
    pub goal_description: Option<String>,
    pub goal_currency: Option<Currency>,
    pub can_complete: Option<View<bool>>,
    pub reward_effect: Option<View<EffectVal>>,
    pub reward_display: Option<View<String>>,
    pub reward_description: Option<String>,
    pub counts_as: Vec<i64>,
    pub can_enter: View<bool>,
    pub can_exit: View<bool>,
    pub marked: View<bool>,
    pub style: View<Style>,
    pub full_display: Option<View<String>>,
    pub on_complete: Option<Act>,
    pub on_enter: Option<Act>,
    pub on_exit: Option<Act>,
}

impl Default for ChallengeDef {
    fn default() -> Self {
        ChallengeDef {
            id: 0,
            name: String::new(),
            completion_limit: 1,
            challenge_description: view(|_| String::new()),
            unlocked: view(|_| true),
            goal: Decimal::zero(),
            goal_description: None,
            goal_currency: None,
            can_complete: None,
            reward_effect: None,
            reward_display: None,
            reward_description: None,
            counts_as: Vec::new(),
            can_enter: view(|_| true),
            can_exit: view(|_| true),
            marked: view(|_| false),
            style: view(|_| String::new()),
            full_display: None,
            on_complete: None,
            on_enter: None,
            on_exit: None,
        }
    }
}

pub struct BuyableDef {
    pub id: &'static str,
    pub title: Option<String>,
    pub cost: View<Decimal>,
    pub effect: View<EffectVal>,
    pub display: View<String>,
    pub unlocked: View<bool>,
    pub can_afford: View<bool>,
    pub buy: Option<Act>,
    pub buy_max: Option<Act>,
    pub sell_one: Option<Act>,
    pub sell_all: Option<Act>,
    pub style: View<Style>,
    pub tooltip: View<Option<String>>,
    pub marked: View<bool>,
    pub purchase_limit: Decimal,
}

impl Default for BuyableDef {
    fn default() -> Self {
        BuyableDef {
            id: "",
            title: None,
            cost: view(|_| Decimal::zero()),
            effect: view(|_| EffectVal::Dec(Decimal::one())),
            display: view(|_| String::new()),
            unlocked: view(|_| true),
            can_afford: view(|_| false),
            buy: None,
            buy_max: None,
            sell_one: None,
            sell_all: None,
            style: view(|_| String::new()),
            tooltip: view(|_| None),
            marked: view(|_| false),
            purchase_limit: Decimal::inf(),
        }
    }
}

pub struct BuyablesDef {
    pub respec: Option<Act>,
    pub show_respec: bool,
    pub respec_text: String,
    pub respec_message: String,
    pub sell_one_text: String,
    pub sell_all_text: String,
    pub items: Vec<BuyableDef>,
}

impl Default for BuyablesDef {
    fn default() -> Self {
        BuyablesDef {
            respec: None,
            show_respec: true,
            respec_text: "Respec".to_string(),
            respec_message: "Are you sure you want to respec? This will force you to do a reset as well!".to_string(),
            sell_one_text: "Sell One".to_string(),
            sell_all_text: "Sell All".to_string(),
            items: Vec::new(),
        }
    }
}

pub struct ClickableDef {
    pub id: &'static str,
    pub title: Option<String>,
    pub display: View<String>,
    pub unlocked: View<bool>,
    pub can_click: View<bool>,
    pub on_click: Option<Act>,
    pub on_hold: Option<Act>,
    pub style: View<Style>,
    pub tooltip: View<Option<String>>,
    pub marked: View<bool>,
}

impl Default for ClickableDef {
    fn default() -> Self {
        ClickableDef {
            id: "",
            title: None,
            display: view(|_| String::new()),
            unlocked: view(|_| true),
            can_click: view(|_| false),
            on_click: None,
            on_hold: None,
            style: view(|_| String::new()),
            tooltip: view(|_| None),
            marked: view(|_| false),
        }
    }
}

pub struct ClickablesDef {
    pub master_button_press: Option<Act>,
    pub master_button_text: View<String>,
    pub show_master_button: bool,
    pub items: Vec<ClickableDef>,
}

impl Default for ClickablesDef {
    fn default() -> Self {
        ClickablesDef {
            master_button_press: None,
            master_button_text: view(|_| "Click me!".to_string()),
            show_master_button: true,
            items: Vec::new(),
        }
    }
}

pub struct BarDef {
    pub id: &'static str,
    pub direction: Direction,
    pub width: f64,
    pub height: f64,
    pub progress: View<f64>,
    pub display: View<String>,
    pub unlocked: View<bool>,
    pub instant: bool,
    pub style: View<Style>,
    pub fill_style: View<Style>,
    pub base_style: View<Style>,
    pub text_style: View<Style>,
    pub border_style: View<Style>,
}

impl Default for BarDef {
    fn default() -> Self {
        BarDef {
            id: "",
            direction: Direction::Up,
            width: 100.0,
            height: 30.0,
            progress: view(|_| 0.0),
            display: view(|_| String::new()),
            unlocked: view(|_| true),
            instant: false,
            style: view(|_| String::new()),
            fill_style: view(|_| String::new()),
            base_style: view(|_| String::new()),
            text_style: view(|_| String::new()),
            border_style: view(|_| String::new()),
        }
    }
}

pub struct InfoboxDef {
    pub id: &'static str,
    pub title: View<String>,
    pub body: View<String>,
    pub unlocked: View<bool>,
    pub title_style: View<Style>,
    pub body_style: View<Style>,
    pub style: View<Style>,
}

impl Default for InfoboxDef {
    fn default() -> Self {
        InfoboxDef {
            id: "",
            title: view(|_| String::new()),
            body: view(|_| String::new()),
            unlocked: view(|_| true),
            title_style: view(|_| String::new()),
            body_style: view(|_| String::new()),
            style: view(|_| String::new()),
        }
    }
}

pub struct GridDef {
    pub rows: i64,
    pub cols: i64,
    pub max_rows: i64,
    pub max_cols: i64,
    pub get_start_data: Rc<dyn Fn(i64) -> crate::game::PVal>,
    pub get_unlocked: View2<bool>,
    pub get_can_click: View2<bool>,
    pub get_style: View2<Style>,
    pub on_click: Option<Act2>,
    pub on_hold: Option<Act2>,
    pub get_title: View2<String>,
    pub get_display: View2<String>,
    pub get_tooltip: View2<Option<String>>,
}

/// Grid callbacks take (data, cell id) in TMT; the id alone is enough here.
pub type View2<T> = Rc<dyn Fn(&crate::game::Game, i64) -> T>;

/// Argument passed to layer/component action callbacks (onPrestige gain,
/// doReset resetting-layer, update diff, grid cell id, ...).
#[derive(Clone, Debug)]
pub enum Arg {
    None,
    Num(f64),
    Dec(Decimal),
    Str(String),
}

pub type Act2 = Rc<dyn Fn(&mut crate::game::Game, Arg)>;

impl Default for GridDef {
    fn default() -> Self {
        GridDef {
            rows: 0,
            cols: 0,
            max_rows: 0,
            max_cols: 0,
            get_start_data: Rc::new(|_| crate::game::PVal::Num(0.0)),
            get_unlocked: view2(|_, _| true),
            get_can_click: view2(|_, _| true),
            get_style: view2(|_, _| String::new()),
            on_click: None,
            on_hold: None,
            get_title: view2(|_, _| String::new()),
            get_display: view2(|_, _| String::new()),
            get_tooltip: view2(|_, _| None),
        }
    }
}

pub struct MicrotabsDef {
    pub family: &'static str,
    pub tabs: Vec<SubtabDef>,
}

pub struct HotkeyDef {
    /// Key name, e.g. "p" or "ctrl+c" (TMT format).
    pub key: &'static str,
    pub description: String,
    pub on_press: Act,
    pub unlocked: View<bool>,
}

// ---------------------------------------------------------------------------
// The layer definition
// ---------------------------------------------------------------------------

pub struct LayerDef {
    pub id: &'static str,
    pub is_layer: bool,

    pub name: Option<&'static str>,
    pub symbol: Option<&'static str>,
    pub color: &'static str,
    pub position: Option<f64>,
    pub row: Row,
    pub display_row: Option<Row>,

    // Prestige mechanics
    pub prestige_type: PrestigeType,
    pub resource: &'static str,
    pub base_resource: &'static str,
    pub base_amount: View<Decimal>,
    pub requires: DecOrFn,
    pub exponent: View<Decimal>,
    pub base: View<Decimal>,
    pub round_up_cost: bool,
    pub gain_mult: View<Decimal>,
    pub gain_exp: View<Decimal>,
    pub direct_mult: View<Decimal>,
    pub softcap: View<Decimal>,
    pub softcap_power: View<Decimal>,
    pub passive_generation: View<Option<Decimal>>,
    pub can_buy_max: View<Option<bool>>,
    pub get_reset_gain: Option<View<Decimal>>,
    pub get_next_at: Option<View2<Decimal>>,
    pub can_reset: Option<View<bool>>,
    pub resets_nothing: View<bool>,
    pub on_prestige: Option<Act2>,

    // Node display
    pub node_style: View<Style>,
    pub image: Option<String>,
    pub glow_color: Option<&'static str>,
    pub tooltip: View<Option<String>>,
    pub tooltip_locked: View<Option<String>>,
    pub layer_shown: View<LayerShown>,
    pub branches: Vec<Branch>,
    pub marked: View<Option<String>>,
    pub should_notify: View<bool>,
    pub prestige_notify: Option<View<bool>>,

    // Layout
    pub tab_format: Option<TabFormat>,
    pub midsection: Vec<ContentItem>,
    pub component_styles: ComponentStyles,
    pub style: View<Style>,
    pub left_tab: bool,
    pub previous_tab: &'static str,

    // Effects on the layer's resource
    pub effect: Option<View<EffectVal>>,
    pub effect_description: Option<View<String>>,

    // Components
    pub upgrades: Vec<UpgradeDef>,
    pub milestones: Vec<MilestoneDef>,
    pub achievements: Vec<AchievementDef>,
    pub challenges: Vec<ChallengeDef>,
    pub buyables: BuyablesDef,
    pub clickables: ClickablesDef,
    pub bars: Vec<BarDef>,
    pub infoboxes: Vec<InfoboxDef>,
    pub grid: Option<GridDef>,
    pub microtabs: Vec<MicrotabsDef>,
    pub hotkeys: Vec<HotkeyDef>,

    // Automation / popups
    pub auto_prestige: View<bool>,
    pub auto_upgrade: View<bool>,
    pub deactivated: View<bool>,
    pub milestone_popups: bool,
    pub achievement_popups: bool,
    pub increase_unlock_order: Vec<&'static str>,

    // Non-layer nodes (TMT addNode): clickable tree buttons.
    pub node_on_click: Option<Act>,
    pub node_can_click: Option<View<bool>>,

    // Callbacks
    pub update: Option<Act2>,
    pub automate: Option<Act>,
    pub do_reset: Option<Act2>,
    pub reset_description: Option<&'static str>,
    pub prestige_button_text: Option<View<String>>,

    /// Extra start data (fields beyond the engine-managed ones).
    pub start_data: Vec<(&'static str, crate::game::PVal)>,
    pub start_unlocked: bool,
}

/// TMT layerShown may be true, false or "ghost" (invisible spacer).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LayerShown {
    Shown,
    Hidden,
    Ghost,
}

/// Per-component-type extra CSS (TMT componentStyles).
pub struct ComponentStyles {
    pub upgrade: View<Style>,
    pub milestone: View<Style>,
    pub achievement: View<Style>,
    pub challenge: View<Style>,
    pub buyable: View<Style>,
    pub clickable: View<Style>,
    pub bar: View<Style>,
    pub gridable: View<Style>,
    pub infobox: View<Style>,
    pub toggle: View<Style>,
    pub tab_button: View<Style>,
    pub tab_buttons: View<Style>,
    pub main_display: View<Style>,
    pub prestige_button: View<Style>,
    pub resource_display: View<Style>,
    pub respec_button: View<Style>,
    pub master_button: View<Style>,
    pub sell_one: View<Style>,
    pub sell_all: View<Style>,
}

impl Default for ComponentStyles {
    fn default() -> Self {
        macro_rules! v {
            () => {
                view(|_| String::new())
            };
        }
        ComponentStyles {
            upgrade: v!(),
            milestone: v!(),
            achievement: v!(),
            challenge: v!(),
            buyable: v!(),
            clickable: v!(),
            bar: v!(),
            gridable: v!(),
            infobox: v!(),
            toggle: v!(),
            tab_button: v!(),
            tab_buttons: v!(),
            main_display: v!(),
            prestige_button: v!(),
            resource_display: v!(),
            respec_button: v!(),
            master_button: v!(),
            sell_one: v!(),
            sell_all: v!(),
        }
    }
}

impl Default for LayerDef {
    fn default() -> Self {
        LayerDef {
            id: "",
            is_layer: true,
            name: None,
            symbol: None,
            color: "#ffffff",
            position: None,
            row: Row::Tree(0),
            display_row: None,
            prestige_type: PrestigeType::None,
            resource: "",
            base_resource: "",
            base_amount: view(|_| Decimal::zero()),
            requires: DecOrFn::Dec(Decimal::zero()),
            exponent: view(|_| Decimal::one()),
            base: view(|_| Decimal::from(2)),
            round_up_cost: false,
            gain_mult: view(|_| Decimal::one()),
            gain_exp: view(|_| Decimal::one()),
            direct_mult: view(|_| Decimal::one()),
            softcap: view(|_| crate::format::dec("e1e7")),
            softcap_power: view(|_| Decimal::from_finite(0.5)),
            passive_generation: view(|_| None),
            can_buy_max: view(|_| None),
            get_reset_gain: None,
            get_next_at: None,
            can_reset: None,
            resets_nothing: view(|_| false),
            on_prestige: None,
            node_style: view(|_| String::new()),
            image: None,
            glow_color: None,
            tooltip: view(|_| None),
            tooltip_locked: view(|_| None),
            layer_shown: view(|_| LayerShown::Shown),
            branches: Vec::new(),
            marked: view(|_| None),
            should_notify: view(|_| false),
            prestige_notify: None,
            tab_format: None,
            midsection: Vec::new(),
            component_styles: ComponentStyles::default(),
            style: view(|_| String::new()),
            left_tab: false,
            previous_tab: "",
            effect: None,
            effect_description: None,
            upgrades: Vec::new(),
            milestones: Vec::new(),
            achievements: Vec::new(),
            challenges: Vec::new(),
            buyables: BuyablesDef::default(),
            clickables: ClickablesDef::default(),
            bars: Vec::new(),
            infoboxes: Vec::new(),
            grid: None,
            microtabs: Vec::new(),
            hotkeys: Vec::new(),
            auto_prestige: view(|_| false),
            auto_upgrade: view(|_| false),
            deactivated: view(|_| false),
            milestone_popups: true,
            achievement_popups: true,
            increase_unlock_order: Vec::new(),
            node_on_click: None,
            node_can_click: None,
            update: None,
            automate: None,
            do_reset: None,
            reset_description: None,
            prestige_button_text: None,
            start_data: Vec::new(),
            start_unlocked: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Global game info (mod.js equivalent)
// ---------------------------------------------------------------------------

pub struct ModInfo {
    pub name: &'static str,
    pub id: &'static str,
    pub points_name: &'static str,
    pub initial_start_points: Decimal,
    pub offline_limit: f64,
    pub allow_small: bool,
    pub author: &'static str,
}

pub struct GameDefs {
    pub mod_info: ModInfo,
    pub version: &'static str,
    pub version_name: &'static str,
    pub win_text: &'static str,
    pub changelog: &'static str,
    pub layers: Vec<LayerDef>,
    pub start_tab: &'static str,
    pub start_nav_tab: &'static str,
    pub show_tree: bool,
    pub can_gen_points: View<bool>,
    pub get_point_gen: View<Decimal>,
    pub is_endgame: View<bool>,
    pub display_things: Vec<View<String>>,
    pub background_style: View<Style>,
    pub max_tick_length: f64,
    /// Extra top-level player data (mod.js addedPlayerData).
    pub added_player_data: Vec<(&'static str, crate::game::PVal)>,
}

impl GameDefs {
    /// Look up a layer index by id.
    pub fn layer_index(&self, id: &str) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    pub fn layer(&self, id: &str) -> &LayerDef {
        &self.layers[self.layer_index(id).expect("unknown layer")]
    }

    /// All tree rows (numeric display rows 0..=max_row), each sorted by position.
    pub fn tree_rows(&self) -> Vec<Vec<&'static str>> {
        let mut max_row = -1;
        for layer in &self.layers {
            let r = layer.display_row.unwrap_or(layer.row);
            if let Row::Tree(n) = r {
                if n > max_row {
                    max_row = n;
                }
            }
        }
        let mut rows: Vec<Vec<(f64, &'static str)>> = vec![Vec::new(); (max_row + 1) as usize];
        for layer in &self.layers {
            let r = layer.display_row.unwrap_or(layer.row);
            if let Row::Tree(n) = r {
                if n >= 0 {
                    let pos = layer.position.unwrap_or_else(|| default_position(layer.id));
                    rows[n as usize].push((pos, layer.id));
                }
            }
        }
        rows.into_iter()
            .map(|mut row| {
                row.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                row.into_iter().map(|(_, id)| id).collect()
            })
            .collect()
    }

    /// Layers in the "side" row, sorted by position.
    pub fn side_layers(&self) -> Vec<&'static str> {
        self.special_row(Row::Side)
    }

    /// Layers in the "otherside" row, sorted by position.
    pub fn other_side_layers(&self) -> Vec<&'static str> {
        self.special_row(Row::Otherside)
    }

    fn special_row(&self, row: Row) -> Vec<&'static str> {
        let mut layers: Vec<(f64, &'static str)> = self
            .layers
            .iter()
            .filter(|l| l.row == row)
            .map(|l| (l.position.unwrap_or_else(|| default_position(l.id)), l.id))
            .collect();
        layers.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        layers.into_iter().map(|(_, id)| id).collect()
    }

    /// All hotkeys across layers, keyed by key name (TMT updateHotkeys).
    pub fn hotkeys(&self) -> HashMap<&'static str, (&'static str, usize, usize)> {
        let mut map = HashMap::new();
        for (li, layer) in self.layers.iter().enumerate() {
            for (hi, hk) in layer.hotkeys.iter().enumerate() {
                map.insert(hk.key, (layer.id, li, hi));
            }
        }
        map
    }

    /// The default order of a layer's rows/cols grid for numeric ids.
    pub fn upgrade_rows_cols(ids: &[i64]) -> (i64, i64) {
        let mut max_row = 0;
        let mut max_col = 0;
        for id in ids {
            max_row = max_row.max(id / 10);
            max_col = max_col.max(id % 10);
        }
        (max_row, max_col)
    }
}

/// TMT sorts same-position layers alphabetically by id.
fn default_position(id: &str) -> f64 {
    // Cheap deterministic order: sum of bytes scaled to fractions.
    let mut v = 0f64;
    for (i, b) in id.bytes().enumerate() {
        v += (b as f64) * 10f64.powi(-(i as i32));
    }
    v
}
