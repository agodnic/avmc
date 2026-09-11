//! Tests for the diagnostic catalogue.

use super::kind::{Code, Kind, Severity};
use crate::typed_ast;

/// One sample of every variant, with the code and message it renders as.
///
/// A new variant does not compile until the match below covers it, which
/// is the reminder to list a sample for it here.
fn samples() -> Vec<(Kind, &'static str, &'static str)> {
    let samples = vec![
        (Kind::UnexpectedCharacter, "E0001", "unexpected character"),
        (
            Kind::UnexpectedToken {
                expected: "`)`",
                found: "an identifier",
            },
            "E0002",
            "expected `)`, found an identifier",
        ),
        (
            Kind::IntegerLiteralOutOfRange,
            "E0003",
            "integer literal out of range",
        ),
        (
            Kind::UnknownType {
                name: "bytes".to_string(),
            },
            "E0004",
            "unknown type `bytes`",
        ),
        (Kind::MissingReturn, "E0005", "missing return"),
        (Kind::UnreachableStatement, "E0006", "unreachable statement"),
        (
            Kind::DuplicateFunction {
                name: "a".to_string(),
            },
            "E0007",
            "duplicate function `a`",
        ),
        (
            Kind::MissingEntryPoint { name: "approval" },
            "E0008",
            "missing entry point `approval`",
        ),
        (
            Kind::AmbiguousPrecedence {
                left: "`%`",
                right: "`+`",
            },
            "E0009",
            "`%` and `+` need parentheses to disambiguate",
        ),
        (
            Kind::UndefinedVariable {
                name: "x".to_string(),
            },
            "E0010",
            "undefined variable `x`",
        ),
        (
            Kind::DuplicateVariable {
                name: "x".to_string(),
            },
            "E0011",
            "duplicate variable `x`",
        ),
        (
            Kind::TooManyVariables { max: 128 },
            "E0012",
            "a function may declare at most 128 variables",
        ),
        (
            Kind::TypeMismatch {
                expected: typed_ast::Type::Bool,
                found: typed_ast::Type::Uint64,
            },
            "E0013",
            "mismatched types: expected `bool`, found `uint64`",
        ),
    ];

    // Exhaustive, with no wildcard arm, so that adding a variant to
    // `Kind` stops the tests from compiling.
    for (kind, _, _) in &samples {
        match kind {
            Kind::UnexpectedCharacter
            | Kind::UnexpectedToken { .. }
            | Kind::IntegerLiteralOutOfRange
            | Kind::UnknownType { .. }
            | Kind::MissingReturn
            | Kind::UnreachableStatement
            | Kind::DuplicateFunction { .. }
            | Kind::MissingEntryPoint { .. }
            | Kind::AmbiguousPrecedence { .. }
            | Kind::UndefinedVariable { .. }
            | Kind::DuplicateVariable { .. }
            | Kind::TooManyVariables { .. }
            | Kind::TypeMismatch { .. } => {}
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
