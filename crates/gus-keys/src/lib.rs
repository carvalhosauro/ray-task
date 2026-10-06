//! Headless keyboard shortcuts.
//!
//! Parse chords like `Ctrl+Shift+N`, bind them to your app's actions in a [`Keymap`], and look
//! up the action for a key event. The keymap is plain data, so the same list can drive the app,
//! a help screen and documentation tests. Part of the GusStack family; extracted from ray-task.

mod chord;
mod keymap;

pub use chord::{Chord, ChordError, Key, Mods};
pub use keymap::{Keymap, KeymapError};
