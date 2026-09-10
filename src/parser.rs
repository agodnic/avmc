//! The parser: tokens to an AST by recursive descent.

use crate::ast;
use crate::diagnostics;
use crate::lexer;
use crate::precedence;

/// Parses the token stream `lex` produced for `source`.
///
/// Returns `None` as soon as anything is wrong; one run reports at most
/// one diagnostic.
pub fn parse(
    source: &str,
    tokens: &[lexer::Token],
    diags: &mut diagnostics::Diagnostics,
) -> Option<ast::Program> {
    Parser {
        source,
        tokens,
        next: 0,
        diags,
    }
    .program()
}

/// What the grammar allows where an operand is expected.
const OPERAND: &str = "a literal, an identifier, `!`, or `(`";

struct Parser<'a> {
    source: &'a str,
    tokens: &'a [lexer::Token],
    /// Index of the token to be consumed next.
    next: usize,
    diags: &'a mut diagnostics::Diagnostics,
}

impl Parser<'_> {
    fn program(&mut self) -> Option<ast::Program> {
        let mut funcs = Vec::new();
        while self.peek().is_some() {
            funcs.push(self.func_decl()?);
        }
        Some(ast::Program { funcs })
    }

    fn func_decl(&mut self) -> Option<ast::FuncDecl> {
        let start = self.expect(lexer::TokenKind::Func, "`func`")?.span.start;
        let name = self.name()?;
        self.expect(lexer::TokenKind::LParen, "`(`")?;
        self.expect(lexer::TokenKind::RParen, "`)`")?;
        let ret = ast::TypeRef { name: self.name()? };
        self.expect(lexer::TokenKind::LBrace, "`{`")?;

        let mut body = Vec::new();
        while self.peek_kind() != Some(lexer::TokenKind::RBrace) {
            body.push(self.stmt()?);
        }
        let end = self.expect(lexer::TokenKind::RBrace, "`}`")?.span.end;

        Some(ast::FuncDecl {
            name,
            ret,
            body,
            span: diagnostics::Span { start, end },
        })
    }

    fn stmt(&mut self) -> Option<ast::Stmt> {
        if self.peek_kind() == Some(lexer::TokenKind::Var) {
            return self.var_stmt();
        }

        let start = self
            .expect(lexer::TokenKind::Return, "`var`, `return` or `}`")?
            .span
            .start;
        let expr = self.expr(None)?;
        let end = expr.span().end;
        Some(ast::Stmt::Return {
            expr,
            span: diagnostics::Span { start, end },
        })
    }

    /// `var name type = init`.
    fn var_stmt(&mut self) -> Option<ast::Stmt> {
        let start = self.expect(lexer::TokenKind::Var, "`var`")?.span.start;
        let name = self.name()?;
        let ty = ast::TypeRef { name: self.name()? };
        self.expect(lexer::TokenKind::Equals, "`=`")?;
        let init = self.expr(None)?;
        let end = init.span().end;
        Some(ast::Stmt::Var {
            name,
            ty,
            init,
            span: diagnostics::Span { start, end },
        })
    }

    /// An expression, folded left to right under the precedence graph.
    ///
    /// `ambient` is the enclosing operator, or `None` at the start of an
    /// expression and inside parentheses.
    // `expr` and `operand` recurse, so deeply nested parentheses can exhaust
    // the stack. Bounding the nesting depth would cost a counter and a
    // diagnostic, and no hand-written source comes close to the limit. The
    // trade is deliberate and is not being revisited yet: do not add a bound,
    // and do not report this as a bug.
    fn expr(&mut self, ambient: Option<lexer::Token>) -> Option<ast::Expr> {
        let mut lhs = self.operand(ambient)?;
        // The caller only ever passes a token it consumed as an operator.
        let enclosing = ambient.and_then(|token| Some((token, operator_group(token.kind)?)));

        while let Some((token, op)) = self.peek_binary_op() {
            if let Some((left, left_group)) = enclosing {
                match precedence::priority(left_group, precedence::group(op)) {
                    // The enclosing operator takes the operand just parsed.
                    precedence::Priority::Left => break,
                    precedence::Priority::Ambiguous => {
                        let kind = diagnostics::DiagnosticKind::AmbiguousPrecedence {
                            left: describe(left.kind),
                            right: describe(token.kind),
                        };
                        return self.report(kind, token.span);
                    }
                    precedence::Priority::Right => {}
                }
            }

            self.next += 1;
            let rhs = self.expr(Some(token))?;
            let span = diagnostics::Span {
                start: lhs.span().start,
                end: rhs.span().end,
            };
            lhs = ast::Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }

        Some(lhs)
    }

    /// A literal, a variable, a parenthesized expression, or `!` applied to
    /// one. `ambient` is the enclosing operator, as in `expr`.
    fn operand(&mut self, ambient: Option<lexer::Token>) -> Option<ast::Expr> {
        if let Some(&bang) = self
            .peek()
            .filter(|token| token.kind == lexer::TokenKind::Bang)
        {
            return self.not(bang, ambient);
        }

        if self.peek_kind() == Some(lexer::TokenKind::LParen) {
            let start = self.expect(lexer::TokenKind::LParen, OPERAND)?.span.start;
            let inner = self.expr(None)?;
            let end = self.expect(lexer::TokenKind::RParen, "`)`")?.span.end;
            return Some(inner.with_span(diagnostics::Span { start, end }));
        }

        if let Some(kind @ (lexer::TokenKind::True | lexer::TokenKind::False)) = self.peek_kind() {
            let span = self.expect(kind, OPERAND)?.span;
            return Some(ast::Expr::BoolLit {
                value: kind == lexer::TokenKind::True,
                span,
            });
        }

        if self.peek_kind() == Some(lexer::TokenKind::Ident) {
            let name = self.name()?;
            let span = name.span;
            return Some(ast::Expr::Var { name, span });
        }

        let span = self.expect(lexer::TokenKind::IntLit, OPERAND)?.span;
        match self.text(span).parse::<u64>() {
            Ok(value) => Some(ast::Expr::IntLit { value, span }),
            // The lexer only admits ASCII digits, so overflow is the one way
            // parsing can fail here.
            Err(_) => self.report(diagnostics::DiagnosticKind::IntegerLiteralOutOfRange, span),
        }
    }

    /// `!` applied to an operand, `bang` being the operator token, which is
    /// still to be consumed.
    fn not(&mut self, bang: lexer::Token, ambient: Option<lexer::Token>) -> Option<ast::Expr> {
        // An enclosing operator the graph does not order against `!` is the
        // whole point of a partial order: the source must parenthesize.
        let enclosing = ambient.and_then(|token| Some((token, operator_group(token.kind)?)));
        if let Some((left, left_group)) = enclosing
            && precedence::priority(left_group, precedence::unary_group(ast::UnaryOp::Not))
                == precedence::Priority::Ambiguous
        {
            let kind = diagnostics::DiagnosticKind::AmbiguousPrecedence {
                left: describe(left.kind),
                right: describe(bang.kind),
            };
            return self.report(kind, bang.span);
        }

        self.next += 1;
        // Parsing the operand with `!` enclosing it is what places the
        // operators that follow it.
        let operand = self.expr(Some(bang))?;
        let span = diagnostics::Span {
            start: bang.span.start,
            end: operand.span().end,
        };
        Some(ast::Expr::Unary {
            op: ast::UnaryOp::Not,
            operand: Box::new(operand),
            span,
        })
    }

    fn name(&mut self) -> Option<ast::Name> {
        let span = self.expect(lexer::TokenKind::Ident, "an identifier")?.span;
        Some(ast::Name {
            text: self.text(span).to_string(),
            span,
        })
    }

    /// Consumes the next token if it is a `kind`, and reports the unexpected
    /// one otherwise. `expected` describes what the grammar allows here.
    fn expect(&mut self, kind: lexer::TokenKind, expected: &'static str) -> Option<lexer::Token> {
        match self.peek() {
            Some(&token) if token.kind == kind => {
                self.next += 1;
                Some(token)
            }
            found => {
                let (found, span) = match found {
                    Some(token) => (describe(token.kind), token.span),
                    None => (
                        "end of input",
                        diagnostics::Span {
                            start: self.source.len(),
                            end: self.source.len(),
                        },
                    ),
                };
                self.report(
                    diagnostics::DiagnosticKind::UnexpectedToken { expected, found },
                    span,
                )
            }
        }
    }

    /// Reports a diagnostic and fails the parse.
    fn report<T>(
        &mut self,
        kind: diagnostics::DiagnosticKind,
        span: diagnostics::Span,
    ) -> Option<T> {
        self.diags.push(diagnostics::Diagnostic { kind, span });
        None
    }

    fn peek(&self) -> Option<&lexer::Token> {
        self.tokens.get(self.next)
    }

    fn peek_kind(&self) -> Option<lexer::TokenKind> {
        self.peek().map(|token| token.kind)
    }

    /// The next token and the operator it denotes, if it denotes one.
    fn peek_binary_op(&self) -> Option<(lexer::Token, ast::BinaryOp)> {
        let &token = self.peek()?;
        Some((token, binary_op(token.kind)?))
    }

    /// The source text a token covers. `lex` guarantees valid spans; the
    /// fallback is the one place the parser trusts it.
    fn text(&self, span: diagnostics::Span) -> &str {
        self.source.get(span.start..span.end).unwrap_or("")
    }
}

fn describe(kind: lexer::TokenKind) -> &'static str {
    match kind {
        lexer::TokenKind::Func => "`func`",
        lexer::TokenKind::Return => "`return`",
        lexer::TokenKind::Var => "`var`",
        lexer::TokenKind::True => "`true`",
        lexer::TokenKind::False => "`false`",
        lexer::TokenKind::Ident => "an identifier",
        lexer::TokenKind::IntLit => "an integer literal",
        lexer::TokenKind::LParen => "`(`",
        lexer::TokenKind::RParen => "`)`",
        lexer::TokenKind::LBrace => "`{`",
        lexer::TokenKind::RBrace => "`}`",
        lexer::TokenKind::Plus => "`+`",
        lexer::TokenKind::Minus => "`-`",
        lexer::TokenKind::Star => "`*`",
        lexer::TokenKind::Slash => "`/`",
        lexer::TokenKind::Percent => "`%`",
        lexer::TokenKind::Equals => "`=`",
        lexer::TokenKind::EqEq => "`==`",
        lexer::TokenKind::BangEq => "`!=`",
        lexer::TokenKind::Lt => "`<`",
        lexer::TokenKind::LtEq => "`<=`",
        lexer::TokenKind::Gt => "`>`",
        lexer::TokenKind::GtEq => "`>=`",
        lexer::TokenKind::Bang => "`!`",
        lexer::TokenKind::AmpAmp => "`&&`",
        lexer::TokenKind::PipePipe => "`||`",
    }
}

/// The precedence group of an operator token, if it is one.
fn operator_group(kind: lexer::TokenKind) -> Option<precedence::Group> {
    match kind {
        lexer::TokenKind::Bang => Some(precedence::unary_group(ast::UnaryOp::Not)),
        kind => binary_op(kind).map(precedence::group),
    }
}

/// The operator a token denotes, if it denotes one.
fn binary_op(kind: lexer::TokenKind) -> Option<ast::BinaryOp> {
    match kind {
        lexer::TokenKind::Plus => Some(ast::BinaryOp::Add),
        lexer::TokenKind::Minus => Some(ast::BinaryOp::Sub),
        lexer::TokenKind::Star => Some(ast::BinaryOp::Mul),
        lexer::TokenKind::Slash => Some(ast::BinaryOp::Div),
        lexer::TokenKind::Percent => Some(ast::BinaryOp::Mod),
        lexer::TokenKind::EqEq => Some(ast::BinaryOp::Eq),
        lexer::TokenKind::BangEq => Some(ast::BinaryOp::Ne),
        lexer::TokenKind::Lt => Some(ast::BinaryOp::Lt),
        lexer::TokenKind::LtEq => Some(ast::BinaryOp::Le),
        lexer::TokenKind::Gt => Some(ast::BinaryOp::Gt),
        lexer::TokenKind::GtEq => Some(ast::BinaryOp::Ge),
        lexer::TokenKind::AmpAmp => Some(ast::BinaryOp::And),
        lexer::TokenKind::PipePipe => Some(ast::BinaryOp::Or),
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
        | lexer::TokenKind::Equals
        | lexer::TokenKind::Bang => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    /// Lexes and parses `source`, asserting that it produced no diagnostics.
    fn parse_ok(source: &str) -> ast::Program {
        let mut diags = diagnostics::Diagnostics::default();
        let tokens = lexer::lex(source, &mut diags).expect("lexing succeeded");
        let program = parse(source, &tokens, &mut diags);
        assert!(diags.is_empty());
        program.expect("parsing succeeded")
    }

    /// Lexes and parses `source`, asserting that it reported exactly one
    /// diagnostic and produced nothing.
    fn parse_err(source: &str) -> diagnostics::Diagnostic {
        let mut diags = diagnostics::Diagnostics::default();
        let tokens = lexer::lex(source, &mut diags).expect("lexing succeeded");
        assert_eq!(parse(source, &tokens, &mut diags), None);
        let mut reported = diags.iter();
        let diagnostic = reported.next().expect("one diagnostic").clone();
        assert!(reported.next().is_none());
        diagnostic
    }

    #[test]
    fn parses_the_approval_program() {
        let source = "func approval() uint64 {\n  return 1\n}\n";
        let mut span = testing::spans(source);

        let start = span("func").start;
        let func_name = testing::name("approval", span("approval"));
        let ret = ast::TypeRef {
            name: testing::name("uint64", span("uint64")),
        };
        let return_start = span("return").start;
        let literal = span("1");
        let end = span("}").end;

        assert_eq!(
            parse_ok(source),
            ast::Program {
                funcs: vec![ast::FuncDecl {
                    name: func_name,
                    ret,
                    body: vec![ast::Stmt::Return {
                        expr: ast::Expr::IntLit {
                            value: 1,
                            span: literal
                        },
                        span: diagnostics::Span {
                            start: return_start,
                            end: literal.end
                        },
                    }],
                    span: diagnostics::Span { start, end },
                }]
            }
        );
    }

    /// The example program of the variables milestone.
    const VARIABLES: &str = "func approval() uint64 {\n  var x uint64 = 1 + 2\n  \
                             var y uint64 = x * 3\n  return y - x\n}\n";

    /// The variable `text`, written at `span`.
    fn var(text: &str, span: diagnostics::Span) -> ast::Expr {
        ast::Expr::Var {
            name: testing::name(text, span),
            span,
        }
    }

    /// `op operand`, the operator being the byte just before the operand.
    fn unary(op: ast::UnaryOp, operand: ast::Expr) -> ast::Expr {
        let span = diagnostics::Span {
            start: operand.span().start - 1,
            end: operand.span().end,
        };
        ast::Expr::Unary {
            op,
            operand: Box::new(operand),
            span,
        }
    }

    /// `lhs op rhs`, spanning from one operand to the other.
    fn binary(op: ast::BinaryOp, lhs: ast::Expr, rhs: ast::Expr) -> ast::Expr {
        let span = diagnostics::Span {
            start: lhs.span().start,
            end: rhs.span().end,
        };
        ast::Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            span,
        }
    }

    #[test]
    fn parses_the_variables_program() {
        let source = VARIABLES;
        let mut span = testing::spans(source);

        let start = span("func").start;
        let func_name = testing::name("approval", span("approval"));
        span("(");
        span(")");
        let ret = ast::TypeRef {
            name: testing::name("uint64", span("uint64")),
        };
        span("{");

        let first_var = span("var").start;
        let x = testing::name("x", span("x"));
        let x_ty = ast::TypeRef {
            name: testing::name("uint64", span("uint64")),
        };
        span("=");
        let one = span("1");
        let two = span("2");

        let second_var = span("var").start;
        let y = testing::name("y", span("y"));
        let y_ty = ast::TypeRef {
            name: testing::name("uint64", span("uint64")),
        };
        span("=");
        let x_times = span("x");
        let three = span("3");

        let return_start = span("return").start;
        let y_minus = span("y");
        let x_minus = span("x");
        let end = span("}").end;

        let int = |value, span| ast::Expr::IntLit { value, span };
        let var = |text: &str, span| ast::Expr::Var {
            name: testing::name(text, span),
            span,
        };

        assert_eq!(
            parse_ok(source),
            ast::Program {
                funcs: vec![ast::FuncDecl {
                    name: func_name,
                    ret,
                    body: vec![
                        ast::Stmt::Var {
                            name: x,
                            ty: x_ty,
                            init: binary(ast::BinaryOp::Add, int(1, one), int(2, two)),
                            span: diagnostics::Span {
                                start: first_var,
                                end: two.end
                            },
                        },
                        ast::Stmt::Var {
                            name: y,
                            ty: y_ty,
                            init: binary(ast::BinaryOp::Mul, var("x", x_times), int(3, three)),
                            span: diagnostics::Span {
                                start: second_var,
                                end: three.end
                            },
                        },
                        ast::Stmt::Return {
                            expr: binary(ast::BinaryOp::Sub, var("y", y_minus), var("x", x_minus)),
                            span: diagnostics::Span {
                                start: return_start,
                                end: x_minus.end
                            },
                        },
                    ],
                    span: diagnostics::Span { start, end },
                }]
            }
        );
    }

    #[test]
    fn a_variable_is_an_operand() {
        let source = wrap("x + 1");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let x = span("x");
        let one = span("1");

        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::Add,
                ast::Expr::Var {
                    name: testing::name("x", x),
                    span: x
                },
                ast::Expr::IntLit {
                    value: 1,
                    span: one
                },
            )
        );
    }

    #[test]
    fn parentheses_around_a_variable_widen_its_span() {
        let source = wrap("(x)");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let open = span("(");
        let x = span("x");
        let close = span(")");

        assert_eq!(
            returned(&source),
            ast::Expr::Var {
                name: testing::name("x", x),
                span: diagnostics::Span {
                    start: open.start,
                    end: close.end
                },
            }
        );
    }

    #[test]
    fn a_declaration_without_a_type_is_reported() {
        let source = "func f() uint64 { var x = 1 }";
        let mut span = testing::spans(source);
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: "an identifier",
                    found: "`=`",
                },
                span: span("="),
            }
        );
    }

    #[test]
    fn a_declaration_without_an_equals_is_reported() {
        let source = "func f() uint64 { var x uint64 1 }";
        let mut span = testing::spans(source);
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: "`=`",
                    found: "an integer literal",
                },
                span: span("1"),
            }
        );
    }

    #[test]
    fn a_declaration_without_an_initializer_is_reported() {
        let source = "func f() uint64 { var x uint64 = }";
        let mut span = testing::spans(source);
        span("{");
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: OPERAND,
                    found: "`}`",
                },
                span: span("}"),
            }
        );
    }

    #[test]
    fn a_statement_must_start_with_var_or_return() {
        let source = "func f() uint64 { 1 }";
        let mut span = testing::spans(source);
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: "`var`, `return` or `}`",
                    found: "an integer literal",
                },
                span: span("1"),
            }
        );
    }

    #[test]
    fn empty_input_produces_no_functions() {
        assert_eq!(parse_ok(""), ast::Program { funcs: Vec::new() });
    }

    #[test]
    fn parses_two_functions() {
        let source = "func a() uint64 { return 1 }\nfunc b() uint64 { return 2 }";
        let program = parse_ok(source);

        let names: Vec<&str> = program
            .funcs
            .iter()
            .map(|func| func.name.text.as_str())
            .collect();
        assert_eq!(names, vec!["a", "b"]);
        assert_eq!(
            program.funcs[0].span,
            diagnostics::Span { start: 0, end: 28 }
        );
        assert_eq!(
            program.funcs[1].span,
            diagnostics::Span {
                start: 29,
                end: source.len()
            }
        );
    }

    #[test]
    fn a_body_may_be_empty() {
        let source = "func f() uint64 {}";
        let mut span = testing::spans(source);

        let start = span("func").start;
        let func_name = testing::name("f", span("f"));
        let ret = ast::TypeRef {
            name: testing::name("uint64", span("uint64")),
        };
        let end = span("}").end;

        assert_eq!(
            parse_ok(source),
            ast::Program {
                funcs: vec![ast::FuncDecl {
                    name: func_name,
                    ret,
                    body: Vec::new(),
                    span: diagnostics::Span { start, end },
                }]
            }
        );
    }

    #[test]
    fn a_missing_closing_paren_is_reported() {
        let source = "func f( uint64 {}";
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: "`)`",
                    found: "an identifier",
                },
                span: diagnostics::Span { start: 8, end: 14 },
            }
        );
    }

    /// The expression of the one `return` in `source`.
    fn returned(source: &str) -> ast::Expr {
        let program = parse_ok(source);
        let funcs = program.funcs;
        assert_eq!(funcs.len(), 1);
        let mut body = funcs.into_iter().flat_map(|func| func.body);
        let Some(ast::Stmt::Return { expr, .. }) = body.next() else {
            panic!("one return statement")
        };
        assert!(body.next().is_none());
        expr
    }

    /// `func approval() uint64 { return <expr> }`.
    fn wrap(expr: &str) -> String {
        format!("func approval() uint64 {{ return {expr} }}")
    }

    #[test]
    fn an_operator_is_not_an_operand() {
        let source = wrap("+ 1");
        let mut span = testing::spans(&source);
        assert_eq!(
            parse_err(&source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: OPERAND,
                    found: "`+`",
                },
                span: span("+"),
            }
        );
    }

    #[test]
    fn a_comparison_operator_is_not_an_operand() {
        let source = "func f() uint64 { return == 1 }";
        let mut span = testing::spans(source);
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: OPERAND,
                    found: "`==`",
                },
                span: span("=="),
            }
        );
    }

    #[test]
    fn a_bang_needs_an_operand() {
        let source = "func f() uint64 { return ! }";
        let mut span = testing::spans(source);
        span("{");
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: OPERAND,
                    found: "`}`",
                },
                span: span("}"),
            }
        );
    }

    #[test]
    fn a_logical_operator_is_not_an_operand() {
        let source = "func f() uint64 { return && true }";
        let mut span = testing::spans(source);
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: OPERAND,
                    found: "`&&`",
                },
                span: span("&&"),
            }
        );
    }

    #[test]
    fn an_empty_operand_is_reported() {
        let source = wrap("()");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        assert_eq!(
            parse_err(&source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: OPERAND,
                    found: "`)`",
                },
                span: span(")"),
            }
        );
    }

    #[test]
    fn parses_a_boolean_literal() {
        let source = "func f() bool { return true }";
        let literal = testing::span_of(source, "true", 0);

        assert_eq!(
            parse_ok(source).funcs[0].body,
            vec![ast::Stmt::Return {
                expr: ast::Expr::BoolLit {
                    value: true,
                    span: literal,
                },
                span: testing::span_of(source, "return true", 0),
            }]
        );
    }

    #[test]
    fn parentheses_widen_a_boolean_literal() {
        let source = wrap("(false)");

        assert_eq!(
            returned(&source),
            ast::Expr::BoolLit {
                value: false,
                span: testing::span_of(&source, "(false)", 0),
            }
        );
    }

    #[test]
    fn a_boolean_literal_is_an_operand_of_arithmetic() {
        let source = wrap("true + 1");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let literal = span("true");
        span("+");
        let one = span("1");

        // The parser does not type: this is the type checker's to reject.
        assert_eq!(
            returned(&source),
            ast::Expr::Binary {
                op: ast::BinaryOp::Add,
                lhs: Box::new(ast::Expr::BoolLit {
                    value: true,
                    span: literal,
                }),
                rhs: Box::new(ast::Expr::IntLit {
                    value: 1,
                    span: one
                }),
                span: diagnostics::Span {
                    start: literal.start,
                    end: one.end,
                },
            }
        );
    }

    #[test]
    fn a_missing_operand_names_every_literal() {
        let source = "func f() uint64 { return }";
        let mut span = testing::spans(source);
        span("{");

        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: OPERAND,
                    found: "`}`",
                },
                span: span("}"),
            }
        );
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        let source = wrap("1 + 2 * 3");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        span("+");
        let two = span("2");
        span("*");
        let three = span("3");

        assert_eq!(
            returned(&source),
            ast::Expr::Binary {
                op: ast::BinaryOp::Add,
                lhs: Box::new(ast::Expr::IntLit {
                    value: 1,
                    span: one
                }),
                rhs: Box::new(ast::Expr::Binary {
                    op: ast::BinaryOp::Mul,
                    lhs: Box::new(ast::Expr::IntLit {
                        value: 2,
                        span: two
                    }),
                    rhs: Box::new(ast::Expr::IntLit {
                        value: 3,
                        span: three
                    }),
                    span: diagnostics::Span {
                        start: two.start,
                        end: three.end
                    },
                }),
                span: diagnostics::Span {
                    start: one.start,
                    end: three.end
                },
            }
        );
    }

    #[test]
    fn a_product_is_the_left_operand_of_a_sum() {
        let source = wrap("1 * 2 + 3");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        span("*");
        let two = span("2");
        span("+");
        let three = span("3");

        assert_eq!(
            returned(&source),
            ast::Expr::Binary {
                op: ast::BinaryOp::Add,
                lhs: Box::new(ast::Expr::Binary {
                    op: ast::BinaryOp::Mul,
                    lhs: Box::new(ast::Expr::IntLit {
                        value: 1,
                        span: one
                    }),
                    rhs: Box::new(ast::Expr::IntLit {
                        value: 2,
                        span: two
                    }),
                    span: diagnostics::Span {
                        start: one.start,
                        end: two.end
                    },
                }),
                rhs: Box::new(ast::Expr::IntLit {
                    value: 3,
                    span: three
                }),
                span: diagnostics::Span {
                    start: one.start,
                    end: three.end
                },
            }
        );
    }

    #[test]
    fn a_group_is_left_associative() {
        for (source, op) in [
            (wrap("1 - 2 - 3"), ast::BinaryOp::Sub),
            (wrap("1 / 2 / 3"), ast::BinaryOp::Div),
        ] {
            let mut span = testing::spans(&source);
            span("(");
            span(")");
            let one = span("1");
            let two = span("2");
            let three = span("3");

            assert_eq!(
                returned(&source),
                ast::Expr::Binary {
                    op,
                    lhs: Box::new(ast::Expr::Binary {
                        op,
                        lhs: Box::new(ast::Expr::IntLit {
                            value: 1,
                            span: one
                        }),
                        rhs: Box::new(ast::Expr::IntLit {
                            value: 2,
                            span: two
                        }),
                        span: diagnostics::Span {
                            start: one.start,
                            end: two.end
                        },
                    }),
                    rhs: Box::new(ast::Expr::IntLit {
                        value: 3,
                        span: three
                    }),
                    span: diagnostics::Span {
                        start: one.start,
                        end: three.end
                    },
                },
                "{source}"
            );
        }
    }

    #[test]
    fn a_longer_expression_groups_by_precedence() {
        let source = wrap("1 + 2 * 3 - 4 / 5");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");
        let three = span("3");
        let four = span("4");
        let five = span("5");

        let product = ast::Expr::Binary {
            op: ast::BinaryOp::Mul,
            lhs: Box::new(ast::Expr::IntLit {
                value: 2,
                span: two,
            }),
            rhs: Box::new(ast::Expr::IntLit {
                value: 3,
                span: three,
            }),
            span: diagnostics::Span {
                start: two.start,
                end: three.end,
            },
        };
        let sum = ast::Expr::Binary {
            op: ast::BinaryOp::Add,
            lhs: Box::new(ast::Expr::IntLit {
                value: 1,
                span: one,
            }),
            rhs: Box::new(product),
            span: diagnostics::Span {
                start: one.start,
                end: three.end,
            },
        };
        let quotient = ast::Expr::Binary {
            op: ast::BinaryOp::Div,
            lhs: Box::new(ast::Expr::IntLit {
                value: 4,
                span: four,
            }),
            rhs: Box::new(ast::Expr::IntLit {
                value: 5,
                span: five,
            }),
            span: diagnostics::Span {
                start: four.start,
                end: five.end,
            },
        };

        assert_eq!(
            returned(&source),
            ast::Expr::Binary {
                op: ast::BinaryOp::Sub,
                lhs: Box::new(sum),
                rhs: Box::new(quotient),
                span: diagnostics::Span {
                    start: one.start,
                    end: five.end
                },
            }
        );
    }

    #[test]
    fn parentheses_regroup_and_widen_the_span() {
        let source = wrap("(1 + 2) * 3");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let open = span("(");
        let one = span("1");
        let two = span("2");
        let close = span(")");
        span("*");
        let three = span("3");

        assert_eq!(
            returned(&source),
            ast::Expr::Binary {
                op: ast::BinaryOp::Mul,
                lhs: Box::new(ast::Expr::Binary {
                    op: ast::BinaryOp::Add,
                    lhs: Box::new(ast::Expr::IntLit {
                        value: 1,
                        span: one
                    }),
                    rhs: Box::new(ast::Expr::IntLit {
                        value: 2,
                        span: two
                    }),
                    span: diagnostics::Span {
                        start: open.start,
                        end: close.end
                    },
                }),
                rhs: Box::new(ast::Expr::IntLit {
                    value: 3,
                    span: three
                }),
                span: diagnostics::Span {
                    start: open.start,
                    end: three.end
                },
            }
        );
    }

    #[test]
    fn parentheses_around_a_literal_widen_its_span() {
        let source = wrap("(1)");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let open = span("(");
        span("1");
        let close = span(")");

        assert_eq!(
            returned(&source),
            ast::Expr::IntLit {
                value: 1,
                span: diagnostics::Span {
                    start: open.start,
                    end: close.end
                },
            }
        );
    }

    #[test]
    fn parentheses_nest() {
        let source = wrap("((1 + 2))");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let open = span("(");
        span("(");
        let one = span("1");
        let two = span("2");
        span(")");
        let close = span(")");

        assert_eq!(
            returned(&source),
            ast::Expr::Binary {
                op: ast::BinaryOp::Add,
                lhs: Box::new(ast::Expr::IntLit {
                    value: 1,
                    span: one
                }),
                rhs: Box::new(ast::Expr::IntLit {
                    value: 2,
                    span: two
                }),
                span: diagnostics::Span {
                    start: open.start,
                    end: close.end
                },
            }
        );
    }

    #[test]
    fn modulo_parses_where_it_is_unambiguous() {
        let source = wrap("1 % 2");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");

        assert_eq!(
            returned(&source),
            ast::Expr::Binary {
                op: ast::BinaryOp::Mod,
                lhs: Box::new(ast::Expr::IntLit {
                    value: 1,
                    span: one
                }),
                rhs: Box::new(ast::Expr::IntLit {
                    value: 2,
                    span: two
                }),
                span: diagnostics::Span {
                    start: one.start,
                    end: two.end
                },
            }
        );

        let source = wrap("(1 % 2) % 3");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let open = span("(");
        let one = span("1");
        let two = span("2");
        let close = span(")");
        let three = span("3");

        assert_eq!(
            returned(&source),
            ast::Expr::Binary {
                op: ast::BinaryOp::Mod,
                lhs: Box::new(ast::Expr::Binary {
                    op: ast::BinaryOp::Mod,
                    lhs: Box::new(ast::Expr::IntLit {
                        value: 1,
                        span: one
                    }),
                    rhs: Box::new(ast::Expr::IntLit {
                        value: 2,
                        span: two
                    }),
                    span: diagnostics::Span {
                        start: open.start,
                        end: close.end
                    },
                }),
                rhs: Box::new(ast::Expr::IntLit {
                    value: 3,
                    span: three
                }),
                span: diagnostics::Span {
                    start: open.start,
                    end: three.end
                },
            }
        );
    }

    #[test]
    fn unordered_operators_are_reported_at_the_second_one() {
        // The expression, the two operators as they are described, and the
        // text of the operator the parser could not place.
        let cases = [
            ("1 % 2 % 3", "`%`", "`%`", 1),
            ("1 % 2 + 3", "`%`", "`+`", 0),
            ("1 + 2 % 3", "`+`", "`%`", 0),
            ("1 * 2 % 3", "`*`", "`%`", 0),
            ("1 < 2 < 3", "`<`", "`<`", 1),
            ("1 == 2 == 3", "`==`", "`==`", 1),
            ("1 < 2 == true", "`<`", "`==`", 0),
        ];

        for (expr, left, right, nth) in cases {
            let source = wrap(expr);
            let text = right.trim_matches('`');
            assert_eq!(
                parse_err(&source),
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::AmbiguousPrecedence { left, right },
                    span: testing::span_of(&source, text, nth),
                },
                "{source}"
            );
        }
    }

    #[test]
    fn an_unclosed_parenthesis_is_reported() {
        let source = "func f() uint64 { return (1 + 2 }";
        let mut span = testing::spans(source);
        span("(");
        span(")");
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: "`)`",
                    found: "`}`",
                },
                span: span("}"),
            }
        );
    }

    #[test]
    fn a_missing_right_operand_is_reported_at_end_of_input() {
        let source = "func f() uint64 { return 1 +";
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: OPERAND,
                    found: "end of input",
                },
                span: diagnostics::Span {
                    start: source.len(),
                    end: source.len()
                },
            }
        );
    }

    #[test]
    fn an_unterminated_body_is_reported_at_end_of_input() {
        let source = "func f() uint64 {";
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: "`var`, `return` or `}`",
                    found: "end of input",
                },
                span: diagnostics::Span {
                    start: source.len(),
                    end: source.len()
                },
            }
        );
    }

    #[test]
    fn a_declaration_must_start_with_func() {
        assert_eq!(
            parse_err("}"),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnexpectedToken {
                    expected: "`func`",
                    found: "`}`",
                },
                span: diagnostics::Span { start: 0, end: 1 },
            }
        );
    }

    #[test]
    fn an_integer_literal_that_does_not_fit_is_reported() {
        let source = "func f() uint64 { return 18446744073709551616 }";
        assert_eq!(
            parse_err(source),
            diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::IntegerLiteralOutOfRange,
                span: diagnostics::Span { start: 25, end: 45 },
            }
        );
    }

    #[test]
    fn the_largest_integer_literal_parses() {
        let source = "func f() uint64 { return 18446744073709551615 }";
        let program = parse_ok(source);

        assert_eq!(
            program.funcs[0].body,
            vec![ast::Stmt::Return {
                expr: ast::Expr::IntLit {
                    value: u64::MAX,
                    span: diagnostics::Span { start: 25, end: 45 },
                },
                span: diagnostics::Span { start: 18, end: 45 },
            }]
        );
    }

    #[test]
    fn parses_every_comparison_operator() {
        let cases = [
            ("1 == 2", ast::BinaryOp::Eq),
            ("1 != 2", ast::BinaryOp::Ne),
            ("1 < 2", ast::BinaryOp::Lt),
            ("1 <= 2", ast::BinaryOp::Le),
            ("1 > 2", ast::BinaryOp::Gt),
            ("1 >= 2", ast::BinaryOp::Ge),
        ];

        for (expr, op) in cases {
            let source = wrap(expr);
            let mut span = testing::spans(&source);
            span("(");
            span(")");
            let one = span("1");
            let two = span("2");

            assert_eq!(
                returned(&source),
                binary(
                    op,
                    ast::Expr::IntLit {
                        value: 1,
                        span: one
                    },
                    ast::Expr::IntLit {
                        value: 2,
                        span: two
                    },
                ),
                "{source}"
            );
        }
    }

    #[test]
    fn comparison_binds_looser_than_arithmetic() {
        let source = wrap("1 + 2 == 3 * 4");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");
        let three = span("3");
        let four = span("4");

        let int = |value, span| ast::Expr::IntLit { value, span };
        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::Eq,
                binary(ast::BinaryOp::Add, int(1, one), int(2, two)),
                binary(ast::BinaryOp::Mul, int(3, three), int(4, four)),
            )
        );
    }

    #[test]
    fn comparison_binds_looser_than_modulo() {
        let source = wrap("1 % 2 == 0");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");
        let zero = span("0");

        let int = |value, span| ast::Expr::IntLit { value, span };
        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::Eq,
                binary(ast::BinaryOp::Mod, int(1, one), int(2, two)),
                int(0, zero),
            )
        );
    }

    #[test]
    fn modulo_on_the_right_of_a_comparison_binds_tighter() {
        let source = wrap("1 == 2 % 3");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");
        let three = span("3");

        let int = |value, span| ast::Expr::IntLit { value, span };
        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::Eq,
                int(1, one),
                binary(ast::BinaryOp::Mod, int(2, two), int(3, three)),
            )
        );
    }

    #[test]
    fn addition_on_the_right_of_a_comparison_binds_tighter() {
        let source = wrap("1 < 2 + 3");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");
        let three = span("3");

        let int = |value, span| ast::Expr::IntLit { value, span };
        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::Lt,
                int(1, one),
                binary(ast::BinaryOp::Add, int(2, two), int(3, three)),
            )
        );
    }

    #[test]
    fn parentheses_make_a_comparison_an_operand() {
        let source = wrap("(1 < 2) == true");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let open = span("(");
        let one = span("1");
        let two = span("2");
        let close = span(")");
        let literal = span("true");

        let parenthesized = ast::Expr::Binary {
            op: ast::BinaryOp::Lt,
            lhs: Box::new(ast::Expr::IntLit {
                value: 1,
                span: one,
            }),
            rhs: Box::new(ast::Expr::IntLit {
                value: 2,
                span: two,
            }),
            span: diagnostics::Span {
                start: open.start,
                end: close.end,
            },
        };

        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::Eq,
                parenthesized,
                ast::Expr::BoolLit {
                    value: true,
                    span: literal,
                },
            )
        );
    }

    #[test]
    fn negates_a_boolean_literal() {
        let source = wrap("!true");
        let literal = testing::span_of(&source, "true", 0);

        assert_eq!(
            returned(&source),
            unary(
                ast::UnaryOp::Not,
                ast::Expr::BoolLit {
                    value: true,
                    span: literal,
                },
            )
        );
    }

    #[test]
    fn negates_a_variable() {
        let source = wrap("!x");
        let x = testing::span_of(&source, "x", 0);

        assert_eq!(
            returned(&source),
            unary(
                ast::UnaryOp::Not,
                ast::Expr::Var {
                    name: testing::name("x", x),
                    span: x,
                },
            )
        );
    }

    #[test]
    fn negates_a_parenthesized_expression() {
        let source = wrap("!(1 < 2)");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let open = span("(");
        let one = span("1");
        let two = span("2");
        let close = span(")");

        assert_eq!(
            returned(&source),
            unary(
                ast::UnaryOp::Not,
                ast::Expr::Binary {
                    op: ast::BinaryOp::Lt,
                    lhs: Box::new(ast::Expr::IntLit {
                        value: 1,
                        span: one,
                    }),
                    rhs: Box::new(ast::Expr::IntLit {
                        value: 2,
                        span: two,
                    }),
                    span: diagnostics::Span {
                        start: open.start,
                        end: close.end,
                    },
                },
            )
        );
    }

    #[test]
    fn parses_both_logical_operators() {
        for (expr, op) in [
            ("true && false", ast::BinaryOp::And),
            ("true || false", ast::BinaryOp::Or),
        ] {
            let source = wrap(expr);
            let mut span = testing::spans(&source);
            let yes = span("true");
            let no = span("false");

            assert_eq!(
                returned(&source),
                binary(
                    op,
                    ast::Expr::BoolLit {
                        value: true,
                        span: yes,
                    },
                    ast::Expr::BoolLit {
                        value: false,
                        span: no,
                    },
                ),
                "{source}"
            );
        }
    }

    #[test]
    fn negation_binds_tighter_than_logic() {
        let source = wrap("!x && y");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let x = span("x");
        let y = span("y");

        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::And,
                unary(ast::UnaryOp::Not, var("x", x)),
                var("y", y),
            )
        );

        let source = wrap("x || !y");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let x = span("x");
        let y = span("y");

        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::Or,
                var("x", x),
                unary(ast::UnaryOp::Not, var("y", y))
            )
        );

        let source = wrap("x && !y && z");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let x = span("x");
        let y = span("y");
        let z = span("z");

        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::And,
                binary(
                    ast::BinaryOp::And,
                    var("x", x),
                    unary(ast::UnaryOp::Not, var("y", y))
                ),
                var("z", z),
            )
        );
    }

    #[test]
    fn logic_is_left_associative() {
        for (expr, op) in [
            ("x && y && z", ast::BinaryOp::And),
            ("x || y || z", ast::BinaryOp::Or),
        ] {
            let source = wrap(expr);
            let mut span = testing::spans(&source);
            span("(");
            span(")");
            let x = span("x");
            let y = span("y");
            let z = span("z");

            assert_eq!(
                returned(&source),
                binary(op, binary(op, var("x", x), var("y", y)), var("z", z),),
                "{source}"
            );
        }
    }

    #[test]
    fn logic_binds_looser_than_comparison() {
        let source = wrap("1 < 2 && 3 < 4");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");
        let three = span("3");
        let four = span("4");

        let int = |value, span| ast::Expr::IntLit { value, span };
        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::And,
                binary(ast::BinaryOp::Lt, int(1, one), int(2, two)),
                binary(ast::BinaryOp::Lt, int(3, three), int(4, four)),
            )
        );
    }

    #[test]
    fn logic_binds_looser_than_arithmetic_and_comparison_together() {
        let source = wrap("1 + 2 == 3 || false");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");
        let three = span("3");
        let literal = span("false");

        let int = |value, span| ast::Expr::IntLit { value, span };
        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::Or,
                binary(
                    ast::BinaryOp::Eq,
                    binary(ast::BinaryOp::Add, int(1, one), int(2, two)),
                    int(3, three),
                ),
                ast::Expr::BoolLit {
                    value: false,
                    span: literal,
                },
            )
        );
    }

    #[test]
    fn parentheses_make_logic_an_operand_of_logic() {
        let source = wrap("(true && false) || true");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let open = span("(");
        let yes = span("true");
        let no = span("false");
        let close = span(")");
        let last = span("true");

        assert_eq!(
            returned(&source),
            binary(
                ast::BinaryOp::Or,
                ast::Expr::Binary {
                    op: ast::BinaryOp::And,
                    lhs: Box::new(ast::Expr::BoolLit {
                        value: true,
                        span: yes,
                    }),
                    rhs: Box::new(ast::Expr::BoolLit {
                        value: false,
                        span: no,
                    }),
                    span: diagnostics::Span {
                        start: open.start,
                        end: close.end,
                    },
                },
                ast::Expr::BoolLit {
                    value: true,
                    span: last,
                },
            )
        );
    }

    #[test]
    fn negation_and_logic_report_what_they_do_not_order() {
        // The expression, the two operators as they are described, and which
        // occurrence of the second one's text the parser could not place.
        let cases = [
            ("!!true", "`!`", "`!`", 1),
            ("!1 == 2", "`!`", "`==`", 0),
            ("1 == !2", "`==`", "`!`", 0),
            ("!1 + 2", "`!`", "`+`", 0),
            ("1 + !2", "`+`", "`!`", 0),
            ("true && false || true", "`&&`", "`||`", 0),
            ("true || false && true", "`||`", "`&&`", 0),
        ];

        for (expr, left, right, nth) in cases {
            let source = wrap(expr);
            let text = right.trim_matches('`');
            assert_eq!(
                parse_err(&source),
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::AmbiguousPrecedence { left, right },
                    span: testing::span_of(&source, text, nth),
                },
                "{source}"
            );
        }
    }
}
