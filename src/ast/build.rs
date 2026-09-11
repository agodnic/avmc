//! Building the AST from the concrete syntax tree.

use super::node::{BinOp, Expr, FuncDecl, Name, Param, Program, Stmt, TypeRef, UnOp};
use crate::cst;
use crate::diag;
use crate::lexer;

/// Builds the AST for `program`, reading names and literals from `source`.
///
/// Returns `None` as soon as anything is wrong; one run reports at most
/// one diagnostic.
pub fn from_cst(source: &str, program: &cst::Program, diags: &mut diag::Sink) -> Option<Program> {
    Builder { source, diags }.program(program)
}

/// The operator a token denotes, if it denotes one. The parser needs it for
/// precedence, and the AST stage to build the node.
pub(crate) fn binary_op(kind: lexer::TokenKind) -> Option<BinOp> {
    match kind {
        lexer::TokenKind::Plus => Some(BinOp::Add),
        lexer::TokenKind::Minus => Some(BinOp::Sub),
        lexer::TokenKind::Star => Some(BinOp::Mul),
        lexer::TokenKind::Slash => Some(BinOp::Div),
        lexer::TokenKind::Percent => Some(BinOp::Mod),
        lexer::TokenKind::EqEq => Some(BinOp::Eq),
        lexer::TokenKind::BangEq => Some(BinOp::Ne),
        lexer::TokenKind::Lt => Some(BinOp::Lt),
        lexer::TokenKind::LtEq => Some(BinOp::Le),
        lexer::TokenKind::Gt => Some(BinOp::Gt),
        lexer::TokenKind::GtEq => Some(BinOp::Ge),
        lexer::TokenKind::AmpAmp => Some(BinOp::And),
        lexer::TokenKind::PipePipe => Some(BinOp::Or),
        lexer::TokenKind::Func
        | lexer::TokenKind::Return
        | lexer::TokenKind::Var
        | lexer::TokenKind::True
        | lexer::TokenKind::False
        | lexer::TokenKind::Ident
        | lexer::TokenKind::IntLit
        | lexer::TokenKind::LParen
        | lexer::TokenKind::RParen
        | lexer::TokenKind::LBrace
        | lexer::TokenKind::RBrace
        | lexer::TokenKind::Comma
        | lexer::TokenKind::Equals
        | lexer::TokenKind::Bang
        | lexer::TokenKind::Eof => None,
    }
}

/// The prefix operator a token denotes, if it denotes one.
fn unary_op(kind: lexer::TokenKind) -> Option<UnOp> {
    (kind == lexer::TokenKind::Bang).then_some(UnOp::Not)
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
        let params = func.params.iter().map(|param| self.param(param)).collect();
        let ret = TypeRef {
            name: self.name(func.ret),
        };

        let mut body = Vec::new();
        for stmt in &func.body {
            body.push(self.stmt(stmt)?);
        }

        Some(FuncDecl {
            name,
            params,
            ret,
            body,
            span: diag::Span {
                start: func.func.span.start,
                end: func.rbrace.span.end,
            },
        })
    }

    fn param(&self, param: &cst::Param) -> Param {
        Param {
            name: self.name(param.name),
            ty: TypeRef {
                name: self.name(param.ty),
            },
        }
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
                value: token.kind == lexer::TokenKind::True,
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

    fn name(&self, token: lexer::Token) -> Name {
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
