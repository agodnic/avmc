//! The lexer: source text to a flat token stream.

use crate::diagnostics::{Diagnostic, DiagnosticKind, Diagnostics, Span};
use std::iter::Peekable;
use std::str::CharIndices;

/// The kind of a lexical token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// The keyword `func`.
    Func,
    /// The keyword `return`.
    Return,
    /// The keyword `var`.
    Var,
    /// The keyword `true`.
    True,
    /// The keyword `false`.
    False,
    /// An identifier: `[A-Za-z_][A-Za-z0-9_]*`, keywords excluded.
    Ident,
    /// An integer literal: `[0-9]+`. The value is not parsed here.
    IntLit,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `=`
    Equals,
    /// `==`
    EqEq,
    /// `!=`
    BangEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
    /// `!`
    Bang,
    /// `&&`
    AmpAmp,
    /// `||`
    PipePipe,
}

/// A token: a kind and the source range it covers.
///
/// Tokens carry no text; later stages slice the source with the span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    /// What was matched.
    pub kind: TokenKind,
    /// Where it was matched.
    pub span: Span,
}

/// Tokenises `source`.
///
/// Stops at the first error and returns `None`: where a token ends after
/// an error is not known, so nothing lexed past it is trustworthy.
pub fn lex(source: &str, diags: &mut Diagnostics) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = source.char_indices().peekable();

    while let Some((start, c)) = chars.next() {
        // One past the character just consumed; extended below for the tokens
        // that span more than one character.
        let single = start + c.len_utf8();

        match c {
            ' ' | '\t' | '\n' | '\r' => {}
            '(' => tokens.push(token(TokenKind::LParen, start, single)),
            ')' => tokens.push(token(TokenKind::RParen, start, single)),
            '{' => tokens.push(token(TokenKind::LBrace, start, single)),
            '}' => tokens.push(token(TokenKind::RBrace, start, single)),
            '+' => tokens.push(token(TokenKind::Plus, start, single)),
            '-' => tokens.push(token(TokenKind::Minus, start, single)),
            '*' => tokens.push(token(TokenKind::Star, start, single)),
            '/' => tokens.push(token(TokenKind::Slash, start, single)),
            '%' => tokens.push(token(TokenKind::Percent, start, single)),
            '=' => match consume_if(&mut chars, '=') {
                Some(end) => tokens.push(token(TokenKind::EqEq, start, end)),
                None => tokens.push(token(TokenKind::Equals, start, single)),
            },
            '!' => match consume_if(&mut chars, '=') {
                Some(end) => tokens.push(token(TokenKind::BangEq, start, end)),
                None => tokens.push(token(TokenKind::Bang, start, single)),
            },
            '<' => match consume_if(&mut chars, '=') {
                Some(end) => tokens.push(token(TokenKind::LtEq, start, end)),
                None => tokens.push(token(TokenKind::Lt, start, single)),
            },
            '>' => match consume_if(&mut chars, '=') {
                Some(end) => tokens.push(token(TokenKind::GtEq, start, end)),
                None => tokens.push(token(TokenKind::Gt, start, single)),
            },
            // `&` and `|` are tokens only in pairs.
            '&' => match consume_if(&mut chars, '&') {
                Some(end) => tokens.push(token(TokenKind::AmpAmp, start, end)),
                None => {
                    diags.push(unexpected_character(start, single));
                    return None;
                }
            },
            '|' => match consume_if(&mut chars, '|') {
                Some(end) => tokens.push(token(TokenKind::PipePipe, start, end)),
                None => {
                    diags.push(unexpected_character(start, single));
                    return None;
                }
            },
            _ if is_ident_start(c) => {
                let end = consume_while(&mut chars, source.len(), is_ident_continue);
                let kind = match source.get(start..end) {
                    Some("func") => TokenKind::Func,
                    Some("return") => TokenKind::Return,
                    Some("var") => TokenKind::Var,
                    Some("true") => TokenKind::True,
                    Some("false") => TokenKind::False,
                    _ => TokenKind::Ident,
                };
                tokens.push(token(kind, start, end));
            }
            _ if c.is_ascii_digit() => {
                let end = consume_while(&mut chars, source.len(), |c| c.is_ascii_digit());
                tokens.push(token(TokenKind::IntLit, start, end));
            }
            _ => {
                diags.push(unexpected_character(start, single));
                return None;
            }
        }
    }

    Some(tokens)
}

fn token(kind: TokenKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: Span { start, end },
    }
}

fn unexpected_character(start: usize, end: usize) -> Diagnostic {
    Diagnostic {
        kind: DiagnosticKind::UnexpectedCharacter,
        span: Span { start, end },
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Consumes the next character if it is `expected`, returning the byte offset
/// one past it.
fn consume_if(chars: &mut Peekable<CharIndices<'_>>, expected: char) -> Option<usize> {
    let (offset, c) = chars.next_if(|&(_, c)| c == expected)?;
    Some(offset + c.len_utf8())
}

/// Consumes characters while `accept` holds, returning the byte offset one past
/// the last consumed character. `source_len` is the answer when the input ends.
fn consume_while(
    chars: &mut Peekable<CharIndices<'_>>,
    source_len: usize,
    accept: impl Fn(char) -> bool,
) -> usize {
    while chars.peek().is_some_and(|&(_, c)| accept(c)) {
        chars.next();
    }
    chars.peek().map_or(source_len, |&(offset, _)| offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lexes `source`, asserting that it produced no diagnostics.
    fn lex_ok(source: &str) -> Vec<Token> {
        let mut diags = Diagnostics::default();
        let tokens = lex(source, &mut diags);
        assert!(diags.iter().next().is_none());
        tokens.expect("lexing succeeded")
    }

    /// Lexes `source`, asserting that it produced no tokens, and returns the
    /// diagnostics it reported.
    fn lex_err(source: &str) -> Vec<Diagnostic> {
        let mut diags = Diagnostics::default();
        assert_eq!(lex(source, &mut diags), None);
        diags.iter().cloned().collect()
    }

    fn spans(source: &str, kinds: &[(TokenKind, &str)]) -> Vec<Token> {
        let mut offset = 0;
        let mut expected = Vec::new();
        for &(kind, text) in kinds {
            let start = source[offset..]
                .find(text)
                .expect("expected token text in source")
                + offset;
            let end = start + text.len();
            expected.push(token(kind, start, end));
            offset = end;
        }
        expected
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
    fn empty_input_produces_no_tokens() {
        assert_eq!(lex_ok(""), Vec::new());
    }

    #[test]
    fn keyword_prefixes_are_identifiers() {
        assert_eq!(
            lex_ok("func_ returns"),
            vec![
                token(TokenKind::Ident, 0, 5),
                token(TokenKind::Ident, 6, 13)
            ]
        );
    }

    #[test]
    fn adjacent_integers_stay_separate() {
        assert_eq!(
            lex_ok("1 2"),
            vec![
                token(TokenKind::IntLit, 0, 1),
                token(TokenKind::IntLit, 2, 3)
            ]
        );
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
        assert_eq!(
            lex_ok("1+2*3-4/5"),
            vec![
                token(TokenKind::IntLit, 0, 1),
                token(TokenKind::Plus, 1, 2),
                token(TokenKind::IntLit, 2, 3),
                token(TokenKind::Star, 3, 4),
                token(TokenKind::IntLit, 4, 5),
                token(TokenKind::Minus, 5, 6),
                token(TokenKind::IntLit, 6, 7),
                token(TokenKind::Slash, 7, 8),
                token(TokenKind::IntLit, 8, 9),
            ]
        );
    }

    #[test]
    fn a_minus_is_never_part_of_a_literal() {
        assert_eq!(
            lex_ok("-1"),
            vec![
                token(TokenKind::Minus, 0, 1),
                token(TokenKind::IntLit, 1, 2)
            ]
        );
    }

    #[test]
    fn repeated_operators_are_separate_tokens() {
        assert_eq!(
            lex_ok("--"),
            vec![token(TokenKind::Minus, 0, 1), token(TokenKind::Minus, 1, 2)]
        );
        assert_eq!(
            lex_ok("//"),
            vec![token(TokenKind::Slash, 0, 1), token(TokenKind::Slash, 1, 2)]
        );
    }

    #[test]
    fn a_percent_is_a_token() {
        assert_eq!(lex_ok("%"), vec![token(TokenKind::Percent, 0, 1)]);
        assert_eq!(
            lex_ok("1%2"),
            vec![
                token(TokenKind::IntLit, 0, 1),
                token(TokenKind::Percent, 1, 2),
                token(TokenKind::IntLit, 2, 3),
            ]
        );
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
        assert_eq!(
            lex_ok("var_ variable"),
            vec![
                token(TokenKind::Ident, 0, 4),
                token(TokenKind::Ident, 5, 13)
            ]
        );
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
        assert_eq!(
            lex_ok("x=1"),
            vec![
                token(TokenKind::Ident, 0, 1),
                token(TokenKind::Equals, 1, 2),
                token(TokenKind::IntLit, 2, 3),
            ]
        );
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
        assert_eq!(
            lex_ok("1==2!=3<4<=5>6>=7"),
            vec![
                token(TokenKind::IntLit, 0, 1),
                token(TokenKind::EqEq, 1, 3),
                token(TokenKind::IntLit, 3, 4),
                token(TokenKind::BangEq, 4, 6),
                token(TokenKind::IntLit, 6, 7),
                token(TokenKind::Lt, 7, 8),
                token(TokenKind::IntLit, 8, 9),
                token(TokenKind::LtEq, 9, 11),
                token(TokenKind::IntLit, 11, 12),
                token(TokenKind::Gt, 12, 13),
                token(TokenKind::IntLit, 13, 14),
                token(TokenKind::GtEq, 14, 16),
                token(TokenKind::IntLit, 16, 17),
            ]
        );
    }

    #[test]
    fn logical_operators_need_no_surrounding_whitespace() {
        assert_eq!(
            lex_ok("a&&b||!c"),
            vec![
                token(TokenKind::Ident, 0, 1),
                token(TokenKind::AmpAmp, 1, 3),
                token(TokenKind::Ident, 3, 4),
                token(TokenKind::PipePipe, 4, 6),
                token(TokenKind::Bang, 6, 7),
                token(TokenKind::Ident, 7, 8),
            ]
        );
    }

    #[test]
    fn two_character_tokens_are_matched_greedily() {
        assert_eq!(
            lex_ok("==="),
            vec![token(TokenKind::EqEq, 0, 2), token(TokenKind::Equals, 2, 3)]
        );
        assert_eq!(
            lex_ok("!!"),
            vec![token(TokenKind::Bang, 0, 1), token(TokenKind::Bang, 1, 2)]
        );
        assert_eq!(
            lex_ok("<=="),
            vec![token(TokenKind::LtEq, 0, 2), token(TokenKind::Equals, 2, 3)]
        );
        assert_eq!(
            lex_ok("=!"),
            vec![token(TokenKind::Equals, 0, 1), token(TokenKind::Bang, 1, 2)]
        );
        assert_eq!(
            lex_ok("<>"),
            vec![token(TokenKind::Lt, 0, 1), token(TokenKind::Gt, 1, 2)]
        );
    }

    #[test]
    fn whitespace_breaks_a_two_character_token() {
        assert_eq!(
            lex_ok("= ="),
            vec![
                token(TokenKind::Equals, 0, 1),
                token(TokenKind::Equals, 2, 3)
            ]
        );
    }

    #[test]
    fn a_lone_ampersand_or_pipe_is_reported() {
        assert_eq!(lex_err("1 & 2"), vec![unexpected_character(2, 3)]);
        assert_eq!(lex_err("1 | 2"), vec![unexpected_character(2, 3)]);
    }

    #[test]
    fn an_unpaired_ampersand_beside_a_pair_is_reported() {
        assert_eq!(lex_err("&&&"), vec![unexpected_character(2, 3)]);
        assert_eq!(lex_err("&="), vec![unexpected_character(0, 1)]);
    }

    #[test]
    fn lexing_stops_at_the_first_unexpected_character() {
        assert_eq!(lex_err("@é"), vec![unexpected_character(0, 1)]);
    }

    #[test]
    fn tokens_after_the_first_error_are_not_lexed() {
        assert_eq!(lex_err("1 & 2 |"), vec![unexpected_character(2, 3)]);
    }
}
