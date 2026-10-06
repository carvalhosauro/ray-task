//! Headless combobox for text fields with a suggestion list.
//!
//! [`matches`] ranks the options for the typed text, [`completion`] finds the inline
//! completion, and [`Combobox`] turns ↑ / ↓ / Enter / Esc / Tab into [`Outcome`]s. The app keeps
//! the text and the options; the crate keeps the rules. Part of the GusStack family; extracted
//! from ray-task.

mod combobox;
mod matching;

pub use combobox::{Combobox, Nav, Outcome};
pub use matching::{completion, matches};
