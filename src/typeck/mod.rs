//! The type checker: an AST to a typed AST, resolving every type it names.

mod pass;
#[cfg(test)]
mod tests;

pub use pass::check;
