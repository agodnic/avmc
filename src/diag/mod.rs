//! Spans and diagnostics, shared by every compiler stage.

mod kind;
#[cfg(test)]
mod kind_test;
mod sink;

pub use kind::{Code, Kind, Severity};
pub use sink::{Entry, Sink, Span};
