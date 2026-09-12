//! Tests for rendering a diagnostic for display.

use super::render::render;
use crate::diag;
use crate::testing;

/// A diagnostic covering `span`, for [`render`] to format.
fn diagnostic(span: diag::Span) -> diag::Entry {
    diag::Entry {
        kind: diag::Kind::MissingEntryPoint { name: "approval" },
        span,
    }
}

#[test]
fn renders_the_start_of_an_empty_source() {
    assert_eq!(
        render(&diagnostic(diag::Span { start: 0, end: 0 }), "a.txt", ""),
        "a.txt:1:1: error[E0008]: missing entry point `approval`"
    );
}

#[test]
fn renders_a_position_on_a_later_line() {
    let source = "func f() {\n  return @\n}";
    assert_eq!(
        render(
            &diagnostic(testing::span_of(source, "@", 0)),
            "a.txt",
            source
        ),
        "a.txt:2:10: error[E0008]: missing entry point `approval`"
    );
}

#[test]
fn counts_columns_in_chars_not_bytes() {
    let source = "é@";
    assert_eq!(
        render(
            &diagnostic(testing::span_of(source, "@", 0)),
            "a.txt",
            source
        ),
        "a.txt:1:2: error[E0008]: missing entry point `approval`"
    );
}

#[test]
fn renders_the_end_of_input() {
    let source = "ab\n";
    let span = diag::Span {
        start: source.len(),
        end: source.len(),
    };
    assert_eq!(
        render(&diagnostic(span), "a.txt", source),
        "a.txt:2:1: error[E0008]: missing entry point `approval`"
    );
}

#[test]
fn renders_a_non_boundary_offset_as_the_end_of_input() {
    let source = "é\n";
    let span = diag::Span { start: 1, end: 1 };
    assert_eq!(
        render(&diagnostic(span), "a.txt", source),
        "a.txt:2:1: error[E0008]: missing entry point `approval`"
    );
}
