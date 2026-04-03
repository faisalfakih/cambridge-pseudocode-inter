pub mod cps;
pub mod interpreter;
pub mod builtins;
pub mod step_interpreter;

// Thread+channel web bridge — not available in WASM builds.
#[cfg(not(target_arch = "wasm32"))]
pub mod web;
