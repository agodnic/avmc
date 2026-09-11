//! The parser: tokens to a concrete syntax tree by recursive descent.

mod pass;
#[cfg(test)]
mod tests;

pub use pass::parse;
