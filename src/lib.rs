//! A compiler targeting the Algorand Virtual Machine (AVM).

pub mod ast;
pub mod diagnostics;
pub mod emit;
pub mod ir;
pub mod lexer;
pub mod lower;
pub mod parser;
pub mod typeck;
pub mod typed_ast;
