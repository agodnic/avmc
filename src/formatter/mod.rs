//! The formatter: source text to canonical source text.

mod pass;
#[cfg(test)]
mod tests;

pub use pass::format;
