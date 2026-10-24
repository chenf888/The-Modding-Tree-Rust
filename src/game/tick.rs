//! The main loop — port of gameLoop() and the 50ms setInterval in js/game.js.

use super::Game;
use crate::defs::{PrestigeType, Row};
use crate::format::dnum;
use crate::json::Json;
use crate::player::{nan_check, player_to_json};

impl Game {
    /// One 50ms tick. `now` is Date.now() in ms.
    pub fn tick(&mut self, now: f64, width: f64, height: f64, scrolled: bool) {
        // Endgame check.
        if (self.defs.is_endgame)(self) || self.tmp.game_ended {
            self.tmp.game_ended = true;
        }

        // TMT's gameLoop returns immediately when the game has ended
        // (game.js:405): no time passes, offline time is not consumed and
        // player.time stays stale, so "Keep Going" resumes with a catch-up
        // diff (capped by maxTickLength below).
        if !(self.tmp.game_ended && !self.player.keep_going) {
            // Time since the previous tick, with offline-time consumption.
            let mut diff = if self.last_time == 0.0 {
                0.0
            } else {
                (now - self.last_time) / 1000.0
            };
            if diff.is_nan() || diff < 0.0 {
                diff = 0.0;
            }
            if let Some(remain) = self.player.off_time {
                let limit = self.defs.mod_info.offline_limit * 3600.0;
                let mut remain = remain.min(limit);
                if remain > 0.0 {
                    let offline_diff = (remain / 10.0).max(diff);
                    remain -= offline_diff;
                    diff += offline_diff;
                    self.player.off_time = Some(remain);
                }
                if !self.options.offline_prod || remain <= 0.0 {
                    self.player.off_time = None;
                }
            }
            if let Some(dev) = self.player.dev_speed {
                diff *= dev;
            }
            let diff = diff.min(self.defs.max_tick_length);
            // TMT gameLoop sets player.time = now every frame (game.js:420);
            // only the frozen endgame path leaves it stale on purpose.
            self.player.time = now;
            self.last_time = now;

            // TMT interval order: updateTemp, updateOomps, updateWidth, gameLoop.
            self.update_tmp();
            self.update_oomps(diff);
            self.update_width(width, height, scrolled);
            self.game_loop(diff);

            // fixNaNs.
            if let Some(field) = nan_check(&self.player) {
                self.player.has_nan = true;
                if self.out.nan_alert.is_none() {
                    self.out.nan_alert = Some(field);
                }
            }
        }

        // Autosave every 5 seconds (TMT saveInterval).
        if self.options.autosave && (now - self.last_saved) > 5000.0 {
            self.last_saved = now;
            self.out.pending_save = Some(player_to_json(&self.defs, &self.player).to_string());
        }
    }

    /// gameLoop(diff).
    fn game_loop(&mut self, diff: f64) {
        self.add_time(diff);
        let gain = self.tmp.point_gen * dnum(diff);
        self.player.points = (self.player.points + gain).max(crate::format::Decimal::zero());

        // Collect layer ids in TMT's iteration order: tree rows ascending,
        // then non-numeric rows.
        let mut ordered: Vec<String> = Vec::new();
        for row in self.defs.tree_rows() {
            for id in row {
                ordered.push(id.to_string());
            }
        }
        let mut others: Vec<String> = self
            .defs
            .layers
            .iter()
            .filter(|l| !l.display_row().is_number())
            .map(|l| l.id.to_string())
            .collect();
        ordered.append(&mut others);

        // Passive generation + per-layer update.
        for layer in &ordered {
            self.lstate_mut(layer).reset_time += diff;
            let passive = self.ltmp(layer).passive_generation;
            if let Some(mult) = passive {
                self.generate_points(layer, diff * mult.to_number());
            }
            let update = self.layer_def(layer).update.clone();
            if let Some(f) = update {
                f(self, crate::defs::Arg::Num(diff));
            }
        }

        // Automation, highest row first.
        for layer in ordered.iter().rev() {
            let lt = self.ltmp(layer);
            if lt.auto_prestige && lt.can_reset {
                self.do_reset(layer, false);
            }
            let automate = self.layer_def(layer).automate.clone();
            if let Some(f) = automate {
                f(self);
            }
            if self.ltmp(layer).auto_upgrade {
                self.autobuy_upgrades(layer);
            }
        }

        // Side-layer best tracking (TMT does this in the OTHER_LAYERS loop).
        for layer in &others {
            let best = self.lstate(layer).best.max(self.lstate(layer).points);
            self.lstate_mut(layer).best = best;
        }

        // Milestones & achievements for every layer.
        let all: Vec<String> = self.defs.layers.iter().map(|l| l.id.to_string()).collect();
        for layer in &all {
            if !self.layer_def(layer).milestones.is_empty() {
                self.update_milestones(layer);
            }
            if !self.layer_def(layer).achievements.is_empty() {
                self.update_achievements(layer);
            }
        }
    }

    /// addTime.
    fn add_time(&mut self, diff: f64) {
        self.player.time_played += diff;
    }

    /// updateOomps — points/sec in orders-of-magnitude for huge numbers.
    fn update_oomps(&mut self, diff: f64) {
        self.tmp.oomps_mag = 0;
        let points = self.player.points;
        if points <= dnum(1e100) || diff == 0.0 {
            return;
        }
        let last = self.tmp.last_points;
        if points > last {
            let big = crate::format::dec("10^^8");
            if points >= big {
                let pp = points.slog(None, crate::TetrationMode::Analytic);
                let lp = last.slog(None, crate::TetrationMode::Analytic);
                self.tmp.oomps = (pp - lp) / dnum(diff);
                self.tmp.oomps_mag = -1;
            } else {
                let mut pp = points;
                let mut lp = last;
                let mut mag = 0i32;
                while (pp / lp).log10() / dnum(diff) >= dnum(100.0) && mag <= 5 && lp > crate::format::Decimal::zero() {
                    pp = pp.log10();
                    lp = lp.log10();
                    self.tmp.oomps = (pp - lp) / dnum(diff);
                    mag += 1;
                }
                self.tmp.oomps_mag = mag;
            }
        }
    }

    /// updateWidth — split-screen decision.
    fn update_width(&mut self, width: f64, height: f64, scrolled: bool) {
        let mut split = width >= 1024.0;
        if self.options.force_one_tab {
            split = false;
        }
        if self.player.nav_tab == "none" {
            split = true;
        }
        self.tmp.split_screen = split;
        self.tmp.screen_width = width;
        self.tmp.screen_height = height;
        self.tmp.scrolled = scrolled;
        self.tmp.last_points = self.player.points;
    }

    /// Serialize the current save (export / autosave / manual save).
    pub fn save_json(&self) -> String {
        player_to_json(&self.defs, &self.player).to_string()
    }

    /// Import a save (TMT importSave). Returns Err on a bad payload.
    pub fn import_save(&mut self, text: &str, forced: bool) -> Result<(), String> {
        // JS importSave uses atob; the page decodes base64 before calling here.
        let data = crate::json::Json::parse(text).map_err(|e| format!("invalid save: {}", e))?;
        let start = crate::player::get_start_player(&self.defs);
        let version_type = data.get("versionType").map(|v| v.str_or("")).unwrap_or_default();
        if version_type != start.version_type && !forced {
            return Err("different-mod".to_string());
        }
        self.player = crate::player::player_from_json(&self.defs, &data);
        self.player.version_type = start.version_type;
        self.player.version = self.defs.version.to_string();
        self.player.time = self.last_time;
        self.tmp = crate::game::Tmp::default();
        self.update_tmp();
        self.update_tmp();
        self.out.full_rerender = true;
        self.out.canvas_dirty = true;
        Ok(())
    }

    /// versionCheck — updates the stored version on load.
    pub fn version_check(&mut self) {
        if self.player.version_type != self.defs.mod_info.id {
            self.player.version_type = self.defs.mod_info.id.to_string();
        }
        self.player.version = self.defs.version.to_string();
    }

    /// Offline-time credit for load (TMT load(): offTime.remain += elapsed).
    pub fn credit_offline_time(&mut self, now: f64) {
        if self.options.offline_prod {
            let elapsed = (now - self.player.time) / 1000.0;
            let remain = self.player.off_time.unwrap_or(0.0) + elapsed;
            self.player.off_time = Some(remain);
        }
        self.player.time = now;
        self.last_time = now;
    }

    /// Anything queued for the page since the last call, as JSON.
    pub fn take_out(&mut self) -> Json {
        let mut fields: Vec<(&str, Json)> = Vec::new();

        let popups = std::mem::take(&mut self.out.popups);
        if !popups.is_empty() {
            fields.push((
                "pop",
                Json::Arr(
                    popups
                        .into_iter()
                        .map(|(title, message, kind, color)| {
                            Json::obj(vec![
                                ("title", Json::s(title)),
                                ("message", Json::s(message)),
                                ("type", Json::s(kind)),
                                ("color", Json::s(color)),
                            ])
                        })
                        .collect(),
                ),
            ));
        }

        let particles = std::mem::take(&mut self.out.particles);
        if !particles.is_empty() {
            fields.push((
                "particles",
                Json::Arr(
                    particles
                        .into_iter()
                        .map(|p| {
                            Json::obj(vec![
                                ("kind", Json::s(p.kind)),
                                ("content", Json::s(p.content)),
                                ("x", Json::Num(p.x)),
                                ("y", Json::Num(p.y)),
                                ("count", Json::Num(p.count as f64)),
                                ("spec", crate::json::Json::parse(&p.spec).unwrap_or(Json::Null)),
                            ])
                        })
                        .collect(),
                ),
            ));
        }

        if let Some(save) = self.out.pending_save.take() {
            fields.push(("save", Json::s(save)));
        }

        if self.out.canvas_dirty {
            self.out.canvas_dirty = false;
            fields.push(("tree", crate::render::tree_canvas_data(self)));
        }

        if self.out.full_rerender {
            self.out.full_rerender = false;
            fields.push(("full", Json::Bool(true)));
        }

        if let Some(title) = self.out.title.take() {
            fields.push(("title", Json::s(title)));
        }

        if let Some(nan) = self.out.nan_alert.take() {
            fields.push(("nanAlert", Json::s(nan)));
        }

        Json::obj(fields)
    }
}

impl crate::defs::LayerDef {
    /// The effective display row (displayRow falls back to row).
    pub fn display_row(&self) -> Row {
        self.display_row.unwrap_or(self.row)
    }
}

/// Prestige-type helper used by the renderer.
pub fn is_prestige_layer(t: PrestigeType) -> bool {
    t != PrestigeType::None
}
