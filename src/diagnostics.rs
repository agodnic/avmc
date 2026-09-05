//! Spans and diagnostics, shared by every compiler stage.

use std::fmt;

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
    pub kind: DiagnosticKind,
    /// The source location the diagnostic refers to.
    pub span: Span,
}

/// Every diagnostic the compiler can report, with the data its message needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    UnexpectedCharacter,
    UnexpectedToken {
        expected: &'static str,
        found: &'static str,
    },
    IntegerLiteralOutOfRange,
    UnknownType {
        name: String,
    },
    MissingReturn,
    UnreachableStatement,
    DuplicateFunction {
        name: String,
    },
    MissingEntryPoint {
        name: &'static str,
    },
    OpcodeUnavailable {
        opcode: &'static str,
        min: u8,
        target: u8,
    },
}

impl DiagnosticKind {
    /// The stable code. A code is never reused for a different meaning.
    pub fn code(&self) -> Code {
        let number = match self {
            Self::UnexpectedCharacter => 1,
            Self::UnexpectedToken { .. } => 2,
            Self::IntegerLiteralOutOfRange => 3,
            Self::UnknownType { .. } => 4,
            Self::MissingReturn => 5,
            Self::UnreachableStatement => 6,
            Self::DuplicateFunction { .. } => 7,
            Self::MissingEntryPoint { .. } => 8,
            Self::OpcodeUnavailable { .. } => 9,
        };
        Code {
            severity: Severity::Error,
            number,
        }
    }
}

impl fmt::Display for DiagnosticKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter => write!(f, "unexpected character"),
            Self::UnexpectedToken { expected, found } => {
                write!(f, "expected {expected}, found {found}")
            }
            Self::IntegerLiteralOutOfRange => write!(f, "integer literal out of range"),
            Self::UnknownType { name } => write!(f, "unknown type `{name}`"),
            Self::MissingReturn => write!(f, "missing return"),
            Self::UnreachableStatement => write!(f, "unreachable statement"),
            Self::DuplicateFunction { name } => write!(f, "duplicate function `{name}`"),
            Self::MissingEntryPoint { name } => write!(f, "missing entry point `{name}`"),
            Self::OpcodeUnavailable {
                opcode,
                min,
                target,
            } => {
                write!(
                    f,
                    "`{opcode}` requires TEAL version {min}, target is {target}"
                )
            }
        }
    }
}

/// Whether a diagnostic fails the compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
        }
    }
}

/// A stable identifier: the severity's letter (`E` or `W`) then four digits.
/// Errors and warnings are numbered independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code {
    pub severity: Severity,
    pub number: u16,
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let letter = match self.severity {
            Severity::Error => 'E',
            Severity::Warning => 'W',
        };
        write!(f, "{letter}{:04}", self.number)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// One sample of every variant, with the code and message it renders as.
    ///
    /// A new variant does not compile until the match below covers it, which
    /// is the reminder to list a sample for it here.
    fn samples() -> Vec<(DiagnosticKind, &'static str, &'static str)> {
        let samples = vec![
            (
                DiagnosticKind::UnexpectedCharacter,
                "E0001",
                "unexpected character",
            ),
            (
                DiagnosticKind::UnexpectedToken {
                    expected: "`)`",
                    found: "an identifier",
                },
                "E0002",
                "expected `)`, found an identifier",
            ),
            (
                DiagnosticKind::IntegerLiteralOutOfRange,
                "E0003",
                "integer literal out of range",
            ),
            (
                DiagnosticKind::UnknownType {
                    name: "bytes".to_string(),
                },
                "E0004",
                "unknown type `bytes`",
            ),
            (DiagnosticKind::MissingReturn, "E0005", "missing return"),
            (
                DiagnosticKind::UnreachableStatement,
                "E0006",
                "unreachable statement",
            ),
            (
                DiagnosticKind::DuplicateFunction {
                    name: "a".to_string(),
                },
                "E0007",
                "duplicate function `a`",
            ),
            (
                DiagnosticKind::MissingEntryPoint { name: "approval" },
                "E0008",
                "missing entry point `approval`",
            ),
            (
                DiagnosticKind::OpcodeUnavailable {
                    opcode: "pushint",
                    min: 3,
                    target: 2,
                },
                "E0009",
                "`pushint` requires TEAL version 3, target is 2",
            ),
        ];

        // Exhaustive, with no wildcard arm, so that adding a variant to
        // `DiagnosticKind` stops the tests from compiling.
        for (kind, _, _) in &samples {
            match kind {
                DiagnosticKind::UnexpectedCharacter
                | DiagnosticKind::UnexpectedToken { .. }
                | DiagnosticKind::IntegerLiteralOutOfRange
                | DiagnosticKind::UnknownType { .. }
                | DiagnosticKind::MissingReturn
                | DiagnosticKind::UnreachableStatement
                | DiagnosticKind::DuplicateFunction { .. }
                | DiagnosticKind::MissingEntryPoint { .. }
                | DiagnosticKind::OpcodeUnavailable { .. } => {}
            }
        }

        samples
    }

    #[test]
    fn every_variant_has_its_code_and_message() {
        for (kind, code, message) in samples() {
            assert_eq!(kind.code().to_string(), code, "code of {kind:?}");
            assert_eq!(kind.to_string(), message, "message of {kind:?}");
        }
    }

    #[test]
    fn no_two_variants_share_a_code() {
        let mut codes: Vec<String> = samples()
            .iter()
            .map(|(kind, _, _)| kind.code().to_string())
            .collect();
        let total = codes.len();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), total);
    }

    #[test]
    fn a_code_is_a_letter_and_four_digits() {
        let code = |severity, number| Code { severity, number }.to_string();
        assert_eq!(code(Severity::Error, 1), "E0001");
        assert_eq!(code(Severity::Warning, 1), "W0001");
        assert_eq!(code(Severity::Error, 1234), "E1234");
    }

    #[test]
    fn a_severity_names_itself() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
    }
}
