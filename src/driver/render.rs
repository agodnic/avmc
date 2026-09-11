use crate::diag;

/// Renders `diagnostic` as one line, without a trailing newline.
pub fn render(diagnostic: &diag::Entry, file_name: &str, source: &str) -> String {
    let (line, column) = position(source, diagnostic.span.start);
    let code = diagnostic.kind.code();
    let severity = code.severity;
    let message = &diagnostic.kind;
    format!("{file_name}:{line}:{column}: {severity}[{code}]: {message}")
}

/// The 1-based line and column of `offset` in `source`.
///
/// An offset that is not a char boundary of `source` — a compiler bug — is
/// reported as the end of input rather than panicking.
fn position(source: &str, offset: usize) -> (usize, usize) {
    let before = if source.is_char_boundary(offset) {
        source.get(..offset).unwrap_or(source)
    } else {
        source
    };

    let line = 1 + before.matches('\n').count();
    // Everything after the last newline, or all of `before` if there is none.
    let last_line = before.rsplit('\n').next().unwrap_or(before);
    (line, 1 + last_line.chars().count())
}
