//! Tests for the formatter.

use super::pass::format;
use crate::diag;
use crate::driver;
use crate::lexer;

/// The canonical form of the messy program below.
const CANONICAL: &str = "func approval() uint64 {\n\
                         \t// one more than two\n\
                         \tvar x uint64 = (1 + 2) // trailing\n\
                         \treturn x * 3\n\
                         }\n\
                         \n\
                         func f() bool {\n\
                         \treturn !(true)\n\
                         }\n";

/// The same program, written with every whitespace decision made wrongly.
const MESSY: &str = "func approval ( ) uint64 {\n\
                     \n\
                     \x20 // one more than two\n\
                     \x20 var x uint64 = ( 1+2 )    // trailing\n\
                     \x20 return x*3\n\
                     \n\
                     \n\
                     }\n\
                     func f() bool { return !(true) }\n";

const EMPTY_BODY: &str = "func f() uint64 {}";

/// Two functions with nothing between them.
const UNSEPARATED: &str = "func a() uint64 { return 1 }\n\
                           func approval() uint64 { return 2 }\n";

/// The same two, with three blank lines between them.
const OVERSEPARATED: &str = "func a() uint64 { return 1 }\n\
                             \n\
                             \n\
                             \n\
                             func approval() uint64 { return 2 }\n";

/// Blank lines after `{`, between the statements, and before `}`.
const SPACED_STATEMENTS: &str = "func approval() uint64 {\n\
                                 \n\
                                 \tvar x uint64 = 1\n\
                                 \n\
                                 \n\
                                 \n\
                                 \treturn x\n\
                                 \n\
                                 }\n";

/// A comment in every position a comment can take.
const COMMENTED: &str = "// The approval program.\n\
                         func approval() uint64 { // entry\n\
                         \t// one more than two\n\
                         \tvar x uint64 = 1 + 2 // trailing\n\
                         \treturn x\n\
                         \t// done\n\
                         }\n\
                         // end\n";

/// A comment between an operator and its right operand.
const EXPRESSION_COMMENT: &str = "func approval() uint64 {\n\
                                  \treturn 1 + // why\n\
                                  \t2\n\
                                  }\n";

const ONLY_COMMENTS: &str = "// a\n\n// b\n";

/// A comment after the last function, a blank line away from it.
const TRAILING_COMMENT: &str = "func approval() uint64 {\n\
                                \treturn 1\n\
                                }\n\
                                \n\
                                // end\n";

/// The example program of the parameters milestone.
const PARAMETERS: &str = "func add(a uint64, b uint64) uint64 {\n\tvar sum uint64 = a + b\n\t\
                          return sum\n}\n\nfunc approval() uint64 {\n\treturn 1\n}\n";

/// Every golden input, for the properties to run over.
const GOLDEN: &[&str] = &[
    CANONICAL,
    MESSY,
    EMPTY_BODY,
    UNSEPARATED,
    OVERSEPARATED,
    SPACED_STATEMENTS,
    COMMENTED,
    EXPRESSION_COMMENT,
    ONLY_COMMENTS,
    TRAILING_COMMENT,
    PARAMETERS,
    "",
    "//x\n",
    "//   spaced   \n",
];

/// The program of every end-to-end test, less the one that does not lex,
/// which the formatter reports rather than formats.
const PROGRAMS: &[&str] = &[
    "func approval() uint64 { return 1 }",
    "func approval() uint64 {\n  return (1 + 2) * 3 - 4 / 5\n}\n",
    "func approval() uint64 {\n  var x uint64 = 1 + 2\n  var y uint64 = x * 3\n  return y - x\n}\n",
    "func approval() bool {\n  var ok bool = true\n  return ok\n}\n",
    "func approval() uint64 {\n  return true + 1\n}\n",
    "func approval() uint64 {\n  return x\n}\n",
    "func approval() bool {\n  return 1\n}\n",
    "func f() uint64 { return 1 }",
    "func approval() bool {\n  var x uint64 = 1 + 2\n  var big bool = x * 2 >= 6\n  return big == (x != 4)\n}\n",
    "func approval() bool {\n  var x uint64 = 1 + 2\n  var odd bool = x % 2 == 1\n  return !(x > 5) && (odd || x == 4)\n}\n",
];

/// Formats `source`, asserting that it reported nothing.
fn format_ok(source: &str) -> String {
    let mut diags = diag::Sink::default();
    let formatted = format(source, &mut diags);
    assert!(diags.is_empty());
    formatted.expect("formatting succeeded")
}

/// The diagnostics reported while formatting `source`, asserting that it
/// produced no output.
fn format_err(source: &str) -> Vec<diag::Entry> {
    let mut diags = diag::Sink::default();
    assert_eq!(format(source, &mut diags), None);
    diags.iter().cloned().collect()
}

/// Every source the properties run over.
fn corpus() -> impl Iterator<Item = &'static str> {
    GOLDEN.iter().chain(PROGRAMS).copied()
}

#[test]
fn a_canonical_program_is_unchanged() {
    assert_eq!(format_ok(CANONICAL), CANONICAL);
}

#[test]
fn whitespace_is_normalised() {
    assert_eq!(format_ok(MESSY), CANONICAL);
}

#[test]
fn parameters_are_formatted() {
    let messy = "func add( a uint64 ,b uint64 )uint64 {\n\treturn a + b\n}\n";
    let canonical = "func add(a uint64, b uint64) uint64 {\n\treturn a + b\n}\n";
    assert_eq!(format_ok(messy), canonical);
    assert_eq!(format_ok(canonical), canonical);
}

#[test]
fn an_empty_body_prints_on_two_lines() {
    assert_eq!(format_ok(EMPTY_BODY), "func f() uint64 {\n}\n");
}

#[test]
fn functions_are_separated_by_one_blank_line() {
    let expected = "func a() uint64 {\n\
                    \treturn 1\n\
                    }\n\
                    \n\
                    func approval() uint64 {\n\
                    \treturn 2\n\
                    }\n";
    assert_eq!(format_ok(UNSEPARATED), expected);
    assert_eq!(format_ok(OVERSEPARATED), expected);
}

#[test]
fn blank_lines_between_statements_collapse_to_one() {
    let expected = "func approval() uint64 {\n\
                    \tvar x uint64 = 1\n\
                    \n\
                    \treturn x\n\
                    }\n";
    assert_eq!(format_ok(SPACED_STATEMENTS), expected);
}

#[test]
fn comments_are_kept_where_they_were() {
    assert_eq!(format_ok(COMMENTED), COMMENTED);
}

#[test]
fn a_comment_inside_an_expression_breaks_the_line() {
    assert_eq!(format_ok(EXPRESSION_COMMENT), EXPRESSION_COMMENT);
}

#[test]
fn a_comment_keeps_its_text() {
    assert_eq!(format_ok("//x\n"), "//x\n");
    assert_eq!(format_ok("//   spaced   \n"), "//   spaced\n");
}

#[test]
fn a_file_of_only_comments_formats() {
    assert_eq!(format_ok(ONLY_COMMENTS), ONLY_COMMENTS);
}

#[test]
fn an_empty_source_formats_to_nothing() {
    assert_eq!(format_ok(""), "");
}

#[test]
fn a_blank_line_before_a_trailing_comment_is_kept() {
    assert_eq!(format_ok(TRAILING_COMMENT), TRAILING_COMMENT);
}

#[test]
fn formatting_is_idempotent() {
    for source in corpus() {
        let once = format_ok(source);
        assert_eq!(format_ok(&once), once, "{source:?}");
    }
}

#[test]
fn formatting_preserves_the_tokens() {
    /// The kind of every token of `source`, `Eof` included.
    fn kinds(source: &str) -> Vec<lexer::TokenKind> {
        let mut diags = diag::Sink::default();
        let tokens = lexer::lex(source, &mut diags).expect("lexing succeeded");
        tokens.iter().map(|token| token.kind).collect()
    }

    for source in corpus() {
        assert_eq!(kinds(&format_ok(source)), kinds(source), "{source:?}");
    }
}

#[test]
fn formatting_preserves_every_comment() {
    for source in corpus() {
        let comments = source.matches("//").count();
        assert_eq!(
            format_ok(source).matches("//").count(),
            comments,
            "{source:?}"
        );
    }
}

#[test]
fn formatting_preserves_the_program() {
    let mut compiled = 0;
    for source in corpus() {
        let mut diags = diag::Sink::default();
        let Some(teal) = driver::compile(source, &mut diags) else {
            continue;
        };

        let mut diags = diag::Sink::default();
        let formatted = driver::compile(&format_ok(source), &mut diags);
        assert_eq!(formatted, Some(teal), "{source:?}");
        compiled += 1;
    }
    assert!(compiled > 0, "no source in the corpus compiles");
}

#[test]
fn a_program_that_does_not_parse_is_reported() {
    let source = "func f() uint64 { return }";
    assert_eq!(
        format_err(source),
        [diag::Entry {
            kind: diag::Kind::UnexpectedToken {
                expected: "a literal, an identifier, `!`, or `(`",
                found: "`}`",
            },
            span: diag::Span { start: 25, end: 26 },
        }]
    );
}

#[test]
fn a_program_that_does_not_type_check_still_formats() {
    assert_eq!(
        format_ok("func f() uint64 { return true }"),
        "func f() uint64 {\n\treturn true\n}\n"
    );
}

#[test]
fn an_integer_literal_out_of_range_still_formats() {
    assert_eq!(
        format_ok("func f() uint64 { return 18446744073709551616 }"),
        "func f() uint64 {\n\treturn 18446744073709551616\n}\n"
    );
}
