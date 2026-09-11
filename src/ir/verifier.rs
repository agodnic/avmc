use super::inst::{Function, Inst, ValueId};
use crate::ast;
use crate::typed_ast;

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
        local: typed_ast::LocalId,
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
        found: typed_ast::Type,
        /// The type the instruction needs.
        expected: typed_ast::Type,
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
                typed_ast::LocalId::CAPACITY
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

    if func.locals.len() > typed_ast::LocalId::CAPACITY {
        return Err(Violation::FrameTooLarge {
            count: func.locals.len(),
        });
    }

    // The values defined and not yet consumed, most recent last, each with
    // its type.
    let mut stack: Vec<(ValueId, typed_ast::Type)> = Vec::new();
    // The number of values defined so far: the next definition must be
    // `ValueId(defs)`.
    let mut defs = 0;

    for (index, inst) in func.insts.iter().enumerate() {
        let (dest, ty) = match inst {
            Inst::Const {
                dest, ty, value, ..
            } => {
                if *ty == typed_ast::Type::Bool && *value > 1 {
                    return Err(Violation::BoolOutOfRange {
                        index,
                        value: *value,
                    });
                }
                (dest, *ty)
            }
            Inst::Binary {
                dest, op, lhs, rhs, ..
            } => {
                // `Eq` and `Ne` need only that the operands agree, so the
                // type to expect is the left operand's: the value second
                // from the top. With no such value, `consume` reports the
                // underflow before it looks at a type.
                let ty = typed_ast::operand_type(*op).unwrap_or_else(|| {
                    stack
                        .iter()
                        .rev()
                        .nth(1)
                        .map_or(typed_ast::Type::Uint64, |&(_, ty)| ty)
                });
                consume(&mut stack, index, &[*lhs, *rhs], &[ty, ty])?;
                (dest, typed_ast::result_type(*op))
            }
            Inst::Unary {
                dest, op, operand, ..
            } => {
                match op {
                    ast::UnOp::Not => {
                        consume(&mut stack, index, &[*operand], &[typed_ast::Type::Bool])?
                    }
                }
                (dest, typed_ast::Type::Bool)
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
    stack: &mut Vec<(ValueId, typed_ast::Type)>,
    index: usize,
    operands: &[ValueId],
    expected: &[typed_ast::Type],
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
fn slot(
    locals: &[typed_ast::Type],
    local: typed_ast::LocalId,
    index: usize,
) -> Result<typed_ast::Type, Violation> {
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
