//! WebAssembly FFI — the only boundary between the page and the engine.
//!
//! The page writes a JSON command into wasm memory (`tmt_alloc` + copy +
//! `tmt_call`) and reads back a JSON response (`tmt_result_ptr` plus the
//! length returned by `tmt_call`). No wasm-bindgen needed; this keeps the
//! build to plain `cargo build --target wasm32-unknown-unknown`.

// The engine instance and result buffer live in statics because the
// `extern "C"` entry points need a stable address; wasm is single-threaded
// and every entry point goes through one mutable borrow at a time.
#![allow(static_mut_refs)]

use crate::demo;
use crate::game::Game;
use crate::json::Json;

static mut GAME: Option<Game> = None;
static mut RESULT: Vec<u8> = Vec::new();

fn with_game<R>(f: impl FnOnce(&mut Game) -> R) -> R {
    unsafe {
        let g = GAME.as_mut().expect("engine not initialized");
        f(g)
    }
}

fn set_result(s: String) {
    unsafe {
        RESULT = s.into_bytes();
    }
}

#[no_mangle]
pub extern "C" fn tmt_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn tmt_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

#[no_mangle]
pub extern "C" fn tmt_result_ptr() -> *const u8 {
    unsafe { RESULT.as_ptr() }
}

#[no_mangle]
pub extern "C" fn tmt_call(cmd_ptr: *const u8, cmd_len: usize) -> u32 {
    let cmd = unsafe {
        if cmd_ptr.is_null() {
            ""
        } else {
            match std::str::from_utf8(std::slice::from_raw_parts(cmd_ptr, cmd_len)) {
                Ok(s) => s,
                Err(_) => "",
            }
        }
    };
    let response = dispatch(cmd);
    set_result(response);
    unsafe { RESULT.len() as u32 }
}

fn dispatch(cmd: &str) -> String {
    let parsed = match Json::parse(cmd) {
        Ok(j) => j,
        Err(e) => return Json::obj(vec![("error", Json::s(e))]).to_string(),
    };
    let op = parsed.get("op").map(|v| v.str_or("")).unwrap_or_default();

    match op.as_str() {
        "init" => {
            let now = parsed.get("now").map(|v| v.num_or(0.0)).unwrap_or(0.0);
            let _w = parsed.get("w").map(|v| v.num_or(1024.0)).unwrap_or(1024.0);
            let _h = parsed.get("h").map(|v| v.num_or(768.0)).unwrap_or(768.0);
            let save = parsed.get("save").and_then(|v| v.as_str()).map(|s| s.to_string());
            let options = parsed.get("options").and_then(|v| v.as_str()).map(|s| s.to_string());

            let mut game = Game::new(demo::game_defs());
            if let Some(opts) = options {
                if let Ok(data) = Json::parse(&opts) {
                    apply_options(&mut game, &data);
                }
            }
            if let Some(save) = save {
                if let Ok(data) = Json::parse(&save) {
                    game.player = crate::player::player_from_json(&game.defs, &data);
                }
            } else {
                // Fresh game: no offline time to credit (TMT sets time: Date.now()).
                game.player.time = now;
            }
            game.credit_offline_time(now);
            game.version_check();
            game.update_tmp();
            game.update_tmp();
            // The first response must include tree canvas data (branches).
            game.out.canvas_dirty = true;
            unsafe {
                GAME = Some(game);
            }
            respond()
        }
        "tick" => {
            let now = parsed.get("now").map(|v| v.num_or(0.0)).unwrap_or(0.0);
            let w = parsed.get("w").map(|v| v.num_or(1024.0)).unwrap_or(1024.0);
            let h = parsed.get("h").map(|v| v.num_or(768.0)).unwrap_or(768.0);
            let scrolled = parsed.get("scrolled").and_then(|v| v.as_bool()).unwrap_or(false);
            let mx = parsed.get("mx").map(|v| v.num_or(0.0)).unwrap_or(0.0);
            let my = parsed.get("my").map(|v| v.num_or(0.0)).unwrap_or(0.0);
            with_game(|g| {
                g.set_mouse(mx, my);
                g.tick(now, w, h, scrolled);
            });
            respond()
        }
        "event" => {
            let id = parsed.get("id").map(|v| v.str_or("")).unwrap_or_default();
            let payload = parsed.get("payload").cloned().unwrap_or(Json::Null);
            let confirm = with_game(|g| g.handle_event(&id, &payload));
            if let Some(msg) = confirm {
                // Ask the page to confirm() and re-send with confirmed:true.
                return Json::obj(vec![
                    ("confirm", Json::s(msg)),
                    ("ev", Json::s(id)),
                ])
                .to_string();
            }
            respond()
        }
        "export" => {
            let save = with_game(|g| g.save_json());
            Json::obj(vec![("save", Json::s(save))]).to_string()
        }
        "import" => {
            let data = parsed.get("data").map(|v| v.str_or("")).unwrap_or_default();
            let forced = parsed.get("forced").and_then(|v| v.as_bool()).unwrap_or(false);
            let result = with_game(|g| g.import_save(&data, forced));
            match result {
                Ok(()) => respond(),
                Err(e) if e == "different-mod" => Json::obj(vec![
                    ("confirm", Json::s("This save appears to be for a different mod! Are you sure you want to import?".to_string())),
                    ("ev", Json::s("__import__")),
                    ("data", Json::s(data)),
                ])
                .to_string(),
                Err(e) => Json::obj(vec![("error", Json::s(e))]).to_string(),
            }
        }
        "options" => {
            // Persisted options coming back from localStorage on boot.
            let _ = parsed;
            respond()
        }
        other => Json::obj(vec![("error", Json::s(format!("unknown op: {}", other)))]).to_string(),
    }
}

/// Build the standard response: UI patches plus anything queued in `out`.
fn respond() -> String {
    with_game(|g| {
        let patches = crate::render::render_and_diff(g);
        let extras = g.take_out();
        let mut fields: Vec<(String, Json)> = vec![("p".to_string(), patches)];
        if let Json::Obj(items) = extras {
            for (k, v) in items {
                fields.push((k, v));
            }
        }
        // Always tell the page which theme/tree quality to apply.
        fields.push(("theme".to_string(), Json::s(g.options.theme)));
        fields.push(("hq".to_string(), Json::Bool(g.options.hq_tree)));
        fields.push(("modId".to_string(), Json::s(g.defs.mod_info.id)));
        fields.push(("title".to_string(), Json::s(g.defs.mod_info.name)));
        fields.push(("opts".to_string(), Json::obj(vec![
            ("autosave", Json::Bool(g.options.autosave)),
            ("msDisplay", Json::s(g.options.ms_display.label())),
            ("theme", Json::s(g.options.theme)),
            ("hqTree", Json::Bool(g.options.hq_tree)),
            ("offlineProd", Json::Bool(g.options.offline_prod)),
            ("hideChallenges", Json::Bool(g.options.hide_challenges)),
            ("forceOneTab", Json::Bool(g.options.force_one_tab)),
            ("forceTooltips", Json::Bool(g.options.force_tooltips)),
            ("hideMilestonePopups", Json::Bool(g.options.hide_milestone_popups)),
        ])));
        Json::Obj(fields).to_string()
    })
}

fn apply_options(game: &mut Game, data: &Json) {
    let get_bool = |name: &str, default: bool| data.get(name).and_then(|v| v.as_bool()).unwrap_or(default);
    game.options.autosave = get_bool("autosave", true);
    game.options.hq_tree = get_bool("hqTree", false);
    game.options.offline_prod = get_bool("offlineProd", true);
    game.options.hide_challenges = get_bool("hideChallenges", false);
    game.options.force_one_tab = get_bool("forceOneTab", false);
    game.options.force_tooltips = get_bool("forceTooltips", true);
    game.options.hide_milestone_popups = get_bool("hideMilestonePopups", false);
    if let Some(t) = data.get("theme").map(|v| v.str_or("")) {
        if crate::game::events::THEMES.contains(&t.as_str()) {
            game.options.theme = crate::game::events::THEMES
                .iter()
                .find(|th| **th == t)
                .copied()
                .unwrap_or("default");
        }
    }
    if let Some(m) = data.get("msDisplay").and_then(|v| v.as_str()) {
        game.options.ms_display = match m {
            "last" => crate::game::MsDisplay::Last,
            "automation" => crate::game::MsDisplay::Automation,
            "incomplete" => crate::game::MsDisplay::Incomplete,
            "never" => crate::game::MsDisplay::Never,
            _ => crate::game::MsDisplay::Always,
        };
    }
}
