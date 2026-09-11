use crate::typed_ast;
use std::fmt;

/// Every diagnostic the compiler can report, with the data its message needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
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
    /// `left` and `right` describe the two operators, backticks included.
    AmbiguousPrecedence {
        left: &'static str,
        right: &'static str,
    },
    UndefinedVariable {
        name: String,
    },
    DuplicateVariable {
        name: String,
    },
    TooManyVariables {
        max: usize,
    },
    TypeMismatch {
        expected: typed_ast::Type,
        found: typed_ast::Type,
    },
    TooManyParameters {
        max: usize,
    },
    EntryPointTakesParameters {
        name: &'static str,
    },
    UndefinedFunction {
        name: String,
    },
    WrongArgumentCount {
        name: String,
        expected: usize,
        found: usize,
    },
    RecursiveCall {
        name: String,
    },
}

impl Kind {
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
            Self::AmbiguousPrecedence { .. } => 9,
            Self::UndefinedVariable { .. } => 10,
            Self::DuplicateVariable { .. } => 11,
            Self::TooManyVariables { .. } => 12,
            Self::TypeMismatch { .. } => 13,
            Self::TooManyParameters { .. } => 14,
            Self::EntryPointTakesParameters { .. } => 15,
            Self::UndefinedFunction { .. } => 16,
            Self::WrongArgumentCount { .. } => 17,
            Self::RecursiveCall { .. } => 18,
        };
        Code {
            severity: Severity::Error,
            number,
        }
    }
}

impl fmt::Display for Kind {
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
            Self::AmbiguousPrecedence { left, right } => {
                write!(f, "{left} and {right} need parentheses to disambiguate")
            }
            Self::UndefinedVariable { name } => write!(f, "undefined variable `{name}`"),
            Self::DuplicateVariable { name } => write!(f, "duplicate variable `{name}`"),
            Self::TooManyVariables { max } => {
                write!(f, "a function may declare at most {max} variables")
            }
            Self::TypeMismatch { expected, found } => {
                write!(
                    f,
                    "mismatched types: expected `{expected}`, found `{found}`"
                )
            }
            Self::TooManyParameters { max } => {
                write!(f, "a function may declare at most {max} parameters")
            }
            Self::EntryPointTakesParameters { name } => {
                write!(f, "entry point `{name}` takes parameters")
            }
            Self::UndefinedFunction { name } => write!(f, "undefined function `{name}`"),
            Self::WrongArgumentCount {
                name,
                expected,
                found,
            } => write!(
                f,
                "wrong number of arguments to `{name}`: expected {expected}, found {found}"
            ),
            Self::RecursiveCall { name } => {
                write!(f, "recursive call to `{name}`: recursion is not supported")
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
