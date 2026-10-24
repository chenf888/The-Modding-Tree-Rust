//! UI rendering: the whole app is described as a tree of `UiNode`s, diffed
//! against the previous tick's tree. The page (`glue.js`) builds DOM from the
//! serialized tree once and then applies the patch list — this replaces
//! Vue.js and is what makes many-component games fast.

use crate::defs::*;
use crate::format::{dec, format, format_time, format_whole, Decimal};
use crate::game::Game;
use crate::json::Json;

impl UiNode {
    fn children_push(mut self, child: UiNode) -> UiNode {
        self.children.push(child);
        self
    }
}

pub struct UiNode {
    pub id: String,
    pub tag: &'static str,
    pub class: String,
    pub style: String,
    pub html: String,
    pub attrs: Vec<(&'static str, String)>,
    pub ev: Option<String>,
    pub children: Vec<UiNode>,
}

impl UiNode {
    pub fn new(id: &str, tag: &'static str) -> UiNode {
        UiNode {
            id: id.to_string(),
            tag,
            class: String::new(),
            style: String::new(),
            html: String::new(),
            attrs: Vec::new(),
            ev: None,
            children: Vec::new(),
        }
    }

    pub fn class(mut self, c: impl Into<String>) -> UiNode {
        self.class = c.into();
        self
    }
    pub fn style(mut self, s: impl Into<String>) -> UiNode {
        self.style = s.into();
        self
    }
    pub fn html(mut self, h: String) -> UiNode {
        self.html = h;
        self
    }
    pub fn ev(mut self, e: impl Into<String>) -> UiNode {
        self.ev = Some(e.into());
        self
    }
    pub fn attr(mut self, k: &'static str, v: String) -> UiNode {
        self.attrs.push((k, v));
        self
    }

    pub fn to_json(&self) -> Json {
        let mut fields: Vec<(&str, Json)> = vec![
            ("t", Json::s(self.tag)),
            ("i", Json::s(self.id.clone())),
        ];
        if !self.class.is_empty() {
            fields.push(("c", Json::s(self.class.clone())));
        }
        if !self.style.is_empty() {
            fields.push(("s", Json::s(self.style.clone())));
        }
        if !self.html.is_empty() {
            fields.push(("x", Json::s(self.html.clone())));
        }
        if !self.attrs.is_empty() {
            fields.push((
                "a",
                Json::Obj(
                    self.attrs
                        .iter()
                        .map(|(k, v)| (k.to_string(), Json::s(v.clone())))
                        .collect(),
                ),
            ));
        }
        if let Some(ev) = &self.ev {
            fields.push(("e", Json::s(ev.clone())));
        }
        if !self.children.is_empty() {
            fields.push(("k", Json::Arr(self.children.iter().map(|c| c.to_json()).collect())));
        }
        Json::obj(fields)
    }
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Build the full app tree and diff it against the previous one.
/// Returns the patch array (or the full tree when this is the first render).
pub fn render_and_diff(g: &mut Game) -> Json {
    let new_tree = render_app(g);
    let result = match &g.ui_tree {
        None => Json::obj(vec![("full", new_tree.to_json())]),
        Some(old) => {
            // Every id that still exists somewhere in the new tree. A child
            // that moved to a different parent must never be "rm"'d by its
            // old parent (the rm would land after the ins moved the element).
            let mut live_ids = std::collections::HashSet::new();
            collect_ids(&new_tree, &mut live_ids);
            let mut patches: Vec<Json> = Vec::new();
            diff(old, &new_tree, &live_ids, &mut patches);
            Json::Arr(patches)
        }
    };
    g.ui_tree = Some(new_tree);
    result
}

fn collect_ids(n: &UiNode, set: &mut std::collections::HashSet<String>) {
    set.insert(n.id.clone());
    for c in &n.children {
        collect_ids(c, set);
    }
}

fn diff(old: &UiNode, new: &UiNode, live_ids: &std::collections::HashSet<String>, out: &mut Vec<Json>) {
    if old.id != new.id || old.tag != new.tag {
        // Should not happen with stable ids; caller replaces children by id.
        return;
    }
    if old.class != new.class {
        out.push(Json::Arr(vec![Json::s("a"), Json::s(new.id.clone()), Json::s("class"), Json::s(new.class.clone())]));
    }
    if old.style != new.style {
        out.push(Json::Arr(vec![Json::s("a"), Json::s(new.id.clone()), Json::s("style"), Json::s(new.style.clone())]));
    }
    if old.html != new.html {
        out.push(Json::Arr(vec![Json::s("a"), Json::s(new.id.clone()), Json::s("html"), Json::s(new.html.clone())]));
    }
    if old.ev != new.ev {
        out.push(Json::Arr(vec![Json::s("a"), Json::s(new.id.clone()), Json::s("ev"), Json::s(new.ev.clone().unwrap_or_default())]));
    }
    // Plain attributes: diff by key against the old set.
    for (k, v) in &new.attrs {
        let old_v = old.attrs.iter().find(|(ok, _)| ok == k).map(|(_, ov)| ov);
        if old_v != Some(v) {
            out.push(Json::Arr(vec![Json::s("a"), Json::s(new.id.clone()), Json::s(*k), Json::s(v.clone())]));
        }
    }
    for (k, _) in &old.attrs {
        if !new.attrs.iter().any(|(nk, _)| nk == k) {
            out.push(Json::Arr(vec![Json::s("a"), Json::s(new.id.clone()), Json::s(*k), Json::Null]));
        }
    }

    // Children: keyed reconciliation by id, in order.
    let old_ids: Vec<&str> = old.children.iter().map(|c| c.id.as_str()).collect();
    let new_ids: Vec<&str> = new.children.iter().map(|c| c.id.as_str()).collect();
    if old_ids == new_ids {
        for (o, n) in old.children.iter().zip(new.children.iter()) {
            diff(o, n, live_ids, out);
        }
        return;
    }
    // Insertions / moves: emit "ins" for each new child not at the right
    // place, then removals for children that vanished. A child that moved to
    // a different parent still lives somewhere in the new tree, so the rm is
    // checked against the whole new tree, not just this parent's children.
    for (j, n) in new.children.iter().enumerate() {
        let already = old.children.get(j).map(|o| o.id == n.id).unwrap_or(false);
        if !already {
            out.push(Json::Arr(vec![
                Json::s("ins"),
                Json::s(new.id.clone()),
                Json::Num(j as f64),
                n.to_json(),
            ]));
        }
    }
    for o in &old.children {
        if !live_ids.contains(&o.id) {
            out.push(Json::Arr(vec![Json::s("rm"), Json::s(new.id.clone()), Json::s(o.id.clone())]));
        }
    }
    // Deep-diff the children that line up by id (same id anywhere).
    for n in &new.children {
        if let Some(o) = old.children.iter().find(|o| o.id == n.id) {
            diff(o, n, live_ids, out);
        }
    }
}

// ---------------------------------------------------------------------------
// The app frame
// ---------------------------------------------------------------------------

fn fmt(g: &Game, d: Decimal) -> String {
    format(d, 2, g.defs.mod_info.allow_small)
}

fn fmt_whole(g: &Game, d: Decimal) -> String {
    let _ = g;
    format_whole(d)
}

fn render_app(g: &Game) -> UiNode {
    // Not "app": index.html already has a static #app container, and ids must
    // stay unique for the glue-side getElementById heal to be unambiguous.
    let mut root = UiNode::new("tmtroot", "div");

    let ended = g.tmp.game_ended && !g.player.keep_going;

    if ended {
        root.children.push(render_endgame(g));
        return root;
    }

    // Canvas is JS-owned but must exist in the tree.
    let canvas = UiNode::new("treeCanvas", "canvas").class("canvas");
    root.children.push(canvas);

    let split = g.tmp.split_screen;

    // Left pane (treeTab): visible when tab is none or split screen.
    let show_left = g.player.tab == "none" || split;
    if show_left {
        let left_layer = if g.player.nav_tab == "none" {
            g.player.tab.clone()
        } else {
            g.player.nav_tab.clone()
        };
        let cls = if g.player.tab == "none" || g.player.nav_tab == "none" {
            "fullWidth"
        } else {
            "col left"
        };
        let mut left = UiNode::new("treeTab", "div").class(cls);
        let mut spacer = UiNode::new("treeTab-spacer", "div");
        spacer.style = "height:150px".to_string();
        left.children.push(spacer);
        if left_layer != "none" {
            left.children.push(render_layer_tab(g, &left_layer, false, false));
        }
        root.children.push(left);
    }

    // Vertical divider.
    if g.player.nav_tab != "none" && split && g.player.tab != "none" {
        root.children.push(UiNode::new("vl", "div").class("vl"));
    }

    // Right pane: visible when navTab and tab both set.
    if g.player.nav_tab != "none" && g.player.tab != "none" && split {
        let mut right = UiNode::new("rightTab", "div").class("col right fast tab");
        let tab = g.player.tab.clone();
        if g.defs.layers.iter().any(|l| l.id == tab) {
            right.children.push(render_layer_tab(g, &tab, true, false));
        }
        root.children.push(right);
    }

    // Overlay (version, wheel, info, points header, side layers).
    root.children.push(render_overlay(g));

    // Background styles layer.
    let bg2 = UiNode::new("bg2", "div").class("bg2").style(g.tmp.background_style.clone());
    root.children.push(bg2);

    root
}

fn render_endgame(g: &Game) -> UiNode {
    let mut n = UiNode::new("endgame", "div").class("fullWidth");
    n.html = format!(
        "<br><h2>{name} v{version}</h2><br><br><h3>{win}</h3><br>\
         <h3>Please check the Discord to see if there are new content updates!</h3><br><br>\
         <div>It took you {time} to beat the game.</div><br>\
         <button class=\"longUpg can\" data-ev=\"opt:hardreset\">Play Again</button>&nbsp;&nbsp;&nbsp;&nbsp;\
         <button class=\"longUpg can\" data-ev=\"keepgoing\">Keep Going</button><br><br><br>",
        name = g.defs.mod_info.name,
        version = g.defs.version,
        win = g.defs.win_text,
        time = format_time(g.player.time_played),
    );
    n
}

fn render_overlay(g: &Game) -> UiNode {
    let mut o = UiNode::new("treeOverlay", "div").class("treeOverlay");
    let mut cls = "treeOverlay".to_string();
    let show_tree = g.defs.show_tree;
    let overlay_visible = g.player.tab == "none" || g.tmp.split_screen || !show_tree;
    if !overlay_visible {
        cls = "treeOverlay hidden".to_string();
    }
    if g.player.tab == "none" || g.player.nav_tab == "none" {
        cls = format!("{} fullWidth", cls);
    } else {
        cls = format!("{} col left", cls);
    }
    o.class = cls.clone();

    // Version.
    let version = UiNode::new("version", "div")
        .class("overlayThing")
        .style("margin-right:13px")
        .ev("tab:changelog-tab")
        .html(format!("v{}", g.defs.version));
    o.children.push(version);

    // Back button (side/otherside layers on the tree view).
    let tab = g.player.tab.clone();
    let nav_tab = g.player.nav_tab.clone();
    let mut show_back = false;
    let back_layer = if nav_tab == "none" { tab.clone() } else { nav_tab.clone() };
    if g.defs.layers.iter().any(|l| l.id == back_layer) {
        let lt = g.ltmp(&back_layer);
        if nav_tab == "none" && (lt.row == crate::defs::Row::Side || lt.row == crate::defs::Row::Otherside)
            || (!g.lstate(&back_layer).prev_tab.is_empty() && nav_tab != "none")
        {
            show_back = true;
        }
    }
    if show_back {
        o.children.push(
            UiNode::new("backBtn", "button")
                .class("other-back overlayThing")
                .ev(format!("back:{}", back_layer))
                .html("←".to_string()),
        );
    }

    // Option wheel / info icon.
    if g.player.tab != "options-tab" {
        o.children.push(
            UiNode::new("optionWheel", "img")
                .class("overlayThing")
                .attr("src", "resources/options_wheel.png".to_string())
                .ev("tab:options-tab"),
        );
    }
    if g.player.tab != "info-tab" {
        o.children.push(
            UiNode::new("info", "div")
                .class("overlayThing")
                .ev("tab:info-tab")
                .html("<br>i".to_string()),
        );
    }

    // Points header.
    o.children.push(render_overlay_head(g));

    // Side layers.
    let mut side = UiNode::new("sideLayers", "div").class("sideLayers");
    for id in g.defs.side_layers() {
        if g.ltmp(id).layer_shown == LayerShown::Shown {
            side.children.push(render_tree_node(g, id));
        }
    }
    o.children.push(side);

    o
}

fn render_overlay_head(g: &Game) -> UiNode {
    let mut h = UiNode::new("overlayHead", "div")
        .class("overlayThing")
        .style("padding-bottom:7px;width:90%;z-index:1000;position:relative");

    let mut html = String::from("<br>");
    if let Some(dev) = g.player.dev_speed {
        if dev != 1.0 {
            html += &format!("<br>Dev Speed: {}x<br>", fmt(g, d_num(dev)));
        }
    }
    if let Some(remain) = g.player.off_time {
        html += &format!("<br>Offline Time: {}<br>", format_time(remain));
    }
    html += "<br>";
    if g.player.points < dec("1e1000") {
        html += "You have ";
    }
    html += &format!("<h2 id=\"points\">{}</h2>", fmt(g, g.player.points));
    if g.player.points < dec("1e1e6") {
        html += &format!(" {}", g.defs.mod_info.points_name);
    }
    html += "<br>";
    if (g.defs.can_gen_points)(g) {
        let rate = if g.tmp.oomps_mag != 0 {
            let mag = g.tmp.oomps_mag;
            let suffix = if mag < 0 {
                "^OOM".to_string()
            } else if mag > 1 {
                format!("^{}", mag)
            } else {
                String::new()
            };
            format!("{} OOM{}s", fmt(g, g.tmp.oomps), suffix)
        } else {
            format!("{}/sec", crate::format::format_small(g.tmp.point_gen, 2))
        };
        html += &format!("({})", rate);
    }
    for thing in &g.tmp.display_things {
        if !thing.is_empty() {
            html += &format!("<div>{}</div>", thing);
        }
    }

    h.html = html;
    h
}

// ---------------------------------------------------------------------------
// Layer tabs
// ---------------------------------------------------------------------------

fn layer_bg_style(g: &Game, layer: &str) -> String {
    // TMT 'bg' component: layer style + current subtab style.
    let mut style = g.ltmp(layer).style.clone();
    if let Some(TabFormat::Tabs(tabs)) = &g.layer_def(layer).tab_format {
        let current = g
            .player
            .subtabs
            .get(layer)
            .and_then(|f| f.get("mainTabs"))
            .cloned()
            .unwrap_or_default();
        if let Some(tab) = tabs.iter().find(|t| t.name == current) {
            let tab_style = (tab.style)(g);
            if !tab_style.is_empty() {
                style = format!("{};{}", style, tab_style);
            }
        }
    }
    style
}

fn render_layer_tab(g: &Game, layer: &str, back: bool, embedded: bool) -> UiNode {
    let mut n = UiNode::new(&format!("lt-{}", layer), "div").class("noBackground");
    n.style = layer_bg_style(g, layer);

    if back {
        n.children.push(
            UiNode::new(&format!("back-{}", layer), "button")
                .class("back")
                .ev(format!("back:{}", layer))
                .html("←".to_string()),
        );
    }

    // Background div (absolute, behind content).
    n.children.push(
        UiNode::new(&format!("bg-{}", layer), "div").class("bg").style(layer_bg_style(g, layer)),
    );

    let def = g.layer_def(layer);
    let lt = g.ltmp(layer);

    match &def.tab_format {
        None => {
            // Default layout.
            if !def.infoboxes.is_empty() {
                let first = def.infoboxes[0].id;
                n.children.push(render_infobox(g, layer, first));
            }
            n.children.push(render_main_display(g, layer));
            if lt.prestige_type != PrestigeType::None {
                n.children.push(render_prestige_button(g, layer));
            }
            n.children.push(render_resource_display(g, layer));
            n.children.push(render_milestones(g, layer));
            if !def.midsection.is_empty() {
                n.children.push(render_column(g, layer, &def.midsection, &format!("mid-{}", layer)));
            }
            n.children.push(render_clickables(g, layer));
            n.children.push(render_buyables(g, layer));
            n.children.push(render_upgrades(g, layer));
            n.children.push(render_challenges(g, layer));
            n.children.push(render_achievements(g, layer));
        }
        Some(TabFormat::Array(content)) => {
            let mut col = render_column(g, layer, content, &format!("tf-{}", layer));
            if !embedded {
                col.style = format!("padding-top:25px;{}", col.style);
            }
            n.children.push(col);
        }
        Some(TabFormat::Tabs(tabs)) => {
            // Subtab buttons row.
            let mut button_row = UiNode::new(&format!("tb-{}", layer), "div").class("upgTable");
            button_row.style = "padding-top:25px;margin-bottom:24px".to_string();
            button_row.children.push(render_tab_buttons(g, layer, "mainTabs"));
            n.children.push(button_row);

            let current = g
                .player
                .subtabs
                .get(layer)
                .and_then(|f| f.get("mainTabs"))
                .cloned()
                .unwrap_or_default();
            if let Some(tab) = tabs.iter().find(|t| t.name == current) {
                match tab.embed_layer {
                    Some(embed) => {
                        n.children.push(render_layer_tab(g, embed, false, true));
                    }
                    None => {
                        let style = (tab.style)(g);
                        n.children
                            .push(render_column(g, layer, &tab.content, &format!("tf-{}", layer)).style(style));
                    }
                }
            }
        }
    }

    n.children.push(UiNode::new(&format!("tail-{}", layer), "div").html("<br><br>".to_string()));
    n
}

fn render_tab_buttons(g: &Game, layer: &str, family: &str) -> UiNode {
    let mut row = UiNode::new(&format!("tbs-{}-{}", layer, family), "div").class("upgRow");
    let tabs: Vec<&SubtabDef> = if family == "mainTabs" {
        match &g.layer_def(layer).tab_format {
            Some(TabFormat::Tabs(tabs)) => tabs.iter().collect(),
            _ => Vec::new(),
        }
    } else {
        g.layer_def(layer)
            .microtabs
            .iter()
            .find(|f| f.family == family)
            .map(|f| f.tabs.iter().collect())
            .unwrap_or_default()
    };

    let _current = g
        .player
        .subtabs
        .get(layer)
        .and_then(|f| f.get(family))
        .cloned()
        .unwrap_or_default();

    for tab in tabs {
        if !(tab.unlocked)(g) {
            continue;
        }
        let notify = g.subtab_should_notify(layer, family, tab.name);
        let reset_notify = g.subtab_reset_notify(layer, family, tab.name);
        let mut class = "tabButton".to_string();
        if notify {
            class += " notify";
        }
        if reset_notify {
            class += " resetNotify";
        }
        let mut style = format!("border-color:{}", g.ltmp(layer).color);
        if notify {
            let glow = tab.glow_color.unwrap_or("#ff0000");
            style += &format!(";box-shadow:var(--hqProperty2a), 0 0 20px {}", glow);
        }
        let button_style = (tab.button_style)(g);
        if !button_style.is_empty() {
            style += &format!(";{}", button_style);
        }
        row.children.push(
            UiNode::new(&format!("tb-{}-{}-{}", layer, family, tab.name), "button")
                .class(&class)
                .style(&style)
                .ev(format!("subtab:{}:{}:{}", layer, family, tab.name))
                .html(tab.name.to_string()),
        );
    }
    row
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

fn render_main_display(g: &Game, layer: &str) -> UiNode {
    let mut n = UiNode::new(&format!("md-{}", layer), "div");
    let lt = g.ltmp(layer);
    let mut html = String::new();
    if g.lstate(layer).points < dec("1e1000") {
        html += "You have ";
    }
    html += &format!(
        "<h2 style=\"color:{};text-shadow:0px 0px 10px {}\">{}</h2> {}",
        lt.color,
        lt.color,
        fmt_whole(g, g.lstate(layer).points),
        lt.resource
    );
    if !lt.effect_description.is_empty() {
        html += &format!(", {}", lt.effect_description);
    }
    html += "<br><br>";
    n.html = html;
    n
}

fn render_resource_display(g: &Game, layer: &str) -> UiNode {
    let mut n = UiNode::new(&format!("rd-{}", layer), "div");
    n.style = "margin-top:-13px".to_string();
    let lt = g.ltmp(layer);
    let mut html = String::new();
    if lt.base_amount != Decimal::zero() || !lt.base_resource.is_empty() {
        if !lt.base_resource.is_empty() {
            html += &format!(
                "<br>You have {} {}",
                fmt_whole(g, lt.base_amount),
                lt.base_resource
            );
        }
    }
    if let Some(passive) = lt.passive_generation {
        html += &format!(
            "<br>You are gaining {} {} per second",
            fmt(g, lt.reset_gain * passive),
            lt.resource
        );
    }
    html += "<br><br>";
    if lt.show_best {
        html += &format!("Your best {} is {}<br>", lt.resource, fmt_whole(g, g.lstate(layer).best));
    }
    if lt.show_total {
        html += &format!(
            "You have made a total of {} {}<br>",
            fmt_whole(g, g.lstate(layer).total),
            lt.resource
        );
    }
    n.html = html;
    n
}

fn prestige_button_text(g: &Game, layer: &str) -> String {
    let def = g.layer_def(layer);
    if let Some(f) = &def.prestige_button_text {
        return f(g);
    }
    let lt = g.ltmp(layer);
    match lt.prestige_type {
        PrestigeType::Normal => {
            let mut s = String::new();
            if g.lstate(layer).points < d_num(1000.0) {
                s += def.reset_description.unwrap_or("Reset for ");
            }
            s += &format!("+<b>{}</b> {}", fmt_whole(g, lt.reset_gain), lt.resource);
            if lt.reset_gain < d_num(100.0) && g.lstate(layer).points < d_num(1000.0) {
                let next = if lt.round_up_cost {
                    fmt_whole(g, lt.next_at)
                } else {
                    fmt(g, lt.next_at)
                };
                s += &format!("<br><br>Next at {} {}", next, lt.base_resource);
            }
            s
        }
        PrestigeType::Static => {
            let mut s = String::new();
            s += def.reset_description.unwrap_or("Reset for ");
            s += &format!("+<b>{}</b> {}", fmt_whole(g, lt.reset_gain), lt.resource);
            s += "<br><br>";
            if g.lstate(layer).points < d_num(30.0) {
                let prefix = if lt.base_amount >= lt.next_at && lt.can_buy_max == Some(true) {
                    "Next:"
                } else {
                    "Req:"
                };
                s += prefix;
            }
            let next_disp = if lt.round_up_cost {
                fmt_whole(g, lt.next_at_disp)
            } else {
                fmt(g, lt.next_at_disp)
            };
            s += &format!(" {} / {} {}", fmt_whole(g, lt.base_amount), next_disp, lt.base_resource);
            s
        }
        _ => String::new(),
    }
}

fn render_prestige_button(g: &Game, layer: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let can = lt.can_reset;
    let mut class = format!("reset {}", layer);
    if can {
        class += " can";
    } else {
        class += " locked";
    }
    let mut style = String::new();
    if can {
        style = format!("background-color:{}", lt.color);
    }
    let component_style = (&g.layer_def(layer).component_styles.prestige_button)(g);
    if !component_style.is_empty() {
        style += &format!(";{}", component_style);
    }
    UiNode::new(&format!("pb-{}", layer), "button")
        .class(&class)
        .style(&style)
        .ev(format!("prestige:{}", layer))
        .html(prestige_button_text(g, layer))
}

fn render_upgrades(g: &Game, layer: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let mut table = UiNode::new(&format!("upgs-{}", layer), "div").class("upgTable");
    if g.layer_def(layer).upgrades.is_empty() {
        return table;
    }
    for row in 1..=lt.upgrade_rows.max(1) {
        let mut row_n = UiNode::new(&format!("upgsr-{}-{}", layer, row), "div").class("upgRow");
        let mut any = false;
        for col in 1..=lt.upgrade_cols.max(1) {
            let id = row * 10 + col;
            if lt.upgrades.get(&id).map(|u| u.unlocked).unwrap_or(false) {
                any = true;
                let mut align = UiNode::new(&format!("upga-{}-{}", layer, id), "div").class("upgAlign");
                align.children.push(render_upgrade(g, layer, id));
                row_n.children.push(align);
            }
        }
        if any {
            table.children.push(row_n);
        }
    }
    table
}

fn render_upgrade(g: &Game, layer: &str, id: i64) -> UiNode {
    let lt = g.ltmp(layer);
    let def = g.layer_def(layer).upgrades.iter().find(|u| u.id == id).unwrap();
    let ut = lt.upgrades.get(&id).unwrap();
    let bought = g.has_upgrade(layer, id);
    let can_afford = g.can_afford_upgrade(layer, id);

    let mut class = format!("{} tooltipBox upg", layer);
    if bought {
        class += " bought";
    } else if can_afford {
        class += " can";
    } else {
        class += " locked";
    }

    let mut style = String::new();
    if can_afford && !bought {
        style = format!("background-color:{}", lt.color);
    }
    let upg_style = (&def.style)(g);
    if !upg_style.is_empty() {
        style += &format!(";{}", upg_style);
    }

    let mut html = String::new();
    if let Some(f) = &def.full_display {
        html += &f(g);
    } else {
        if let Some(title) = &def.title {
            html += &format!("<h3>{}</h3><br>", title);
        }
        if let Some(desc) = &def.description {
            html += desc;
        }
        if let Some(f) = &def.effect_display {
            html += &format!("<br>Currently: {}", f(g));
        }
        let currency_name = match &def.currency {
            Some(Currency::Buyable { .. }) => "exhancers".to_string(),
            _ => lt.resource.clone(),
        };
        html += &format!(
            "<br><br>Cost: {} {}",
            fmt_whole(g, ut.cost),
            currency_name
        );
    }
    if let Some(tooltip) = &ut.tooltip {
        html += &format!("<div class=\"tooltip\">{}</div>", tooltip);
    }

    UiNode::new(&format!("upgrade-{}-{}", layer, id), "button")
        .class(&class)
        .style(&style)
        .ev(format!("upg:{}:{}", layer, id))
        .html(html)
}

fn render_milestones(g: &Game, layer: &str) -> UiNode {
    let mut table = UiNode::new(&format!("mss-{}", layer), "table");
    if g.layer_def(layer).milestones.is_empty() {
        return table;
    }
    for m in &g.layer_def(layer).milestones {
        let mt = g.ltmp(layer).milestones.get(&m.id);
        let unlocked = mt.map(|t| t.unlocked).unwrap_or(true);
        if !unlocked || !g.milestone_shown(layer, m.id) {
            continue;
        }
        let done = g.has_milestone(layer, m.id);
        let mut class = "tooltipBox".to_string();
        if done {
            class += " milestoneDone";
        } else {
            class += " milestone";
        }
        let style = (&m.style)(g);
        let mut html = format!("<h3>{}</h3><br>{}<br>", m.requirement_description, (m.effect_description)(g));
        if let Some(tooltip) = (m.tooltip)(g) {
            html += &format!("<div class=\"tooltip\">{}</div>", tooltip);
        }
        if done && !m.toggles.is_empty() {
            for (tl, field) in &m.toggles {
                let on = g.lstate(tl).extra_bool(field);
                html += &format!(
                    "<button class=\"smallUpg can\" style=\"background-color:{}\" data-tmt=\"toggle:{}:{}\">{}</button>&nbsp;",
                    g.ltmp(tl).color,
                    tl,
                    field,
                    if on { "ON" } else { "OFF" }
                );
            }
        }
        table.children.push(
            UiNode::new(&format!("ms-{}-{}", layer, m.id), "tr").children_push(
                UiNode::new(&format!("msc-{}-{}", layer, m.id), "td")
                    .class(&class)
                    .style(&style)
                    .html(html),
            ),
        );
    }
    table
}

fn render_challenges(g: &Game, layer: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let mut table = UiNode::new(&format!("chs-{}", layer), "div").class("upgTable");
    if g.layer_def(layer).challenges.is_empty() {
        return table;
    }
    for row in 1..=lt.challenge_rows.max(1) {
        let mut row_n = UiNode::new(&format!("chsr-{}-{}", layer, row), "div").class("upgRow");
        let mut any = false;
        for col in 1..=lt.challenge_cols.max(1) {
            let id = row * 10 + col;
            let unlocked = lt.challenges.get(&id).map(|c| c.unlocked).unwrap_or(false);
            let hidden = g.options.hide_challenges && g.maxed_challenge(layer, id) && !g.in_challenge(layer, id);
            if unlocked && !hidden {
                any = true;
                row_n.children.push(render_challenge(g, layer, id));
            }
        }
        if any {
            table.children.push(row_n);
        }
    }
    table
}

fn render_challenge(g: &Game, layer: &str, id: i64) -> UiNode {
    let lt = g.ltmp(layer);
    let def = g.layer_def(layer).challenges.iter().find(|c| c.id == id).unwrap();
    let active = g.lstate(layer).active_challenge == Some(id);

    let mut class = "challenge".to_string();
    if active && g.can_complete_challenge(layer, id) {
        class += " canComplete";
    } else if g.maxed_challenge(layer, id) {
        class += " done";
    } else {
        class += " locked";
    }
    if active {
        class += " resetNotify";
    }
    let style = (&def.style)(g);

    let button_text = if active {
        if g.can_complete_challenge(layer, id) { "Finish" } else { "Exit Early" }
    } else if g.maxed_challenge(layer, id) {
        "Completed"
    } else {
        "Start"
    };

    let mut html = format!("<br><h3>{}</h3><br><br>", def.name);
    html += &format!(
        "<button class=\"longUpg can {}\" style=\"background-color:{}\" data-tmt=\"challenge:{}:{}\">{}</button><br><br>",
        layer, lt.color, layer, id, button_text
    );
    if let Some(f) = &def.full_display {
        html += &f(g);
    } else {
        html += &format!("{}<br>", (def.challenge_description)(g));
        let goal_text = match &def.goal_description {
            Some(t) => t.clone(),
            None => format!(
                "{} {}",
                fmt(g, def.goal),
                match &def.goal_currency {
                    Some(Currency::PlayerField(name)) => name.to_string(),
                    _ => g.defs.mod_info.points_name.to_string(),
                }
            ),
        };
        html += &format!("Goal: {}<br>", goal_text);
        if let Some(reward) = &def.reward_description {
            html += &format!("Reward: {}<br>", reward);
        }
        if def.reward_display.is_some() || def.reward_effect.is_some() {
            let current = match &def.reward_display {
                Some(f) => f(g),
                None => {
                    let eff = lt.challenges.get(&id).and_then(|c| c.reward_effect.clone());
                    fmt(g, eff.map(|e| e.dec()).unwrap_or_else(Decimal::one))
                }
            };
            html += &format!("<span>Currently: {}</span>", current);
        }
    }
    if (def.marked)(g) {
        html += "<div class=\"star\" style=\"position:absolute;left:10px;top:10px\"></div>";
    }

    UiNode::new(&format!("challenge-{}-{}", layer, id), "div")
        .class(&class)
        .style(&style)
        .html(html)
}

fn render_buyables(g: &Game, layer: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let def = &g.layer_def(layer).buyables;
    let mut table = UiNode::new(&format!("bs-{}", layer), "div").class("upgTable");
    if def.items.is_empty() {
        return table;
    }
    if def.respec.is_some() && def.show_respec {
        let mut wrap = UiNode::new(&format!("respecw-{}", layer), "div");
        wrap.style = "margin-bottom:12px".to_string();
        let checked = if g.lstate(layer).no_respec_confirm { "checked" } else { "" };
        let mut html = format!(
            "<span class=\"tooltipBox respecCheckbox\"><input type=\"checkbox\" {}><span class=\"tooltip\">Disable respec confirmation</span></span>",
            checked
        );
        html += &format!(
            "<button class=\"longUpg can\" data-tmt=\"toggle:{}:noRespecConfirm\">{}</button>",
            layer,
            if def.respec_text.is_empty() { "Respec" } else { &def.respec_text }
        );
        wrap.html = html;
        wrap.ev = Some(format!("respec:{}", layer));
        table.children.push(wrap);
    }
    for row in 1..=lt.buyable_rows.max(1) {
        let mut row_n = UiNode::new(&format!("bsr-{}-{}", layer, row), "div").class("upgRow");
        let mut any = false;
        for col in 1..=lt.buyable_cols.max(1) {
            let id = format!("{}", row * 10 + col);
            if lt.buyables.get(&id).map(|b| b.unlocked).unwrap_or(false) {
                any = true;
                let mut align = UiNode::new(&format!("ba-{}-{}", layer, id), "div");
                align.style = "margin-left:7px;margin-right:7px".to_string();
                align.children.push(render_buyable(g, layer, &id));
                row_n.children.push(align);
            }
        }
        if any {
            table.children.push(row_n);
        }
    }
    table
}

fn render_buyable(g: &Game, layer: &str, id: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let def = g.layer_def(layer).buyables.items.iter().find(|b| b.id == id).unwrap();
    let bt = lt.buyables.get(id).unwrap();

    let mut class = "buyable tooltipBox".to_string();
    if bt.can_buy {
        class += " can";
    } else if bt.at_limit {
        class += " bought";
    } else {
        class += " locked";
    }

    let mut style = String::new();
    if bt.can_buy {
        style = format!("background-color:{}", lt.color);
    }
    let cs = (&g.layer_def(layer).component_styles.buyable)(g);
    if !cs.is_empty() {
        style += &format!(";{}", cs);
    }
    let own = (&def.style)(g);
    if !own.is_empty() {
        style += &format!(";{}", own);
    }

    let mut html = String::new();
    if let Some(title) = &def.title {
        html += &format!("<h2>{}</h2><br>", title);
    }
    html += &format!("<span style=\"white-space:pre-line\">{}</span>", (def.display)(g));
    if bt.marked {
        html += "<div class=\"star\" style=\"position:absolute;left:-8px;top:-8px\"></div>";
    }
    if let Some(tooltip) = (&def.tooltip)(g) {
        html += &format!("<div class=\"tooltip\">{}</div>", tooltip);
    }

    let mut n = UiNode::new(&format!("buyable-{}-{}", layer, id), "div");
    let mut button = UiNode::new(&format!("buy-{}-{}", layer, id), "button")
        .class(&class)
        .style(&style)
        .ev(format!("buy:{}:{}", layer, id))
        .html(html);
    button = button.attr("data-hold", "1".to_string());
    n.children.push(button);

    if bt.has_sell_one || bt.has_sell_all {
        if bt.has_sell_one {
            n.children.push(
                UiNode::new(&format!("s1-{}-{}", layer, id), "button")
                    .class("longUpg can")
                    .ev(format!("sell1:{}:{}", layer, id))
                    .html(def_sell_text(&g.layer_def(layer).buyables.sell_one_text, "Sell One")),
            );
        }
        if bt.has_sell_all {
            n.children.push(
                UiNode::new(&format!("sa-{}-{}", layer, id), "button")
                    .class("longUpg can")
                    .ev(format!("sellall:{}:{}", layer, id))
                    .html(def_sell_text(&g.layer_def(layer).buyables.sell_all_text, "Sell All")),
            );
        }
    }
    n
}

fn def_sell_text(text: &str, default: &str) -> String {
    if text.is_empty() { default.to_string() } else { text.to_string() }
}

fn render_clickables(g: &Game, layer: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let def = &g.layer_def(layer).clickables;
    let mut table = UiNode::new(&format!("cs-{}", layer), "div").class("upgTable");
    if def.items.is_empty() {
        return table;
    }
    if def.master_button_press.is_some() && def.show_master_button {
        table.children.push(
            UiNode::new(&format!("mb-{}", layer), "button")
                .class("longUpg can")
                .style("margin-bottom:12px")
                .ev(format!("master:{}", layer))
                .html((def.master_button_text)(g)),
        );
    }
    for row in 1..=lt.clickable_rows.max(1) {
        let mut row_n = UiNode::new(&format!("csr-{}-{}", layer, row), "div").class("upgRow");
        let mut any = false;
        for col in 1..=lt.clickable_cols.max(1) {
            let id = format!("{}", row * 10 + col);
            if lt.clickables.get(&id).map(|c| c.unlocked).unwrap_or(false) {
                any = true;
                let mut align = UiNode::new(&format!("ca-{}-{}", layer, id), "div");
                align.style = "margin-left:7px;margin-right:7px".to_string();
                align.children.push(render_clickable(g, layer, &id));
                row_n.children.push(align);
            }
        }
        if any {
            table.children.push(row_n);
        }
    }
    table
}

fn render_clickable(g: &Game, layer: &str, id: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let def = g.layer_def(layer).clickables.items.iter().find(|c| c.id == id).unwrap();
    let ct = lt.clickables.get(id).unwrap();

    let mut class = "upg tooltipBox".to_string();
    if ct.can_click {
        class += " can";
    } else {
        class += " locked";
    }
    let mut style = String::new();
    if ct.can_click {
        style = format!("background-color:{}", lt.color);
    }
    let own = (&def.style)(g);
    if !own.is_empty() {
        style += &format!(";{}", own);
    }

    let mut html = String::new();
    if let Some(title) = &def.title {
        html += &format!("<h2>{}</h2><br>", title);
    }
    html += &format!("<span style=\"white-space:pre-line\">{}</span>", (def.display)(g));
    if ct.marked {
        html += "<div class=\"star\" style=\"position:absolute;left:-8px;top:-8px\"></div>";
    }
    if let Some(tooltip) = (&def.tooltip)(g) {
        html += &format!("<div class=\"tooltip\">{}</div>", tooltip);
    }

    let mut n = UiNode::new(&format!("clickable-{}-{}", layer, id), "button")
        .class(&class)
        .style(&style)
        .ev(format!("click:{}:{}", layer, id))
        .html(html);
    if def.on_hold.is_some() {
        n = n.attr("data-hold", "1".to_string());
    }
    n
}

fn render_bar(g: &Game, layer: &str, id: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let def = g.layer_def(layer).bars.iter().find(|b| b.id == id).unwrap();
    let bt = match lt.bars.get(id) {
        Some(b) if b.unlocked => b,
        _ => return UiNode::new(&format!("barw-{}-{}", layer, id), "div"),
    };

    let progress = bt.progress.clamp(0.0, 1.0);
    let remaining = (1.0 - progress) * 100.0;
    let dims = format!("width:{}px;height:{}px", def.width, def.height);
    let (clip, extra_w) = match def.direction {
        Direction::Up => (format!("inset({}% 0% 0% 0%)", remaining), format!("width:{}px", def.width + 1.0)),
        Direction::Down => (format!("inset(0% 0% {}% 0%)", remaining), format!("width:{}px", def.width + 1.0)),
        Direction::Right => (format!("inset(0% {}% 0% 0%)", remaining), String::new()),
        Direction::Left => (format!("inset(0% 0% 0% {}%)", remaining), String::new()),
    };
    let mut fill_style = format!("{};clip-path:{}", extra_w, clip);
    if def.instant {
        fill_style += ";transition-duration:0s";
    }
    let custom = (&def.fill_style)(g);
    let fill = if custom.is_empty() {
        format!("background-color:var(--color);{}", fill_style)
    } else {
        format!("background-color:var(--color);{};{}", custom, fill_style)
    };

    let _wrap = UiNode::new(&format!("barw-{}-{}", layer, id), "div").style("position:relative;display:table");
    let html = format!(
        "<div class=\"overlayTextContainer barBorder\" style=\"{}\">\
         <span class=\"overlayText\" style=\"{};{}\">{}</span></div>\
         <div class=\"barBG barBorder\" style=\"{};{};{}\">\
         <div class=\"fill\" id=\"fill-{}-{}\" style=\"{}\"></div></div>",
        dims,
        (&def.style)(g),
        (&def.text_style)(g),
        (def.display)(g),
        (&def.style)(g),
        (&def.base_style)(g),
        dims,
        layer,
        id,
        fill,
    );
    let mut n = UiNode::new(&format!("bar-{}-{}", layer, id), "div").html(html);
    n.style = (&def.style)(g);
    n
}

fn render_infobox(g: &Game, layer: &str, id: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let def = g.layer_def(layer).infoboxes.iter().find(|b| b.id == id).unwrap();
    if !lt.infoboxes.get(id).copied().unwrap_or(true) {
        return UiNode::new(&format!("ibw-{}-{}", layer, id), "div");
    }
    let open = g
        .player
        .infoboxes
        .get(layer)
        .and_then(|b| b.get(id))
        .copied()
        .unwrap_or(false);

    let mut html = format!(
        "<button class=\"story-title\" style=\"background-color:{};{}\" data-tmt=\"infobox:{}:{}\">\
         <span class=\"story-toggle\">{}</span><span>{}</span></button>",
        lt.color,
        (def.title_style)(g),
        layer,
        id,
        if open { "+" } else { "-" },
        (def.title)(g),
    );
    if !open {
        html += &format!(
            "<div class=\"story-text\" style=\"{}\"><span>{}</span></div>",
            (def.body_style)(g),
            (def.body)(g)
        );
    }
    UiNode::new(&format!("infobox-{}-{}", layer, id), "div")
        .class("story")
        .style(format!("border-color:{};border-radius:{};{}",
            lt.color,
            if open { "8px" } else { "8px 8px 0 0" },
            (def.style)(g)))
        .html(html)
}

fn render_grid(g: &Game, layer: &str) -> UiNode {
    let def = match &g.layer_def(layer).grid {
        Some(d) => d,
        None => return UiNode::new(&format!("gw-{}", layer), "div"),
    };
    let lt = g.ltmp(layer);
    let mut table = UiNode::new(&format!("grid-{}", layer), "div").class("upgTable");
    for row in 1..=def.rows {
        let mut row_n = UiNode::new(&format!("gridr-{}-{}", layer, row), "div").class("upgRow");
        let mut any = false;
        for col in 1..=def.cols {
            let id = row * 100 + col;
            if !(def.get_unlocked)(g, id) {
                continue;
            }
            if g.lstate(layer).grid.get(&id).is_none() {
                continue;
            }
            any = true;
            let can_click = (def.get_can_click)(g, id);
            let mut class = "tile tooltipBox".to_string();
            if can_click {
                class += " can";
            } else {
                class += " locked";
            }
            let mut style = String::new();
            if can_click {
                style = format!("background-color:{}", lt.color);
            }
            let own = (def.get_style)(g, id);
            if !own.is_empty() {
                style += &format!(";{}", own);
            }
            let mut html = String::new();
            let title = (def.get_title)(g, id);
            if !title.is_empty() {
                html += &format!("<h3>{}</h3><br>", title);
            }
            html += &format!("<span style=\"white-space:pre-line\">{}</span>", (def.get_display)(g, id));
            if let Some(tooltip) = (def.get_tooltip)(g, id) {
                html += &format!("<div class=\"tooltip\">{}</div>", tooltip);
            }
            let mut tile = UiNode::new(&format!("tile-{}-{}", layer, id), "button")
                .class(&class)
                .style(&style)
                .ev(format!("grid:{}:{}", layer, id))
                .html(html);
            if def.on_hold.is_some() {
                tile = tile.attr("data-hold", "1".to_string());
            }
            let mut align = UiNode::new(&format!("ga-{}-{}", layer, id), "div");
            align.style = "margin:1px".to_string();
            align.children.push(tile);
            row_n.children.push(align);
        }
        if any {
            table.children.push(row_n);
        }
    }
    table
}

fn render_achievements(g: &Game, layer: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let mut table = UiNode::new(&format!("achs-{}", layer), "div").class("upgTable");
    if g.layer_def(layer).achievements.is_empty() {
        return table;
    }
    for row in 1..=lt.achievement_rows.max(1) {
        let mut row_n = UiNode::new(&format!("achsr-{}-{}", layer, row), "div").class("upgRow");
        let mut any = false;
        for col in 1..=lt.achievement_cols.max(1) {
            let id = row * 10 + col;
            if lt.achievements.get(&id).map(|a| a.unlocked).unwrap_or(false) {
                any = true;
                row_n.children.push(render_achievement(g, layer, id));
            }
        }
        if any {
            table.children.push(row_n);
        }
    }
    table
}

fn render_achievement(g: &Game, layer: &str, id: i64) -> UiNode {
    let _lt = g.ltmp(layer);
    let def = g.layer_def(layer).achievements.iter().find(|a| a.id == id).unwrap();
    let done = g.has_achievement(layer, id);

    let mut class = format!("{} achievement tooltipBox", layer);
    if done {
        class += " bought";
    } else {
        class += " locked";
    }
    let mut style = String::new();
    if let Some(image) = &def.image {
        style += &format!("background-image:url(\"{}\")", image);
    }
    if !done {
        style += ";filter:grayscale(0.7);opacity:0.55";
    }
    let own = (&def.style)(g);
    if !own.is_empty() {
        style += &format!(";{}", own);
    }

    let tooltip = if done {
        def.done_tooltip
            .clone()
            .or_else(|| def.tooltip.clone())
            .unwrap_or_else(|| "You did it!".to_string())
    } else {
        def.goal_tooltip
            .clone()
            .or_else(|| def.tooltip.clone())
            .unwrap_or_else(|| "LOCKED".to_string())
    };

    let mut html = format!("<div class=\"tooltip\">{}</div>", tooltip);
    if !def.name.is_empty() {
        html += &format!("<br><h3 style=\"{}\">{}</h3><br>", def.text_style, def.name);
    }

    UiNode::new(&format!("ach-{}-{}", layer, id), "div")
        .class(&class)
        .style(&style)
        .html(html)
}

fn render_tree_node(g: &Game, layer: &str) -> UiNode {
    let lt = g.ltmp(layer);
    let def = g.layer_def(layer);

    let mut class = if def.is_layer { "treeNode" } else { "treeButton" }.to_string();
    class += &format!(" {}", layer);
    class += " tooltipBox";
    if g.lstate(layer).force_tooltip {
        class += " forceTooltip";
    }
    if lt.layer_shown == LayerShown::Ghost {
        class += " ghost";
    }
    if lt.layer_shown == LayerShown::Hidden {
        class += " hidden";
    }
    let unlocked_or_can = if def.is_layer {
        g.lstate(layer).unlocked || lt.can_reset
    } else {
        lt.can_click
    };
    if def.is_layer {
        if !unlocked_or_can {
            class += " locked";
        }
        if unlocked_or_can {
            class += " can";
        }
    } else if lt.can_click {
        class += " can";
    } else {
        class += " locked";
    }
    if lt.notify && g.lstate(layer).unlocked {
        class += " notify";
    }
    if lt.prestige_notify {
        class += " resetNotify";
    }
    if !g.tmp.scrolled {
        class += " front";
    }

    let mut style = String::new();
    if (def.is_layer && g.layer_unlocked(layer)) || (!def.is_layer && lt.can_click) {
        style = format!("background-color:{}", lt.color);
    }
    if let Some(image) = &lt.image {
        style += &format!(";background-image:url(\"{}\")", image);
    }
    if lt.notify && g.lstate(layer).unlocked {
        style += &format!(";box-shadow:var(--hqProperty2a), 0 0 20px {}", lt.true_glow);
    }
    if !lt.node_style.is_empty() {
        style += &format!(";{}", lt.node_style);
    }

    // Tooltip content.
    let tooltip = if def.is_layer {
        if g.lstate(layer).unlocked {
            lt.tooltip
                .clone()
                .unwrap_or_else(|| format!("{} {}", fmt_whole(g, g.lstate(layer).points), lt.resource))
        } else {
            lt.tooltip_locked.clone().unwrap_or_else(|| {
                format!(
                    "Reach {} {} to unlock (You have {} {})",
                    fmt_whole(g, lt.requires),
                    lt.base_resource,
                    fmt_whole(g, lt.base_amount),
                    lt.base_resource
                )
            })
        }
    } else if lt.can_click {
        lt.tooltip.clone().unwrap_or_else(|| "I am a button!".to_string())
    } else {
        lt.tooltip_locked.clone().unwrap_or_else(|| "I am a button!".to_string())
    };

    let mut html = format!(
        "<span class=\"nodeLabel\">{}</span><div class=\"tooltip\">{}</div>",
        if lt.image.is_none() { lt.symbol.clone() } else { "&nbsp;".to_string() },
        tooltip
    );
    if let Some(mark) = &lt.marked {
        html += &format!(
            "<img class=\"mark\" style=\"position:absolute;left:-22px;top:-15px\" src=\"{}\">",
            mark
        );
    }

    let mut n = UiNode::new(layer, "button")
        .class(&class)
        .style(&style)
        .ev(format!("node:{}", layer))
        .html(html);
    n.attrs.retain(|(k, _)| *k != "id");
    n
}

fn render_tree(g: &Game, layout: &[Vec<TreeNode>], key: &str) -> UiNode {
    let mut n = UiNode::new(&format!("tree-{}", key), "div");
    for (r, row) in layout.iter().enumerate() {
        let mut row_n = UiNode::new(&format!("tree-{}-{}", key, r), "span").class("upgRow");
        for TreeNode::Layer(id) in row {
            {
                if g.ltmp(id).layer_shown == LayerShown::Shown {
                    row_n.children.push(render_tree_node(g, id));
                } else if g.ltmp(id).layer_shown == LayerShown::Ghost {
                    // Ghost nodes still occupy space.
                    let mut ghost = render_tree_node(g, id);
                    ghost.class += " ghost";
                    row_n.children.push(ghost);
                }
            }
        }
        n.children.push(row_n);
    }
    n
}

fn render_thing_tree(g: &Game, layer: &str, kind: &str, layout: &[Vec<i64>], key: &str) -> UiNode {
    let mut n = UiNode::new(&format!("tt-{}", key), "div");
    for (r, row) in layout.iter().enumerate() {
        let mut row_n = UiNode::new(&format!("ttr-{}-{}", key, r), "span").class("upgRow");
        for id in row {
            let exists = match kind {
                "upgrade" => g.ltmp(layer).upgrades.get(id).map(|u| u.unlocked).unwrap_or(false),
                "buyable" => g.ltmp(layer).buyables.get(&id.to_string()).map(|b| b.unlocked).unwrap_or(false),
                "clickable" => g.ltmp(layer).clickables.get(&id.to_string()).map(|c| c.unlocked).unwrap_or(false),
                _ => false,
            };
            if !exists {
                continue;
            }
            let mut align = UiNode::new(&format!("tta-{}-{}-{}", key, r, id), "span").class("upgAlign treeThing");
            match kind {
                "upgrade" => align.children.push(render_upgrade(g, layer, *id)),
                "buyable" => align.children.push(render_buyable(g, layer, &id.to_string())),
                "clickable" => align.children.push(render_clickable(g, layer, &id.to_string())),
                _ => {}
            }
            row_n.children.push(align);
        }
        n.children.push(row_n);
    }
    n
}

// ---------------------------------------------------------------------------
// tabFormat content
// ---------------------------------------------------------------------------

fn render_column(g: &Game, layer: &str, items: &[ContentItem], key: &str) -> UiNode {
    let mut col = UiNode::new(&format!("col-{}", key), "div").class("upgCol");
    let table = render_content_list(g, layer, items, key);
    col.children.push(table);
    col
}

fn render_content_list(g: &Game, layer: &str, items: &[ContentItem], key: &str) -> UiNode {
    let mut table = UiNode::new(&format!("tbl-{}", key), "div").class("upgTable");
    for (i, item) in items.iter().enumerate() {
        let k = format!("{}-{}", key, i);
        let n = render_content(g, layer, item, &k);
        table.children.push(n);
    }
    table
}

fn render_content(g: &Game, layer: &str, item: &ContentItem, key: &str) -> UiNode {
    let cs = &g.layer_def(layer).component_styles;
    let style = item.style.clone().unwrap_or_default();
    let with_component_style = |base: &str, comp: &str| -> String {
        let extra = match comp {
            "upgrade" => (&cs.upgrade)(g),
            "milestone" => (&cs.milestone)(g),
            "challenge" => (&cs.challenge)(g),
            "buyable" => (&cs.buyable)(g),
            "clickable" => (&cs.clickable)(g),
            "bar" => (&cs.bar)(g),
            "toggle" => (&cs.toggle)(g),
            _ => String::new(),
        };
        let mut s = String::new();
        if !base.is_empty() {
            s = base.to_string();
        }
        if !extra.is_empty() {
            if !s.is_empty() {
                s += ";";
            }
            s += &extra;
        }
        s
    };

    let n = match &item.content {
        Content::MainDisplay => render_main_display(g, layer),
        Content::PrestigeButton => render_prestige_button(g, layer),
        Content::ResourceDisplay => render_resource_display(g, layer),
        Content::Milestones => render_milestones(g, layer),
        Content::Upgrades => render_upgrades(g, layer),
        Content::Challenges => render_challenges(g, layer),
        Content::Buyables => render_buyables(g, layer),
        Content::Clickables => render_clickables(g, layer),
        Content::Achievements => render_achievements(g, layer),
        Content::Grid => render_grid(g, layer),
        Content::Blank(size) => {
            let mut inner = UiNode::new(&format!("bl-{}", key), "div").class("instant");
            inner.style = match size {
                None => "width:8px;height:17px".to_string(),
                Some(Size::Height(h)) => format!("width:8px;height:{}px", h),
                Some(Size::WH(w, h)) => format!("width:{}px;height:{}px", w, h),
            };
            let mut wrap = UiNode::new(&format!("blw-{}", key), "div").class("instant");
            wrap.children.push(inner);
            wrap
        }
        Content::RawHtml(f) => {
            let mut n = UiNode::new(&format!("rh-{}", key), "span").class("instant");
            n.html = f(g);
            n
        }
        Content::DisplayText(f) => {
            let mut n = UiNode::new(&format!("dt-{}", key), "span").class("instant");
            n.html = f(g);
            n
        }
        Content::DisplayImage(url) => {
            UiNode::new(&format!("di-{}", key), "img").class("instant").attr("src", url.clone())
        }
        Content::TextInput(field) => {
            let value = g.lstate(layer).extra_str(field);
            UiNode::new(&format!("ti-{}", key), "input")
                .class("instant")
                .attr("value", value)
                .ev(format!("input:{}:{}", layer, field))
        }
        Content::Slider(field, min, max) => {
            let value = g.lstate(layer).extra_num(field);
            UiNode::new(&format!("sl-{}", key), "input")
                .class("instant")
                .attr("type", "range".to_string())
                .attr("min", crate::json::num_to_json_string(*min))
                .attr("max", crate::json::num_to_json_string(*max))
                .attr("value", crate::json::num_to_json_string(value))
                .ev(format!("slider:{}:{}", layer, field))
        }
        Content::DropDown(field, options) => {
            let value = g.lstate(layer).extra_str(field);
            let mut html = String::new();
            for opt in options {
                let selected = if *opt == value { " selected" } else { "" };
                html += &format!("<option value=\"{}\"{}>{}</option>", opt, selected, opt);
            }
            UiNode::new(&format!("dd-{}", key), "select")
                .html(html)
                .ev(format!("dropdown:{}:{}", layer, field))
        }
        Content::Toggle(tlayer, field) => {
            let on = g.lstate(tlayer).extra_bool(field);
            UiNode::new(&format!("tg-{}", key), "button")
                .class("smallUpg can")
                .style(format!("background-color:{}", g.ltmp(tlayer).color))
                .ev(format!("toggle:{}:{}", tlayer, field))
                .html(if on { "ON" } else { "OFF" }.to_string())
        }
        Content::Row(items) => {
            let mut row = UiNode::new(&format!("rw-{}", key), "div").class("upgRow");
            row.children.push(render_content_list(g, layer, items, &format!("r-{}", key)));
            let mut table = UiNode::new(&format!("rwt-{}", key), "div").class("upgTable");
            table.children.push(row);
            table
        }
        Content::Column(items) => {
            let style2 = style.clone();
            let mut n = render_column(g, layer, items, key);
            if !style2.is_empty() {
                n.style = format!("{};{}", n.style, style2);
            }
            return n;
        }
        Content::HLine(w) => UiNode::new(&format!("hl-{}", key), "hr")
            .class("instant hl")
            .style(w.map(|w| format!("width:{}px", w)).unwrap_or_default()),
        Content::VLine(h) => UiNode::new(&format!("vl2-{}", key), "div")
            .class("instant vl2")
            .style(h.map(|h| format!("height:{}px", h)).unwrap_or_default()),
        Content::Upgrade(id) => {
            let mut n = render_upgrade(g, layer, *id);
            n.style = with_component_style(&n.style, "upgrade");
            n
        }
        Content::Challenge(id) => {
            let mut n = render_challenge(g, layer, *id);
            n.style = with_component_style(&n.style, "challenge");
            n
        }
        Content::Bar(id) => {
            let mut n = render_bar(g, layer, id);
            n.style = with_component_style(&n.style, "bar");
            n
        }
        Content::Infobox(id) => render_infobox(g, layer, id),
        Content::Microtabs(family) => {
            let mut n = UiNode::new(&format!("mt-{}", key), "div").style("border-style:solid");
            let mut buttons = UiNode::new(&format!("mtb-{}", key), "div").class("upgTable instant");
            buttons.children.push(render_tab_buttons(g, layer, family));
            n.children.push(buttons);
            let current = g
                .player
                .subtabs
                .get(layer)
                .and_then(|f| f.get(*family))
                .cloned()
                .unwrap_or_default();
            let family_def = g.layer_def(layer).microtabs.iter().find(|f| f.family == *family);
            if let Some(family_def) = family_def {
                if let Some(tab) = family_def.tabs.iter().find(|t| t.name == current) {
                    match tab.embed_layer {
                        Some(embed) => n.children.push(render_layer_tab(g, embed, false, true)),
                        None => {
                            let tab_style = (tab.style)(g);
                            n.children
                                .push(render_column(g, layer, &tab.content, &format!("mt-{}", key)).style(tab_style));
                        }
                    }
                }
            }
            n
        }
        Content::Tree(layout) => render_tree(g, layout, key),
        Content::ThingTree(kind, layout) => render_thing_tree(g, layer, kind, layout, key),
        Content::InfoTab => render_info_tab(g),
        Content::OptionsTab => render_options_tab(g),
        Content::Changelog => {
            let mut n = UiNode::new(&format!("cl-{}", key), "div");
            n.html = g.defs.changelog.to_string();
            n
        }
    };

    let mut n = n;
    if !style.is_empty() && !matches!(item.content, Content::Column(_)) {
        n.style = format!("{};{}", n.style, style);
    }
    n
}

// ---------------------------------------------------------------------------
// System tabs
// ---------------------------------------------------------------------------

fn render_info_tab(g: &Game) -> UiNode {
    let mut n = UiNode::new("infoTab", "div");
    let mut html = format!("<h2>{}</h2><br><h3>v{}{}</h3>", g.defs.mod_info.name, g.defs.version, name_suffix(g));
    if !g.defs.mod_info.author.is_empty() {
        html += &format!("<br>Made by {}", g.defs.mod_info.author);
    }
    html += "<br>The Modding Tree (Rust edition) — original by Acamaeda and FlamemasterNXF<br>The Prestige Tree made by Jacorb and Aarex<br><br>";
    html += "<div class=\"link\" data-tmt=\"tab:changelog-tab\">Changelog</div><br><br><br>";
    html += &format!("Time Played: {}<br><br><h3>Hotkeys</h3><br>", format_time(g.player.time_played));
    for layer in &g.defs.layers {
        for hk in &layer.hotkeys {
            if g.lstate(layer.id).unlocked && (hk.unlocked)(g) {
                html += &format!("<br>{}", hk.description);
            }
        }
    }
    n.html = html;
    n
}

fn name_suffix(g: &Game) -> String {
    if g.defs.version_name.is_empty() {
        String::new()
    } else {
        format!(": {}", g.defs.version_name)
    }
}

fn render_options_tab(g: &Game) -> UiNode {
    let mut n = UiNode::new("optionsTab", "table");
    let on = |b: bool| if b { "ON" } else { "OFF" };
    let html = format!(
        "<tr><td><button class=\"opt\" data-tmt=\"opt:save\">Save</button></td>\
         <td><button class=\"opt\" data-tmt=\"opt:autosave\">Autosave: {autosave}</button></td>\
         <td><button class=\"opt\" data-tmt=\"opt:hardreset\">HARD RESET</button></td></tr>\
         <tr><td><button class=\"opt\" data-tmt=\"js:export\">Export to clipboard</button></td>\
         <td><button class=\"opt\" data-tmt=\"js:import\">Import</button></td>\
         <td><button class=\"opt\" data-tmt=\"opt:offlineprod\">Offline Prod: {offline}</button></td></tr>\
         <tr><td><button class=\"opt\" data-tmt=\"opt:theme\">Theme: {theme}</button></td>\
         <td><button class=\"opt\" data-tmt=\"opt:msdisplay\">Show Milestones: {ms}</button></td>\
         <td><button class=\"opt\" data-tmt=\"opt:hqtree\">High-Quality Tree: {hq}</button></td></tr>\
         <tr><td><button class=\"opt\" data-tmt=\"opt:hidechallenges\">Completed Challenges: {hc}</button></td>\
         <td><button class=\"opt\" data-tmt=\"opt:forceonetab\">Single-Tab Mode: {ft}</button></td>\
         <td><button class=\"opt\" data-tmt=\"opt:forcetooltips\">Shift-Click to Toggle Tooltips: {tt}</button></td></tr>\
         <tr><td><button class=\"opt\" data-tmt=\"opt:hidemilestonepopups\">Show Milestone Popups: {mp}</button></td></tr>",
        autosave = on(g.options.autosave),
        offline = on(g.options.offline_prod),
        theme = g.options.theme,
        ms = g.options.ms_display.display_name(),
        hq = on(g.options.hq_tree),
        hc = if g.options.hide_challenges { "HIDDEN" } else { "SHOWN" },
        ft = if g.options.force_one_tab { "ALWAYS" } else { "AUTO" },
        tt = on(g.options.force_tooltips),
        mp = on(!g.options.hide_milestone_popups),
    );
    n.html = html;
    n
}

// ---------------------------------------------------------------------------
// Canvas tree data (branch lines)
// ---------------------------------------------------------------------------

fn branch_color(_g: &Game, color: &str) -> String {
    match color {
        "1" => "#ffffff".to_string(),
        "2" => "#bfbfbf".to_string(),
        "3" => "#7f7f7f".to_string(),
        other => other.to_string(),
    }
}

/// Data for drawing branch lines between DOM elements (glue.js canvas).
pub fn tree_canvas_data(g: &Game) -> Json {
    let mut branches: Vec<Json> = Vec::new();
    for layer in &g.defs.layers {
        let lt = g.ltmp(layer.id);
        if lt.layer_shown == LayerShown::Shown && !layer.branches.is_empty() {
            for b in &layer.branches {
                let (target, color, width) = match b {
                    Branch::Plain(t) => (t, "1", 15.0),
                    Branch::Colored(t, c, w) => (t, *c, w.unwrap_or(15.0)),
                };
                branches.push(Json::Arr(vec![
                    Json::s(layer.id.to_string()),
                    Json::s(target.to_string()),
                    Json::s(branch_color(g, color)),
                    Json::Num(width),
                ]));
            }
        }
        // Component branches (upgrade/buyable/clickable).
        let mut comp_branches = |items: &[i64], kind: &str| {
            for id in items {
                let def_branches = match kind {
                    "upgrade" => g
                        .layer_def(layer.id)
                        .upgrades
                        .iter()
                        .find(|u| u.id == *id)
                        .map(|u| u.branches.clone())
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                for b in &def_branches {
                    let (target, color, width) = match b {
                        Branch::Plain(t) => (t, "1", 15.0),
                        Branch::Colored(t, c, w) => (t, *c, w.unwrap_or(15.0)),
                    };
                    // Numeric branch ids reference component ids of the same kind.
                    branches.push(Json::Arr(vec![
                        Json::s(format!("{}-{}-{}", kind, layer.id, id)),
                        Json::s(format!("{}-{}-{}", kind, layer.id, target)),
                        Json::s(branch_color(g, color)),
                        Json::Num(width),
                    ]));
                }
            }
        };
        let upg_ids: Vec<i64> = layer.upgrades.iter().map(|u| u.id).collect();
        comp_branches(&upg_ids, "upgrade");
    }

    // Layers that should be drawn (visible nodes), for redraw checks.
    Json::obj(vec![("branches", Json::Arr(branches))])
}

fn d_num(n: f64) -> Decimal {
    crate::format::dnum(n)
}
