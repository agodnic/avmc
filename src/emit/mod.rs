//! Emission: IR to TEAL text, in a single linear pass.

mod pass;
#[cfg(test)]
mod tests;

pub use pass::emit;
