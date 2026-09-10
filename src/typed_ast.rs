//! The typed AST: the type checker's output, an AST in which every expression
//! has a resolved type.

use crate::ast;
use crate::diag;

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
pub fn operand_type(op: ast::BinaryOp) -> Option<Type> {
    match op {
        ast::BinaryOp::Add
        | ast::BinaryOp::Sub
        | ast::BinaryOp::Mul
        | ast::BinaryOp::Div
        | ast::BinaryOp::Mod
        | ast::BinaryOp::Lt
        | ast::BinaryOp::Le
        | ast::BinaryOp::Gt
        | ast::BinaryOp::Ge => Some(Type::Uint64),
        ast::BinaryOp::Eq | ast::BinaryOp::Ne => None,
        ast::BinaryOp::And | ast::BinaryOp::Or => Some(Type::Bool),
    }
}

/// The type `op` produces.
pub fn result_type(op: ast::BinaryOp) -> Type {
    match op {
        ast::BinaryOp::Add
        | ast::BinaryOp::Sub
        | ast::BinaryOp::Mul
        | ast::BinaryOp::Div
        | ast::BinaryOp::Mod => Type::Uint64,
        ast::BinaryOp::Eq
        | ast::BinaryOp::Ne
        | ast::BinaryOp::Lt
        | ast::BinaryOp::Le
        | ast::BinaryOp::Gt
        | ast::BinaryOp::Ge
        | ast::BinaryOp::And
        | ast::BinaryOp::Or => Type::Bool,
    }
}

/// A variable's position in its function's frame: declarations counted
/// from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalId(pub u8);

impl LocalId {
    /// How many variables a frame can hold. `frame_dig` addresses a local
    /// with the non-negative half of a signed byte.
    pub const CAPACITY: usize = 128;

    /// The slot of the `index`th declaration, or `None` if a frame cannot
    /// hold that many.
    pub fn new(index: usize) -> Option<Self> {
        let slot = u8::try_from(index).ok()?;
        (usize::from(slot) < Self::CAPACITY).then_some(Self(slot))
    }
}

/// A whole source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The functions it declares, in source order.
    pub funcs: Vec<FuncDecl>,
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncDecl {
    /// The declared name.
    pub name: ast::Name,
    /// The resolved return type.
    pub ret: Type,
    /// The statements in the body, in source order.
    pub body: Vec<Stmt>,
    /// From `func` through the closing `}`.
    pub span: diag::Span,
}

/// A statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// `var`, with its name resolved to a frame slot.
    Var {
        /// The slot it declares.
        local: LocalId,
        /// The declared type.
        ty: Type,
        /// The initializer.
        init: Expr,
        /// From `var` through the initializer.
        span: diag::Span,
    },
    /// `return expr`.
    Return {
        /// The returned expression.
        expr: Expr,
        /// From `return` through the expression.
        span: diag::Span,
    },
}

/// An expression and its type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    /// What the expression is.
    pub kind: ExprKind,
    /// The type it has.
    pub ty: Type,
    /// Where it was written.
    pub span: diag::Span,
}

/// The kinds of expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    /// An integer literal.
    IntLit(u64),
    /// A boolean literal.
    BoolLit(bool),
    /// A binary operation.
    Binary {
        /// The operator it applies.
        op: ast::BinaryOp,
        /// The left operand.
        lhs: Box<Expr>,
        /// The right operand.
        rhs: Box<Expr>,
    },
    /// A prefix operation.
    Unary {
        /// The operator it applies.
        op: ast::UnaryOp,
        /// The operand.
        operand: Box<Expr>,
    },
    /// A variable, by frame slot.
    Var(LocalId),
}

impl Expr {
    /// Where it was written.
    pub fn span(&self) -> diag::Span {
        self.span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_takes_and_produces_uint64() {
        for op in [
            ast::BinaryOp::Add,
            ast::BinaryOp::Sub,
            ast::BinaryOp::Mul,
            ast::BinaryOp::Div,
            ast::BinaryOp::Mod,
        ] {
            assert_eq!(operand_type(op), Some(Type::Uint64), "{op:?}");
            assert_eq!(result_type(op), Type::Uint64, "{op:?}");
        }
    }

    #[test]
    fn equality_takes_operands_that_agree() {
        for op in [ast::BinaryOp::Eq, ast::BinaryOp::Ne] {
            assert_eq!(operand_type(op), None, "{op:?}");
            assert_eq!(result_type(op), Type::Bool, "{op:?}");
        }
    }

    #[test]
    fn ordering_takes_uint64_and_produces_bool() {
        for op in [
            ast::BinaryOp::Lt,
            ast::BinaryOp::Le,
            ast::BinaryOp::Gt,
            ast::BinaryOp::Ge,
        ] {
            assert_eq!(operand_type(op), Some(Type::Uint64), "{op:?}");
            assert_eq!(result_type(op), Type::Bool, "{op:?}");
        }
    }

    #[test]
    fn logic_takes_and_produces_bool() {
        for op in [ast::BinaryOp::And, ast::BinaryOp::Or] {
            assert_eq!(operand_type(op), Some(Type::Bool), "{op:?}");
            assert_eq!(result_type(op), Type::Bool, "{op:?}");
        }
    }
}
