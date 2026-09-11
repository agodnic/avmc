use super::inst::{Function, Inst, Program, ValueId};
use super::violation::Violation;
use crate::ast;
use crate::typed_ast;

/// Checks the v0 IR invariant, returning the first violation. `program` is
/// the unit `func` belongs to, which holds the functions it calls.
pub fn verify(program: &Program, func: &Function) -> Result<(), Violation> {
    verify_return(func)?;

    if func.params.len() > typed_ast::ParamId::CAPACITY {
        return Err(Violation::TooManyParams {
            count: func.params.len(),
        });
    }
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
            Inst::LoadParam { dest, param, .. } => {
                let ty = parameter(&func.params, *param, index)?;
                (dest, ty)
            }
            Inst::Call {
                dest, callee, args, ..
            } => {
                let called = callee_of(program, *callee, index)?;
                consume(&mut stack, index, args, &called.params)?;
                (dest, called.ret)
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

/// Checks that `param` is one the function takes, returning its type.
fn parameter(
    params: &[typed_ast::Type],
    param: typed_ast::ParamId,
    index: usize,
) -> Result<typed_ast::Type, Violation> {
    params
        .get(usize::from(param.0))
        .copied()
        .ok_or(Violation::ParamOutOfRange {
            index,
            param,
            count: params.len(),
        })
}

/// Checks that `callee` is a function the program defines, returning it.
fn callee_of(
    program: &Program,
    callee: typed_ast::FuncId,
    index: usize,
) -> Result<&Function, Violation> {
    usize::try_from(callee.0)
        .ok()
        .and_then(|position| program.funcs.get(position))
        .ok_or(Violation::UnknownCallee { index, callee })
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
