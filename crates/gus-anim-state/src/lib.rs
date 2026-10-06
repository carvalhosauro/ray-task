//! Headless animation timing for retained-mode UIs.
//!
//! Each key plays a script of timed phases; the app passes "now" and reacts to the phase
//! changes [`Timeline::advance`] returns. No clock, no threads, no callbacks: easy to drive from
//! any event loop and to test with fixed times. Part of the GusStack family; extracted from
//! ray-task.

mod timeline;

pub use timeline::{Change, Timeline};
