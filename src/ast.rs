//! The AST: the surface syntax, built from the concrete syntax tree.

use crate::cst;
use crate::diag;
use crate::token;

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
    pub span: diag::Span,
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

/// Builds the AST for `program`, reading names and literals from `source`.
///
/// Returns `None` as soon as anything is wrong; one run reports at most
/// one diagnostic.
pub fn from_cst(source: &str, program: &cst::Program, diags: &mut diag::Sink) -> Option<Program> {
    Builder { source, diags }.program(program)
}

/// The operator a token denotes, if it denotes one. The parser needs it for
/// precedence, and the AST stage to build the node.
pub(crate) fn binary_op(kind: token::Kind) -> Option<BinOp> {
    match kind {
        token::Kind::Plus => Some(BinOp::Add),
        token::Kind::Minus => Some(BinOp::Sub),
        token::Kind::Star => Some(BinOp::Mul),
        token::Kind::Slash => Some(BinOp::Div),
        token::Kind::Percent => Some(BinOp::Mod),
        token::Kind::EqEq => Some(BinOp::Eq),
        token::Kind::BangEq => Some(BinOp::Ne),
        token::Kind::Lt => Some(BinOp::Lt),
        token::Kind::LtEq => Some(BinOp::Le),
        token::Kind::Gt => Some(BinOp::Gt),
        token::Kind::GtEq => Some(BinOp::Ge),
        token::Kind::AmpAmp => Some(BinOp::And),
        token::Kind::PipePipe => Some(BinOp::Or),
        token::Kind::Func
        | token::Kind::Return
        | token::Kind::Var
        | token::Kind::True
        | token::Kind::False
        | token::Kind::Ident
        | token::Kind::IntLit
        | token::Kind::LParen
        | token::Kind::RParen
        | token::Kind::LBrace
        | token::Kind::RBrace
        | token::Kind::Equals
        | token::Kind::Bang
        | token::Kind::Eof => None,
    }
}

/// The prefix operator a token denotes, if it denotes one.
fn unary_op(kind: token::Kind) -> Option<UnOp> {
    (kind == token::Kind::Bang).then_some(UnOp::Not)
}

struct Builder<'a> {
    source: &'a str,
    diags: &'a mut diag::Sink,
}

impl Builder<'_> {
    fn program(&mut self, program: &cst::Program) -> Option<Program> {
        let mut funcs = Vec::new();
        for func in &program.funcs {
            funcs.push(self.func_decl(func)?);
        }
        Some(Program { funcs })
    }

    fn func_decl(&mut self, func: &cst::FuncDecl) -> Option<FuncDecl> {
        let name = self.name(func.name);
        let ret = TypeRef {
            name: self.name(func.ret),
        };

        let mut body = Vec::new();
        for stmt in &func.body {
            body.push(self.stmt(stmt)?);
        }

        Some(FuncDecl {
            name,
            ret,
            body,
            span: diag::Span {
                start: func.func.span.start,
                end: func.rbrace.span.end,
            },
        })
    }

    fn stmt(&mut self, stmt: &cst::Stmt) -> Option<Stmt> {
        match stmt {
            cst::Stmt::Var {
                var,
                name,
                ty,
                init,
                ..
            } => {
                let name = self.name(*name);
                let ty = TypeRef {
                    name: self.name(*ty),
                };
                let init = self.expr(init)?;
                let span = diag::Span {
                    start: var.span.start,
                    end: init.span().end,
                };
                Some(Stmt::Var {
                    name,
                    ty,
                    init,
                    span,
                })
            }
            cst::Stmt::Return { ret, expr } => {
                let expr = self.expr(expr)?;
                let span = diag::Span {
                    start: ret.span.start,
                    end: expr.span().end,
                };
                Some(Stmt::Return { expr, span })
            }
        }
    }

    fn expr(&mut self, expr: &cst::Expr) -> Option<Expr> {
        match expr {
            cst::Expr::IntLit(token) => {
                let span = token.span;
                match self.text(span).parse::<u64>() {
                    Ok(value) => Some(Expr::IntLit { value, span }),
                    // The lexer only admits ASCII digits, so overflow is the
                    // one way parsing can fail here.
                    Err(_) => self.report(diag::Kind::IntegerLiteralOutOfRange, span),
                }
            }
            cst::Expr::BoolLit(token) => Some(Expr::BoolLit {
                value: token.kind == token::Kind::True,
                span: token.span,
            }),
            cst::Expr::Var(token) => {
                let name = self.name(*token);
                let span = name.span;
                Some(Expr::Var { name, span })
            }
            cst::Expr::Binary { lhs, op, rhs } => {
                let lhs = self.expr(lhs)?;
                let rhs = self.expr(rhs)?;
                let span = diag::Span {
                    start: lhs.span().start,
                    end: rhs.span().end,
                };
                Some(Expr::Binary {
                    // The parser stores an operator token here, so the
                    // mapping never fails; failing beats mistranslating.
                    op: binary_op(op.kind)?,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                })
            }
            cst::Expr::Unary { op, operand } => {
                let operand = self.expr(operand)?;
                let span = diag::Span {
                    start: op.span.start,
                    end: operand.span().end,
                };
                Some(Expr::Unary {
                    op: unary_op(op.kind)?,
                    operand: Box::new(operand),
                    span,
                })
            }
            cst::Expr::Paren { inner, .. } => self.expr(inner),
        }
    }

    fn name(&self, token: token::Token) -> Name {
        Name {
            text: self.text(token.span).to_string(),
            span: token.span,
        }
    }

    /// Reports a diagnostic and fails the stage.
    fn report<T>(&mut self, kind: diag::Kind, span: diag::Span) -> Option<T> {
        self.diags.push(diag::Entry { kind, span });
        None
    }

    /// The source text a token covers. `lex` guarantees valid spans; the
    /// fallback is the one place this stage trusts it.
    fn text(&self, span: diag::Span) -> &str {
        self.source.get(span.start..span.end).unwrap_or("")
    }
}
