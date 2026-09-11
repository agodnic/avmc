//! Helpers shared by the stage tests.

use crate::ast;
use crate::diag;
use crate::lexer;
use crate::parser;
use crate::typeck;
use crate::typed_ast;

/// The example program used by the stage tests.
pub(crate) const EXAMPLE: &str = "func approval() uint64 { return 1 }";

/// The span of the `nth` occurrence of `text` in `source`, counting from 0.
pub(crate) fn span_of(source: &str, text: &str, nth: usize) -> diag::Span {
    let start = source
        .match_indices(text)
        .nth(nth)
        .expect("text in source")
        .0;
    diag::Span {
        start,
        end: start + text.len(),
    }
}

/// Returns a closure giving the span of the next occurrence of its argument,
/// so expected spans are written in source order.
pub(crate) fn spans(source: &str) -> impl FnMut(&str) -> diag::Span + '_ {
    let mut offset = 0;
    move |text| {
        let start = source[offset..].find(text).expect("text in source") + offset;
        offset = start + text.len();
        diag::Span { start, end: offset }
    }
}

pub(crate) fn name(text: &str, span: diag::Span) -> ast::Name {
    ast::Name {
        text: text.to_string(),
        span,
    }
}

/// The AST of `source`, for a test whose stage starts at one.
///
/// Asserts that lexing and parsing succeeded without diagnostics.
pub(crate) fn lex_parse(source: &str) -> ast::Program {
    let mut diags = diag::Sink::default();
    let tokens = lexer::lex(source, &mut diags).expect("lexing succeeded");
    let parsed = parser::parse(&tokens, &mut diags).expect("parsing succeeded");
    let program = ast::from_cst(source, &parsed, &mut diags).expect("the AST was built");
    assert!(diags.is_empty());
    program
}

/// The typed AST of `source`, for a test whose stage starts at one.
///
/// Asserts that checking, too, succeeded without diagnostics.
pub(crate) fn lex_parse_check(source: &str) -> typed_ast::Program {
    let mut diags = diag::Sink::default();
    let program = typeck::check(&lex_parse(source), &mut diags).expect("checking succeeded");
    assert!(diags.is_empty());
    program
}
