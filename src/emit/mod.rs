//! Emission: IR to TEAL text, in a single linear pass.

mod pass;
#[cfg(test)]
mod pass_test;

pub use pass::emit;
