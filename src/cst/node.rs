use crate::lexer;

/// A whole source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The functions it declares, in source order.
    pub funcs: Vec<FuncDecl>,
    /// The token that ends the stream.
    pub eof: lexer::Token,
}

/// A function declaration: `func name(params) ret { body }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncDecl {
    /// The `func` keyword.
    pub func: lexer::Token,
    /// The declared name.
    pub name: lexer::Token,
    /// The `(` of the parameter list.
    pub lparen: lexer::Token,
    /// The parameters it declares, in source order.
    pub params: Vec<Param>,
    /// The `)` of the parameter list.
    pub rparen: lexer::Token,
    /// The declared return type.
    pub ret: lexer::Token,
    /// The `{` opening the body.
    pub lbrace: lexer::Token,
    /// The statements in the body, in source order.
    pub body: Vec<Stmt>,
    /// The `}` closing the body.
    pub rbrace: lexer::Token,
}

/// `name ty`, and the `,` after it if one follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// The declared name.
    pub name: lexer::Token,
    /// The declared type.
    pub ty: lexer::Token,
    /// The `,` separating it from the next parameter; `None` on the last.
    pub comma: Option<lexer::Token>,
}

/// An argument, and the `,` after it if one follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arg {
    /// The argument itself.
    pub expr: Expr,
    /// The `,` separating it from the next argument; `None` on the last.
    pub comma: Option<lexer::Token>,
}

/// A statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// `var name ty = init`.
    Var {
        /// The `var` keyword.
        var: lexer::Token,
        /// The declared name.
        name: lexer::Token,
        /// The declared type.
        ty: lexer::Token,
        /// The `=`.
        equals: lexer::Token,
        /// The initializer.
        init: Expr,
    },
    /// `return expr`.
    Return {
        /// The `return` keyword.
        ret: lexer::Token,
        /// The returned expression.
        expr: Expr,
    },
}

/// An expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// An integer literal. Its value is not parsed here.
    IntLit(lexer::Token),
    /// A boolean literal.
    BoolLit(lexer::Token),
    /// A variable, by name.
    Var(lexer::Token),
    /// Two operands joined by a binary operator.
    Binary {
        /// The left operand.
        lhs: Box<Expr>,
        /// The operator.
        op: lexer::Token,
        /// The right operand.
        rhs: Box<Expr>,
    },
    /// A prefix operator applied to an operand.
    Unary {
        /// The operator.
        op: lexer::Token,
        /// The operand.
        operand: Box<Expr>,
    },
    /// `callee(args)`.
    Call {
        /// The called name.
        callee: lexer::Token,
        /// The `(` of the argument list.
        lparen: lexer::Token,
        /// The arguments, in source order.
        args: Vec<Arg>,
        /// The `)` of the argument list.
        rparen: lexer::Token,
    },
    /// `( inner )`.
    Paren {
        /// The `(`.
        lparen: lexer::Token,
        /// The parenthesized expression.
        inner: Box<Expr>,
        /// The `)`.
        rparen: lexer::Token,
    },
}
