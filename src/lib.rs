//! A compiler targeting the Algorand Virtual Machine (AVM).

pub mod ast;
pub mod diagnostics;
pub mod driver;
pub mod emit;
pub mod ir;
pub mod lexer;
pub mod lower;
pub mod parser;
#[cfg(test)]
pub(crate) mod testing;
pub mod typeck;
pub mod typed_ast;
