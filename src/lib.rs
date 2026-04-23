mod app;
mod camera;
mod instance;
mod simulation;
mod state;
mod texture;
mod vertex;

/// Makes `run()` reachable as `ferrolife::run()` respectively.
pub use app::run;
#[cfg(target_arch = "wasm32")]
/// Makes `run_web()` reachable as `ferrolife::run_web()` respectively.
pub use app::run_web;
