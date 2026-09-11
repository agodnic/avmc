use super::ty::Type;
use crate::ast;
use crate::diag;

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
        op: ast::BinOp,
        /// The left operand.
        lhs: Box<Expr>,
        /// The right operand.
        rhs: Box<Expr>,
    },
    /// A prefix operation.
    Unary {
        /// The operator it applies.
        op: ast::UnOp,
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
