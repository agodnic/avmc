//! The concrete syntax tree: the parser's output, holding every token.

mod flatten;
mod node;
#[cfg(test)]
mod tests;

pub use flatten::tokens;
pub use node::{Expr, FuncDecl, Program, Stmt};
