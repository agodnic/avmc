//! The AST: the parser's output, mirroring the surface syntax (see
//! `ARCHITECTURE.md` §2.1).

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

/// An identifier and where it was written (R2).
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

/// An expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// An integer literal.
    IntLit {
        /// Its value.
        value: u64,
        /// Where it was written.
        span: Span,
    },
}
