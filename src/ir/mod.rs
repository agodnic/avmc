//! The IR: a flat single-assignment instruction list, and the verifier that
//! enforces its invariant.

mod inst;
#[cfg(test)]
mod tests;
mod verifier;

pub use inst::{Function, Inst, Program, ValueId};
pub use verifier::{Violation, verify};
