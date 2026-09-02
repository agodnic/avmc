//! Spans and diagnostics, shared by every compiler stage.

/// A half-open range of byte offsets into the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Byte offset of the first byte, inclusive.
    pub start: usize,
    /// Byte offset one past the last byte, exclusive.
    pub end: usize,
}

/// A single problem found in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable identifier, never reused for a different meaning.
    pub code: &'static str,
    /// Human-readable description of the problem.
    pub message: String,
    /// The source location the diagnostic refers to.
    pub span: Span,
}

/// The diagnostics reported by a stage, in the order they were found.
#[derive(Debug, Default)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    /// Records one diagnostic.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    /// Returns `true` when nothing has been reported.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterates the diagnostics in the order they were reported.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter()
    }
}
