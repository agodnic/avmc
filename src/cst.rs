//! The concrete syntax tree: the parser's output, holding every token.

use crate::lexer;

/// A whole source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The functions it declares, in source order.
    pub funcs: Vec<FuncDecl>,
    /// The token that ends the stream.
    pub eof: lexer::Token,
}

/// A function declaration: `func name() ret { body }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncDecl {
    /// The `func` keyword.
    pub func: lexer::Token,
    /// The declared name.
    pub name: lexer::Token,
    /// The `(` of the parameter list.
    pub lparen: lexer::Token,
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

/// The tokens of `program` in source order, ending with `eof`.
pub fn tokens(program: &Program) -> Vec<lexer::Token> {
    let mut tokens = Vec::new();
    for func in &program.funcs {
        push_func(&mut tokens, func);
    }
    tokens.push(program.eof);
    tokens
}

fn push_func(tokens: &mut Vec<lexer::Token>, func: &FuncDecl) {
    tokens.extend([
        func.func,
        func.name,
        func.lparen,
        func.rparen,
        func.ret,
        func.lbrace,
    ]);
    for stmt in &func.body {
        push_stmt(tokens, stmt);
    }
    tokens.push(func.rbrace);
}

fn push_stmt(tokens: &mut Vec<lexer::Token>, stmt: &Stmt) {
    match stmt {
        Stmt::Var {
            var,
            name,
            ty,
            equals,
            init,
        } => {
            tokens.extend([*var, *name, *ty, *equals]);
            push_expr(tokens, init);
        }
        Stmt::Return { ret, expr } => {
            tokens.push(*ret);
            push_expr(tokens, expr);
        }
    }
}

fn push_expr(tokens: &mut Vec<lexer::Token>, expr: &Expr) {
    match expr {
        Expr::IntLit(token) | Expr::BoolLit(token) | Expr::Var(token) => tokens.push(*token),
        Expr::Binary { lhs, op, rhs } => {
            push_expr(tokens, lhs);
            tokens.push(*op);
            push_expr(tokens, rhs);
        }
        Expr::Unary { op, operand } => {
            tokens.push(*op);
            push_expr(tokens, operand);
        }
        Expr::Paren {
            lparen,
            inner,
            rparen,
        } => {
            tokens.push(*lparen);
            push_expr(tokens, inner);
            tokens.push(*rparen);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::diag;
    use crate::parser;

    /// The approval program of the v0 milestone, with comments.
    const COMMENTED_APPROVAL: &str = "// The approval program.\nfunc approval() uint64 {\n  var x uint64 = 1 + 2 // one more than two\n  return x\n}\n";

    /// The example program of the variables milestone.
    const VARIABLES: &str = "func approval() uint64 {\n  var x uint64 = 1 + 2\n  \
                             var y uint64 = x * 3\n  return y - x\n}\n";

    /// Lexes and parses `source`, asserting that both succeeded without
    /// diagnostics.
    fn parse_cst(source: &str) -> Program {
        let mut diags = diag::Sink::default();
        let tokens = lexer::lex(source, &mut diags).expect("lexing succeeded");
        let program = parser::parse(&tokens, &mut diags).expect("parsing succeeded");
        assert!(diags.is_empty());
        program
    }

    /// The expression the single function of `source` returns.
    fn returned(source: &str) -> Expr {
        let program = parse_cst(source);
        let mut funcs = program.funcs.into_iter();
        let func = funcs.next().expect("one function");
        assert!(funcs.next().is_none());
        let mut body = func.body.into_iter();
        let Some(Stmt::Return { expr, .. }) = body.next() else {
            panic!("one return statement")
        };
        assert!(body.next().is_none());
        expr
    }

    #[test]
    fn parentheses_are_a_node() {
        let source = "func f() bool { return (true) }";
        let Expr::Paren { inner, .. } = returned(source) else {
            panic!("a parenthesized expression")
        };
        assert!(matches!(*inner, Expr::BoolLit(_)));
    }

    #[test]
    fn nested_parentheses_are_nested_nodes() {
        let source = "func f() uint64 { return ((1)) }";
        let Expr::Paren { inner, .. } = returned(source) else {
            panic!("a parenthesized expression")
        };
        let Expr::Paren { inner, .. } = *inner else {
            panic!("a nested parenthesized expression")
        };
        assert!(matches!(*inner, Expr::IntLit(_)));
    }

    #[test]
    fn the_tree_is_lossless() {
        let sources = [
            COMMENTED_APPROVAL,
            VARIABLES,
            "func f() uint64 { return (1 + 2) * 3 // grouped\n}\n",
        ];

        for source in sources {
            let mut diags = diag::Sink::default();
            let lexed = lexer::lex(source, &mut diags).expect("lexing succeeded");
            let tokens = tokens(&parse_cst(source));
            assert_eq!(tokens, lexed, "{source}");

            let text = |span: diag::Span| source.get(span.start..span.end).unwrap_or("");
            let reassembled: String = tokens
                .iter()
                .map(|token| format!("{}{}", text(token.trivia), text(token.span)))
                .collect();
            assert_eq!(reassembled, source);
        }
    }

    #[test]
    fn an_integer_literal_out_of_range_is_reported() {
        let source = "func f() uint64 { return 18446744073709551616 }";
        let program = parse_cst(source);

        let mut diags = diag::Sink::default();
        assert_eq!(ast::from_cst(source, &program, &mut diags), None);
        assert_eq!(
            diags.iter().cloned().collect::<Vec<_>>(),
            [diag::Entry {
                kind: diag::Kind::IntegerLiteralOutOfRange,
                span: diag::Span { start: 25, end: 45 },
            }]
        );
    }
}
