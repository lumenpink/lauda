pub mod adapters;
pub mod domain;
#[cfg(not(target_arch = "wasm32"))]
pub mod tui;
#[cfg(not(target_arch = "wasm32"))]
pub mod update;
pub mod ui;

pub use ui::App;
