//! Lexing: source text to tokens.

mod pass;
#[cfg(test)]
mod tests;
mod token;

pub use pass::lex;
pub use token::{Token, TokenKind};
