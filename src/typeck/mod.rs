//! The type checker: an AST to a typed AST, resolving every type it names.

mod pass;
mod recursion;
#[cfg(test)]
mod tests;

pub use pass::check;
