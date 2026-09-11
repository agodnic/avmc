//! Spans and diagnostics, shared by every compiler stage.

mod kind;
mod sink;
#[cfg(test)]
mod tests;

pub use kind::{Code, Kind, Severity};
pub use sink::{Entry, Sink, Span};
