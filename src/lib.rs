pub mod game;

#[cfg(all(feature = "solver", not(target_arch = "wasm32")))]
pub mod solver;

pub mod ui;

#[cfg(target_arch = "wasm32")]
mod web;
