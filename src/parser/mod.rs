//! The parser: tokens to a concrete syntax tree by recursive descent.

mod pass;
#[cfg(test)]
mod pass_test;

pub use pass::parse;
