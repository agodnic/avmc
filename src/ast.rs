//! The AST: the parser's output, mirroring the surface syntax.

use crate::diagnostics;

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
    pub span: diagnostics::Span,
}

/// An identifier and where it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The identifier text, sliced from the source.
    pub text: String,
    /// Where it was written.
    pub span: diagnostics::Span,
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
    /// `var name type = init`.
    Var {
        /// The declared name.
        name: Name,
        /// The declared type.
        ty: TypeRef,
        /// The initializer.
        init: Expr,
        /// From `var` through the initializer.
        span: diagnostics::Span,
    },
    /// `return expr`.
    Return {
        /// The returned expression.
        expr: Expr,
        /// From `return` through the expression.
        span: diagnostics::Span,
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
        span: diagnostics::Span,
    },
    /// A boolean literal.
    BoolLit {
        /// Its value.
        value: bool,
        /// Where it was written.
        span: diagnostics::Span,
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
        span: diagnostics::Span,
    },
    /// A prefix operator applied to an operand.
    Unary {
        /// The operator.
        op: UnaryOp,
        /// The operand.
        operand: Box<Expr>,
        /// From the operator through the last byte of `operand`.
        span: diagnostics::Span,
    },
    /// A variable, by name.
    Var {
        /// The name it was written as.
        name: Name,
        /// `name.span`, unless parentheses widened it.
        span: diagnostics::Span,
    },
}

impl Expr {
    /// Where it was written.
    pub fn span(&self) -> diagnostics::Span {
        match self {
            Expr::IntLit { span, .. }
            | Expr::BoolLit { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Var { span, .. } => *span,
        }
    }

    /// The same expression, written at `span`.
    pub fn with_span(self, span: diagnostics::Span) -> Expr {
        match self {
            Expr::IntLit { value, .. } => Expr::IntLit { value, span },
            Expr::BoolLit { value, .. } => Expr::BoolLit { value, span },
            Expr::Binary { op, lhs, rhs, .. } => Expr::Binary { op, lhs, rhs, span },
            Expr::Unary { op, operand, .. } => Expr::Unary { op, operand, span },
            Expr::Var { name, .. } => Expr::Var { name, span },
        }
    }
}

/// A binary operator.
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
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    And,
    /// `||`
    Or,
}

/// A prefix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `!`
    Not,
}
