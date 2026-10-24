//! The Modding Tree — Rust edition.
//!
//! A Rust rewrite of the The Modding Tree incremental-game template
//! (https://github.com/Acamaeda/The-Modding-Tree). The whole engine — game
//! state, layer logic, reset math, number formatting and UI tree diffing —
//! lives in Rust and is compiled to WebAssembly; `glue.js` on the page is a
//! thin renderer that applies DOM patches produced here.
//!
//! Number handling is provided by `break-eternity-rs`
//! (https://github.com/MaddisonM79/break-eternity-rs), a maintained Rust port
//! of the break_eternity.js library The Modding Tree uses.

pub mod defs;
pub mod demo;
pub mod ffi;
pub mod format;
pub mod game;
pub mod json;
pub mod player;
pub mod render;

pub use break_eternity::{Decimal, TetrationMode};
