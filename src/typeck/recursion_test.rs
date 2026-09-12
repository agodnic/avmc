//! Tests for the recursion check.

use super::pass;
use crate::diag;
use crate::testing;
use crate::typed_ast;

/// Checks `source`, asserting that it produced no diagnostics.
fn check_ok(source: &str) -> typed_ast::Program {
    let mut diags = diag::Sink::default();
    let program = pass::check(&testing::lex_parse(source), &mut diags);
    assert!(diags.is_empty());
    program.expect("checking succeeded")
}

/// Checks `source`, asserting that checking produced nothing, and
/// returning the diagnostics in the order they were reported.
fn check_err(source: &str) -> Vec<diag::Entry> {
    let mut diags = diag::Sink::default();
    assert_eq!(pass::check(&testing::lex_parse(source), &mut diags), None);
    diags.iter().cloned().collect()
}

/// The example program of the recursion milestone.
const MUTUAL: &str = "func approval() uint64 {\n\treturn f(1)\n}\n\n\
                      func f(n uint64) uint64 {\n\treturn g(n)\n}\n\n\
                      func g(n uint64) uint64 {\n\treturn f(n)\n}\n";

/// The recursion diagnostic naming `name`, at the `nth` occurrence of
/// `call` in `source`.
fn recursive(source: &str, call: &str, nth: usize, name: &str) -> diag::Entry {
    diag::Entry {
        kind: diag::Kind::RecursiveCall {
            name: name.to_string(),
        },
        span: testing::span_of(source, call, nth),
    }
}

#[test]
fn a_function_calling_itself_is_reported() {
    let source = "func approval() uint64 { return 1 } func f(n uint64) uint64 { return f(n) }";
    assert_eq!(check_err(source), vec![recursive(source, "f(n)", 0, "f")]);
}

#[test]
fn mutual_recursion_is_reported_at_the_call_that_closes_the_cycle() {
    assert_eq!(
        check_err(MUTUAL),
        vec![recursive(MUTUAL, "f(n)", 0, "f")],
        "the call to `f` in `g`"
    );
}

#[test]
fn a_longer_cycle_is_reported_once() {
    let source = "func approval() uint64 { return f(1) } \
                  func f(n uint64) uint64 { return g(n) } \
                  func g(n uint64) uint64 { return h(n) } \
                  func h(n uint64) uint64 { return f(n) }";
    assert_eq!(check_err(source), vec![recursive(source, "f(n)", 0, "f")]);
}

#[test]
fn every_cycle_is_reported() {
    let source = "func approval() uint64 { return 1 } \
                  func a(n uint64) uint64 { return a(n) } \
                  func b(n uint64) uint64 { return c(n) } \
                  func c(n uint64) uint64 { return b(n) }";
    assert_eq!(
        check_err(source),
        vec![
            recursive(source, "a(n)", 0, "a"),
            recursive(source, "b(n)", 0, "b"),
        ]
    );
}

#[test]
fn a_shared_callee_is_not_recursion() {
    let source = "func approval() uint64 { return a(1) + b(2) } \
                  func a(n uint64) uint64 { return c(n) } \
                  func b(n uint64) uint64 { return c(n) } \
                  func c(n uint64) uint64 { return n }";
    assert_eq!(check_ok(source).funcs.len(), 4);
}

#[test]
fn the_entry_point_calling_itself_is_reported() {
    let source = "func approval() uint64 { return approval() }";
    // The declaration holds the first `approval()`; the call, the second.
    assert_eq!(
        check_err(source),
        vec![recursive(source, "approval()", 1, "approval")]
    );
}

#[test]
fn recursion_is_checked_only_when_the_bodies_check() {
    let source = "func approval() uint64 { return f(1) } \
                  func f(n uint64) uint64 { return f(true) }";
    assert_eq!(
        check_err(source),
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
                expected: typed_ast::Type::Uint64,
                found: typed_ast::Type::Bool,
            },
            span: testing::span_of(source, "true", 0),
        }]
    );
}

#[test]
fn a_call_before_the_cycle_is_not_reported() {
    let source = "func approval() uint64 { return f(1) } func f(n uint64) uint64 { return f(n) }";
    assert_eq!(check_err(source), vec![recursive(source, "f(n)", 0, "f")]);
}
