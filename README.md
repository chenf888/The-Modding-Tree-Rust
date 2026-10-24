# The Modding Tree — Rust Edition

**English** · [中文](#中文)

## English

A Rust rewrite of [The Modding Tree](https://github.com/Acamaeda/The-Modding-Tree) (TMT) incremental-game template. The original runs on Vue 2.7's full-reactivity rendering: every state change triggers dependency tracking and re-rendering whose cost grows with the total number of components, so large games stutter. This project moves the **entire engine** — game state, layer logic, reset math, number formatting, UI-tree building and diffing — into Rust compiled to WebAssembly, leaving only a ~500-line thin renderer on the page.

Rendering model: every tick (50ms) Rust builds the complete UI node tree, **key-diffs** it against the previous frame, and sends only the changes to JS as JSON patches. DOM update cost is proportional to *what changed*, not to *how many components exist* — which is exactly what fixes the original's lag.

### Architecture

```
Browser page (index.html + glue.js + original TMT CSS)
   │   JSON commands in/out of wasm memory (tmt_alloc / tmt_call / tmt_result_ptr)
   ▼
engine.wasm  (Rust, no wasm-bindgen dependency)
   ├─ src/defs.rs     game-definition API: LayerDef + every TMT component
   ├─ src/demo.rs     complete port of the official demo game (layers c/f/a + g/h/spook nodes)
   ├─ src/player.rs   save structure & JSON serialization (same shape as TMT saves)
   ├─ src/game/       engine core: tmp cache · reset cascade · purchases · main loop · events
   ├─ src/render.rs   UiNode tree → keyed diff → JSON patches + tree-canvas data
   ├─ src/format.rs   exact port of TMT's NumberFormating (incl. JS toFixed tie-breaking)
   ├─ src/json.rs     dependency-free JSON parse / serialize
   └─ src/ffi.rs      wasm entry points (init / tick / event / export / import)
```

Big-number math uses [break-eternity-rs](https://crates.io/crates/break-eternity-rs) 0.5.2 — an actively maintained Rust port of break_eternity.js on crates.io (supports `eee...N` strings and Analytic-mode slog/tetrate, matching TMT).

### Build & run

Prerequisites: a Rust toolchain via [rustup](https://rustup.rs) + the wasm32 target.

```bash
rustup target add wasm32-unknown-unknown
./build.sh          # build.bat on Windows; runs cargo build --release and copies engine.wasm
python -m http.server 8765    # or any static file server
# open http://127.0.0.1:8765/ in a browser
```

Headless tests (no browser needed):

```bash
cargo test
```

### Writing your own game

`src/demo.rs` is a line-by-line port of the official demo (`js/Demo/` — c.js / f.js / a.js and demoTree.js) and doubles as the template for your own game:

- Define layers with `LayerDef` (`resource` / `requires` / `exponent` / `type: PrestigeType` / component lists / `tab_format`…)
- Dynamic fields are always callbacks: `view(|g| ...)` (read-only, evaluated every tick — the equivalent of TMT's plain function fields) and `act(|g| ...)` / `act2_str(|g, arg| ...)` (side effects — onClick / onPrestige / doReset and friends)
- Components: upgrades, milestones (with toggles), challenges (with countsAs / completionLimit), buyables (with sellOne / respec), clickables, bars, infoboxes, grid, achievements, microtabs, tabFormat arrays
- Tree layout, branch lines, hotkeys, offline production, achievement popups and other system behavior are handled by the engine automatically

Swap `demo::game_defs()` (referenced in `src/ffi.rs`) for your own definition function — nothing else needs to change.

### Compatibility (verified item by item in a browser)

- Tree layout with canvas branch lines, live node tooltips, ghost nodes, notify glow
- normal / static prestige formulas (gainExp, gainMult, directMult, softcap, roundUpCost, canBuyMax) and the `doReset` row cascade: side-row layers are never reset by the cascade, `layerDataReset` keep-fields, same-row layers don't wipe each other
- Challenges: entry (with forced reset), goal checks, completion counting, reward effects, auto-exit on higher-row resets
- Buyables: cost/effect evaluated at the current amount, Sell One / Respec (with noRespecConfirm)
- Milestone toggles (options and in-layer display), the `msDisplay` filter, milestone/achievement popups
- Achievements survive every prestige permanently; grid click conditions; tree nodes (addNode) canClick/onClick
- Endgame: the freeze on trigger (timePlayed and points all stop), Play Again / Keep Going (resumes with a catch-up diff capped by maxTickLength), hotkeys disabled while ended
- Hotkeys (including ctrl+ combos), offline production (`offTime.remain` drains at `remain/10` per tick, `offlineLimit` cap, Offline Time display)
- Saves: localStorage format identical to TMT (`btoa(unescape(encodeURIComponent(...)))`), import/export, different-mod warning, synchronous beforeunload save, 5-second autosave
- Number formatting bit-for-bit identical to TMT (including JS `toFixed`'s round-toward-+∞ tie rule)

### Known differences from the original

- Rendering is driven by Rust diff patches; there is no Vue. A few layout-only options such as `layer.minWidth` are not ported
- The `custom` prestige type is an enum placeholder; for fully custom formulas see the demo's `f` layer (`prestige_button_text` + a custom gain)
- No mod loader: game definitions are compiled into the wasm — which is precisely where the performance comes from

### Browser test record

The demo game's full flow passed: fresh start → prestige ×N → upgrades/buy-sell → challenge 1/3 completed → layer f static prestige → EIEIO achievement → grid conditional click (points exactly 10) → Thanos/H nodes → hotkey prestige → save reload restore → endgame freeze and Keep Going.

---

## 中文

[English](#english) · **中文**

[The Modding Tree](https://github.com/Acamaeda/The-Modding-Tree)（TMT）增量游戏模板的 Rust 重写。原版基于 Vue 2.7 的全量响应式渲染：状态一变，依赖追踪与重渲染的开销随组件总数增长，游戏做大后明显卡顿。本项目把**整个引擎**——游戏状态、层逻辑、重置数学、数字格式化、UI 树构建与差分——全部移入 Rust 并编译为 WebAssembly；页面侧只保留一个约 500 行的薄渲染层。

渲染模型：Rust 每 tick（50ms）构建完整的 UI 节点树，与上一帧的树做**键控 diff**，只把变化的部分作为 JSON 补丁发给 JS。DOM 更新量与"变化量"成正比，而与"组件总数"无关——这正是解决原版卡顿的关键。

### 架构

```
浏览器页面 (index.html + glue.js + 原版 TMT CSS)
   │   JSON 命令进出 wasm 内存 (tmt_alloc / tmt_call / tmt_result_ptr)
   ▼
engine.wasm  (Rust，不依赖 wasm-bindgen)
   ├─ src/defs.rs     游戏定义 API：LayerDef + 全部 TMT 组件结构
   ├─ src/demo.rs     官方 demo 游戏的完整移植（c / f / a 层 + g / h / spook 节点）
   ├─ src/player.rs   存档结构与 JSON 序列化（与 TMT 存档同构，可直接互通）
   ├─ src/game/       引擎核心：tmp 缓存 · 重置级联 · 购买 · 主循环 · 事件路由
   ├─ src/render.rs   UiNode 树 → 键控 diff → JSON 补丁 + 树 canvas 数据
   ├─ src/format.rs   TMT NumberFormating 的精确移植（含 JS toFixed 舍入平局规则）
   ├─ src/json.rs     零依赖 JSON 解析 / 序列化
   └─ src/ffi.rs      wasm 入口（init / tick / event / export / import）
```

大数运算使用 [break-eternity-rs](https://crates.io/crates/break-eternity-rs) 0.5.2 —— crates.io 上活跃维护的 break_eternity.js 移植（支持 `eee...N` 字符串、Analytic 模式的 slog/tetrate，与 TMT 行为一致）。

### 构建与运行

前置：[rustup](https://rustup.rs) 安装的 Rust 工具链 + wasm32 目标。

```bash
rustup target add wasm32-unknown-unknown
./build.sh          # Windows 下用 build.bat；即 cargo build --release + 拷贝 engine.wasm
python -m http.server 8765    # 或任意静态文件服务器
# 浏览器打开 http://127.0.0.1:8765/
```

测试（无头，不依赖浏览器）：

```bash
cargo test
```

### 写你自己的游戏

`src/demo.rs` 是官方 demo（`js/Demo/` 下的 c.js / f.js / a.js 与 demoTree.js）的逐行移植，也是写自己游戏的模板：

- 定义层：`LayerDef`（`resource` / `requires` / `exponent` / `type: PrestigeType` / 组件列表 / `tab_format`…）
- 动态字段一律用回调：`view(|g| ...)`（只读，每 tick 求值，对应 TMT 的普通函数字段）、`act(|g| ...)` / `act2_str(|g, arg| ...)`（带副作用，对应 onClick / onPrestige / doReset 等）
- 组件：upgrades、milestones（含 toggles）、challenges（含 countsAs / completionLimit）、buyables（含 sellOne / respec）、clickables、bars、infoboxes、grid、achievements、microtabs、tabFormat 数组
- 树布局、分支线（branches）、热键、离线生产、成就弹窗等系统行为由引擎自动处理

把 `demo::game_defs()`（`src/ffi.rs` 中引用）换成你自己的定义函数即可，其余不用动。

### 兼容性（浏览器逐项实测通过）

- 树布局与 canvas 分支线、节点 tooltip 实时刷新、ghost 节点、notify 红光
- normal / static prestige 公式（gainExp、gainMult、directMult、softcap、roundUpCost、canBuyMax）、`doReset` 行级联：side 行层从不被级联重置、`layerDataReset` 的 keep 字段、同层不互删
- 挑战：进出（进入时强制重置）、目标判定、完成计数、奖励效果、退出高行时自动退出
- buyable：cost/effect 公式按当前数量求值、Sell One / Respec（含 noRespecConfirm）
- 里程碑 toggles（options 与层内显示）、`msDisplay` 过滤、里程碑/成就弹窗
- 成就跨 prestige 永久保留；grid 可点击条件；树节点（addNode）的 canClick/onClick
- endgame：触发冻结（timePlayed/点数全部停走）、Play Again / Keep Going（恢复时追赶 diff，受 maxTickLength 封顶）、endgame 期间热键禁用
- 热键（含 ctrl+ 组合）、离线生产（`offTime.remain` 按每 tick `remain/10` 消耗、`offlineLimit` 上限、Offline Time 显示）
- 存档：localStorage 格式与 TMT 完全一致（`btoa(unescape(encodeURIComponent(...)))`），导入/导出、异 mod 警告、beforeunload 同步保存、5 秒自动保存
- 数字格式化与 TMT 逐位一致（含 JS `toFixed` 舍入向 +∞ 平局的规则）

### 与原版的已知差异

- 渲染由 Rust diff 补丁驱动，无 Vue；`layer.minWidth` 等极少数仅影响布局的选项未移植
- `custom` prestige 类型仅为枚举占位，完整自定义公式请参照 demo 的 `f` 层用 `prestige_button_text` + 自定义 gain 实现
- 无 mod 加载器：游戏定义编译进 wasm（这正是性能的来源）

### 浏览器实测记录

demo 游戏全流程：全新开局 → prestige ×N → 升级/买进卖出 → 挑战 1/3 完成 → f 层 static prestige → 成就 EIEIO → grid 条件点击（points 恰好 = 10）→ Thanos/H 节点 → 热键 prestige → 存档刷新恢复 → endgame 冻结与 Keep Going，全部通过。
