//! The typed AST: the type checker's output, an AST in which every expression
//! has a resolved type.

mod node;
#[cfg(test)]
mod tests;
mod ty;

pub use node::{Expr, ExprKind, FuncDecl, FuncId, LocalId, Param, ParamId, Program, Stmt};
pub use ty::{Type, operand_type, result_type};
