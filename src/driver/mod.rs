//! The driver: chains the stages and renders diagnostics for display.
//!
//! Both functions are pure; all I/O lives in `src/bin`.

mod pipeline;
mod render;
#[cfg(test)]
mod tests;

pub use pipeline::compile;
pub use render::render;
