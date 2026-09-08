//! The AST: the parser's output, mirroring the surface syntax.

use crate::diagnostics::Span;

/// A whole source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The functions it declares, in source order.
    pub funcs: Vec<FuncDecl>,
}

/// A function declaration: `func name() ret { body }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncDecl {
    /// The declared name.
    pub name: Name,
    /// The declared return type.
    pub ret: TypeRef,
    /// The statements in the body, in source order.
    pub body: Vec<Stmt>,
    /// From `func` through the closing `}`.
    pub span: Span,
}

/// An identifier and where it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The identifier text, sliced from the source.
    pub text: String,
    /// Where it was written.
    pub span: Span,
}

/// A written type. Unresolved: nothing checks that the name names a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    /// The type's name as written.
    pub name: Name,
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

/// An expression. Parentheses are not a node: they only widen a span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// An integer literal.
    IntLit {
        /// Its value.
        value: u64,
        /// Where it was written.
        span: Span,
    },
    /// Two operands joined by a binary operator.
    Binary {
        /// The operator.
        op: BinaryOp,
        /// The left operand.
        lhs: Box<Expr>,
        /// The right operand.
        rhs: Box<Expr>,
        /// From the first byte of `lhs` through the last byte of `rhs`.
        span: Span,
    },
}

impl Expr {
    /// Where it was written.
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit { span, .. } | Expr::Binary { span, .. } => *span,
        }
    }

    /// The same expression, written at `span`.
    pub fn with_span(self, span: Span) -> Expr {
        match self {
            Expr::IntLit { value, .. } => Expr::IntLit { value, span },
            Expr::Binary { op, lhs, rhs, .. } => Expr::Binary { op, lhs, rhs, span },
        }
    }
}

/// A binary arithmetic operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
}
