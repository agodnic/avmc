//! The language's expression precedence: a partial order over operator groups.
//!
//! Two operators the order does not relate are a compile error rather than a
//! silent grouping, so this module knows nothing about tokens or parsing.

mod group;
mod table;
#[cfg(test)]
mod tests;

pub use group::{Group, group, unary_group};
pub use table::{Priority, priority};
