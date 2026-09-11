//! The IR: a flat single-assignment instruction list, and the verifier that
//! enforces its invariant.

mod inst;
#[cfg(test)]
mod tests;
mod verifier;
#[cfg(test)]
mod verifier_test;
mod violation;

pub use inst::{Function, Inst, Program, ValueId};
pub use verifier::verify;
pub use violation::Violation;
