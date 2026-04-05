//! Cambridge Pseudocode Interpreter - library crate.
//!
//! The `[lib]` target is used by `wasm-pack` to produce the WASM package.
//! The `[[bin]]` target (`src/main.rs`) provides the `cps` CLI and is
//! unaffected by this file.

#![allow(non_snake_case)]

pub mod errortype;
pub mod Lexer;
pub mod Parser;
pub mod Inter;

#[cfg(target_arch = "wasm32")]
pub mod wasm_api;
