//! The lexer: source text to a flat token stream (see `ARCHITECTURE.md` §2.1).

use crate::diagnostics::{Diagnostic, Diagnostics, Span};
use std::iter::Peekable;
use std::str::CharIndices;

/// The kind of a lexical token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// The keyword `func`.
    Func,
    /// The keyword `return`.
    Return,
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
}

/// A token: a kind and the source range it covers (R2).
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
/// Returns `None` if any diagnostic was reported (R3). Lexing still continues
/// past an error, so one run reports every unexpected character.
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
            _ if is_ident_start(c) => {
                let end = consume_while(&mut chars, source.len(), is_ident_continue);
                let kind = match source.get(start..end) {
                    Some("func") => TokenKind::Func,
                    Some("return") => TokenKind::Return,
                    _ => TokenKind::Ident,
                };
                tokens.push(token(kind, start, end));
            }
            _ if c.is_ascii_digit() => {
                let end = consume_while(&mut chars, source.len(), |c| c.is_ascii_digit());
                tokens.push(token(TokenKind::IntLit, start, end));
            }
            _ => diags.push(Diagnostic {
                code: "E0001",
                message: "unexpected character".to_string(),
                span: Span { start, end: single },
            }),
        }
    }

    if diags.is_empty() { Some(tokens) } else { None }
}

fn token(kind: TokenKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: Span { start, end },
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
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
    fn unexpected_characters_are_reported_and_skipped() {
        let source = "@é";
        let mut diags = Diagnostics::default();

        assert_eq!(lex(source, &mut diags), None);

        let reported: Vec<&Diagnostic> = diags.iter().collect();
        assert_eq!(
            reported,
            vec![
                &Diagnostic {
                    code: "E0001",
                    message: "unexpected character".to_string(),
                    span: Span { start: 0, end: 1 },
                },
                &Diagnostic {
                    code: "E0001",
                    message: "unexpected character".to_string(),
                    span: Span { start: 1, end: 3 },
                },
            ]
        );
    }
}
