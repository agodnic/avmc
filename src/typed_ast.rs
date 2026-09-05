//! The typed AST: the type checker's output, an AST in which every expression
//! has a resolved type.

use crate::ast::Name;
use crate::diagnostics::Span;

/// A resolved type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// A 64-bit unsigned integer.
    Uint64,
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
    pub name: Name,
    /// The resolved return type.
    pub ret: Type,
    /// The statements in the body, in source order.
    pub body: Vec<Stmt>,
    /// From `func` through the closing `}`.
    pub span: Span,
}

/// A statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// `return expr`.
    Return {
        /// The returned expression.
        expr: Expr,
        /// From `return` through the expression.
        span: Span,
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
    pub span: Span,
}

/// The kinds of expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    /// An integer literal.
    IntLit(u64),
}

impl Expr {
    /// Where it was written.
    pub fn span(&self) -> Span {
        self.span
    }
}
