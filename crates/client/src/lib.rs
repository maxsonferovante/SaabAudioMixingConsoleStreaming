pub mod adapters;

#[cfg(not(target_os = "android"))]
pub mod ui;

pub use adapters::*;

#[cfg(not(target_os = "android"))]
pub use ui::{ConsoleApp, ConsoleFlags};
