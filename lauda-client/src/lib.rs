pub mod adapters;
pub mod domain;
#[cfg(not(target_arch = "wasm32"))]
pub mod tui;
pub mod ui;
#[cfg(not(target_arch = "wasm32"))]
pub mod update;

pub use ui::App;
