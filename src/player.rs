//! Player state and the save system — a port of js/utils/save.js plus the
//! dynamic layer data (TMT stores whatever startData returns; here extra
//! fields live in `LayerState::extra` as `PVal`s).

use crate::defs::GameDefs;
use crate::format::Decimal;
use crate::json::Json;
use std::collections::HashMap;

/// A dynamically-typed player value (TMT player fields can be Decimal,
/// number, string or boolean).
#[derive(Clone, Debug)]
pub enum PVal {
    Dec(Decimal),
    Num(f64),
    Str(String),
    Bool(bool),
}

impl PVal {
    pub fn as_dec(&self) -> Decimal {
        match self {
            PVal::Dec(d) => *d,
            PVal::Num(n) => crate::format::dnum(*n),
            PVal::Str(s) => crate::format::dec(s),
            PVal::Bool(b) => Decimal::from(*b as i64),
        }
    }
    pub fn as_num(&self) -> f64 {
        match self {
            PVal::Dec(d) => d.to_number(),
            PVal::Num(n) => *n,
            PVal::Str(s) => s.parse().unwrap_or(0.0),
            PVal::Bool(b) => *b as i64 as f64,
        }
    }
    pub fn as_str_val(&self) -> String {
        match self {
            PVal::Dec(d) => d.to_string(),
            PVal::Num(n) => crate::json::num_to_json_string(*n),
            PVal::Str(s) => s.clone(),
            PVal::Bool(b) => b.to_string(),
        }
    }
    pub fn as_bool(&self) -> bool {
        match self {
            PVal::Bool(b) => *b,
            PVal::Dec(d) => *d != Decimal::zero(),
            PVal::Num(n) => *n != 0.0,
            PVal::Str(s) => s == "true",
        }
    }

    fn to_json(&self) -> Json {
        match self {
            PVal::Dec(d) => Json::s(d.to_string()),
            PVal::Num(n) => Json::Num(*n),
            PVal::Str(s) => Json::s(s.clone()),
            PVal::Bool(b) => Json::Bool(*b),
        }
    }

    /// Convert to the same type as `default` (fixData semantics).
    fn coerce_like(default: &PVal, value: &Json) -> PVal {
        match default {
            PVal::Dec(_) => PVal::Dec(match value {
                Json::Str(s) => crate::format::dec(s),
                Json::Num(n) => crate::format::dnum(*n),
                _ => Decimal::zero(),
            }),
            PVal::Num(_) => PVal::Num(value.num_or(0.0)),
            PVal::Str(_) => PVal::Str(value.str_or("")),
            PVal::Bool(_) => PVal::Bool(value.as_bool().unwrap_or(false)),
        }
    }
}

/// Everything TMT stores per layer.
#[derive(Clone, Debug)]
pub struct LayerState {
    pub unlocked: bool,
    pub points: Decimal,
    pub best: Decimal,
    pub total: Decimal,
    pub reset_time: f64,
    pub spent_on_buyables: Decimal,
    pub upgrades: Vec<i64>,
    pub milestones: Vec<i64>,
    pub achievements: Vec<i64>,
    pub buyables: HashMap<String, Decimal>,
    pub clickables: HashMap<String, PVal>,
    pub challenges: HashMap<String, i64>,
    pub grid: HashMap<i64, PVal>,
    pub active_challenge: Option<i64>,
    pub last_milestone: Option<i64>,
    pub no_respec_confirm: bool,
    pub force_tooltip: bool,
    pub prev_tab: String,
    pub unlock_order: i64,
    pub extra: HashMap<String, PVal>,
}

impl LayerState {
    pub fn extra_dec(&self, name: &str) -> Decimal {
        self.extra.get(name).map(|v| v.as_dec()).unwrap_or_else(Decimal::zero)
    }
    pub fn extra_num(&self, name: &str) -> f64 {
        self.extra.get(name).map(|v| v.as_num()).unwrap_or(0.0)
    }
    pub fn extra_str(&self, name: &str) -> String {
        self.extra.get(name).map(|v| v.as_str_val()).unwrap_or_default()
    }
    pub fn extra_bool(&self, name: &str) -> bool {
        self.extra.get(name).map(|v| v.as_bool()).unwrap_or(false)
    }
    pub fn set_extra(&mut self, name: &str, value: PVal) {
        self.extra.insert(name.to_string(), value);
    }
}

/// The global player object (startPlayerBase + addedPlayerData + layers).
#[derive(Clone, Debug)]
pub struct Player {
    pub tab: String,
    pub nav_tab: String,
    pub time: f64,
    pub time_played: f64,
    pub notify: HashMap<String, i64>,
    pub version_type: String,
    pub version: String,
    pub keep_going: bool,
    pub has_nan: bool,
    pub points: Decimal,
    pub subtabs: HashMap<String, HashMap<String, String>>,
    pub infoboxes: HashMap<String, HashMap<String, bool>>,
    pub last_safe_tab: String,
    pub off_time: Option<f64>,
    pub dev_speed: Option<f64>,
    pub extra: HashMap<String, PVal>,
    pub layers: HashMap<String, LayerState>,
}

impl Player {
    pub fn extra_dec(&self, name: &str) -> Decimal {
        self.extra.get(name).map(|v| v.as_dec()).unwrap_or_else(Decimal::zero)
    }
    pub fn extra_str(&self, name: &str) -> String {
        self.extra.get(name).map(|v| v.as_str_val()).unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Start data
// ---------------------------------------------------------------------------

pub fn get_start_player(defs: &GameDefs) -> Player {
    let mut player = Player {
        tab: defs.start_tab.to_string(),
        nav_tab: if defs.show_tree { defs.start_nav_tab.to_string() } else { "none".to_string() },
        time: 0.0,
        time_played: 0.0,
        notify: HashMap::new(),
        version_type: defs.mod_info.id.to_string(),
        version: defs.version.to_string(),
        keep_going: false,
        has_nan: false,
        points: defs.mod_info.initial_start_points,
        subtabs: HashMap::new(),
        infoboxes: HashMap::new(),
        last_safe_tab: if defs.show_tree { "none".to_string() } else { defs.start_tab.to_string() },
        off_time: None,
        dev_speed: None,
        extra: HashMap::new(),
        layers: HashMap::new(),
    };

    for (name, value) in &defs.added_player_data {
        player.extra.insert(name.to_string(), value.clone());
    }

    for layer in &defs.layers {
        player.layers.insert(layer.id.to_string(), get_start_layer_state(layer));
        let mut subtabs: HashMap<String, String> = HashMap::new();
        if let Some(crate::defs::TabFormat::Tabs(tabs)) = &layer.tab_format {
            if let Some(first) = tabs.first() {
                subtabs.insert("mainTabs".to_string(), first.name.to_string());
            }
        }
        for family in &layer.microtabs {
            if let Some(first) = family.tabs.first() {
                subtabs.insert(family.family.to_string(), first.name.to_string());
            }
        }
        if !subtabs.is_empty() {
            player.subtabs.insert(layer.id.to_string(), subtabs);
        }
        if !layer.infoboxes.is_empty() {
            let boxes = layer
                .infoboxes
                .iter()
                .map(|b| (b.id.to_string(), false))
                .collect();
            player.infoboxes.insert(layer.id.to_string(), boxes);
        }
    }

    player
}

fn get_start_layer_state(def: &crate::defs::LayerDef) -> LayerState {
    let mut extra: HashMap<String, PVal> = def
        .start_data
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    extra.remove("unlocked");

    let mut buyables = HashMap::new();
    for b in &def.buyables.items {
        buyables.insert(b.id.to_string(), Decimal::zero());
    }
    let mut clickables = HashMap::new();
    for c in &def.clickables.items {
        // TMT startData may seed a clickable state ("Start"); defaults to "".
        let initial = def
            .start_data
            .iter()
            .find(|(k, _)| *k == format!("clickable:{}", c.id).as_str())
            .map(|(_, v)| v.clone());
        clickables.insert(
            c.id.to_string(),
            initial.unwrap_or(PVal::Str(String::new())),
        );
    }
    let mut challenges = HashMap::new();
    for c in &def.challenges {
        challenges.insert(c.id.to_string(), 0);
    }
    let mut grid = HashMap::new();
    if let Some(grid_def) = &def.grid {
        let max_rows = if grid_def.max_rows > 0 { grid_def.max_rows } else { grid_def.rows };
        let max_cols = if grid_def.max_cols > 0 { grid_def.max_cols } else { grid_def.cols };
        for y in 1..=max_rows {
            for x in 1..=max_cols {
                let id = 100 * y + x;
                grid.insert(id, (grid_def.get_start_data)(id));
            }
        }
    }

    LayerState {
        unlocked: def.start_unlocked,
        points: Decimal::zero(),
        best: Decimal::zero(),
        total: Decimal::zero(),
        reset_time: 0.0,
        spent_on_buyables: Decimal::zero(),
        upgrades: Vec::new(),
        milestones: Vec::new(),
        achievements: Vec::new(),
        buyables,
        clickables,
        challenges,
        grid,
        active_challenge: None,
        last_milestone: None,
        no_respec_confirm: false,
        force_tooltip: false,
        prev_tab: String::new(),
        unlock_order: 0,
        extra,
    }
}

/// layerDataReset: restore a layer to its start state, keeping the fields TMT
/// always keeps plus any requested ones.
pub fn layer_data_reset(player: &mut Player, defs: &GameDefs, layer: &str, keep: &[&str]) {
    let start = get_start_layer_state(defs.layer(layer));
    let old = player.layers.get_mut(layer).expect("unknown layer");

    // Fields always kept (TMT layerDataReset).
    let mut stored: Vec<(String, PVal)> = vec![
        ("unlocked".into(), PVal::Bool(old.unlocked)),
        ("forceTooltip".into(), PVal::Bool(old.force_tooltip)),
        ("noRespecConfirm".into(), PVal::Bool(old.no_respec_confirm)),
        ("prevTab".into(), PVal::Str(old.prev_tab.clone())),
    ];
    for field in keep {
        if let Some(v) = old.extra.get(*field) {
            stored.push((field.to_string(), v.clone()));
        } else if *field == "points" {
            stored.push(("points".into(), PVal::Dec(old.points)));
        } else if *field == "best" {
            stored.push(("best".into(), PVal::Dec(old.best)));
        } else if *field == "total" {
            stored.push(("total".into(), PVal::Dec(old.total)));
        } else if *field == "spentOnBuyables" {
            stored.push(("spentOnBuyables".into(), PVal::Dec(old.spent_on_buyables)));
        }
    }

    *old = start;

    for (field, value) in stored {
        match field.as_str() {
            "unlocked" => old.unlocked = value.as_bool(),
            "forceTooltip" => old.force_tooltip = value.as_bool(),
            "noRespecConfirm" => old.no_respec_confirm = value.as_bool(),
            "prevTab" => old.prev_tab = value.as_str_val(),
            "points" => old.points = value.as_dec(),
            "best" => old.best = value.as_dec(),
            "total" => old.total = value.as_dec(),
            "spentOnBuyables" => old.spent_on_buyables = value.as_dec(),
            other => {
                old.extra.insert(other.to_string(), value);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Serialization (TMT-shaped JSON: Decimals become strings)
// ---------------------------------------------------------------------------

pub fn player_to_json(defs: &GameDefs, player: &Player) -> Json {
    let mut fields: Vec<(String, Json)> = vec![
        ("tab".into(), Json::s(player.tab.clone())),
        ("navTab".into(), Json::s(player.nav_tab.clone())),
        ("time".into(), Json::Num(player.time)),
        ("timePlayed".into(), Json::Num(player.time_played)),
        ("versionType".into(), Json::s(player.version_type.clone())),
        ("version".into(), Json::s(player.version.clone())),
        ("keepGoing".into(), Json::Bool(player.keep_going)),
        ("hasNaN".into(), Json::Bool(player.has_nan)),
        ("points".into(), Json::s(player.points.to_string())),
        ("lastSafeTab".into(), Json::s(player.last_safe_tab.clone())),
    ];
    if !player.notify.is_empty() {
        fields.push((
            "notify".into(),
            Json::Obj(
                player
                    .notify
                    .iter()
                    .map(|(k, v)| (k.clone(), Json::Num(*v as f64)))
                    .collect(),
            ),
        ));
    }
    if let Some(off) = player.off_time {
        fields.push(("offTime".into(), Json::obj(vec![("remain", Json::Num(off))])));
    }
    if let Some(dev) = player.dev_speed {
        fields.push(("devSpeed".into(), Json::Num(dev)));
    }
    for (k, v) in &player.extra {
        fields.push((k.clone(), v.to_json()));
    }

    let subtabs = Json::Obj(
        player
            .subtabs
            .iter()
            .map(|(layer, fams)| {
                (
                    layer.clone(),
                    Json::Obj(fams.iter().map(|(f, t)| (f.clone(), Json::s(t.clone()))).collect()),
                )
            })
            .collect(),
    );
    fields.push(("subtabs".into(), subtabs));

    let infoboxes = Json::Obj(
        player
            .infoboxes
            .iter()
            .map(|(layer, boxes)| {
                (
                    layer.clone(),
                    Json::Obj(boxes.iter().map(|(b, v)| (b.clone(), Json::Bool(*v))).collect()),
                )
            })
            .collect(),
    );
    fields.push(("infoboxes".into(), infoboxes));

    for layer_def in &defs.layers {
        fields.push((layer_def.id.to_string(), layer_to_json(player, layer_def.id)));
    }

    Json::Obj(fields)
}

fn layer_to_json(player: &Player, layer: &str) -> Json {
    let s = match player.layers.get(layer) {
        Some(s) => s,
        None => return Json::Null,
    };
    let mut fields: Vec<(String, Json)> = vec![
        ("unlocked".into(), Json::Bool(s.unlocked)),
        ("points".into(), Json::s(s.points.to_string())),
        ("best".into(), Json::s(s.best.to_string())),
        ("total".into(), Json::s(s.total.to_string())),
        ("resetTime".into(), Json::Num(s.reset_time)),
        ("spentOnBuyables".into(), Json::s(s.spent_on_buyables.to_string())),
        ("upgrades".into(), Json::Arr(s.upgrades.iter().map(|u| Json::Num(*u as f64)).collect())),
        ("milestones".into(), Json::Arr(s.milestones.iter().map(|m| Json::Num(*m as f64)).collect())),
        (
            "achievements".into(),
            Json::Arr(s.achievements.iter().map(|a| Json::Num(*a as f64)).collect()),
        ),
        (
            "buyables".into(),
            Json::Obj(
                s.buyables
                    .iter()
                    .map(|(k, v)| (k.clone(), Json::s(v.to_string())))
                    .collect(),
            ),
        ),
        (
            "clickables".into(),
            Json::Obj(s.clickables.iter().map(|(k, v)| (k.clone(), v.to_json())).collect()),
        ),
        (
            "challenges".into(),
            Json::Obj(s.challenges.iter().map(|(k, v)| (k.clone(), Json::Num(*v as f64))).collect()),
        ),
        (
            "grid".into(),
            Json::Obj(
                s.grid
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_json()))
                    .collect(),
            ),
        ),
        ("noRespecConfirm".into(), Json::Bool(s.no_respec_confirm)),
        ("forceTooltip".into(), Json::Bool(s.force_tooltip)),
        ("prevTab".into(), Json::s(s.prev_tab.clone())),
        ("unlockOrder".into(), Json::Num(s.unlock_order as f64)),
    ];
    if let Some(c) = s.active_challenge {
        fields.push(("activeChallenge".into(), Json::Num(c as f64)));
    }
    if let Some(m) = s.last_milestone {
        fields.push(("lastMilestone".into(), Json::Num(m as f64)));
    }
    for (k, v) in &s.extra {
        fields.push((k.clone(), v.to_json()));
    }
    Json::Obj(fields)
}

/// Load a player from parsed JSON, merging onto fresh start data (TMT's
/// Object.assign(getStartPlayer(), parsed) + fixSave).
pub fn player_from_json(defs: &GameDefs, data: &Json) -> Player {
    let mut player = get_start_player(defs);
    if data.as_obj().is_none() {
        return player;
    }

    player.tab = data.get("tab").map(|v| v.str_or(&player.tab)).unwrap_or(player.tab);
    player.nav_tab = data.get("navTab").map(|v| v.str_or(&player.nav_tab)).unwrap_or(player.nav_tab);
    if let Some(t) = data.get("time") {
        player.time = t.num_or(0.0);
    }
    if let Some(t) = data.get("timePlayed") {
        player.time_played = t.num_or(0.0);
    }
    if let Some(v) = data.get("keepGoing") {
        player.keep_going = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = data.get("hasNaN") {
        player.has_nan = v.as_bool().unwrap_or(false);
    }
    if let Some(p) = data.get("points") {
        player.points = match p {
            Json::Str(s) => crate::format::dec(s),
            Json::Num(n) => crate::format::dnum(*n),
            _ => player.points,
        };
    }
    if let Some(v) = data.get("lastSafeTab") {
        player.last_safe_tab = v.str_or("none");
    }
    if let Some(v) = data.get("offTime") {
        player.off_time = v.get("remain").map(|r| r.num_or(0.0));
    }
    if let Some(v) = data.get("devSpeed") {
        player.dev_speed = Some(v.num_or(1.0));
    }

    // Extra top-level fields, coerced to the declared default types.
    for (name, default) in &defs.added_player_data {
        if let Some(v) = data.get(name) {
            player.extra.insert(name.to_string(), PVal::coerce_like(default, v));
        }
    }

    if let Some(Json::Obj(notify)) = data.get("notify") {
        for (k, v) in notify {
            player.notify.insert(k.clone(), v.num_or(0.0) as i64);
        }
    }

    if let Some(Json::Obj(subtabs)) = data.get("subtabs") {
        for (layer, fams) in subtabs {
            if let Some(Json::Obj(fams)) = Some(fams) {
                let entry = player.subtabs.entry(layer.clone()).or_default();
                for (fam, tab) in fams {
                    entry.insert(fam.clone(), tab.str_or(""));
                }
            }
        }
    }
    if let Some(Json::Obj(infoboxes)) = data.get("infoboxes") {
        for (layer, boxes) in infoboxes {
            if let Json::Obj(boxes) = boxes {
                let entry = player.infoboxes.entry(layer.clone()).or_default();
                for (b, v) in boxes {
                    entry.insert(b.clone(), v.as_bool().unwrap_or(false));
                }
            }
        }
    }

    for layer_def in &defs.layers {
        let id = layer_def.id;
        if let Some(layer_json) = data.get(id) {
            let s = player.layers.get_mut(id).expect("start state exists");
            layer_from_json(layer_def, layer_json, s);
        }
    }

    fix_save(defs, &mut player);
    player
}

fn layer_from_json(def: &crate::defs::LayerDef, data: &Json, s: &mut LayerState) {
    if data.as_obj().is_none() {
        return;
    }
    if let Some(v) = data.get("unlocked") {
        s.unlocked = v.as_bool().unwrap_or(true);
    }
    for field in ["points", "best", "total", "spentOnBuyables"] {
        if let Some(v) = data.get(field) {
            let d = match v {
                Json::Str(t) => crate::format::dec(t),
                Json::Num(n) => crate::format::dnum(*n),
                _ => Decimal::zero(),
            };
            match field {
                "points" => s.points = d,
                "best" => s.best = d,
                "total" => s.total = d,
                _ => s.spent_on_buyables = d,
            }
        }
    }
    if let Some(v) = data.get("resetTime") {
        s.reset_time = v.num_or(0.0);
    }
    if let Some(Json::Arr(items)) = data.get("upgrades") {
        s.upgrades = items.iter().filter_map(|v| v.as_f64().map(|n| n as i64)).collect();
    }
    if let Some(Json::Arr(items)) = data.get("milestones") {
        s.milestones = items.iter().filter_map(|v| v.as_f64().map(|n| n as i64)).collect();
    }
    if let Some(Json::Arr(items)) = data.get("achievements") {
        s.achievements = items.iter().filter_map(|v| v.as_f64().map(|n| n as i64)).collect();
    }
    if let Some(Json::Obj(buyables)) = data.get("buyables") {
        for (k, v) in buyables {
            let d = match v {
                Json::Str(t) => crate::format::dec(t),
                Json::Num(n) => crate::format::dnum(*n),
                _ => Decimal::zero(),
            };
            s.buyables.insert(k.clone(), d);
        }
    }
    if let Some(Json::Obj(clickables)) = data.get("clickables") {
        for (k, v) in clickables {
            let default = s.clickables.get(k).cloned().unwrap_or(PVal::Str(String::new()));
            s.clickables.insert(k.clone(), PVal::coerce_like(&default, v));
        }
    }
    if let Some(Json::Obj(challenges)) = data.get("challenges") {
        for (k, v) in challenges {
            s.challenges.insert(k.clone(), v.num_or(0.0) as i64);
        }
    }
    if let Some(Json::Obj(grid)) = data.get("grid") {
        for (k, v) in grid {
            if let Ok(id) = k.parse::<i64>() {
                let default = s.grid.get(&id).cloned().unwrap_or(PVal::Num(0.0));
                s.grid.insert(id, PVal::coerce_like(&default, v));
            }
        }
    }
    s.active_challenge = data.get("activeChallenge").and_then(|v| v.as_f64()).map(|n| n as i64);
    s.last_milestone = data.get("lastMilestone").and_then(|v| v.as_f64()).map(|n| n as i64);
    if let Some(v) = data.get("noRespecConfirm") {
        s.no_respec_confirm = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = data.get("forceTooltip") {
        s.force_tooltip = v.as_bool().unwrap_or(false);
    }
    if let Some(v) = data.get("prevTab") {
        s.prev_tab = v.str_or("");
    }
    if let Some(v) = data.get("unlockOrder") {
        s.unlock_order = v.num_or(0.0) as i64;
    }

    // Extra layer fields from startData, coerced to their declared types.
    for (name, default) in &def.start_data {
        if let Some(v) = data.get(name) {
            s.extra.insert(name.to_string(), PVal::coerce_like(default, v));
        }
    }
}

/// fixSave: make sure subtab/infobox references still exist and Decimal-ish
/// fields read from saves are valid.
fn fix_save(defs: &GameDefs, player: &mut Player) {
    for layer_def in &defs.layers {
        let id = layer_def.id;
        if let Some(crate::defs::TabFormat::Tabs(tabs)) = &layer_def.tab_format {
            if let Some(entry) = player.subtabs.get_mut(id) {
                let valid = tabs.iter().map(|t| t.name).collect::<Vec<_>>();
                let current = entry.get("mainTabs").cloned().unwrap_or_default();
                if !valid.contains(&current.as_str()) {
                    if let Some(first) = valid.first() {
                        entry.insert("mainTabs".to_string(), first.to_string());
                    }
                }
            }
        }
        for family in &layer_def.microtabs {
            if let Some(entry) = player.subtabs.get_mut(id) {
                let valid = family.tabs.iter().map(|t| t.name).collect::<Vec<_>>();
                let current = entry.get(family.family).cloned().unwrap_or_default();
                if !valid.contains(&current.as_str()) {
                    if let Some(first) = valid.first() {
                        entry.insert(family.family.to_string(), first.to_string());
                    }
                }
            }
        }
    }
}

/// NaN check over every Decimal in the player (TMT fixNaNs); returns the name
/// of the first invalid field, if any.
pub fn nan_check(player: &Player) -> Option<String> {
    if player.points.mag().is_nan() {
        return Some("points".to_string());
    }
    for (name, v) in &player.extra {
        if let PVal::Dec(d) = v {
            if d.mag().is_nan() {
                return Some(name.clone());
            }
        }
    }
    for (layer, s) in &player.layers {
        let check = |field: &str, d: &Decimal| {
            if d.mag().is_nan() {
                Some(format!("{}:{}", layer, field))
            } else {
                None
            }
        };
        if let Some(x) = check("points", &s.points) {
            return Some(x);
        }
        if let Some(x) = check("best", &s.best) {
            return Some(x);
        }
        if let Some(x) = check("total", &s.total) {
            return Some(x);
        }
        for (k, d) in &s.buyables {
            if d.mag().is_nan() {
                return Some(format!("{}:buyable:{}", layer, k));
            }
        }
        for (k, v) in &s.extra {
            if let PVal::Dec(d) = v {
                if d.mag().is_nan() {
                    return Some(format!("{}:{}", layer, k));
                }
            }
        }
    }
    None
}
