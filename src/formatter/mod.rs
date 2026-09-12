//! The formatter: source text to canonical source text.

mod pass;
#[cfg(test)]
mod pass_test;

pub use pass::format;
