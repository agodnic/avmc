//! The parser: tokens to an AST by recursive descent (see `ARCHITECTURE.md`
//! §2.1).

use crate::ast::{Expr, FuncDecl, Name, Program, Stmt, TypeRef};
use crate::diagnostics::{Diagnostic, Diagnostics, Span};
use crate::lexer::{Token, TokenKind};

/// Parses the token stream `lex` produced for `source`.
///
/// Returns `None` as soon as anything is wrong (R2); one run reports at most
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
        let start = self
            .expect(TokenKind::Return, "`return` or `}`")?
            .span
            .start;
        let expr = self.expr()?;
        let Expr::IntLit { span, .. } = expr;
        Some(Stmt::Return {
            expr,
            span: Span {
                start,
                end: span.end,
            },
        })
    }

    fn expr(&mut self) -> Option<Expr> {
        let span = self.expect(TokenKind::IntLit, "an integer literal")?.span;
        match self.text(span).parse::<u64>() {
            Ok(value) => Some(Expr::IntLit { value, span }),
            // The lexer only admits ASCII digits, so overflow is the one way
            // parsing can fail here.
            Err(_) => self.report("E0003", "integer literal out of range".to_string(), span),
        }
    }

    fn name(&mut self) -> Option<Name> {
        let span = self.expect(TokenKind::Ident, "an identifier")?.span;
        Some(Name {
            text: self.text(span).to_string(),
            span,
        })
    }

    /// Consumes the next token if it is a `kind`, and reports E0002 otherwise.
    /// `expected` describes what the grammar allows here.
    fn expect(&mut self, kind: TokenKind, expected: &str) -> Option<Token> {
        match self.peek() {
            Some(&token) if token.kind == kind => {
                self.next += 1;
                Some(token)
            }
            found => {
                let (found_desc, span) = match found {
                    Some(token) => (describe(token.kind), token.span),
                    None => (
                        "end of input",
                        Span {
                            start: self.source.len(),
                            end: self.source.len(),
                        },
                    ),
                };
                self.report(
                    "E0002",
                    format!("expected {expected}, found {found_desc}"),
                    span,
                )
            }
        }
    }

    /// Reports a diagnostic and fails the parse (R2).
    fn report<T>(&mut self, code: &'static str, message: String, span: Span) -> Option<T> {
        self.diags.push(Diagnostic {
            code,
            message,
            span,
        });
        None
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.next)
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|token| token.kind)
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
        TokenKind::Ident => "an identifier",
        TokenKind::IntLit => "an integer literal",
        TokenKind::LParen => "`(`",
        TokenKind::RParen => "`)`",
        TokenKind::LBrace => "`{`",
        TokenKind::RBrace => "`}`",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

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

    /// Returns a closure giving the span of the next occurrence of its
    /// argument, so expected spans are written in source order.
    fn spans(source: &str) -> impl FnMut(&str) -> Span + '_ {
        let mut offset = 0;
        move |text| {
            let start = source[offset..].find(text).expect("text in source") + offset;
            offset = start + text.len();
            Span { start, end: offset }
        }
    }

    fn name(text: &str, span: Span) -> Name {
        Name {
            text: text.to_string(),
            span,
        }
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
                code: "E0002",
                message: "expected `)`, found an identifier".to_string(),
                span: Span { start: 8, end: 14 },
            }
        );
    }

    #[test]
    fn an_unterminated_body_is_reported_at_end_of_input() {
        let source = "func f() uint64 {";
        assert_eq!(
            parse_err(source),
            Diagnostic {
                code: "E0002",
                message: "expected `return` or `}`, found end of input".to_string(),
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
                code: "E0002",
                message: "expected `func`, found `}`".to_string(),
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
                code: "E0003",
                message: "integer literal out of range".to_string(),
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
