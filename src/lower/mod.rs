//! Lowering: a typed AST to IR.

mod pass;
#[cfg(test)]
mod pass_test;

pub use pass::lower;
