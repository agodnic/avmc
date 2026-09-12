//! The concrete syntax tree: the parser's output, holding every token.

mod flatten;
#[cfg(test)]
mod flatten_test;
mod node;
#[cfg(test)]
mod node_test;

pub use flatten::tokens;
pub use node::{Arg, Expr, FuncDecl, Param, Program, Stmt};
