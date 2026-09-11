use crate::ast;

/// A node in the precedence graph. Operators in one group share a precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// `+` and `-`.
    Additive,
    /// `*` and `/`.
    Multiplicative,
    /// `%`.
    Modulo,
    /// `==`, `!=`, `<`, `<=`, `>`, and `>=`.
    Comparison,
    /// `!`.
    Not,
    /// `&&`.
    And,
    /// `||`.
    Or,
}

/// The group an operator belongs to.
pub fn group(op: ast::BinOp) -> Group {
    match op {
        ast::BinOp::Add | ast::BinOp::Sub => Group::Additive,
        ast::BinOp::Mul | ast::BinOp::Div => Group::Multiplicative,
        ast::BinOp::Mod => Group::Modulo,
        ast::BinOp::Eq
        | ast::BinOp::Ne
        | ast::BinOp::Lt
        | ast::BinOp::Le
        | ast::BinOp::Gt
        | ast::BinOp::Ge => Group::Comparison,
        ast::BinOp::And => Group::And,
        ast::BinOp::Or => Group::Or,
    }
}

/// The group a prefix operator belongs to.
pub fn unary_group(op: ast::UnOp) -> Group {
    match op {
        ast::UnOp::Not => Group::Not,
    }
}
