//! The driver: chains the stages and renders diagnostics for display.
//!
//! Both functions are pure; all I/O lives in `src/bin`.

mod pipeline;
#[cfg(test)]
mod pipeline_test;
mod render;
#[cfg(test)]
mod render_test;

pub use pipeline::compile;
pub use render::render;
