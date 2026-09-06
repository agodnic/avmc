//! The IR: a flat single-assignment instruction list, and the verifier that
//! enforces its invariant.

use crate::diagnostics::Span;
use crate::typed_ast::Type;

/// The value a defining instruction produces. Numbered per function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueId(pub u32);

/// A single instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inst {
    /// Defines `dest` as the `uint64` constant `value`.
    Const {
        /// The value it defines.
        dest: ValueId,
        /// The constant it holds.
        value: u64,
        /// The literal it came from.
        span: Span,
    },
    /// Returns `value` from the enclosing function.
    Return {
        /// The value it returns.
        value: ValueId,
        /// The `return` statement it came from.
        span: Span,
    },
}

impl Inst {
    /// The source it came from.
    pub fn span(&self) -> Span {
        match self {
            Inst::Const { span, .. } | Inst::Return { span, .. } => *span,
        }
    }
}

/// A function's body, as instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// The declared name.
    pub name: String,
    /// The return type.
    pub ret: Type,
    /// The instructions, in execution order.
    pub insts: Vec<Inst>,
    /// From `func` through the closing `}`.
    pub span: Span,
}

/// A whole compilation unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The functions it defines, in source order.
    pub funcs: Vec<Function>,
}

/// A way a function can fail the v0 IR invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// An instruction defines a value out of the dense numbering.
    SparseDefinition {
        /// The offending instruction's position.
        index: usize,
        /// The value it defines.
        dest: ValueId,
        /// The value it should have defined.
        expected: ValueId,
    },
    /// An instruction uses a value no earlier instruction defines.
    UseBeforeDefinition {
        /// The offending instruction's position.
        index: usize,
        /// The value it uses.
        value: ValueId,
    },
    /// An instruction uses a value out of definition order.
    UseOutOfOrder {
        /// The offending instruction's position.
        index: usize,
        /// The value it uses.
        value: ValueId,
        /// The value it should have used.
        expected: ValueId,
    },
    /// The function defines more values than it uses.
    UnusedValues {
        /// How many values it defines.
        defs: u32,
        /// How many of them it uses.
        uses: u32,
    },
    /// An instruction returns without being the last one.
    ReturnNotLast {
        /// The offending instruction's position.
        index: usize,
    },
    /// The function's last instruction is not a `Return`.
    MissingReturn,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Violation::SparseDefinition {
                index,
                dest,
                expected,
            } => write!(
                f,
                "dense single assignment: instruction {index} defines %{}, expected %{}",
                dest.0, expected.0
            ),
            Violation::UseBeforeDefinition { index, value } => write!(
                f,
                "defined before used: instruction {index} uses %{}, which is not yet defined",
                value.0
            ),
            Violation::UseOutOfOrder {
                index,
                value,
                expected,
            } => write!(
                f,
                "used exactly once, in definition order: instruction {index} uses %{}, expected %{}",
                value.0, expected.0
            ),
            Violation::UnusedValues { defs, uses } => write!(
                f,
                "used exactly once, in definition order: {defs} values defined but {uses} used"
            ),
            Violation::ReturnNotLast { index } => write!(
                f,
                "ends with `Return`: instruction {index} returns but is not the last"
            ),
            Violation::MissingReturn => write!(
                f,
                "ends with `Return`: the function does not end with a return"
            ),
        }
    }
}

/// Checks the v0 IR invariant, returning the first violation.
///
/// Type correctness is vacuous with one type and is not checked.
pub fn verify(func: &Function) -> Result<(), Violation> {
    verify_return(func)?;

    // The number of values defined and the number used so far: the next
    // definition must be `ValueId(defs)`, and the next use must be
    // `ValueId(uses)`, which was defined only if it is below `defs`.
    let mut defs = 0;
    let mut uses = 0;

    for (index, inst) in func.insts.iter().enumerate() {
        match inst {
            Inst::Const { dest, .. } => {
                if dest.0 != defs {
                    return Err(Violation::SparseDefinition {
                        index,
                        dest: *dest,
                        expected: ValueId(defs),
                    });
                }
                defs += 1;
            }
            Inst::Return { value, .. } => {
                if value.0 >= defs {
                    return Err(Violation::UseBeforeDefinition {
                        index,
                        value: *value,
                    });
                }
                if value.0 != uses {
                    return Err(Violation::UseOutOfOrder {
                        index,
                        value: *value,
                        expected: ValueId(uses),
                    });
                }
                uses += 1;
            }
        }
    }

    if uses != defs {
        return Err(Violation::UnusedValues { defs, uses });
    }
    Ok(())
}

/// Checks that the last instruction is a `Return`, and no other one is.
fn verify_return(func: &Function) -> Result<(), Violation> {
    let last = func.insts.len().checked_sub(1);
    for (index, inst) in func.insts.iter().enumerate() {
        if matches!(inst, Inst::Return { .. }) && Some(index) != last {
            return Err(Violation::ReturnNotLast { index });
        }
    }
    match func.insts.last() {
        Some(Inst::Return { .. }) => Ok(()),
        _ => Err(Violation::MissingReturn),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The span every hand-built instruction carries: the verifier ignores
    /// spans, so which one it is does not matter.
    const SPAN: Span = Span { start: 0, end: 0 };

    fn function(insts: Vec<Inst>) -> Function {
        Function {
            name: "approval".to_string(),
            ret: Type::Uint64,
            insts,
            span: SPAN,
        }
    }

    fn constant(dest: u32, value: u64) -> Inst {
        Inst::Const {
            dest: ValueId(dest),
            value,
            span: SPAN,
        }
    }

    fn ret(value: u32) -> Inst {
        Inst::Return {
            value: ValueId(value),
            span: SPAN,
        }
    }

    #[test]
    fn const_then_return_is_valid() {
        assert_eq!(verify(&function(vec![constant(0, 1), ret(0)])), Ok(()));
    }

    #[test]
    fn use_out_of_order_is_rejected() {
        assert_eq!(
            verify(&function(vec![constant(0, 1), constant(1, 2), ret(1)])),
            Err(Violation::UseOutOfOrder {
                index: 2,
                value: ValueId(1),
                expected: ValueId(0),
            })
        );
    }

    #[test]
    fn unused_value_is_rejected() {
        assert_eq!(
            verify(&function(vec![constant(0, 1), constant(1, 2), ret(0)])),
            Err(Violation::UnusedValues { defs: 2, uses: 1 })
        );
    }

    #[test]
    fn sparse_definition_is_rejected() {
        assert_eq!(
            verify(&function(vec![constant(1, 1), ret(1)])),
            Err(Violation::SparseDefinition {
                index: 0,
                dest: ValueId(1),
                expected: ValueId(0),
            })
        );
    }

    #[test]
    fn returning_an_undefined_value_is_rejected() {
        assert_eq!(
            verify(&function(vec![ret(0)])),
            Err(Violation::UseBeforeDefinition {
                index: 0,
                value: ValueId(0),
            })
        );
    }

    #[test]
    fn missing_return_is_rejected() {
        assert_eq!(
            verify(&function(vec![constant(0, 1)])),
            Err(Violation::MissingReturn)
        );
    }

    #[test]
    fn return_that_is_not_last_is_rejected() {
        assert_eq!(
            verify(&function(vec![constant(0, 1), ret(0), constant(1, 2)])),
            Err(Violation::ReturnNotLast { index: 1 })
        );
    }

    #[test]
    fn violations_describe_themselves() {
        assert_eq!(
            Violation::UseBeforeDefinition {
                index: 3,
                value: ValueId(2),
            }
            .to_string(),
            "defined before used: instruction 3 uses %2, which is not yet defined"
        );
    }
}
