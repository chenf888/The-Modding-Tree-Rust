//! Event routing — everything the page can send back: clicks, tab switches,
//! inputs, hotkeys and the options tab.

use super::Game;
use crate::defs::Row;
use crate::json::Json;

pub const THEMES: [&str; 2] = ["default", "aqua"];

impl Game {
    /// Handle an event from the page. Returns `Some(message)` when a JS
    /// confirm() is required first; the page re-sends the same event with
    /// `confirmed: true` after the user accepts.
    pub fn handle_event(&mut self, ev: &str, payload: &Json) -> Option<String> {
        let confirmed = payload.get("confirmed").and_then(|v| v.as_bool()).unwrap_or(false);
        let shift = payload.get("shift").and_then(|v| v.as_bool()).unwrap_or(false);
        let value = payload.get("value").cloned().unwrap_or(Json::Null);

        let parts: Vec<&str> = ev.split(':').collect();
        match parts.first().copied().unwrap_or("") {
            // ----- component clicks -----
            "upg" if parts.len() == 3 => {
                if let Ok(id) = parts[2].parse::<i64>() {
                    self.buy_upg(parts[1], id);
                }
            }
            "buy" if parts.len() == 3 => self.buy_buyable(parts[1], parts[2]),
            "buymax" if parts.len() == 3 => self.buy_max_buyable(parts[1], parts[2]),
            "sell1" if parts.len() == 3 => self.sell_one(parts[1], parts[2]),
            "sellall" if parts.len() == 3 => self.sell_all(parts[1], parts[2]),
            "click" if parts.len() == 3 => self.click_clickable(parts[1], parts[2]),
            "holdclick" if parts.len() == 3 => self.hold_clickable(parts[1], parts[2]),
            "holdbuy" if parts.len() == 3 => self.hold_buyable(parts[1], parts[2]),
            "grid" if parts.len() == 3 => {
                if let Ok(id) = parts[2].parse::<i64>() {
                    self.click_grid(parts[1], id);
                }
            }
            "holdgrid" if parts.len() == 3 => {
                if let Ok(id) = parts[2].parse::<i64>() {
                    self.hold_grid(parts[1], id);
                }
            }
            "master" if parts.len() == 2 => {
                let f = self.layer_def(parts[1]).clickables.master_button_press.clone();
                if let Some(f) = f {
                    f(self);
                }
            }
            "prestige" if parts.len() == 2 => self.do_reset(parts[1], false),
            "challenge" if parts.len() == 3 => {
                if let Ok(id) = parts[2].parse::<i64>() {
                    self.start_challenge(parts[1], id);
                }
            }
            "respec" if parts.len() == 2 => {
                if !self.lstate(parts[1]).no_respec_confirm && !confirmed {
                    let msg = self.layer_def(parts[1]).buyables.respec_message.clone();
                    return Some(msg);
                }
                self.respec_buyables(parts[1]);
            }

            // ----- navigation -----
            "node" if parts.len() == 2 => self.node_click(parts[1], shift),
            "tab" if parts.len() == 2 => self.show_tab(parts[1]),
            "navtab" if parts.len() == 2 => self.show_nav_tab(parts[1], ""),
            "back" if parts.len() == 2 => self.go_back(parts[1]),
            "subtab" if parts.len() == 4 => {
                let (layer, family, tab) = (parts[1], parts[2], parts[3]);
                if let Some(fams) = self.player.subtabs.get_mut(layer) {
                    fams.insert(family.to_string(), tab.to_string());
                }
                self.out.canvas_dirty = true;
            }
            "infobox" if parts.len() == 3 => {
                if let Some(boxes) = self.player.infoboxes.get_mut(parts[1]) {
                    if let Some(v) = boxes.get_mut(parts[2]) {
                        *v = !*v;
                    }
                }
            }

            // ----- inputs -----
            "toggle" if parts.len() == 3 => self.toggle_auto(parts[1], parts[2]),
            "input" if parts.len() == 3 => {
                let text = value.str_or("");
                self.set_player_value(parts[1], parts[2], &text);
            }
            "slider" if parts.len() == 3 => {
                let v = value.num_or(0.0);
                if let Some(s) = self.player.layers.get_mut(parts[1]) {
                    s.extra.insert(
                        parts[2].to_string(),
                        crate::player::PVal::Num(v),
                    );
                }
            }
            "dropdown" if parts.len() == 3 => {
                let text = value.str_or("");
                if let Some(s) = self.player.layers.get_mut(parts[1]) {
                    s.extra
                        .insert(parts[2].to_string(), crate::player::PVal::Str(text));
                }
            }

            // ----- hotkeys -----
            "key" if parts.len() >= 2 => {
                // ev is "key:p" or "key:ctrl+c" — rebuild the key name.
                let key = ev["key:".len()..].to_string();
                self.press_hotkey(&key);
            }

            // ----- options tab -----
            "opt" => {
                self.handle_option(parts.get(1).copied().unwrap_or(""), confirmed);
            }

            // ----- particles (raw-html buttons call window.TMT.particles) -----
            "particles" if parts.len() == 2 => match parts[1] {
                "text" => {
                    let content = format!(
                        "<h1 style='color:yellow'>{}</h1>",
                        crate::format::format(self.player.points, 2, false)
                    );
                    self.make_particles(
                        "text",
                        content,
                        1,
                        r#"{"spread":20,"gravity":0,"time":3,"speed":0,"offset":30,"fadeInTime":1}"#.to_string(),
                    );
                }
                _ => {}
            },

            // ----- endgame -----
            "keepgoing" => {
                self.player.keep_going = true;
                self.out.canvas_dirty = true;
            }

            other => {
                let _ = other;
            }
        }
        None
    }

    fn handle_option(&mut self, what: &str, confirmed: bool) -> Option<String> {
        match what {
            "save" => {
                let save = self.save_json();
                self.out.pending_save = Some(save);
            }
            "autosave" => self.options.autosave = !self.options.autosave,
            "hardreset" => {
                if !confirmed {
                    return Some(
                        "Are you sure you want to do this? You will lose all your progress!"
                            .to_string(),
                    );
                }
                self.hard_reset();
            }
            "offlineprod" => self.options.offline_prod = !self.options.offline_prod,
            "theme" => {
                let idx = THEMES
                    .iter()
                    .position(|t| *t == self.options.theme)
                    .unwrap_or(0);
                self.options.theme = THEMES[(idx + 1) % THEMES.len()];
            }
            "msdisplay" => self.options.ms_display = self.options.ms_display.next(),
            "hqtree" => self.options.hq_tree = !self.options.hq_tree,
            "hidechallenges" => self.options.hide_challenges = !self.options.hide_challenges,
            "forceonetab" => {
                self.options.force_one_tab = !self.options.force_one_tab;
                self.out.canvas_dirty = true;
            }
            "forcetooltips" => {
                self.options.force_tooltips = !self.options.force_tooltips;
                self.out.canvas_dirty = true;
            }
            "hidemilestonepopups" => {
                self.options.hide_milestone_popups = !self.options.hide_milestone_popups;
            }
            _ => {}
        }
        None
    }

    /// tree-node click behaviour (systemComponents 'tree-node').
    fn node_click(&mut self, layer: &str, shift: bool) {
        if shift && self.options.force_tooltips {
            let s = self.lstate_mut(layer);
            s.force_tooltip = !s.force_tooltip;
            return;
        }
        let def = self.layer_def(layer);
        if def.is_layer {
            if def.left_tab {
                self.show_nav_tab(layer, "");
                self.show_tab("none");
            } else {
                self.show_tab(layer);
            }
        } else {
            let on_click = def.node_on_click.clone();
            if let Some(f) = on_click {
                f(self);
            }
        }
    }

    /// showTab.
    pub fn show_tab(&mut self, name: &str) {
        let is_layer = self.defs.layers.iter().any(|l| l.id == name);
        if is_layer && !self.layer_unlocked(name) {
            return;
        }
        // Reopening a subtabbed layer resets to its first subtab.
        if self.player.tab == name && is_layer {
            let first = match &self.layer_def(name).tab_format {
                Some(crate::defs::TabFormat::Tabs(tabs)) => {
                    tabs.first().map(|t| t.name.to_string())
                }
                _ => None,
            };
            if let Some(first) = first {
                if let Some(fams) = self.player.subtabs.get_mut(name) {
                    fams.insert("mainTabs".to_string(), first);
                }
            }
        }
        self.player.tab = name.to_string();
        if is_layer {
            let row = self.ltmp(name).row;
            if row != Row::Side && row != Row::Otherside {
                self.player.last_safe_tab = name.to_string();
            }
        }
        self.out.canvas_dirty = true;
    }

    /// showNavTab.
    pub fn show_nav_tab(&mut self, name: &str, prev: &str) {
        let is_layer = self.defs.layers.iter().any(|l| l.id == name);
        if is_layer && !self.layer_unlocked(name) {
            return;
        }
        let mut prev = prev.to_string();
        if is_layer {
            let def_prev = self.layer_def(name).previous_tab.to_string();
            if !def_prev.is_empty() {
                prev = def_prev;
            }
        }
        if name != "none" && !prev.is_empty() && self.defs.layers.iter().any(|l| l.id == prev) {
            let prev_left = self.layer_def(&prev).left_tab;
            let name_left = if is_layer { self.layer_def(name).left_tab } else { false };
            if !prev_left == !name_left {
                if let Some(s) = self.player.layers.get_mut(name) {
                    s.prev_tab = prev.clone();
                }
            } else if let Some(s) = self.player.layers.get_mut(name) {
                s.prev_tab = String::new();
            }
        } else if let Some(s) = self.player.layers.get_mut(name) {
            s.prev_tab = String::new();
        }
        self.player.nav_tab = name.to_string();
        self.out.canvas_dirty = true;
    }

    /// goBack.
    pub fn go_back(&mut self, layer: &str) {
        let mut next = "none".to_string();
        if !self.lstate(layer).prev_tab.is_empty() {
            next = self.lstate(layer).prev_tab.clone();
        }
        let row = self.ltmp(layer).row;
        if self.player.nav_tab == "none" && (row == Row::Side || row == Row::Otherside) {
            next = self.player.last_safe_tab.clone();
        }
        if self.defs.layers.iter().any(|l| l.id == layer) && self.layer_def(layer).left_tab {
            self.show_nav_tab(&next, layer);
        } else {
            self.show_tab(&next);
        }
    }

    /// Hotkey dispatch (TMT document.onkeydown).
    pub fn press_hotkey(&mut self, key: &str) {
        if self.tmp.game_ended && !self.player.keep_going {
            return;
        }
        let entry = self.defs.hotkeys().get(key).copied();
        if let Some((layer, li, hi)) = entry {
            let unlocked = (self.defs.layers[li].hotkeys[hi].unlocked)(self);
            if self.lstate(layer).unlocked && unlocked {
                let f = self.defs.layers[li].hotkeys[hi].on_press.clone();
                f(self);
            }
        }
    }
}
