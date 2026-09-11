//! The surface-syntax node types.

use crate::diag;

/// A whole source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The functions it declares, in source order.
    pub funcs: Vec<FuncDecl>,
}

/// A function declaration: `func name(params) ret { body }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncDecl {
    /// The declared name.
    pub name: Name,
    /// The parameters it declares, in source order.
    pub params: Vec<Param>,
    /// The declared return type.
    pub ret: TypeRef,
    /// The statements in the body, in source order.
    pub body: Vec<Stmt>,
    /// From `func` through the closing `}`.
    pub span: diag::Span,
}

/// A parameter: `name type`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// The declared name.
    pub name: Name,
    /// The declared type.
    pub ty: TypeRef,
}

/// An identifier and where it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The identifier text, sliced from the source.
    pub text: String,
    /// Where it was written.
    pub span: diag::Span,
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

/// An expression. Parentheses are not a node: the AST of `(x)` is that of `x`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// An integer literal.
    IntLit {
        /// Its value.
        value: u64,
        /// Where it was written.
        span: diag::Span,
    },
    /// A boolean literal.
    BoolLit {
        /// Its value.
        value: bool,
        /// Where it was written.
        span: diag::Span,
    },
    /// Two operands joined by a binary operator.
    Binary {
        /// The operator.
        op: BinOp,
        /// The left operand.
        lhs: Box<Expr>,
        /// The right operand.
        rhs: Box<Expr>,
        /// From the first byte of `lhs` through the last byte of `rhs`.
        span: diag::Span,
    },
    /// A prefix operator applied to an operand.
    Unary {
        /// The operator.
        op: UnOp,
        /// The operand.
        operand: Box<Expr>,
        /// From the operator through the last byte of `operand`.
        span: diag::Span,
    },
    /// A variable, by name.
    Var {
        /// The name it was written as.
        name: Name,
        /// `name.span`.
        span: diag::Span,
    },
}

impl Expr {
    /// Where it was written.
    pub fn span(&self) -> diag::Span {
        match self {
            Expr::IntLit { span, .. }
            | Expr::BoolLit { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Var { span, .. } => *span,
        }
    }
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
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
pub enum UnOp {
    /// `!`
    Not,
}
