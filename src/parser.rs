//! The parser: tokens to a concrete syntax tree by recursive descent.

use crate::ast;
use crate::cst;
use crate::diag;
use crate::precedence;
use crate::token;

/// Parses the token stream `lex` produced.
///
/// Returns `None` as soon as anything is wrong; one run reports at most
/// one diagnostic.
pub fn parse(tokens: &[token::Token], diags: &mut diag::Sink) -> Option<cst::Program> {
    Parser {
        tokens,
        next: 0,
        diags,
    }
    .program()
}

/// What the grammar allows where an operand is expected.
const OPERAND: &str = "a literal, an identifier, `!`, or `(`";

struct Parser<'a> {
    tokens: &'a [token::Token],
    /// Index of the token to be consumed next.
    next: usize,
    diags: &'a mut diag::Sink,
}

impl Parser<'_> {
    fn program(&mut self) -> Option<cst::Program> {
        let mut funcs = Vec::new();
        while self.peek_kind() != Some(token::Kind::Eof) {
            funcs.push(self.func_decl()?);
        }
        let eof = self.expect(token::Kind::Eof, "`func`")?;
        Some(cst::Program { funcs, eof })
    }

    fn func_decl(&mut self) -> Option<cst::FuncDecl> {
        let func = self.expect(token::Kind::Func, "`func`")?;
        let name = self.name()?;
        let lparen = self.expect(token::Kind::LParen, "`(`")?;
        let rparen = self.expect(token::Kind::RParen, "`)`")?;
        let ret = self.name()?;
        let lbrace = self.expect(token::Kind::LBrace, "`{`")?;

        let mut body = Vec::new();
        while self.peek_kind() != Some(token::Kind::RBrace) {
            body.push(self.stmt()?);
        }
        let rbrace = self.expect(token::Kind::RBrace, "`}`")?;

        Some(cst::FuncDecl {
            func,
            name,
            lparen,
            rparen,
            ret,
            lbrace,
            body,
            rbrace,
        })
    }

    fn stmt(&mut self) -> Option<cst::Stmt> {
        if self.peek_kind() == Some(token::Kind::Var) {
            return self.var_stmt();
        }

        let ret = self.expect(token::Kind::Return, "`var`, `return` or `}`")?;
        let expr = self.expr(None)?;
        Some(cst::Stmt::Return { ret, expr })
    }

    /// `var name type = init`.
    fn var_stmt(&mut self) -> Option<cst::Stmt> {
        let var = self.expect(token::Kind::Var, "`var`")?;
        let name = self.name()?;
        let ty = self.name()?;
        let equals = self.expect(token::Kind::Equals, "`=`")?;
        let init = self.expr(None)?;
        Some(cst::Stmt::Var {
            var,
            name,
            ty,
            equals,
            init,
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
    fn expr(&mut self, ambient: Option<token::Token>) -> Option<cst::Expr> {
        let mut lhs = self.operand(ambient)?;
        // The caller only ever passes a token it consumed as an operator.
        let enclosing = ambient.and_then(|token| Some((token, operator_group(token.kind)?)));

        while let Some((token, op)) = self.peek_binary_op() {
            if let Some((left, left_group)) = enclosing {
                match precedence::priority(left_group, precedence::group(op)) {
                    // The enclosing operator takes the operand just parsed.
                    precedence::Priority::Left => break,
                    precedence::Priority::Ambiguous => {
                        let kind = diag::Kind::AmbiguousPrecedence {
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
            lhs = cst::Expr::Binary {
                lhs: Box::new(lhs),
                op: token,
                rhs: Box::new(rhs),
            };
        }

        Some(lhs)
    }

    /// A literal, a variable, a parenthesized expression, or `!` applied to
    /// one. `ambient` is the enclosing operator, as in `expr`.
    fn operand(&mut self, ambient: Option<token::Token>) -> Option<cst::Expr> {
        if let Some(&bang) = self.peek().filter(|token| token.kind == token::Kind::Bang) {
            return self.not(bang, ambient);
        }

        if self.peek_kind() == Some(token::Kind::LParen) {
            let lparen = self.expect(token::Kind::LParen, OPERAND)?;
            let inner = self.expr(None)?;
            let rparen = self.expect(token::Kind::RParen, "`)`")?;
            return Some(cst::Expr::Paren {
                lparen,
                inner: Box::new(inner),
                rparen,
            });
        }

        if let Some(kind @ (token::Kind::True | token::Kind::False)) = self.peek_kind() {
            return Some(cst::Expr::BoolLit(self.expect(kind, OPERAND)?));
        }

        if self.peek_kind() == Some(token::Kind::Ident) {
            return Some(cst::Expr::Var(self.name()?));
        }

        Some(cst::Expr::IntLit(
            self.expect(token::Kind::IntLit, OPERAND)?,
        ))
    }

    /// `!` applied to an operand, `bang` being the operator token, which is
    /// still to be consumed.
    fn not(&mut self, bang: token::Token, ambient: Option<token::Token>) -> Option<cst::Expr> {
        // An enclosing operator the graph does not order against `!` is the
        // whole point of a partial order: the source must parenthesize.
        let enclosing = ambient.and_then(|token| Some((token, operator_group(token.kind)?)));
        if let Some((left, left_group)) = enclosing
            && precedence::priority(left_group, precedence::unary_group(ast::UnOp::Not))
                == precedence::Priority::Ambiguous
        {
            let kind = diag::Kind::AmbiguousPrecedence {
                left: describe(left.kind),
                right: describe(bang.kind),
            };
            return self.report(kind, bang.span);
        }

        self.next += 1;
        // Parsing the operand with `!` enclosing it is what places the
        // operators that follow it.
        let operand = self.expr(Some(bang))?;
        Some(cst::Expr::Unary {
            op: bang,
            operand: Box::new(operand),
        })
    }

    fn name(&mut self) -> Option<token::Token> {
        self.expect(token::Kind::Ident, "an identifier")
    }

    /// Consumes the next token if it is a `kind`, and reports the unexpected
    /// one otherwise. `expected` describes what the grammar allows here.
    fn expect(&mut self, kind: token::Kind, expected: &'static str) -> Option<token::Token> {
        match self.peek() {
            Some(&token) if token.kind == kind => {
                self.next += 1;
                Some(token)
            }
            found => {
                let (found, span) = match found {
                    Some(token) => (describe(token.kind), token.span),
                    // A stream that ran out has no `Eof` to point at, so the
                    // last token there is stands in for the end of input.
                    None => ("end of input", self.last_span()),
                };
                self.report(diag::Kind::UnexpectedToken { expected, found }, span)
            }
        }
    }

    /// Reports a diagnostic and fails the parse.
    fn report<T>(&mut self, kind: diag::Kind, span: diag::Span) -> Option<T> {
        self.diags.push(diag::Entry { kind, span });
        None
    }

    fn peek(&self) -> Option<&token::Token> {
        self.tokens.get(self.next)
    }

    fn peek_kind(&self) -> Option<token::Kind> {
        self.peek().map(|token| token.kind)
    }

    /// The next token and the operator it denotes, if it denotes one.
    fn peek_binary_op(&self) -> Option<(token::Token, ast::BinOp)> {
        let &token = self.peek()?;
        Some((token, ast::binary_op(token.kind)?))
    }

    /// The span of the last token in the stream, empty if there is none.
    fn last_span(&self) -> diag::Span {
        self.tokens
            .last()
            .map_or(diag::Span { start: 0, end: 0 }, |token| token.span)
    }
}

fn describe(kind: token::Kind) -> &'static str {
    match kind {
        token::Kind::Func => "`func`",
        token::Kind::Return => "`return`",
        token::Kind::Var => "`var`",
        token::Kind::True => "`true`",
        token::Kind::False => "`false`",
        token::Kind::Ident => "an identifier",
        token::Kind::IntLit => "an integer literal",
        token::Kind::LParen => "`(`",
        token::Kind::RParen => "`)`",
        token::Kind::LBrace => "`{`",
        token::Kind::RBrace => "`}`",
        token::Kind::Plus => "`+`",
        token::Kind::Minus => "`-`",
        token::Kind::Star => "`*`",
        token::Kind::Slash => "`/`",
        token::Kind::Percent => "`%`",
        token::Kind::Equals => "`=`",
        token::Kind::EqEq => "`==`",
        token::Kind::BangEq => "`!=`",
        token::Kind::Lt => "`<`",
        token::Kind::LtEq => "`<=`",
        token::Kind::Gt => "`>`",
        token::Kind::GtEq => "`>=`",
        token::Kind::Bang => "`!`",
        token::Kind::AmpAmp => "`&&`",
        token::Kind::PipePipe => "`||`",
        token::Kind::Eof => "end of input",
    }
}

/// The precedence group of an operator token, if it is one.
fn operator_group(kind: token::Kind) -> Option<precedence::Group> {
    match kind {
        token::Kind::Bang => Some(precedence::unary_group(ast::UnOp::Not)),
        kind => ast::binary_op(kind).map(precedence::group),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    /// Lexes `source`, parses it, and builds its AST, asserting that nothing
    /// reported a diagnostic. The tests assert on the AST, not the CST.
    fn parse_ok(source: &str) -> ast::Program {
        let mut diags = diag::Sink::default();
        let tokens = token::lex(source, &mut diags).expect("lexing succeeded");
        let parsed = parse(&tokens, &mut diags).expect("parsing succeeded");
        let program = ast::from_cst(source, &parsed, &mut diags);
        assert!(diags.is_empty());
        program.expect("the AST was built")
    }

    /// The same, for a `source` that must report exactly one diagnostic and
    /// produce no AST.
    fn parse_err(source: &str) -> diag::Entry {
        let mut diags = diag::Sink::default();
        let tokens = token::lex(source, &mut diags).expect("lexing succeeded");
        let parsed = parse(&tokens, &mut diags);
        let program = parsed.and_then(|parsed| ast::from_cst(source, &parsed, &mut diags));
        assert_eq!(program, None);
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
                        span: diag::Span {
                            start: return_start,
                            end: literal.end
                        },
                    }],
                    span: diag::Span { start, end },
                }]
            }
        );
    }

    /// The example program of the variables milestone.
    const VARIABLES: &str = "func approval() uint64 {\n  var x uint64 = 1 + 2\n  \
                             var y uint64 = x * 3\n  return y - x\n}\n";

    /// The variable `text`, written at `span`.
    fn var(text: &str, span: diag::Span) -> ast::Expr {
        ast::Expr::Var {
            name: testing::name(text, span),
            span,
        }
    }

    /// `op operand`, the operator being the byte just before the operand.
    fn unary(op: ast::UnOp, operand: ast::Expr) -> ast::Expr {
        let span = diag::Span {
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
    fn binary(op: ast::BinOp, lhs: ast::Expr, rhs: ast::Expr) -> ast::Expr {
        let span = diag::Span {
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
                            init: binary(ast::BinOp::Add, int(1, one), int(2, two)),
                            span: diag::Span {
                                start: first_var,
                                end: two.end
                            },
                        },
                        ast::Stmt::Var {
                            name: y,
                            ty: y_ty,
                            init: binary(ast::BinOp::Mul, var("x", x_times), int(3, three)),
                            span: diag::Span {
                                start: second_var,
                                end: three.end
                            },
                        },
                        ast::Stmt::Return {
                            expr: binary(ast::BinOp::Sub, var("y", y_minus), var("x", x_minus)),
                            span: diag::Span {
                                start: return_start,
                                end: x_minus.end
                            },
                        },
                    ],
                    span: diag::Span { start, end },
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
                ast::BinOp::Add,
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
    fn parentheses_around_a_variable_are_dropped() {
        let source = wrap("(x)");
        let x = testing::span_of(&source, "x", 0);

        assert_eq!(
            returned(&source),
            ast::Expr::Var {
                name: testing::name("x", x),
                span: x,
            }
        );
    }

    #[test]
    fn a_declaration_without_a_type_is_reported() {
        let source = "func f() uint64 { var x = 1 }";
        let mut span = testing::spans(source);
        assert_eq!(
            parse_err(source),
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
        assert_eq!(program.funcs[0].span, diag::Span { start: 0, end: 28 });
        assert_eq!(
            program.funcs[1].span,
            diag::Span {
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
                    span: diag::Span { start, end },
                }]
            }
        );
    }

    #[test]
    fn a_missing_closing_paren_is_reported() {
        let source = "func f( uint64 {}";
        assert_eq!(
            parse_err(source),
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
                    expected: "`)`",
                    found: "an identifier",
                },
                span: diag::Span { start: 8, end: 14 },
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
    fn parentheses_around_a_boolean_literal_are_dropped() {
        let source = wrap("(false)");

        assert_eq!(
            returned(&source),
            ast::Expr::BoolLit {
                value: false,
                span: testing::span_of(&source, "false", 0),
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
                op: ast::BinOp::Add,
                lhs: Box::new(ast::Expr::BoolLit {
                    value: true,
                    span: literal,
                }),
                rhs: Box::new(ast::Expr::IntLit {
                    value: 1,
                    span: one
                }),
                span: diag::Span {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
                op: ast::BinOp::Add,
                lhs: Box::new(ast::Expr::IntLit {
                    value: 1,
                    span: one
                }),
                rhs: Box::new(ast::Expr::Binary {
                    op: ast::BinOp::Mul,
                    lhs: Box::new(ast::Expr::IntLit {
                        value: 2,
                        span: two
                    }),
                    rhs: Box::new(ast::Expr::IntLit {
                        value: 3,
                        span: three
                    }),
                    span: diag::Span {
                        start: two.start,
                        end: three.end
                    },
                }),
                span: diag::Span {
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
                op: ast::BinOp::Add,
                lhs: Box::new(ast::Expr::Binary {
                    op: ast::BinOp::Mul,
                    lhs: Box::new(ast::Expr::IntLit {
                        value: 1,
                        span: one
                    }),
                    rhs: Box::new(ast::Expr::IntLit {
                        value: 2,
                        span: two
                    }),
                    span: diag::Span {
                        start: one.start,
                        end: two.end
                    },
                }),
                rhs: Box::new(ast::Expr::IntLit {
                    value: 3,
                    span: three
                }),
                span: diag::Span {
                    start: one.start,
                    end: three.end
                },
            }
        );
    }

    #[test]
    fn a_group_is_left_associative() {
        for (source, op) in [
            (wrap("1 - 2 - 3"), ast::BinOp::Sub),
            (wrap("1 / 2 / 3"), ast::BinOp::Div),
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
                        span: diag::Span {
                            start: one.start,
                            end: two.end
                        },
                    }),
                    rhs: Box::new(ast::Expr::IntLit {
                        value: 3,
                        span: three
                    }),
                    span: diag::Span {
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
            op: ast::BinOp::Mul,
            lhs: Box::new(ast::Expr::IntLit {
                value: 2,
                span: two,
            }),
            rhs: Box::new(ast::Expr::IntLit {
                value: 3,
                span: three,
            }),
            span: diag::Span {
                start: two.start,
                end: three.end,
            },
        };
        let sum = ast::Expr::Binary {
            op: ast::BinOp::Add,
            lhs: Box::new(ast::Expr::IntLit {
                value: 1,
                span: one,
            }),
            rhs: Box::new(product),
            span: diag::Span {
                start: one.start,
                end: three.end,
            },
        };
        let quotient = ast::Expr::Binary {
            op: ast::BinOp::Div,
            lhs: Box::new(ast::Expr::IntLit {
                value: 4,
                span: four,
            }),
            rhs: Box::new(ast::Expr::IntLit {
                value: 5,
                span: five,
            }),
            span: diag::Span {
                start: four.start,
                end: five.end,
            },
        };

        assert_eq!(
            returned(&source),
            ast::Expr::Binary {
                op: ast::BinOp::Sub,
                lhs: Box::new(sum),
                rhs: Box::new(quotient),
                span: diag::Span {
                    start: one.start,
                    end: five.end
                },
            }
        );
    }

    #[test]
    fn parentheses_regroup() {
        let source = wrap("(1 + 2) * 3");
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
                ast::BinOp::Mul,
                binary(ast::BinOp::Add, int(1, one), int(2, two)),
                int(3, three),
            )
        );
    }

    #[test]
    fn parentheses_around_a_literal_are_dropped() {
        let source = wrap("(1)");

        assert_eq!(
            returned(&source),
            ast::Expr::IntLit {
                value: 1,
                span: testing::span_of(&source, "1", 0),
            }
        );
    }

    #[test]
    fn parentheses_nest() {
        let source = wrap("((1 + 2))");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");

        let int = |value, span| ast::Expr::IntLit { value, span };
        assert_eq!(
            returned(&source),
            binary(ast::BinOp::Add, int(1, one), int(2, two))
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
                op: ast::BinOp::Mod,
                lhs: Box::new(ast::Expr::IntLit {
                    value: 1,
                    span: one
                }),
                rhs: Box::new(ast::Expr::IntLit {
                    value: 2,
                    span: two
                }),
                span: diag::Span {
                    start: one.start,
                    end: two.end
                },
            }
        );

        let source = wrap("(1 % 2) % 3");
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
                ast::BinOp::Mod,
                binary(ast::BinOp::Mod, int(1, one), int(2, two)),
                int(3, three),
            )
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
                diag::Entry {
                    kind: diag::Kind::AmbiguousPrecedence { left, right },
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
                    expected: OPERAND,
                    found: "end of input",
                },
                span: diag::Span {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
                    expected: "`var`, `return` or `}`",
                    found: "end of input",
                },
                span: diag::Span {
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
            diag::Entry {
                kind: diag::Kind::UnexpectedToken {
                    expected: "`func`",
                    found: "`}`",
                },
                span: diag::Span { start: 0, end: 1 },
            }
        );
    }

    #[test]
    fn an_integer_literal_that_does_not_fit_is_reported() {
        let source = "func f() uint64 { return 18446744073709551616 }";
        assert_eq!(
            parse_err(source),
            diag::Entry {
                kind: diag::Kind::IntegerLiteralOutOfRange,
                span: diag::Span { start: 25, end: 45 },
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
                    span: diag::Span { start: 25, end: 45 },
                },
                span: diag::Span { start: 18, end: 45 },
            }]
        );
    }

    #[test]
    fn parses_every_comparison_operator() {
        let cases = [
            ("1 == 2", ast::BinOp::Eq),
            ("1 != 2", ast::BinOp::Ne),
            ("1 < 2", ast::BinOp::Lt),
            ("1 <= 2", ast::BinOp::Le),
            ("1 > 2", ast::BinOp::Gt),
            ("1 >= 2", ast::BinOp::Ge),
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
                ast::BinOp::Eq,
                binary(ast::BinOp::Add, int(1, one), int(2, two)),
                binary(ast::BinOp::Mul, int(3, three), int(4, four)),
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
                ast::BinOp::Eq,
                binary(ast::BinOp::Mod, int(1, one), int(2, two)),
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
                ast::BinOp::Eq,
                int(1, one),
                binary(ast::BinOp::Mod, int(2, two), int(3, three)),
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
                ast::BinOp::Lt,
                int(1, one),
                binary(ast::BinOp::Add, int(2, two), int(3, three)),
            )
        );
    }

    #[test]
    fn parentheses_make_a_comparison_an_operand() {
        let source = wrap("(1 < 2) == true");
        let mut span = testing::spans(&source);
        span("(");
        span(")");
        let one = span("1");
        let two = span("2");
        let literal = span("true");

        let int = |value, span| ast::Expr::IntLit { value, span };
        let parenthesized = binary(ast::BinOp::Lt, int(1, one), int(2, two));

        assert_eq!(
            returned(&source),
            binary(
                ast::BinOp::Eq,
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
                ast::UnOp::Not,
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
                ast::UnOp::Not,
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
        let bang = span("!");
        let one = span("1");
        let two = span("2");

        let int = |value, span| ast::Expr::IntLit { value, span };
        assert_eq!(
            returned(&source),
            ast::Expr::Unary {
                op: ast::UnOp::Not,
                operand: Box::new(binary(ast::BinOp::Lt, int(1, one), int(2, two))),
                span: diag::Span {
                    start: bang.start,
                    end: two.end,
                },
            }
        );
    }

    #[test]
    fn parses_both_logical_operators() {
        for (expr, op) in [
            ("true && false", ast::BinOp::And),
            ("true || false", ast::BinOp::Or),
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
                ast::BinOp::And,
                unary(ast::UnOp::Not, var("x", x)),
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
                ast::BinOp::Or,
                var("x", x),
                unary(ast::UnOp::Not, var("y", y))
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
                ast::BinOp::And,
                binary(
                    ast::BinOp::And,
                    var("x", x),
                    unary(ast::UnOp::Not, var("y", y))
                ),
                var("z", z),
            )
        );
    }

    #[test]
    fn logic_is_left_associative() {
        for (expr, op) in [
            ("x && y && z", ast::BinOp::And),
            ("x || y || z", ast::BinOp::Or),
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
                ast::BinOp::And,
                binary(ast::BinOp::Lt, int(1, one), int(2, two)),
                binary(ast::BinOp::Lt, int(3, three), int(4, four)),
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
                ast::BinOp::Or,
                binary(
                    ast::BinOp::Eq,
                    binary(ast::BinOp::Add, int(1, one), int(2, two)),
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
        let yes = span("true");
        let no = span("false");
        let last = span("true");

        let boolean = |value, span| ast::Expr::BoolLit { value, span };
        assert_eq!(
            returned(&source),
            binary(
                ast::BinOp::Or,
                binary(ast::BinOp::And, boolean(true, yes), boolean(false, no)),
                boolean(true, last),
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
                diag::Entry {
                    kind: diag::Kind::AmbiguousPrecedence { left, right },
                    span: testing::span_of(&source, text, nth),
                },
                "{source}"
            );
        }
    }

    /// `source` with every comment blanked out, so that the tokens around it
    /// keep the spans they had.
    fn blank_comments(source: &str) -> String {
        let lines: Vec<String> = source
            .split('\n')
            .map(|line| match line.split_once("//") {
                Some((before, comment)) => format!("{before}{}", " ".repeat(comment.len() + 2)),
                None => line.to_string(),
            })
            .collect();
        lines.join("\n")
    }

    #[test]
    fn comments_are_ignored() {
        let source = "// The approval program.\nfunc approval() uint64 {\n  var x uint64 = 1 + 2 // one more than two\n  return x\n}\n";
        assert_eq!(parse_ok(source), parse_ok(&blank_comments(source)));
    }
}
