//! The parser: tokens to an AST by recursive descent.

use crate::ast::{BinaryOp, Expr, FuncDecl, Name, Program, Stmt, TypeRef};
use crate::diagnostics::{Diagnostic, DiagnosticKind, Diagnostics, Span};
use crate::lexer::{Token, TokenKind};
use crate::precedence::{Priority, group, priority};

/// Parses the token stream `lex` produced for `source`.
///
/// Returns `None` as soon as anything is wrong; one run reports at most
/// one diagnostic.
pub fn parse(source: &str, tokens: &[Token], diags: &mut Diagnostics) -> Option<Program> {
    Parser {
        source,
        tokens,
        next: 0,
        diags,
    }
    .program()
}

/// What the grammar allows where an operand is expected.
const OPERAND: &str = "an integer literal, an identifier, or `(`";

struct Parser<'a> {
    source: &'a str,
    tokens: &'a [Token],
    /// Index of the token to be consumed next.
    next: usize,
    diags: &'a mut Diagnostics,
}

impl Parser<'_> {
    fn program(&mut self) -> Option<Program> {
        let mut funcs = Vec::new();
        while self.peek().is_some() {
            funcs.push(self.func_decl()?);
        }
        Some(Program { funcs })
    }

    fn func_decl(&mut self) -> Option<FuncDecl> {
        let start = self.expect(TokenKind::Func, "`func`")?.span.start;
        let name = self.name()?;
        self.expect(TokenKind::LParen, "`(`")?;
        self.expect(TokenKind::RParen, "`)`")?;
        let ret = TypeRef { name: self.name()? };
        self.expect(TokenKind::LBrace, "`{`")?;

        let mut body = Vec::new();
        while self.peek_kind() != Some(TokenKind::RBrace) {
            body.push(self.stmt()?);
        }
        let end = self.expect(TokenKind::RBrace, "`}`")?.span.end;

        Some(FuncDecl {
            name,
            ret,
            body,
            span: Span { start, end },
        })
    }

    fn stmt(&mut self) -> Option<Stmt> {
        if self.peek_kind() == Some(TokenKind::Var) {
            return self.var_stmt();
        }

        let start = self
            .expect(TokenKind::Return, "`var`, `return` or `}`")?
            .span
            .start;
        let expr = self.expr(None)?;
        let end = expr.span().end;
        Some(Stmt::Return {
            expr,
            span: Span { start, end },
        })
    }

    /// `var name type = init`.
    fn var_stmt(&mut self) -> Option<Stmt> {
        let start = self.expect(TokenKind::Var, "`var`")?.span.start;
        let name = self.name()?;
        let ty = TypeRef { name: self.name()? };
        self.expect(TokenKind::Equals, "`=`")?;
        let init = self.expr(None)?;
        let end = init.span().end;
        Some(Stmt::Var {
            name,
            ty,
            init,
            span: Span { start, end },
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
    fn expr(&mut self, ambient: Option<Token>) -> Option<Expr> {
        let mut lhs = self.operand()?;
        // The caller only ever passes a token it consumed as an operator.
        let enclosing = ambient.and_then(|token| Some((token, binary_op(token.kind)?)));

        while let Some((token, op)) = self.peek_binary_op() {
            if let Some((left, left_op)) = enclosing {
                match priority(group(left_op), group(op)) {
                    // The enclosing operator takes the operand just parsed.
                    Priority::Left => break,
                    Priority::Ambiguous => {
                        let kind = DiagnosticKind::AmbiguousPrecedence {
                            left: describe(left.kind),
                            right: describe(token.kind),
                        };
                        return self.report(kind, token.span);
                    }
                    Priority::Right => {}
                }
            }

            self.next += 1;
            let rhs = self.expr(Some(token))?;
            let span = Span {
                start: lhs.span().start,
                end: rhs.span().end,
            };
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }

        Some(lhs)
    }

    /// An integer literal, a variable, or a parenthesized expression.
    fn operand(&mut self) -> Option<Expr> {
        if self.peek_kind() == Some(TokenKind::LParen) {
            let start = self.expect(TokenKind::LParen, OPERAND)?.span.start;
            let inner = self.expr(None)?;
            let end = self.expect(TokenKind::RParen, "`)`")?.span.end;
            return Some(inner.with_span(Span { start, end }));
        }

        if self.peek_kind() == Some(TokenKind::Ident) {
            let name = self.name()?;
            let span = name.span;
            return Some(Expr::Var { name, span });
        }

        let span = self.expect(TokenKind::IntLit, OPERAND)?.span;
        match self.text(span).parse::<u64>() {
            Ok(value) => Some(Expr::IntLit { value, span }),
            // The lexer only admits ASCII digits, so overflow is the one way
            // parsing can fail here.
            Err(_) => self.report(DiagnosticKind::IntegerLiteralOutOfRange, span),
        }
    }

    fn name(&mut self) -> Option<Name> {
        let span = self.expect(TokenKind::Ident, "an identifier")?.span;
        Some(Name {
            text: self.text(span).to_string(),
            span,
        })
    }

    /// Consumes the next token if it is a `kind`, and reports the unexpected
    /// one otherwise. `expected` describes what the grammar allows here.
    fn expect(&mut self, kind: TokenKind, expected: &'static str) -> Option<Token> {
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
                        Span {
                            start: self.source.len(),
                            end: self.source.len(),
                        },
                    ),
                };
                self.report(DiagnosticKind::UnexpectedToken { expected, found }, span)
            }
        }
    }

    /// Reports a diagnostic and fails the parse.
    fn report<T>(&mut self, kind: DiagnosticKind, span: Span) -> Option<T> {
        self.diags.push(Diagnostic { kind, span });
        None
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.next)
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|token| token.kind)
    }

    /// The next token and the operator it denotes, if it denotes one.
    fn peek_binary_op(&self) -> Option<(Token, BinaryOp)> {
        let &token = self.peek()?;
        Some((token, binary_op(token.kind)?))
    }

    /// The source text a token covers. `lex` guarantees valid spans; the
    /// fallback is the one place the parser trusts it.
    fn text(&self, span: Span) -> &str {
        self.source.get(span.start..span.end).unwrap_or("")
    }
}

fn describe(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Func => "`func`",
        TokenKind::Return => "`return`",
        TokenKind::Var => "`var`",
        TokenKind::Ident => "an identifier",
        TokenKind::IntLit => "an integer literal",
        TokenKind::LParen => "`(`",
        TokenKind::RParen => "`)`",
        TokenKind::LBrace => "`{`",
        TokenKind::RBrace => "`}`",
        TokenKind::Plus => "`+`",
        TokenKind::Minus => "`-`",
        TokenKind::Star => "`*`",
        TokenKind::Slash => "`/`",
        TokenKind::Percent => "`%`",
        TokenKind::Equals => "`=`",
    }
}

/// The operator a token denotes, if it denotes one.
fn binary_op(kind: TokenKind) -> Option<BinaryOp> {
    match kind {
        TokenKind::Plus => Some(BinaryOp::Add),
        TokenKind::Minus => Some(BinaryOp::Sub),
        TokenKind::Star => Some(BinaryOp::Mul),
        TokenKind::Slash => Some(BinaryOp::Div),
        TokenKind::Percent => Some(BinaryOp::Mod),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::testing::{name, span_of, spans};

    /// Lexes and parses `source`, asserting that it produced no diagnostics.
    fn parse_ok(source: &str) -> Program {
        let mut diags = Diagnostics::default();
        let tokens = lex(source, &mut diags).expect("lexing succeeded");
        let program = parse(source, &tokens, &mut diags);
        assert!(diags.is_empty());
        program.expect("parsing succeeded")
    }

    /// Lexes and parses `source`, asserting that it reported exactly one
    /// diagnostic and produced nothing.
    fn parse_err(source: &str) -> Diagnostic {
        let mut diags = Diagnostics::default();
        let tokens = lex(source, &mut diags).expect("lexing succeeded");
        assert_eq!(parse(source, &tokens, &mut diags), None);
        let mut reported = diags.iter();
        let diagnostic = reported.next().expect("one diagnostic").clone();
        assert!(reported.next().is_none());
        diagnostic
    }

    #[test]
    fn parses_the_approval_program() {
        let source = "func approval() uint64 {\n  return 1\n}\n";
        let mut span = spans(source);

        let start = span("func").start;
        let func_name = name("approval", span("approval"));
        let ret = TypeRef {
            name: name("uint64", span("uint64")),
        };
        let return_start = span("return").start;
        let literal = span("1");
        let end = span("}").end;

        assert_eq!(
            parse_ok(source),
            Program {
                funcs: vec![FuncDecl {
                    name: func_name,
                    ret,
                    body: vec![Stmt::Return {
                        expr: Expr::IntLit {
                            value: 1,
                            span: literal
                        },
                        span: Span {
                            start: return_start,
                            end: literal.end
                        },
                    }],
                    span: Span { start, end },
                }]
            }
        );
    }

    /// The example program of the variables milestone.
    const VARIABLES: &str = "func approval() uint64 {\n  var x uint64 = 1 + 2\n  \
                             var y uint64 = x * 3\n  return y - x\n}\n";

    /// `lhs op rhs`, spanning from one operand to the other.
    fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Expr {
        let span = Span {
            start: lhs.span().start,
            end: rhs.span().end,
        };
        Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            span,
        }
    }

    #[test]
    fn parses_the_variables_program() {
        let source = VARIABLES;
        let mut span = spans(source);

        let start = span("func").start;
        let func_name = name("approval", span("approval"));
        span("(");
        span(")");
        let ret = TypeRef {
            name: name("uint64", span("uint64")),
        };
        span("{");

        let first_var = span("var").start;
        let x = name("x", span("x"));
        let x_ty = TypeRef {
            name: name("uint64", span("uint64")),
        };
        span("=");
        let one = span("1");
        let two = span("2");

        let second_var = span("var").start;
        let y = name("y", span("y"));
        let y_ty = TypeRef {
            name: name("uint64", span("uint64")),
        };
        span("=");
        let x_times = span("x");
        let three = span("3");

        let return_start = span("return").start;
        let y_minus = span("y");
        let x_minus = span("x");
        let end = span("}").end;

        let int = |value, span| Expr::IntLit { value, span };
        let var = |text: &str, span| Expr::Var {
            name: name(text, span),
            span,
        };

        assert_eq!(
            parse_ok(source),
            Program {
                funcs: vec![FuncDecl {
                    name: func_name,
                    ret,
                    body: vec![
                        Stmt::Var {
                            name: x,
                            ty: x_ty,
                            init: binary(BinaryOp::Add, int(1, one), int(2, two)),
                            span: Span {
                                start: first_var,
                                end: two.end
                            },
                        },
                        Stmt::Var {
                            name: y,
                            ty: y_ty,
                            init: binary(BinaryOp::Mul, var("x", x_times), int(3, three)),
                            span: Span {
                                start: second_var,
                                end: three.end
                            },
                        },
                        Stmt::Return {
                            expr: binary(BinaryOp::Sub, var("y", y_minus), var("x", x_minus)),
                            span: Span {
                                start: return_start,
                                end: x_minus.end
                            },
                        },
                    ],
                    span: Span { start, end },
                }]
            }
        );
    }

    #[test]
    fn a_variable_is_an_operand() {
        let source = wrap("x + 1");
        let mut span = spans(&source);
        span("(");
        span(")");
        let x = span("x");
        let one = span("1");

        assert_eq!(
            returned(&source),
            binary(
                BinaryOp::Add,
                Expr::Var {
                    name: name("x", x),
                    span: x
                },
                Expr::IntLit {
                    value: 1,
                    span: one
                },
            )
        );
    }

    #[test]
    fn parentheses_around_a_variable_widen_its_span() {
        let source = wrap("(x)");
        let mut span = spans(&source);
        span("(");
        span(")");
        let open = span("(");
        let x = span("x");
        let close = span(")");

        assert_eq!(
            returned(&source),
            Expr::Var {
                name: name("x", x),
                span: Span {
                    start: open.start,
                    end: close.end
                },
            }
        );
    }

    #[test]
    fn a_declaration_without_a_type_is_reported() {
        let source = "func f() uint64 { var x = 1 }";
        let mut span = spans(source);
        assert_eq!(
            parse_err(source),
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
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
        let mut span = spans(source);
        assert_eq!(
            parse_err(source),
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
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
        let mut span = spans(source);
        span("{");
        assert_eq!(
            parse_err(source),
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
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
        let mut span = spans(source);
        assert_eq!(
            parse_err(source),
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
                    expected: "`var`, `return` or `}`",
                    found: "an integer literal",
                },
                span: span("1"),
            }
        );
    }

    #[test]
    fn empty_input_produces_no_functions() {
        assert_eq!(parse_ok(""), Program { funcs: Vec::new() });
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
        assert_eq!(program.funcs[0].span, Span { start: 0, end: 28 });
        assert_eq!(
            program.funcs[1].span,
            Span {
                start: 29,
                end: source.len()
            }
        );
    }

    #[test]
    fn a_body_may_be_empty() {
        let source = "func f() uint64 {}";
        let mut span = spans(source);

        let start = span("func").start;
        let func_name = name("f", span("f"));
        let ret = TypeRef {
            name: name("uint64", span("uint64")),
        };
        let end = span("}").end;

        assert_eq!(
            parse_ok(source),
            Program {
                funcs: vec![FuncDecl {
                    name: func_name,
                    ret,
                    body: Vec::new(),
                    span: Span { start, end },
                }]
            }
        );
    }

    #[test]
    fn a_missing_closing_paren_is_reported() {
        let source = "func f( uint64 {}";
        assert_eq!(
            parse_err(source),
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
                    expected: "`)`",
                    found: "an identifier",
                },
                span: Span { start: 8, end: 14 },
            }
        );
    }

    /// The expression of the one `return` in `source`.
    fn returned(source: &str) -> Expr {
        let program = parse_ok(source);
        let funcs = program.funcs;
        assert_eq!(funcs.len(), 1);
        let mut body = funcs.into_iter().flat_map(|func| func.body);
        let Some(Stmt::Return { expr, .. }) = body.next() else {
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
        let mut span = spans(&source);
        assert_eq!(
            parse_err(&source),
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
                    expected: "an integer literal, an identifier, or `(`",
                    found: "`+`",
                },
                span: span("+"),
            }
        );
    }

    #[test]
    fn an_empty_operand_is_reported() {
        let source = wrap("()");
        let mut span = spans(&source);
        span("(");
        span(")");
        assert_eq!(
            parse_err(&source),
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
                    expected: "an integer literal, an identifier, or `(`",
                    found: "`)`",
                },
                span: span(")"),
            }
        );
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        let source = wrap("1 + 2 * 3");
        let mut span = spans(&source);
        span("(");
        span(")");
        let one = span("1");
        span("+");
        let two = span("2");
        span("*");
        let three = span("3");

        assert_eq!(
            returned(&source),
            Expr::Binary {
                op: BinaryOp::Add,
                lhs: Box::new(Expr::IntLit {
                    value: 1,
                    span: one
                }),
                rhs: Box::new(Expr::Binary {
                    op: BinaryOp::Mul,
                    lhs: Box::new(Expr::IntLit {
                        value: 2,
                        span: two
                    }),
                    rhs: Box::new(Expr::IntLit {
                        value: 3,
                        span: three
                    }),
                    span: Span {
                        start: two.start,
                        end: three.end
                    },
                }),
                span: Span {
                    start: one.start,
                    end: three.end
                },
            }
        );
    }

    #[test]
    fn a_product_is_the_left_operand_of_a_sum() {
        let source = wrap("1 * 2 + 3");
        let mut span = spans(&source);
        span("(");
        span(")");
        let one = span("1");
        span("*");
        let two = span("2");
        span("+");
        let three = span("3");

        assert_eq!(
            returned(&source),
            Expr::Binary {
                op: BinaryOp::Add,
                lhs: Box::new(Expr::Binary {
                    op: BinaryOp::Mul,
                    lhs: Box::new(Expr::IntLit {
                        value: 1,
                        span: one
                    }),
                    rhs: Box::new(Expr::IntLit {
                        value: 2,
                        span: two
                    }),
                    span: Span {
                        start: one.start,
                        end: two.end
                    },
                }),
                rhs: Box::new(Expr::IntLit {
                    value: 3,
                    span: three
                }),
                span: Span {
                    start: one.start,
                    end: three.end
                },
            }
        );
    }

    #[test]
    fn a_group_is_left_associative() {
        for (source, op) in [
            (wrap("1 - 2 - 3"), BinaryOp::Sub),
            (wrap("1 / 2 / 3"), BinaryOp::Div),
        ] {
            let mut span = spans(&source);
            span("(");
            span(")");
            let one = span("1");
            let two = span("2");
            let three = span("3");

            assert_eq!(
                returned(&source),
                Expr::Binary {
                    op,
                    lhs: Box::new(Expr::Binary {
                        op,
                        lhs: Box::new(Expr::IntLit {
                            value: 1,
                            span: one
                        }),
                        rhs: Box::new(Expr::IntLit {
                            value: 2,
                            span: two
                        }),
                        span: Span {
                            start: one.start,
                            end: two.end
                        },
                    }),
                    rhs: Box::new(Expr::IntLit {
                        value: 3,
                        span: three
                    }),
                    span: Span {
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
        let mut span = spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");
        let three = span("3");
        let four = span("4");
        let five = span("5");

        let product = Expr::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::IntLit {
                value: 2,
                span: two,
            }),
            rhs: Box::new(Expr::IntLit {
                value: 3,
                span: three,
            }),
            span: Span {
                start: two.start,
                end: three.end,
            },
        };
        let sum = Expr::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::IntLit {
                value: 1,
                span: one,
            }),
            rhs: Box::new(product),
            span: Span {
                start: one.start,
                end: three.end,
            },
        };
        let quotient = Expr::Binary {
            op: BinaryOp::Div,
            lhs: Box::new(Expr::IntLit {
                value: 4,
                span: four,
            }),
            rhs: Box::new(Expr::IntLit {
                value: 5,
                span: five,
            }),
            span: Span {
                start: four.start,
                end: five.end,
            },
        };

        assert_eq!(
            returned(&source),
            Expr::Binary {
                op: BinaryOp::Sub,
                lhs: Box::new(sum),
                rhs: Box::new(quotient),
                span: Span {
                    start: one.start,
                    end: five.end
                },
            }
        );
    }

    #[test]
    fn parentheses_regroup_and_widen_the_span() {
        let source = wrap("(1 + 2) * 3");
        let mut span = spans(&source);
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
            Expr::Binary {
                op: BinaryOp::Mul,
                lhs: Box::new(Expr::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::IntLit {
                        value: 1,
                        span: one
                    }),
                    rhs: Box::new(Expr::IntLit {
                        value: 2,
                        span: two
                    }),
                    span: Span {
                        start: open.start,
                        end: close.end
                    },
                }),
                rhs: Box::new(Expr::IntLit {
                    value: 3,
                    span: three
                }),
                span: Span {
                    start: open.start,
                    end: three.end
                },
            }
        );
    }

    #[test]
    fn parentheses_around_a_literal_widen_its_span() {
        let source = wrap("(1)");
        let mut span = spans(&source);
        span("(");
        span(")");
        let open = span("(");
        span("1");
        let close = span(")");

        assert_eq!(
            returned(&source),
            Expr::IntLit {
                value: 1,
                span: Span {
                    start: open.start,
                    end: close.end
                },
            }
        );
    }

    #[test]
    fn parentheses_nest() {
        let source = wrap("((1 + 2))");
        let mut span = spans(&source);
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
            Expr::Binary {
                op: BinaryOp::Add,
                lhs: Box::new(Expr::IntLit {
                    value: 1,
                    span: one
                }),
                rhs: Box::new(Expr::IntLit {
                    value: 2,
                    span: two
                }),
                span: Span {
                    start: open.start,
                    end: close.end
                },
            }
        );
    }

    #[test]
    fn modulo_parses_where_it_is_unambiguous() {
        let source = wrap("1 % 2");
        let mut span = spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");

        assert_eq!(
            returned(&source),
            Expr::Binary {
                op: BinaryOp::Mod,
                lhs: Box::new(Expr::IntLit {
                    value: 1,
                    span: one
                }),
                rhs: Box::new(Expr::IntLit {
                    value: 2,
                    span: two
                }),
                span: Span {
                    start: one.start,
                    end: two.end
                },
            }
        );

        let source = wrap("(1 % 2) % 3");
        let mut span = spans(&source);
        span("(");
        span(")");
        let open = span("(");
        let one = span("1");
        let two = span("2");
        let close = span(")");
        let three = span("3");

        assert_eq!(
            returned(&source),
            Expr::Binary {
                op: BinaryOp::Mod,
                lhs: Box::new(Expr::Binary {
                    op: BinaryOp::Mod,
                    lhs: Box::new(Expr::IntLit {
                        value: 1,
                        span: one
                    }),
                    rhs: Box::new(Expr::IntLit {
                        value: 2,
                        span: two
                    }),
                    span: Span {
                        start: open.start,
                        end: close.end
                    },
                }),
                rhs: Box::new(Expr::IntLit {
                    value: 3,
                    span: three
                }),
                span: Span {
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
        ];

        for (expr, left, right, nth) in cases {
            let source = wrap(expr);
            let text = right.trim_matches('`');
            assert_eq!(
                parse_err(&source),
                Diagnostic {
                    kind: DiagnosticKind::AmbiguousPrecedence { left, right },
                    span: span_of(&source, text, nth),
                },
                "{source}"
            );
        }
    }

    #[test]
    fn an_unclosed_parenthesis_is_reported() {
        let source = "func f() uint64 { return (1 + 2 }";
        let mut span = spans(source);
        span("(");
        span(")");
        assert_eq!(
            parse_err(source),
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
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
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
                    expected: "an integer literal, an identifier, or `(`",
                    found: "end of input",
                },
                span: Span {
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
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
                    expected: "`var`, `return` or `}`",
                    found: "end of input",
                },
                span: Span {
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
            Diagnostic {
                kind: DiagnosticKind::UnexpectedToken {
                    expected: "`func`",
                    found: "`}`",
                },
                span: Span { start: 0, end: 1 },
            }
        );
    }

    #[test]
    fn an_integer_literal_that_does_not_fit_is_reported() {
        let source = "func f() uint64 { return 18446744073709551616 }";
        assert_eq!(
            parse_err(source),
            Diagnostic {
                kind: DiagnosticKind::IntegerLiteralOutOfRange,
                span: Span { start: 25, end: 45 },
            }
        );
    }

    #[test]
    fn the_largest_integer_literal_parses() {
        let source = "func f() uint64 { return 18446744073709551615 }";
        let program = parse_ok(source);

        assert_eq!(
            program.funcs[0].body,
            vec![Stmt::Return {
                expr: Expr::IntLit {
                    value: u64::MAX,
                    span: Span { start: 25, end: 45 },
                },
                span: Span { start: 18, end: 45 },
            }]
        );
    }
}
