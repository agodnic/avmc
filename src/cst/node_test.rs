//! Tests for the CST nodes.

use super::node::{Expr, Program, Stmt};
use crate::ast;
use crate::diag;
use crate::lexer;
use crate::parser;

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
