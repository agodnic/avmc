//! The IR: a flat single-assignment instruction list, and the verifier that
//! enforces its invariant.

use crate::ast::BinaryOp;
use crate::diagnostics::Span;
use crate::typed_ast::{LocalId, Type};

/// The value a defining instruction produces. Numbered per function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueId(pub u32);

/// A single instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inst {
    /// Defines `dest` as the constant `value` of type `ty`.
    Const {
        /// The value it defines.
        dest: ValueId,
        /// The type it has.
        ty: Type,
        /// The constant it holds: for a `Bool`, `0` or `1`.
        value: u64,
        /// The literal it came from.
        span: Span,
    },
    /// Defines `dest` as `lhs op rhs`.
    Binary {
        /// The value it defines.
        dest: ValueId,
        /// The operator it applies.
        op: BinaryOp,
        /// The left operand, consumed first.
        lhs: ValueId,
        /// The right operand, consumed second.
        rhs: ValueId,
        /// The expression it came from.
        span: Span,
    },
    /// Writes `value` into frame slot `local`.
    Store {
        /// The slot it writes.
        local: LocalId,
        /// The value it writes, consumed.
        value: ValueId,
        /// The declaration it came from.
        span: Span,
    },
    /// Defines `dest` as a copy of frame slot `local`.
    Load {
        /// The value it defines.
        dest: ValueId,
        /// The slot it reads.
        local: LocalId,
        /// The expression it came from.
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
            Inst::Const { span, .. }
            | Inst::Binary { span, .. }
            | Inst::Store { span, .. }
            | Inst::Load { span, .. }
            | Inst::Return { span, .. } => *span,
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
    /// The frame: one slot per variable, indexed by `LocalId`.
    pub locals: Vec<Type>,
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
    /// An instruction names an operand that is not the one on top of the
    /// stack.
    UseOutOfOrder {
        /// The offending instruction's position.
        index: usize,
        /// Which operand it is, counting from 0.
        position: usize,
        /// The value it uses.
        value: ValueId,
        /// The value it should have used.
        expected: ValueId,
    },
    /// An instruction names more operands than there are live values.
    StackUnderflow {
        /// The offending instruction's position.
        index: usize,
        /// How many operands it names.
        needed: usize,
        /// How many values are live.
        available: usize,
    },
    /// The function ends with values that nothing consumed.
    ValuesLeftOnStack {
        /// How many of them there are.
        count: usize,
    },
    /// An instruction returns without being the last one.
    ReturnNotLast {
        /// The offending instruction's position.
        index: usize,
    },
    /// The function's last instruction is not a `Return`.
    MissingReturn,
    /// The frame holds more slots than one can address.
    FrameTooLarge {
        /// How many slots it holds.
        count: usize,
    },
    /// An instruction names a slot the frame does not have.
    LocalOutOfRange {
        /// The offending instruction's position.
        index: usize,
        /// The slot it names.
        local: LocalId,
        /// How many slots the frame holds.
        count: usize,
    },
    /// An instruction uses an operand of the wrong type.
    OperandType {
        /// The offending instruction's position.
        index: usize,
        /// Which operand it is, counting from 0.
        position: usize,
        /// The value it uses.
        value: ValueId,
        /// The type that value has.
        found: Type,
        /// The type the instruction needs.
        expected: Type,
    },
    /// A `Bool` constant holds something other than `0` or `1`.
    BoolOutOfRange {
        /// The offending instruction's position.
        index: usize,
        /// The constant it holds.
        value: u64,
    },
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
            Violation::UseOutOfOrder {
                index,
                position,
                value,
                expected,
            } => write!(
                f,
                "consumed in stack order: instruction {index} uses %{} as operand {position}, \
                 expected %{}",
                value.0, expected.0
            ),
            Violation::StackUnderflow {
                index,
                needed,
                available,
            } => write!(
                f,
                "consumed in stack order: instruction {index} needs {needed} operands but \
                 {available} values are live"
            ),
            Violation::ValuesLeftOnStack { count } => write!(
                f,
                "consumed in stack order: {count} values are left unconsumed"
            ),
            Violation::ReturnNotLast { index } => write!(
                f,
                "ends with `Return`: instruction {index} returns but is not the last"
            ),
            Violation::MissingReturn => write!(
                f,
                "ends with `Return`: the function does not end with a return"
            ),
            Violation::FrameTooLarge { count } => write!(
                f,
                "addressed within the frame: {count} locals exceed the capacity of {}",
                LocalId::CAPACITY
            ),
            Violation::LocalOutOfRange {
                index,
                local,
                count,
            } => write!(
                f,
                "addressed within the frame: instruction {index} names l{} but the frame has \
                 {count} locals",
                local.0
            ),
            Violation::OperandType {
                index,
                position,
                value,
                found,
                expected,
            } => write!(
                f,
                "well typed: instruction {index} uses %{} of type {found} as operand {position}, \
                 expected {expected}",
                value.0
            ),
            Violation::BoolOutOfRange { index, value } => write!(
                f,
                "well typed: instruction {index} defines a bool constant of {value}, expected 0 \
                 or 1"
            ),
        }
    }
}

/// Checks the v0 IR invariant, returning the first violation.
pub fn verify(func: &Function) -> Result<(), Violation> {
    verify_return(func)?;

    if func.locals.len() > LocalId::CAPACITY {
        return Err(Violation::FrameTooLarge {
            count: func.locals.len(),
        });
    }

    // The values defined and not yet consumed, most recent last, each with
    // its type.
    let mut stack: Vec<(ValueId, Type)> = Vec::new();
    // The number of values defined so far: the next definition must be
    // `ValueId(defs)`.
    let mut defs = 0;

    for (index, inst) in func.insts.iter().enumerate() {
        let (dest, ty) = match inst {
            Inst::Const {
                dest, ty, value, ..
            } => {
                if *ty == Type::Bool && *value > 1 {
                    return Err(Violation::BoolOutOfRange {
                        index,
                        value: *value,
                    });
                }
                (dest, *ty)
            }
            Inst::Binary { dest, lhs, rhs, .. } => {
                consume(
                    &mut stack,
                    index,
                    &[*lhs, *rhs],
                    &[Type::Uint64, Type::Uint64],
                )?;
                (dest, Type::Uint64)
            }
            Inst::Store { local, value, .. } => {
                let ty = slot(&func.locals, *local, index)?;
                consume(&mut stack, index, &[*value], &[ty])?;
                continue;
            }
            Inst::Load { dest, local, .. } => {
                let ty = slot(&func.locals, *local, index)?;
                (dest, ty)
            }
            Inst::Return { value, .. } => {
                consume(&mut stack, index, &[*value], &[func.ret])?;
                continue;
            }
        };

        if dest.0 != defs {
            return Err(Violation::SparseDefinition {
                index,
                dest: *dest,
                expected: ValueId(defs),
            });
        }
        defs += 1;
        stack.push((*dest, ty));
    }

    if !stack.is_empty() {
        return Err(Violation::ValuesLeftOnStack { count: stack.len() });
    }
    Ok(())
}

/// Consumes `operands` off `stack`, checking that they are the values on top
/// of it, in order, and that they have the types `expected`.
fn consume(
    stack: &mut Vec<(ValueId, Type)>,
    index: usize,
    operands: &[ValueId],
    expected: &[Type],
) -> Result<(), Violation> {
    let available = stack.len();
    let top = available
        .checked_sub(operands.len())
        .ok_or(Violation::StackUnderflow {
            index,
            needed: operands.len(),
            available,
        })?;

    for (position, ((operand, wanted), (live, found))) in operands
        .iter()
        .zip(expected)
        .zip(stack.iter().skip(top))
        .enumerate()
    {
        if operand != live {
            return Err(Violation::UseOutOfOrder {
                index,
                position,
                value: *operand,
                expected: *live,
            });
        }
        if found != wanted {
            return Err(Violation::OperandType {
                index,
                position,
                value: *operand,
                found: *found,
                expected: *wanted,
            });
        }
    }

    stack.truncate(top);
    Ok(())
}

/// Checks that `local` is a slot of the frame, returning its type.
fn slot(locals: &[Type], local: LocalId, index: usize) -> Result<Type, Violation> {
    locals
        .get(usize::from(local.0))
        .copied()
        .ok_or(Violation::LocalOutOfRange {
            index,
            local,
            count: locals.len(),
        })
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

    /// A function with an empty frame.
    fn function(insts: Vec<Inst>) -> Function {
        framed(0, insts)
    }

    /// A function whose frame is `locals` slots of `Type::Uint64`, returning
    /// `Type::Uint64`.
    fn framed(locals: usize, insts: Vec<Inst>) -> Function {
        shaped(Type::Uint64, vec![Type::Uint64; locals], insts)
    }

    /// A function returning `ret`, with `locals` as its frame.
    fn shaped(ret: Type, locals: Vec<Type>, insts: Vec<Inst>) -> Function {
        Function {
            name: "approval".to_string(),
            ret,
            locals,
            insts,
            span: SPAN,
        }
    }

    fn constant(dest: u32, value: u64) -> Inst {
        constant_of(dest, Type::Uint64, value)
    }

    fn constant_of(dest: u32, ty: Type, value: u64) -> Inst {
        Inst::Const {
            dest: ValueId(dest),
            ty,
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

    fn store(local: u8, value: u32) -> Inst {
        Inst::Store {
            local: LocalId(local),
            value: ValueId(value),
            span: SPAN,
        }
    }

    fn load(dest: u32, local: u8) -> Inst {
        Inst::Load {
            dest: ValueId(dest),
            local: LocalId(local),
            span: SPAN,
        }
    }

    fn binary(dest: u32, op: BinaryOp, lhs: u32, rhs: u32) -> Inst {
        Inst::Binary {
            dest: ValueId(dest),
            op,
            lhs: ValueId(lhs),
            rhs: ValueId(rhs),
            span: SPAN,
        }
    }

    #[test]
    fn const_then_return_is_valid() {
        assert_eq!(verify(&function(vec![constant(0, 1), ret(0)])), Ok(()));
    }

    #[test]
    fn a_binary_over_two_constants_is_valid() {
        assert_eq!(
            verify(&function(vec![
                constant(0, 1),
                constant(1, 2),
                binary(2, BinaryOp::Add, 0, 1),
                ret(2),
            ])),
            Ok(())
        );
    }

    #[test]
    fn a_right_leaning_tree_is_valid() {
        // `1 + 2 * 3`, which the definition-order invariant used to reject.
        assert_eq!(
            verify(&function(vec![
                constant(0, 1),
                constant(1, 2),
                constant(2, 3),
                binary(3, BinaryOp::Mul, 1, 2),
                binary(4, BinaryOp::Add, 0, 3),
                ret(4),
            ])),
            Ok(())
        );
    }

    #[test]
    fn swapped_operands_are_rejected() {
        assert_eq!(
            verify(&function(vec![
                constant(0, 1),
                constant(1, 2),
                binary(2, BinaryOp::Sub, 1, 0),
                ret(2),
            ])),
            Err(Violation::UseOutOfOrder {
                index: 2,
                position: 0,
                value: ValueId(1),
                expected: ValueId(0),
            })
        );
    }

    #[test]
    fn a_binary_without_enough_live_values_is_rejected() {
        assert_eq!(
            verify(&function(vec![
                constant(0, 1),
                binary(1, BinaryOp::Add, 0, 0),
                ret(1),
            ])),
            Err(Violation::StackUnderflow {
                index: 1,
                needed: 2,
                available: 1,
            })
        );
    }

    #[test]
    fn a_value_left_on_the_stack_is_rejected() {
        assert_eq!(
            verify(&function(vec![constant(0, 1), constant(1, 2), ret(1)])),
            Err(Violation::ValuesLeftOnStack { count: 1 })
        );
    }

    #[test]
    fn using_a_value_that_is_not_on_top_is_rejected() {
        assert_eq!(
            verify(&function(vec![constant(0, 1), constant(1, 2), ret(0)])),
            Err(Violation::UseOutOfOrder {
                index: 2,
                position: 0,
                value: ValueId(0),
                expected: ValueId(1),
            })
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
            Err(Violation::StackUnderflow {
                index: 0,
                needed: 1,
                available: 0,
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
    fn storing_and_loading_a_slot_is_valid() {
        assert_eq!(
            verify(&framed(
                1,
                vec![constant(0, 1), store(0, 0), load(1, 0), ret(1)]
            )),
            Ok(())
        );
    }

    #[test]
    fn two_slots_are_valid() {
        // The `var x = 1 + 2; var y = x * 3; return y - x` of the milestone.
        assert_eq!(
            verify(&framed(
                2,
                vec![
                    constant(0, 1),
                    constant(1, 2),
                    binary(2, BinaryOp::Add, 0, 1),
                    store(0, 2),
                    load(3, 0),
                    constant(4, 3),
                    binary(5, BinaryOp::Mul, 3, 4),
                    store(1, 5),
                    load(6, 1),
                    load(7, 0),
                    binary(8, BinaryOp::Sub, 6, 7),
                    ret(8),
                ]
            )),
            Ok(())
        );
    }

    #[test]
    fn storing_a_value_that_is_not_on_top_is_rejected() {
        assert_eq!(
            verify(&framed(
                1,
                vec![constant(0, 1), constant(1, 2), store(0, 0), ret(1)]
            )),
            Err(Violation::UseOutOfOrder {
                index: 2,
                position: 0,
                value: ValueId(0),
                expected: ValueId(1),
            })
        );
    }

    #[test]
    fn storing_without_a_live_value_is_rejected() {
        assert_eq!(
            verify(&framed(1, vec![store(0, 0), ret(0)])),
            Err(Violation::StackUnderflow {
                index: 0,
                needed: 1,
                available: 0,
            })
        );
    }

    #[test]
    fn loading_from_an_empty_frame_is_rejected() {
        assert_eq!(
            verify(&function(vec![load(0, 0), ret(0)])),
            Err(Violation::LocalOutOfRange {
                index: 0,
                local: LocalId(0),
                count: 0,
            })
        );
    }

    #[test]
    fn storing_past_the_frame_is_rejected() {
        assert_eq!(
            verify(&framed(
                2,
                vec![constant(0, 1), store(2, 0), constant(1, 2), ret(1)]
            )),
            Err(Violation::LocalOutOfRange {
                index: 1,
                local: LocalId(2),
                count: 2,
            })
        );
    }

    #[test]
    fn a_frame_past_the_capacity_is_rejected() {
        let count = LocalId::CAPACITY + 1;
        assert_eq!(
            verify(&framed(count, vec![constant(0, 1), ret(0)])),
            Err(Violation::FrameTooLarge { count })
        );
    }

    #[test]
    fn a_frame_at_the_capacity_is_valid() {
        assert_eq!(
            verify(&framed(LocalId::CAPACITY, vec![constant(0, 1), ret(0)])),
            Ok(())
        );
    }

    #[test]
    fn a_bool_constant_is_returned() {
        for value in [0, 1] {
            assert_eq!(
                verify(&shaped(
                    Type::Bool,
                    vec![],
                    vec![constant_of(0, Type::Bool, value), ret(0)]
                )),
                Ok(())
            );
        }
    }

    #[test]
    fn a_bool_constant_outside_its_range_is_rejected() {
        assert_eq!(
            verify(&shaped(
                Type::Bool,
                vec![],
                vec![constant_of(0, Type::Bool, 2), ret(0)]
            )),
            Err(Violation::BoolOutOfRange { index: 0, value: 2 })
        );
    }

    #[test]
    fn returning_a_uint64_as_a_bool_is_rejected() {
        assert_eq!(
            verify(&shaped(Type::Bool, vec![], vec![constant(0, 1), ret(0)])),
            Err(Violation::OperandType {
                index: 1,
                position: 0,
                value: ValueId(0),
                found: Type::Uint64,
                expected: Type::Bool,
            })
        );
    }

    #[test]
    fn returning_a_bool_as_a_uint64_is_rejected() {
        assert_eq!(
            verify(&function(vec![constant_of(0, Type::Bool, 1), ret(0)])),
            Err(Violation::OperandType {
                index: 1,
                position: 0,
                value: ValueId(0),
                found: Type::Bool,
                expected: Type::Uint64,
            })
        );
    }

    #[test]
    fn storing_and_loading_a_bool_slot_is_valid() {
        assert_eq!(
            verify(&shaped(
                Type::Bool,
                vec![Type::Bool],
                vec![
                    constant_of(0, Type::Bool, 1),
                    store(0, 0),
                    load(1, 0),
                    ret(1),
                ]
            )),
            Ok(())
        );
    }

    #[test]
    fn storing_a_bool_in_a_uint64_slot_is_rejected() {
        assert_eq!(
            verify(&framed(
                1,
                vec![
                    constant_of(0, Type::Bool, 1),
                    store(0, 0),
                    load(1, 0),
                    ret(1),
                ]
            )),
            Err(Violation::OperandType {
                index: 1,
                position: 0,
                value: ValueId(0),
                found: Type::Bool,
                expected: Type::Uint64,
            })
        );
    }

    #[test]
    fn a_load_carries_its_slots_type() {
        assert_eq!(
            verify(&shaped(
                Type::Uint64,
                vec![Type::Bool],
                vec![
                    constant_of(0, Type::Bool, 1),
                    store(0, 0),
                    load(1, 0),
                    ret(1),
                ]
            )),
            Err(Violation::OperandType {
                index: 3,
                position: 0,
                value: ValueId(1),
                found: Type::Bool,
                expected: Type::Uint64,
            })
        );
    }

    #[test]
    fn a_bool_as_the_left_operand_of_a_binary_is_rejected() {
        assert_eq!(
            verify(&function(vec![
                constant_of(0, Type::Bool, 1),
                constant(1, 2),
                binary(2, BinaryOp::Add, 0, 1),
                ret(2),
            ])),
            Err(Violation::OperandType {
                index: 2,
                position: 0,
                value: ValueId(0),
                found: Type::Bool,
                expected: Type::Uint64,
            })
        );
    }

    #[test]
    fn a_bool_as_the_right_operand_of_a_binary_is_rejected() {
        assert_eq!(
            verify(&function(vec![
                constant(0, 1),
                constant_of(1, Type::Bool, 1),
                binary(2, BinaryOp::Add, 0, 1),
                ret(2),
            ])),
            Err(Violation::OperandType {
                index: 2,
                position: 1,
                value: ValueId(1),
                found: Type::Bool,
                expected: Type::Uint64,
            })
        );
    }

    #[test]
    fn an_operand_out_of_order_is_rejected_before_its_type() {
        assert_eq!(
            verify(&framed(
                1,
                vec![
                    constant(0, 1),
                    constant_of(1, Type::Bool, 0),
                    store(0, 0),
                    ret(1),
                ]
            )),
            Err(Violation::UseOutOfOrder {
                index: 2,
                position: 0,
                value: ValueId(0),
                expected: ValueId(1),
            })
        );
    }

    #[test]
    fn violations_describe_themselves() {
        assert_eq!(
            Violation::UseOutOfOrder {
                index: 3,
                position: 0,
                value: ValueId(1),
                expected: ValueId(0),
            }
            .to_string(),
            "consumed in stack order: instruction 3 uses %1 as operand 0, expected %0"
        );
        assert_eq!(
            Violation::StackUnderflow {
                index: 1,
                needed: 2,
                available: 1,
            }
            .to_string(),
            "consumed in stack order: instruction 1 needs 2 operands but 1 values are live"
        );
        assert_eq!(
            Violation::ValuesLeftOnStack { count: 1 }.to_string(),
            "consumed in stack order: 1 values are left unconsumed"
        );
        assert_eq!(
            Violation::FrameTooLarge { count: 129 }.to_string(),
            "addressed within the frame: 129 locals exceed the capacity of 128"
        );
        assert_eq!(
            Violation::LocalOutOfRange {
                index: 3,
                local: LocalId(2),
                count: 2,
            }
            .to_string(),
            "addressed within the frame: instruction 3 names l2 but the frame has 2 locals"
        );
        assert_eq!(
            Violation::OperandType {
                index: 2,
                position: 0,
                value: ValueId(0),
                found: Type::Bool,
                expected: Type::Uint64,
            }
            .to_string(),
            "well typed: instruction 2 uses %0 of type bool as operand 0, expected uint64"
        );
        assert_eq!(
            Violation::BoolOutOfRange { index: 0, value: 2 }.to_string(),
            "well typed: instruction 0 defines a bool constant of 2, expected 0 or 1"
        );
    }
}
