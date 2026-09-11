use super::token::{Token, TokenKind};
use crate::diag;
use std::iter::Peekable;
use std::str::CharIndices;

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
            '(' => tokens.push(token(TokenKind::LParen, start, single)),
            ')' => tokens.push(token(TokenKind::RParen, start, single)),
            '{' => tokens.push(token(TokenKind::LBrace, start, single)),
            '}' => tokens.push(token(TokenKind::RBrace, start, single)),
            ',' => tokens.push(token(TokenKind::Comma, start, single)),
            '+' => tokens.push(token(TokenKind::Plus, start, single)),
            '-' => tokens.push(token(TokenKind::Minus, start, single)),
            '*' => tokens.push(token(TokenKind::Star, start, single)),
            // `//` starts a comment, which runs to the end of the line and
            // is trivia, not a token.
            '/' => match consume_if(&mut chars, '/') {
                Some(_) => {
                    consume_while(&mut chars, source.len(), |c| c != '\n');
                }
                None => tokens.push(token(TokenKind::Slash, start, single)),
            },
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

    Some(finish(tokens, source.len()))
}

/// Fills in the trivia `lex` left empty — the gap before each token — and ends
/// the stream with `Eof`, whose trivia is everything after the last token.
fn finish(mut tokens: Vec<Token>, source_len: usize) -> Vec<Token> {
    let mut end = 0;
    for token in &mut tokens {
        token.trivia = diag::Span {
            start: end,
            end: token.span.start,
        };
        end = token.span.end;
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: diag::Span {
            start: source_len,
            end: source_len,
        },
        trivia: diag::Span {
            start: end,
            end: source_len,
        },
    });
    tokens
}

/// A token with empty trivia; [`finish`] fills it in once the gaps are known.
fn token(kind: TokenKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: diag::Span { start, end },
        trivia: diag::Span { start, end: start },
    }
}

pub(super) fn unexpected_character(start: usize, end: usize) -> diag::Entry {
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
