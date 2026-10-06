//! Headless list logic for retained-mode UIs.
//!
//! Pure functions over slices and iterators: the app owns its state and renders however it
//! likes. Part of the GusStack family; extracted from ray-task.

mod group;

pub use group::group_runs;
