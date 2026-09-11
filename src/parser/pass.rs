use crate::ast;
use crate::cst;
use crate::diag;
use crate::lexer;
use crate::precedence;

/// Parses the token stream `lex` produced.
///
/// Returns `None` as soon as anything is wrong; one run reports at most
/// one diagnostic.
pub fn parse(tokens: &[lexer::Token], diags: &mut diag::Sink) -> Option<cst::Program> {
    Parser {
        tokens,
        next: 0,
        diags,
    }
    .program()
}

/// What the grammar allows where an operand is expected.
pub(super) const OPERAND: &str = "a literal, an identifier, `!`, or `(`";

struct Parser<'a> {
    tokens: &'a [lexer::Token],
    /// Index of the token to be consumed next.
    next: usize,
    diags: &'a mut diag::Sink,
}

impl Parser<'_> {
    fn program(&mut self) -> Option<cst::Program> {
        let mut funcs = Vec::new();
        while self.peek_kind() != Some(lexer::TokenKind::Eof) {
            funcs.push(self.func_decl()?);
        }
        let eof = self.expect(lexer::TokenKind::Eof, "`func`")?;
        Some(cst::Program { funcs, eof })
    }

    fn func_decl(&mut self) -> Option<cst::FuncDecl> {
        let func = self.expect(lexer::TokenKind::Func, "`func`")?;
        let name = self.name()?;
        let lparen = self.expect(lexer::TokenKind::LParen, "`(`")?;
        let (params, rparen) = self.params()?;
        let ret = self.name()?;
        let lbrace = self.expect(lexer::TokenKind::LBrace, "`{`")?;

        let mut body = Vec::new();
        while self.peek_kind() != Some(lexer::TokenKind::RBrace) {
            body.push(self.stmt()?);
        }
        let rbrace = self.expect(lexer::TokenKind::RBrace, "`}`")?;

        Some(cst::FuncDecl {
            func,
            name,
            lparen,
            params,
            rparen,
            ret,
            lbrace,
            body,
            rbrace,
        })
    }

    /// The parameter list and the `)` that ends it, the `(` already consumed.
    fn params(&mut self) -> Option<(Vec<cst::Param>, lexer::Token)> {
        let mut params = Vec::new();
        if let Some(rparen) = self.consume(lexer::TokenKind::RParen) {
            return Some((params, rparen));
        }

        loop {
            let name = self.name()?;
            let ty = self.name()?;
            let comma = self.consume(lexer::TokenKind::Comma);
            params.push(cst::Param { name, ty, comma });
            if comma.is_none() {
                let rparen = self.expect(lexer::TokenKind::RParen, "`,` or `)`")?;
                return Some((params, rparen));
            }
        }
    }

    fn stmt(&mut self) -> Option<cst::Stmt> {
        if self.peek_kind() == Some(lexer::TokenKind::Var) {
            return self.var_stmt();
        }

        let ret = self.expect(lexer::TokenKind::Return, "`var`, `return` or `}`")?;
        let expr = self.expr(None)?;
        Some(cst::Stmt::Return { ret, expr })
    }

    /// `var name type = init`.
    fn var_stmt(&mut self) -> Option<cst::Stmt> {
        let var = self.expect(lexer::TokenKind::Var, "`var`")?;
        let name = self.name()?;
        let ty = self.name()?;
        let equals = self.expect(lexer::TokenKind::Equals, "`=`")?;
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
    fn expr(&mut self, ambient: Option<lexer::Token>) -> Option<cst::Expr> {
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
    fn operand(&mut self, ambient: Option<lexer::Token>) -> Option<cst::Expr> {
        if let Some(&bang) = self
            .peek()
            .filter(|token| token.kind == lexer::TokenKind::Bang)
        {
            return self.not(bang, ambient);
        }

        if self.peek_kind() == Some(lexer::TokenKind::LParen) {
            let lparen = self.expect(lexer::TokenKind::LParen, OPERAND)?;
            let inner = self.expr(None)?;
            let rparen = self.expect(lexer::TokenKind::RParen, "`)`")?;
            return Some(cst::Expr::Paren {
                lparen,
                inner: Box::new(inner),
                rparen,
            });
        }

        if let Some(kind @ (lexer::TokenKind::True | lexer::TokenKind::False)) = self.peek_kind() {
            return Some(cst::Expr::BoolLit(self.expect(kind, OPERAND)?));
        }

        if self.peek_kind() == Some(lexer::TokenKind::Ident) {
            return Some(cst::Expr::Var(self.name()?));
        }

        Some(cst::Expr::IntLit(
            self.expect(lexer::TokenKind::IntLit, OPERAND)?,
        ))
    }

    /// `!` applied to an operand, `bang` being the operator token, which is
    /// still to be consumed.
    fn not(&mut self, bang: lexer::Token, ambient: Option<lexer::Token>) -> Option<cst::Expr> {
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

    fn name(&mut self) -> Option<lexer::Token> {
        self.expect(lexer::TokenKind::Ident, "an identifier")
    }

    /// Consumes the next token if it is a `kind`, and leaves it otherwise.
    fn consume(&mut self, kind: lexer::TokenKind) -> Option<lexer::Token> {
        let &token = self.peek().filter(|token| token.kind == kind)?;
        self.next += 1;
        Some(token)
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

    fn peek(&self) -> Option<&lexer::Token> {
        self.tokens.get(self.next)
    }

    fn peek_kind(&self) -> Option<lexer::TokenKind> {
        self.peek().map(|token| token.kind)
    }

    /// The next token and the operator it denotes, if it denotes one.
    fn peek_binary_op(&self) -> Option<(lexer::Token, ast::BinOp)> {
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
        lexer::TokenKind::Comma => "`,`",
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
        lexer::TokenKind::Eof => "end of input",
    }
}

/// The precedence group of an operator token, if it is one.
fn operator_group(kind: lexer::TokenKind) -> Option<precedence::Group> {
    match kind {
        lexer::TokenKind::Bang => Some(precedence::unary_group(ast::UnOp::Not)),
        kind => ast::binary_op(kind).map(precedence::group),
    }
}
