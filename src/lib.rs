//! A compiler targeting the Algorand Virtual Machine (AVM).

pub mod ast;
pub mod cst;
pub mod diag;
pub mod driver;
pub mod emit;
pub mod ir;
pub mod lower;
pub mod parser;
pub mod precedence;
#[cfg(test)]
pub(crate) mod testing;
pub mod token;
pub mod typeck;
pub mod typed_ast;
