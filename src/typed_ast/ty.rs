use crate::ast;

/// A resolved type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// A 64-bit unsigned integer.
    Uint64,
    /// A truth value, held as the AVM holds one: a `uint64` that is `0` for
    /// `false` and nonzero for `true`. The compiler only ever produces `1`
    /// for `true`.
    Bool,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Uint64 => write!(f, "uint64"),
            Type::Bool => write!(f, "bool"),
        }
    }
}

/// The type both operands of `op` must have, or `None` if they need only
/// agree with each other.
pub fn operand_type(op: ast::BinOp) -> Option<Type> {
    match op {
        ast::BinOp::Add
        | ast::BinOp::Sub
        | ast::BinOp::Mul
        | ast::BinOp::Div
        | ast::BinOp::Mod
        | ast::BinOp::Lt
        | ast::BinOp::Le
        | ast::BinOp::Gt
        | ast::BinOp::Ge => Some(Type::Uint64),
        ast::BinOp::Eq | ast::BinOp::Ne => None,
        ast::BinOp::And | ast::BinOp::Or => Some(Type::Bool),
    }
}

/// The type `op` produces.
pub fn result_type(op: ast::BinOp) -> Type {
    match op {
        ast::BinOp::Add | ast::BinOp::Sub | ast::BinOp::Mul | ast::BinOp::Div | ast::BinOp::Mod => {
            Type::Uint64
        }
        ast::BinOp::Eq
        | ast::BinOp::Ne
        | ast::BinOp::Lt
        | ast::BinOp::Le
        | ast::BinOp::Gt
        | ast::BinOp::Ge
        | ast::BinOp::And
        | ast::BinOp::Or => Type::Bool,
    }
}
