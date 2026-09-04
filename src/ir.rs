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

/// Checks the v0 IR invariant, returning a short description of the first
/// violation.
///
/// Type correctness is vacuous with one type and is not checked.
pub fn verify(func: &Function) -> Result<(), String> {
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
                    return Err(format!(
                        "dense single assignment: instruction {index} defines %{}, expected %{defs}",
                        dest.0
                    ));
                }
                defs += 1;
            }
            Inst::Return { value, .. } => {
                if value.0 >= defs {
                    return Err(format!(
                        "defined before used: instruction {index} uses %{}, which is not yet defined",
                        value.0
                    ));
                }
                if value.0 != uses {
                    return Err(format!(
                        "used exactly once, in definition order: instruction {index} uses %{}, expected %{uses}",
                        value.0
                    ));
                }
                uses += 1;
            }
        }
    }

    if uses != defs {
        return Err(format!(
            "used exactly once, in definition order: {defs} values defined but {uses} used"
        ));
    }
    Ok(())
}

/// Checks that the last instruction is a `Return`, and no other one is.
fn verify_return(func: &Function) -> Result<(), String> {
    let last = func.insts.len().checked_sub(1);
    for (index, inst) in func.insts.iter().enumerate() {
        if matches!(inst, Inst::Return { .. }) && Some(index) != last {
            return Err(format!(
                "ends with `Return`: instruction {index} returns but is not the last"
            ));
        }
    }
    match func.insts.last() {
        Some(Inst::Return { .. }) => Ok(()),
        _ => Err("ends with `Return`: the function does not end with a return".to_string()),
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
        assert!(verify(&function(vec![constant(0, 1), ret(0)])).is_ok());
    }

    #[test]
    fn unused_value_is_rejected() {
        assert!(verify(&function(vec![constant(0, 1), constant(1, 2), ret(1)])).is_err());
    }

    #[test]
    fn sparse_definition_is_rejected() {
        assert!(verify(&function(vec![constant(1, 1), ret(1)])).is_err());
    }

    #[test]
    fn use_before_definition_is_rejected() {
        assert!(verify(&function(vec![ret(0), constant(0, 1)])).is_err());
    }

    #[test]
    fn missing_return_is_rejected() {
        assert!(verify(&function(vec![constant(0, 1)])).is_err());
    }

    #[test]
    fn return_that_is_not_last_is_rejected() {
        assert!(verify(&function(vec![constant(0, 1), ret(0), constant(1, 2)])).is_err());
    }
}
