//! Tokens, and the lexer that produces them.

use crate::diag;
use std::iter::Peekable;
use std::str::CharIndices;

/// The kind of a lexical token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
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
    pub kind: Kind,
    /// Where it was matched.
    pub span: diag::Span,
}

/// Tokenises `source`.
///
/// Stops at the first error and returns `None`: where a token ends after
/// an error is not known, so nothing lexed past it is trustworthy.
pub fn lex(source: &str, diags: &mut diag::Sink) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = source.char_indices().peekable();

    while let Some((start, c)) = chars.next() {
        // One past the character just consumed; extended below for the tokens
        // that span more than one character.
        let single = start + c.len_utf8();

        match c {
            ' ' | '\t' | '\n' | '\r' => {}
            '(' => tokens.push(token(Kind::LParen, start, single)),
            ')' => tokens.push(token(Kind::RParen, start, single)),
            '{' => tokens.push(token(Kind::LBrace, start, single)),
            '}' => tokens.push(token(Kind::RBrace, start, single)),
            '+' => tokens.push(token(Kind::Plus, start, single)),
            '-' => tokens.push(token(Kind::Minus, start, single)),
            '*' => tokens.push(token(Kind::Star, start, single)),
            '/' => tokens.push(token(Kind::Slash, start, single)),
            '%' => tokens.push(token(Kind::Percent, start, single)),
            '=' => match consume_if(&mut chars, '=') {
                Some(end) => tokens.push(token(Kind::EqEq, start, end)),
                None => tokens.push(token(Kind::Equals, start, single)),
            },
            '!' => match consume_if(&mut chars, '=') {
                Some(end) => tokens.push(token(Kind::BangEq, start, end)),
                None => tokens.push(token(Kind::Bang, start, single)),
            },
            '<' => match consume_if(&mut chars, '=') {
                Some(end) => tokens.push(token(Kind::LtEq, start, end)),
                None => tokens.push(token(Kind::Lt, start, single)),
            },
            '>' => match consume_if(&mut chars, '=') {
                Some(end) => tokens.push(token(Kind::GtEq, start, end)),
                None => tokens.push(token(Kind::Gt, start, single)),
            },
            // `&` and `|` are tokens only in pairs.
            '&' => match consume_if(&mut chars, '&') {
                Some(end) => tokens.push(token(Kind::AmpAmp, start, end)),
                None => {
                    diags.push(unexpected_character(start, single));
                    return None;
                }
            },
            '|' => match consume_if(&mut chars, '|') {
                Some(end) => tokens.push(token(Kind::PipePipe, start, end)),
                None => {
                    diags.push(unexpected_character(start, single));
                    return None;
                }
            },
            _ if is_ident_start(c) => {
                let end = consume_while(&mut chars, source.len(), is_ident_continue);
                let kind = match source.get(start..end) {
                    Some("func") => Kind::Func,
                    Some("return") => Kind::Return,
                    Some("var") => Kind::Var,
                    Some("true") => Kind::True,
                    Some("false") => Kind::False,
                    _ => Kind::Ident,
                };
                tokens.push(token(kind, start, end));
            }
            _ if c.is_ascii_digit() => {
                let end = consume_while(&mut chars, source.len(), |c| c.is_ascii_digit());
                tokens.push(token(Kind::IntLit, start, end));
            }
            _ => {
                diags.push(unexpected_character(start, single));
                return None;
            }
        }
    }

    Some(tokens)
}

fn token(kind: Kind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: diag::Span { start, end },
    }
}

fn unexpected_character(start: usize, end: usize) -> diag::Entry {
    diag::Entry {
        kind: diag::Kind::UnexpectedCharacter,
        span: diag::Span { start, end },
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
        let mut diags = diag::Sink::default();
        let tokens = lex(source, &mut diags);
        assert!(diags.iter().next().is_none());
        tokens.expect("lexing succeeded")
    }

    /// Lexes `source`, asserting that it produced no tokens, and returns the
    /// diagnostics it reported.
    fn lex_err(source: &str) -> Vec<diag::Entry> {
        let mut diags = diag::Sink::default();
        assert_eq!(lex(source, &mut diags), None);
        diags.iter().cloned().collect()
    }

    fn spans(source: &str, kinds: &[(Kind, &str)]) -> Vec<Token> {
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
                (Kind::Func, "func"),
                (Kind::Ident, "approval"),
                (Kind::LParen, "("),
                (Kind::RParen, ")"),
                (Kind::Ident, "uint64"),
                (Kind::LBrace, "{"),
                (Kind::Return, "return"),
                (Kind::IntLit, "1"),
                (Kind::RBrace, "}"),
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
                (Kind::Func, "func"),
                (Kind::Ident, "approval"),
                (Kind::LParen, "("),
                (Kind::RParen, ")"),
                (Kind::Ident, "uint64"),
                (Kind::LBrace, "{"),
                (Kind::Return, "return"),
                (Kind::IntLit, "1"),
                (Kind::RBrace, "}"),
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
            vec![token(Kind::Ident, 0, 5), token(Kind::Ident, 6, 13)]
        );
    }

    #[test]
    fn adjacent_integers_stay_separate() {
        assert_eq!(
            lex_ok("1 2"),
            vec![token(Kind::IntLit, 0, 1), token(Kind::IntLit, 2, 3)]
        );
    }

    #[test]
    fn lexes_an_arithmetic_program() {
        let source = "func approval() uint64 {\n  return (1 + 2) * 3 - 4 / 5\n}\n";
        let expected = spans(
            source,
            &[
                (Kind::Func, "func"),
                (Kind::Ident, "approval"),
                (Kind::LParen, "("),
                (Kind::RParen, ")"),
                (Kind::Ident, "uint64"),
                (Kind::LBrace, "{"),
                (Kind::Return, "return"),
                (Kind::LParen, "("),
                (Kind::IntLit, "1"),
                (Kind::Plus, "+"),
                (Kind::IntLit, "2"),
                (Kind::RParen, ")"),
                (Kind::Star, "*"),
                (Kind::IntLit, "3"),
                (Kind::Minus, "-"),
                (Kind::IntLit, "4"),
                (Kind::Slash, "/"),
                (Kind::IntLit, "5"),
                (Kind::RBrace, "}"),
            ],
        );
        assert_eq!(lex_ok(source), expected);
    }

    #[test]
    fn operators_need_no_surrounding_whitespace() {
        assert_eq!(
            lex_ok("1+2*3-4/5"),
            vec![
                token(Kind::IntLit, 0, 1),
                token(Kind::Plus, 1, 2),
                token(Kind::IntLit, 2, 3),
                token(Kind::Star, 3, 4),
                token(Kind::IntLit, 4, 5),
                token(Kind::Minus, 5, 6),
                token(Kind::IntLit, 6, 7),
                token(Kind::Slash, 7, 8),
                token(Kind::IntLit, 8, 9),
            ]
        );
    }

    #[test]
    fn a_minus_is_never_part_of_a_literal() {
        assert_eq!(
            lex_ok("-1"),
            vec![token(Kind::Minus, 0, 1), token(Kind::IntLit, 1, 2)]
        );
    }

    #[test]
    fn repeated_operators_are_separate_tokens() {
        assert_eq!(
            lex_ok("--"),
            vec![token(Kind::Minus, 0, 1), token(Kind::Minus, 1, 2)]
        );
        assert_eq!(
            lex_ok("//"),
            vec![token(Kind::Slash, 0, 1), token(Kind::Slash, 1, 2)]
        );
    }

    #[test]
    fn a_percent_is_a_token() {
        assert_eq!(lex_ok("%"), vec![token(Kind::Percent, 0, 1)]);
        assert_eq!(
            lex_ok("1%2"),
            vec![
                token(Kind::IntLit, 0, 1),
                token(Kind::Percent, 1, 2),
                token(Kind::IntLit, 2, 3),
            ]
        );
    }

    #[test]
    fn lexes_a_variable_declaration() {
        let source = "var x uint64 = 1";
        let expected = spans(
            source,
            &[
                (Kind::Var, "var"),
                (Kind::Ident, "x"),
                (Kind::Ident, "uint64"),
                (Kind::Equals, "="),
                (Kind::IntLit, "1"),
            ],
        );
        assert_eq!(lex_ok(source), expected);
    }

    #[test]
    fn var_prefixes_are_identifiers() {
        assert_eq!(
            lex_ok("var_ variable"),
            vec![token(Kind::Ident, 0, 4), token(Kind::Ident, 5, 13)]
        );
    }

    #[test]
    fn lexes_the_boolean_literals() {
        let source = "return true";
        let expected = spans(source, &[(Kind::Return, "return"), (Kind::True, "true")]);
        assert_eq!(lex_ok(source), expected);

        let source = "var ok bool = false";
        let expected = spans(
            source,
            &[
                (Kind::Var, "var"),
                (Kind::Ident, "ok"),
                (Kind::Ident, "bool"),
                (Kind::Equals, "="),
                (Kind::False, "false"),
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
                (Kind::Ident, "truest"),
                (Kind::Ident, "false_"),
                (Kind::Ident, "True"),
            ],
        );
        assert_eq!(lex_ok(source), expected);
    }

    #[test]
    fn an_equals_needs_no_surrounding_whitespace() {
        assert_eq!(
            lex_ok("x=1"),
            vec![
                token(Kind::Ident, 0, 1),
                token(Kind::Equals, 1, 2),
                token(Kind::IntLit, 2, 3),
            ]
        );
    }

    #[test]
    fn lexes_the_comparison_and_logic_program() {
        let source = "func approval() bool {\n  var x uint64 = 1 + 2\n  return x >= 3 && !(x == 4) || x != 5\n}\n";
        let expected = spans(
            source,
            &[
                (Kind::Func, "func"),
                (Kind::Ident, "approval"),
                (Kind::LParen, "("),
                (Kind::RParen, ")"),
                (Kind::Ident, "bool"),
                (Kind::LBrace, "{"),
                (Kind::Var, "var"),
                (Kind::Ident, "x"),
                (Kind::Ident, "uint64"),
                (Kind::Equals, "="),
                (Kind::IntLit, "1"),
                (Kind::Plus, "+"),
                (Kind::IntLit, "2"),
                (Kind::Return, "return"),
                (Kind::Ident, "x"),
                (Kind::GtEq, ">="),
                (Kind::IntLit, "3"),
                (Kind::AmpAmp, "&&"),
                (Kind::Bang, "!"),
                (Kind::LParen, "("),
                (Kind::Ident, "x"),
                (Kind::EqEq, "=="),
                (Kind::IntLit, "4"),
                (Kind::RParen, ")"),
                (Kind::PipePipe, "||"),
                (Kind::Ident, "x"),
                (Kind::BangEq, "!="),
                (Kind::IntLit, "5"),
                (Kind::RBrace, "}"),
            ],
        );
        assert_eq!(lex_ok(source), expected);
    }

    #[test]
    fn comparisons_need_no_surrounding_whitespace() {
        assert_eq!(
            lex_ok("1==2!=3<4<=5>6>=7"),
            vec![
                token(Kind::IntLit, 0, 1),
                token(Kind::EqEq, 1, 3),
                token(Kind::IntLit, 3, 4),
                token(Kind::BangEq, 4, 6),
                token(Kind::IntLit, 6, 7),
                token(Kind::Lt, 7, 8),
                token(Kind::IntLit, 8, 9),
                token(Kind::LtEq, 9, 11),
                token(Kind::IntLit, 11, 12),
                token(Kind::Gt, 12, 13),
                token(Kind::IntLit, 13, 14),
                token(Kind::GtEq, 14, 16),
                token(Kind::IntLit, 16, 17),
            ]
        );
    }

    #[test]
    fn logical_operators_need_no_surrounding_whitespace() {
        assert_eq!(
            lex_ok("a&&b||!c"),
            vec![
                token(Kind::Ident, 0, 1),
                token(Kind::AmpAmp, 1, 3),
                token(Kind::Ident, 3, 4),
                token(Kind::PipePipe, 4, 6),
                token(Kind::Bang, 6, 7),
                token(Kind::Ident, 7, 8),
            ]
        );
    }

    #[test]
    fn two_character_tokens_are_matched_greedily() {
        assert_eq!(
            lex_ok("==="),
            vec![token(Kind::EqEq, 0, 2), token(Kind::Equals, 2, 3)]
        );
        assert_eq!(
            lex_ok("!!"),
            vec![token(Kind::Bang, 0, 1), token(Kind::Bang, 1, 2)]
        );
        assert_eq!(
            lex_ok("<=="),
            vec![token(Kind::LtEq, 0, 2), token(Kind::Equals, 2, 3)]
        );
        assert_eq!(
            lex_ok("=!"),
            vec![token(Kind::Equals, 0, 1), token(Kind::Bang, 1, 2)]
        );
        assert_eq!(
            lex_ok("<>"),
            vec![token(Kind::Lt, 0, 1), token(Kind::Gt, 1, 2)]
        );
    }

    #[test]
    fn whitespace_breaks_a_two_character_token() {
        assert_eq!(
            lex_ok("= ="),
            vec![token(Kind::Equals, 0, 1), token(Kind::Equals, 2, 3)]
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
