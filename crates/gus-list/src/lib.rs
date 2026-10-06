//! Headless list logic for retained-mode UIs.
//!
//! Pure functions over slices and iterators: the app owns its state and renders however it
//! likes. Part of the GusStack family; extracted from ray-task.

mod diff;
mod group;
mod select;

pub use diff::{diff, Op, Plan, DEFAULT_MAX_IN_PLACE};
pub use group::group_runs;
pub use select::step;
