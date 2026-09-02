//! The driver: chains the stages and renders diagnostics for display.
//!
//! Both functions are pure (ARCHITECTURE.md §2.4); all I/O lives in
//! `src/main.rs`.

use crate::diagnostics::{Diagnostic, Diagnostics};
use crate::emit::{TealVersion, emit};
use crate::lexer::lex;
use crate::lower::lower;
use crate::parser::parse;
use crate::typeck::check;

/// Compiles `source` to TEAL text, stopping at the first stage that fails.
pub fn compile(source: &str, version: TealVersion, diags: &mut Diagnostics) -> Option<String> {
    let tokens = lex(source, diags)?;
    let parsed = parse(source, &tokens, diags)?;
    let checked = check(&parsed, diags)?;
    let ir = lower(&checked, diags)?;
    emit(&ir, version, diags)
}

/// Renders `diagnostic` as one line, without a trailing newline.
pub fn render(diagnostic: &Diagnostic, file_name: &str, source: &str) -> String {
    let (line, column) = position(source, diagnostic.span.start);
    let code = diagnostic.code;
    let message = &diagnostic.message;
    format!("{file_name}:{line}:{column}: error[{code}]: {message}")
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
    use crate::diagnostics::Span;

    /// The example program of the v0 milestone.
    const EXAMPLE: &str = "func approval() uint64 { return 1 }";

    /// The diagnostic codes reported while compiling `source` for `version`,
    /// asserting that nothing was emitted.
    fn compile_err(source: &str, version: u8) -> Vec<&'static str> {
        let mut diags = Diagnostics::default();
        assert_eq!(compile(source, TealVersion(version), &mut diags), None);
        diags.iter().map(|diag| diag.code).collect()
    }

    /// A diagnostic covering `span`, for [`render`] to format.
    fn diagnostic(span: Span) -> Diagnostic {
        Diagnostic {
            code: "E0008",
            message: "missing entry point `approval`".to_string(),
            span,
        }
    }

    /// The span of the first occurrence of `text` in `source`.
    fn span_of(source: &str, text: &str) -> Span {
        let start = source.find(text).expect("text in source");
        Span {
            start,
            end: start + text.len(),
        }
    }

    #[test]
    fn example_program_compiles() {
        let mut diags = Diagnostics::default();
        assert_eq!(
            compile(EXAMPLE, TealVersion(10), &mut diags),
            Some("#pragma version 10\npushint 1\nreturn\n".to_string())
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn lexing_stops_the_pipeline() {
        assert_eq!(
            compile_err("func approval() uint64 { return @ }", 10),
            ["E0001"]
        );
    }

    #[test]
    fn emission_reports_an_unsupported_version() {
        assert_eq!(compile_err(EXAMPLE, 2), ["E0009"]);
    }

    #[test]
    fn renders_the_start_of_an_empty_source() {
        assert_eq!(
            render(&diagnostic(Span { start: 0, end: 0 }), "a.txt", ""),
            "a.txt:1:1: error[E0008]: missing entry point `approval`"
        );
    }

    #[test]
    fn renders_a_position_on_a_later_line() {
        let source = "func f() {\n  return @\n}";
        assert_eq!(
            render(&diagnostic(span_of(source, "@")), "a.txt", source),
            "a.txt:2:10: error[E0008]: missing entry point `approval`"
        );
    }

    #[test]
    fn counts_columns_in_chars_not_bytes() {
        let source = "é@";
        assert_eq!(
            render(&diagnostic(span_of(source, "@")), "a.txt", source),
            "a.txt:1:2: error[E0008]: missing entry point `approval`"
        );
    }

    #[test]
    fn renders_the_end_of_input() {
        let source = "ab\n";
        let span = Span {
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
        let span = Span { start: 1, end: 1 };
        assert_eq!(
            render(&diagnostic(span), "a.txt", source),
            "a.txt:2:1: error[E0008]: missing entry point `approval`"
        );
    }
}
