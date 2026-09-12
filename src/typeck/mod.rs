//! The type checker: an AST to a typed AST, resolving every type it names.

mod pass;
#[cfg(test)]
mod pass_test;
mod recursion;
#[cfg(test)]
mod recursion_test;

pub use pass::check;
