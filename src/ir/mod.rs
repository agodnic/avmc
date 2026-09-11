//! The IR: a flat single-assignment instruction list, and the verifier that
//! enforces its invariant.

mod inst;
mod verifier;
#[cfg(test)]
mod verifier_test;
mod violation;
#[cfg(test)]
mod violation_test;

pub use inst::{Function, Inst, Program, ValueId};
pub use verifier::verify;
pub use violation::Violation;
