//! Tests for the driver.

use super::pipeline::compile;
use super::render::render;
use crate::diag;
use crate::testing;

/// The diagnostics reported while compiling `source`, asserting that
/// nothing was emitted.
fn compile_err(source: &str) -> Vec<diag::Kind> {
    let mut diags = diag::Sink::default();
    assert_eq!(compile(source, &mut diags), None);
    diags
        .iter()
        .map(|diagnostic| diagnostic.kind.clone())
        .collect()
}

/// The example program of the parameters milestone.
const PARAMETERS: &str = "func add(a uint64, b uint64) uint64 {\n\tvar sum uint64 = a + b\n\t\
                          return sum\n}\n\nfunc approval() uint64 {\n\treturn 1\n}\n";

/// A diagnostic covering `span`, for [`render`] to format.
fn diagnostic(span: diag::Span) -> diag::Entry {
    diag::Entry {
        kind: diag::Kind::MissingEntryPoint { name: "approval" },
        span,
    }
}

#[test]
fn example_program_compiles() {
    let mut diags = diag::Sink::default();
    assert_eq!(
        compile(testing::EXAMPLE, &mut diags),
        Some(
            "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 1\nretsub\n"
                .to_string()
        )
    );
    assert!(diags.is_empty());
}

#[test]
fn a_function_with_parameters_compiles() {
    // `add` is dead code: nothing can call it yet.
    let mut diags = diag::Sink::default();
    assert_eq!(
        compile(PARAMETERS, &mut diags),
        Some(
            "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 1\nretsub\n"
                .to_string()
        )
    );
    assert!(diags.is_empty());
}

#[test]
fn lexing_stops_the_pipeline() {
    assert_eq!(
        compile_err("func approval() uint64 { return @ }"),
        [diag::Kind::UnexpectedCharacter]
    );
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

#[test]
fn comments_do_not_change_the_output() {
    let commented = "// The approval program.\nfunc approval() uint64 {\n  var x uint64 = 1 + 2 // one more than two\n  return x\n}\n";
    let bare = "func approval() uint64 {\n  var x uint64 = 1 + 2\n  return x\n}\n";

    let mut diags = diag::Sink::default();
    let expected = compile(bare, &mut diags);
    assert_eq!(compile(commented, &mut diags), expected);
    assert!(expected.is_some());
    assert!(diags.is_empty());
}
