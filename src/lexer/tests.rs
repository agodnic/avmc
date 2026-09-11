//! Tests for the lexer.

use super::pass;
use super::token::{Token, TokenKind};
use crate::diag;

/// The approval program of the v0 milestone, with comments.
const COMMENTED_APPROVAL: &str = "// The approval program.\nfunc approval() uint64 {\n  var x uint64 = 1 + 2 // one more than two\n  return x\n}\n";

/// Lexes `source`, asserting that it produced no diagnostics.
fn lex_ok(source: &str) -> Vec<Token> {
    let mut diags = diag::Sink::default();
    let tokens = pass::lex(source, &mut diags);
    assert!(diags.iter().next().is_none());
    tokens.expect("lexing succeeded")
}

/// Lexes `source`, asserting that it produced no tokens, and returns the
/// diagnostics it reported.
fn lex_err(source: &str) -> Vec<diag::Entry> {
    let mut diags = diag::Sink::default();
    assert_eq!(pass::lex(source, &mut diags), None);
    diags.iter().cloned().collect()
}

/// The tokens for `kinds`, each named by the text it covers, in source
/// order. Each token's trivia is the gap before it, and the stream ends
/// with `Eof`, as `lex` produces them.
fn spans(source: &str, kinds: &[(TokenKind, &str)]) -> Vec<Token> {
    let mut offset = 0;
    let mut expected = Vec::new();
    for &(kind, text) in kinds {
        let start = source[offset..]
            .find(text)
            .expect("expected token text in source")
            + offset;
        let end = start + text.len();
        expected.push(Token {
            kind,
            span: diag::Span { start, end },
            trivia: diag::Span {
                start: offset,
                end: start,
            },
        });
        offset = end;
    }

    expected.push(Token {
        kind: TokenKind::Eof,
        span: diag::Span {
            start: source.len(),
            end: source.len(),
        },
        trivia: diag::Span {
            start: offset,
            end: source.len(),
        },
    });
    expected
}

/// The source text a token covers, trivia included.
fn text(source: &str, span: diag::Span) -> &str {
    &source[span.start..span.end]
}

/// Reassembles `source` from the tokens `lex` produced for it.
fn reassemble(source: &str) -> String {
    lex_ok(source)
        .iter()
        .map(|token| format!("{}{}", text(source, token.trivia), text(source, token.span)))
        .collect()
}

#[test]
fn lexes_the_approval_program() {
    let source = "func approval() uint64 {\n  return 1\n}\n";
    let expected = spans(
        source,
        &[
            (TokenKind::Func, "func"),
            (TokenKind::Ident, "approval"),
            (TokenKind::LParen, "("),
            (TokenKind::RParen, ")"),
            (TokenKind::Ident, "uint64"),
            (TokenKind::LBrace, "{"),
            (TokenKind::Return, "return"),
            (TokenKind::IntLit, "1"),
            (TokenKind::RBrace, "}"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn whitespace_only_shifts_spans() {
    let source = "func approval ( ) uint64 { return 1 }";
    let expected = spans(
        source,
        &[
            (TokenKind::Func, "func"),
            (TokenKind::Ident, "approval"),
            (TokenKind::LParen, "("),
            (TokenKind::RParen, ")"),
            (TokenKind::Ident, "uint64"),
            (TokenKind::LBrace, "{"),
            (TokenKind::Return, "return"),
            (TokenKind::IntLit, "1"),
            (TokenKind::RBrace, "}"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn empty_input_produces_only_the_end_of_input_token() {
    assert_eq!(lex_ok(""), spans("", &[]));
}

#[test]
fn keyword_prefixes_are_identifiers() {
    let source = "func_ returns";
    let expected = spans(
        source,
        &[(TokenKind::Ident, "func_"), (TokenKind::Ident, "returns")],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn adjacent_integers_stay_separate() {
    let source = "1 2";
    let expected = spans(
        source,
        &[(TokenKind::IntLit, "1"), (TokenKind::IntLit, "2")],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn lexes_an_arithmetic_program() {
    let source = "func approval() uint64 {\n  return (1 + 2) * 3 - 4 / 5\n}\n";
    let expected = spans(
        source,
        &[
            (TokenKind::Func, "func"),
            (TokenKind::Ident, "approval"),
            (TokenKind::LParen, "("),
            (TokenKind::RParen, ")"),
            (TokenKind::Ident, "uint64"),
            (TokenKind::LBrace, "{"),
            (TokenKind::Return, "return"),
            (TokenKind::LParen, "("),
            (TokenKind::IntLit, "1"),
            (TokenKind::Plus, "+"),
            (TokenKind::IntLit, "2"),
            (TokenKind::RParen, ")"),
            (TokenKind::Star, "*"),
            (TokenKind::IntLit, "3"),
            (TokenKind::Minus, "-"),
            (TokenKind::IntLit, "4"),
            (TokenKind::Slash, "/"),
            (TokenKind::IntLit, "5"),
            (TokenKind::RBrace, "}"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn operators_need_no_surrounding_whitespace() {
    let source = "1+2*3-4/5";
    let expected = spans(
        source,
        &[
            (TokenKind::IntLit, "1"),
            (TokenKind::Plus, "+"),
            (TokenKind::IntLit, "2"),
            (TokenKind::Star, "*"),
            (TokenKind::IntLit, "3"),
            (TokenKind::Minus, "-"),
            (TokenKind::IntLit, "4"),
            (TokenKind::Slash, "/"),
            (TokenKind::IntLit, "5"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn a_minus_is_never_part_of_a_literal() {
    let source = "-1";
    let expected = spans(source, &[(TokenKind::Minus, "-"), (TokenKind::IntLit, "1")]);
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn repeated_operators_are_separate_tokens() {
    let source = "--";
    let expected = spans(source, &[(TokenKind::Minus, "-"), (TokenKind::Minus, "-")]);
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn a_percent_is_a_token() {
    let source = "%";
    assert_eq!(lex_ok(source), spans(source, &[(TokenKind::Percent, "%")]));

    let source = "1%2";
    let expected = spans(
        source,
        &[
            (TokenKind::IntLit, "1"),
            (TokenKind::Percent, "%"),
            (TokenKind::IntLit, "2"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn lexes_a_variable_declaration() {
    let source = "var x uint64 = 1";
    let expected = spans(
        source,
        &[
            (TokenKind::Var, "var"),
            (TokenKind::Ident, "x"),
            (TokenKind::Ident, "uint64"),
            (TokenKind::Equals, "="),
            (TokenKind::IntLit, "1"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn var_prefixes_are_identifiers() {
    let source = "var_ variable";
    let expected = spans(
        source,
        &[(TokenKind::Ident, "var_"), (TokenKind::Ident, "variable")],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn lexes_the_boolean_literals() {
    let source = "return true";
    let expected = spans(
        source,
        &[(TokenKind::Return, "return"), (TokenKind::True, "true")],
    );
    assert_eq!(lex_ok(source), expected);

    let source = "var ok bool = false";
    let expected = spans(
        source,
        &[
            (TokenKind::Var, "var"),
            (TokenKind::Ident, "ok"),
            (TokenKind::Ident, "bool"),
            (TokenKind::Equals, "="),
            (TokenKind::False, "false"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn only_the_whole_lowercase_keyword_is_a_boolean_literal() {
    let source = "truest false_ True";
    let expected = spans(
        source,
        &[
            (TokenKind::Ident, "truest"),
            (TokenKind::Ident, "false_"),
            (TokenKind::Ident, "True"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn an_equals_needs_no_surrounding_whitespace() {
    let source = "x=1";
    let expected = spans(
        source,
        &[
            (TokenKind::Ident, "x"),
            (TokenKind::Equals, "="),
            (TokenKind::IntLit, "1"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn lexes_the_comparison_and_logic_program() {
    let source = "func approval() bool {\n  var x uint64 = 1 + 2\n  return x >= 3 && !(x == 4) || x != 5\n}\n";
    let expected = spans(
        source,
        &[
            (TokenKind::Func, "func"),
            (TokenKind::Ident, "approval"),
            (TokenKind::LParen, "("),
            (TokenKind::RParen, ")"),
            (TokenKind::Ident, "bool"),
            (TokenKind::LBrace, "{"),
            (TokenKind::Var, "var"),
            (TokenKind::Ident, "x"),
            (TokenKind::Ident, "uint64"),
            (TokenKind::Equals, "="),
            (TokenKind::IntLit, "1"),
            (TokenKind::Plus, "+"),
            (TokenKind::IntLit, "2"),
            (TokenKind::Return, "return"),
            (TokenKind::Ident, "x"),
            (TokenKind::GtEq, ">="),
            (TokenKind::IntLit, "3"),
            (TokenKind::AmpAmp, "&&"),
            (TokenKind::Bang, "!"),
            (TokenKind::LParen, "("),
            (TokenKind::Ident, "x"),
            (TokenKind::EqEq, "=="),
            (TokenKind::IntLit, "4"),
            (TokenKind::RParen, ")"),
            (TokenKind::PipePipe, "||"),
            (TokenKind::Ident, "x"),
            (TokenKind::BangEq, "!="),
            (TokenKind::IntLit, "5"),
            (TokenKind::RBrace, "}"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn comparisons_need_no_surrounding_whitespace() {
    let source = "1==2!=3<4<=5>6>=7";
    let expected = spans(
        source,
        &[
            (TokenKind::IntLit, "1"),
            (TokenKind::EqEq, "=="),
            (TokenKind::IntLit, "2"),
            (TokenKind::BangEq, "!="),
            (TokenKind::IntLit, "3"),
            (TokenKind::Lt, "<"),
            (TokenKind::IntLit, "4"),
            (TokenKind::LtEq, "<="),
            (TokenKind::IntLit, "5"),
            (TokenKind::Gt, ">"),
            (TokenKind::IntLit, "6"),
            (TokenKind::GtEq, ">="),
            (TokenKind::IntLit, "7"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn logical_operators_need_no_surrounding_whitespace() {
    let source = "a&&b||!c";
    let expected = spans(
        source,
        &[
            (TokenKind::Ident, "a"),
            (TokenKind::AmpAmp, "&&"),
            (TokenKind::Ident, "b"),
            (TokenKind::PipePipe, "||"),
            (TokenKind::Bang, "!"),
            (TokenKind::Ident, "c"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn two_character_tokens_are_matched_greedily() {
    let source = "===";
    let expected = spans(source, &[(TokenKind::EqEq, "=="), (TokenKind::Equals, "=")]);
    assert_eq!(lex_ok(source), expected);

    let source = "!!";
    let expected = spans(source, &[(TokenKind::Bang, "!"), (TokenKind::Bang, "!")]);
    assert_eq!(lex_ok(source), expected);

    let source = "<==";
    let expected = spans(source, &[(TokenKind::LtEq, "<="), (TokenKind::Equals, "=")]);
    assert_eq!(lex_ok(source), expected);

    let source = "=!";
    let expected = spans(source, &[(TokenKind::Equals, "="), (TokenKind::Bang, "!")]);
    assert_eq!(lex_ok(source), expected);

    let source = "<>";
    let expected = spans(source, &[(TokenKind::Lt, "<"), (TokenKind::Gt, ">")]);
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn whitespace_breaks_a_two_character_token() {
    let source = "= =";
    let expected = spans(
        source,
        &[(TokenKind::Equals, "="), (TokenKind::Equals, "=")],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn a_lone_ampersand_or_pipe_is_reported() {
    assert_eq!(lex_err("1 & 2"), vec![pass::unexpected_character(2, 3)]);
    assert_eq!(lex_err("1 | 2"), vec![pass::unexpected_character(2, 3)]);
}

#[test]
fn an_unpaired_ampersand_beside_a_pair_is_reported() {
    assert_eq!(lex_err("&&&"), vec![pass::unexpected_character(2, 3)]);
    assert_eq!(lex_err("&="), vec![pass::unexpected_character(0, 1)]);
}

#[test]
fn lexing_stops_at_the_first_unexpected_character() {
    assert_eq!(lex_err("@é"), vec![pass::unexpected_character(0, 1)]);
}

#[test]
fn tokens_after_the_first_error_are_not_lexed() {
    assert_eq!(lex_err("1 & 2 |"), vec![pass::unexpected_character(2, 3)]);
}

#[test]
fn a_line_comment_is_trivia() {
    let source = "return 1 // one\n";
    let expected = spans(
        source,
        &[(TokenKind::Return, "return"), (TokenKind::IntLit, "1")],
    );
    assert_eq!(lex_ok(source), expected);
    assert_eq!(text(source, expected[2].trivia), " // one\n");
}

#[test]
fn a_comment_on_its_own_line_is_trivia() {
    let source = "// hi\nreturn";
    let expected = spans(source, &[(TokenKind::Return, "return")]);
    assert_eq!(lex_ok(source), expected);
    assert_eq!(text(source, expected[0].trivia), "// hi\n");
}

#[test]
fn a_comment_is_not_lexed() {
    let source = "// @é\n";
    assert_eq!(lex_ok(source), spans(source, &[]));
}

#[test]
fn a_comment_at_the_end_of_input_needs_no_newline() {
    let source = "return // hi";
    assert_eq!(
        lex_ok(source),
        spans(source, &[(TokenKind::Return, "return")])
    );
}

#[test]
fn a_lone_slash_is_division() {
    let source = "1/2";
    let expected = spans(
        source,
        &[
            (TokenKind::IntLit, "1"),
            (TokenKind::Slash, "/"),
            (TokenKind::IntLit, "2"),
        ],
    );
    assert_eq!(lex_ok(source), expected);

    let source = "1 / 2";
    let expected = spans(
        source,
        &[
            (TokenKind::IntLit, "1"),
            (TokenKind::Slash, "/"),
            (TokenKind::IntLit, "2"),
        ],
    );
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn a_third_slash_is_part_of_the_comment() {
    let source = "/// x";
    assert_eq!(lex_ok(source), spans(source, &[]));
}

#[test]
fn two_separated_slashes_are_two_tokens() {
    let source = "/ /";
    let expected = spans(source, &[(TokenKind::Slash, "/"), (TokenKind::Slash, "/")]);
    assert_eq!(lex_ok(source), expected);
}

#[test]
fn tokens_are_lossless() {
    let sources = [
        "",
        "return 1 // one\n",
        "// hi\nreturn",
        "// @é\n",
        "return // hi",
        "1/2",
        "/ /",
        "/// x",
        "func approval() uint64 {\n  return 1\n}\n",
        COMMENTED_APPROVAL,
    ];
    for source in sources {
        assert_eq!(reassemble(source), source);
    }
}
