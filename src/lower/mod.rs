//! Lowering: a typed AST to IR.

mod pass;
#[cfg(test)]
mod tests;

pub use pass::lower;
