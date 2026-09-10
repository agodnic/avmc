//! The driver: chains the stages and renders diagnostics for display.
//!
//! Both functions are pure; all I/O lives in `src/bin`.

use crate::ast;
use crate::diag;
use crate::emit;
use crate::lower;
use crate::parser;
use crate::token;
use crate::typeck;

/// Compiles `source` to TEAL text, stopping at the first stage that fails.
pub fn compile(source: &str, diags: &mut diag::Sink) -> Option<String> {
    let tokens = token::lex(source, diags)?;
    let parsed = parser::parse(&tokens, diags)?;
    let program = ast::from_cst(source, &parsed, diags)?;
    let checked = typeck::check(&program, diags)?;
    let ir = lower::lower(&checked, diags)?;
    emit::emit(&ir, diags)
}

/// Renders `diagnostic` as one line, without a trailing newline.
pub fn render(diagnostic: &diag::Entry, file_name: &str, source: &str) -> String {
    let (line, column) = position(source, diagnostic.span.start);
    let code = diagnostic.kind.code();
    let severity = code.severity;
    let message = &diagnostic.kind;
    format!("{file_name}:{line}:{column}: {severity}[{code}]: {message}")
}

/// The 1-based line and column of `offset` in `source`.
///
/// An offset that is not a char boundary of `source` — a compiler bug — is
/// reported as the end of input rather than panicking.
fn position(source: &str, offset: usize) -> (usize, usize) {
    let before = if source.is_char_boundary(offset) {
        source.get(..offset).unwrap_or(source)
    } else {
        source
    };

    let line = 1 + before.matches('\n').count();
    // Everything after the last newline, or all of `before` if there is none.
    let last_line = before.rsplit('\n').next().unwrap_or(before);
    (line, 1 + last_line.chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
