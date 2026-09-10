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
    /// The end of the input.
    Eof,
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
    /// The whitespace and comments between the previous token and this one.
    /// Empty when there are none.
    pub trivia: diag::Span,
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
            // `//` starts a comment, which runs to the end of the line and
            // is trivia, not a token.
            '/' => match consume_if(&mut chars, '/') {
                Some(_) => {
                    consume_while(&mut chars, source.len(), |c| c != '\n');
                }
                None => tokens.push(token(Kind::Slash, start, single)),
            },
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
        kind: Kind::Eof,
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
fn token(kind: Kind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: diag::Span { start, end },
        trivia: diag::Span { start, end: start },
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

    /// The approval program of the v0 milestone, with comments.
    const COMMENTED_APPROVAL: &str = "// The approval program.\nfunc approval() uint64 {\n  var x uint64 = 1 + 2 // one more than two\n  return x\n}\n";

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

    /// The tokens for `kinds`, each named by the text it covers, in source
    /// order. Each token's trivia is the gap before it, and the stream ends
    /// with `Eof`, as `lex` produces them.
    fn spans(source: &str, kinds: &[(Kind, &str)]) -> Vec<Token> {
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
            kind: Kind::Eof,
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
    fn empty_input_produces_only_the_end_of_input_token() {
        assert_eq!(lex_ok(""), spans("", &[]));
    }

    #[test]
    fn keyword_prefixes_are_identifiers() {
        let source = "func_ returns";
        let expected = spans(source, &[(Kind::Ident, "func_"), (Kind::Ident, "returns")]);
        assert_eq!(lex_ok(source), expected);
    }

    #[test]
    fn adjacent_integers_stay_separate() {
        let source = "1 2";
        let expected = spans(source, &[(Kind::IntLit, "1"), (Kind::IntLit, "2")]);
        assert_eq!(lex_ok(source), expected);
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
        let source = "1+2*3-4/5";
        let expected = spans(
            source,
            &[
                (Kind::IntLit, "1"),
                (Kind::Plus, "+"),
                (Kind::IntLit, "2"),
                (Kind::Star, "*"),
                (Kind::IntLit, "3"),
                (Kind::Minus, "-"),
                (Kind::IntLit, "4"),
                (Kind::Slash, "/"),
                (Kind::IntLit, "5"),
            ],
        );
        assert_eq!(lex_ok(source), expected);
    }

    #[test]
    fn a_minus_is_never_part_of_a_literal() {
        let source = "-1";
        let expected = spans(source, &[(Kind::Minus, "-"), (Kind::IntLit, "1")]);
        assert_eq!(lex_ok(source), expected);
    }

    #[test]
    fn repeated_operators_are_separate_tokens() {
        let source = "--";
        let expected = spans(source, &[(Kind::Minus, "-"), (Kind::Minus, "-")]);
        assert_eq!(lex_ok(source), expected);
    }

    #[test]
    fn a_percent_is_a_token() {
        let source = "%";
        assert_eq!(lex_ok(source), spans(source, &[(Kind::Percent, "%")]));

        let source = "1%2";
        let expected = spans(
            source,
            &[
                (Kind::IntLit, "1"),
                (Kind::Percent, "%"),
                (Kind::IntLit, "2"),
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
        let source = "var_ variable";
        let expected = spans(source, &[(Kind::Ident, "var_"), (Kind::Ident, "variable")]);
        assert_eq!(lex_ok(source), expected);
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
        let source = "x=1";
        let expected = spans(
            source,
            &[(Kind::Ident, "x"), (Kind::Equals, "="), (Kind::IntLit, "1")],
        );
        assert_eq!(lex_ok(source), expected);
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
        let source = "1==2!=3<4<=5>6>=7";
        let expected = spans(
            source,
            &[
                (Kind::IntLit, "1"),
                (Kind::EqEq, "=="),
                (Kind::IntLit, "2"),
                (Kind::BangEq, "!="),
                (Kind::IntLit, "3"),
                (Kind::Lt, "<"),
                (Kind::IntLit, "4"),
                (Kind::LtEq, "<="),
                (Kind::IntLit, "5"),
                (Kind::Gt, ">"),
                (Kind::IntLit, "6"),
                (Kind::GtEq, ">="),
                (Kind::IntLit, "7"),
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
                (Kind::Ident, "a"),
                (Kind::AmpAmp, "&&"),
                (Kind::Ident, "b"),
                (Kind::PipePipe, "||"),
                (Kind::Bang, "!"),
                (Kind::Ident, "c"),
            ],
        );
        assert_eq!(lex_ok(source), expected);
    }

    #[test]
    fn two_character_tokens_are_matched_greedily() {
        let source = "===";
        let expected = spans(source, &[(Kind::EqEq, "=="), (Kind::Equals, "=")]);
        assert_eq!(lex_ok(source), expected);

        let source = "!!";
        let expected = spans(source, &[(Kind::Bang, "!"), (Kind::Bang, "!")]);
        assert_eq!(lex_ok(source), expected);

        let source = "<==";
        let expected = spans(source, &[(Kind::LtEq, "<="), (Kind::Equals, "=")]);
        assert_eq!(lex_ok(source), expected);

        let source = "=!";
        let expected = spans(source, &[(Kind::Equals, "="), (Kind::Bang, "!")]);
        assert_eq!(lex_ok(source), expected);

        let source = "<>";
        let expected = spans(source, &[(Kind::Lt, "<"), (Kind::Gt, ">")]);
        assert_eq!(lex_ok(source), expected);
    }

    #[test]
    fn whitespace_breaks_a_two_character_token() {
        let source = "= =";
        let expected = spans(source, &[(Kind::Equals, "="), (Kind::Equals, "=")]);
        assert_eq!(lex_ok(source), expected);
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

    #[test]
    fn a_line_comment_is_trivia() {
        let source = "return 1 // one\n";
        let expected = spans(source, &[(Kind::Return, "return"), (Kind::IntLit, "1")]);
        assert_eq!(lex_ok(source), expected);
        assert_eq!(text(source, expected[2].trivia), " // one\n");
    }

    #[test]
    fn a_comment_on_its_own_line_is_trivia() {
        let source = "// hi\nreturn";
        let expected = spans(source, &[(Kind::Return, "return")]);
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
        assert_eq!(lex_ok(source), spans(source, &[(Kind::Return, "return")]));
    }

    #[test]
    fn a_lone_slash_is_division() {
        let source = "1/2";
        let expected = spans(
            source,
            &[(Kind::IntLit, "1"), (Kind::Slash, "/"), (Kind::IntLit, "2")],
        );
        assert_eq!(lex_ok(source), expected);

        let source = "1 / 2";
        let expected = spans(
            source,
            &[(Kind::IntLit, "1"), (Kind::Slash, "/"), (Kind::IntLit, "2")],
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
        let expected = spans(source, &[(Kind::Slash, "/"), (Kind::Slash, "/")]);
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
}
