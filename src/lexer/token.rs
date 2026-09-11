use crate::diag;

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
    /// The end of the input.
    Eof,
}

/// A token: a kind and the source range it covers.
///
/// Tokens carry no text; later stages slice the source with the span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    /// What was matched.
    pub kind: TokenKind,
    /// Where it was matched.
    pub span: diag::Span,
    /// The whitespace and comments between the previous token and this one.
    /// Empty when there are none.
    pub trivia: diag::Span,
}
