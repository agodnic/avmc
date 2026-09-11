//! Tests for the IR verifier.

use super::inst::{Function, Inst, Program, ValueId};
use super::verifier::{self, Violation};
use crate::ast;
use crate::diag;
use crate::typed_ast;

/// The span every hand-built instruction carries: the verifier ignores
/// spans, so which one it is does not matter.
const SPAN: diag::Span = diag::Span { start: 0, end: 0 };

/// Verifies `func` as the only function of its program.
fn verify(func: Function) -> Result<(), Violation> {
    verify_calling(func, vec![])
}

/// Verifies `func` in a program that defines it first and `callees` after it,
/// so the first callee is `FuncId(1)`.
fn verify_calling(func: Function, callees: Vec<Function>) -> Result<(), Violation> {
    let program = Program {
        funcs: std::iter::once(func).chain(callees).collect(),
    };
    let func = program.funcs.first().expect("the function under test");
    verifier::verify(&program, func)
}

/// A function taking nothing, with an empty frame.
fn function(insts: Vec<Inst>) -> Function {
    framed(0, insts)
}

/// A function taking nothing, whose frame is `locals` slots of
/// `Type::Uint64`, returning `Type::Uint64`.
fn framed(locals: usize, insts: Vec<Inst>) -> Function {
    shaped(
        typed_ast::Type::Uint64,
        vec![],
        vec![typed_ast::Type::Uint64; locals],
        insts,
    )
}

/// A function returning `ret`, taking `params`, with `locals` as its frame.
fn shaped(
    ret: typed_ast::Type,
    params: Vec<typed_ast::Type>,
    locals: Vec<typed_ast::Type>,
    insts: Vec<Inst>,
) -> Function {
    Function {
        name: "approval".to_string(),
        ret,
        params,
        locals,
        insts,
        span: SPAN,
    }
}

fn constant(dest: u32, value: u64) -> Inst {
    constant_of(dest, typed_ast::Type::Uint64, value)
}

fn constant_of(dest: u32, ty: typed_ast::Type, value: u64) -> Inst {
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
        local: typed_ast::LocalId(local),
        value: ValueId(value),
        span: SPAN,
    }
}

fn load(dest: u32, local: u8) -> Inst {
    Inst::Load {
        dest: ValueId(dest),
        local: typed_ast::LocalId(local),
        span: SPAN,
    }
}

fn load_param(dest: u32, param: u8) -> Inst {
    Inst::LoadParam {
        dest: ValueId(dest),
        param: typed_ast::ParamId(param),
        span: SPAN,
    }
}

fn binary(dest: u32, op: ast::BinOp, lhs: u32, rhs: u32) -> Inst {
    Inst::Binary {
        dest: ValueId(dest),
        op,
        lhs: ValueId(lhs),
        rhs: ValueId(rhs),
        span: SPAN,
    }
}

fn call(dest: u32, callee: u32, args: Vec<u32>) -> Inst {
    Inst::Call {
        dest: ValueId(dest),
        callee: typed_ast::FuncId(callee),
        args: args.into_iter().map(ValueId).collect(),
        span: SPAN,
    }
}

/// A callee taking `params` and returning `ret`. Only its signature matters:
/// the function under test is the caller.
fn callee(ret: typed_ast::Type, params: Vec<typed_ast::Type>) -> Function {
    shaped(ret, params, vec![], vec![])
}

fn unary(dest: u32, op: ast::UnOp, operand: u32) -> Inst {
    Inst::Unary {
        dest: ValueId(dest),
        op,
        operand: ValueId(operand),
        span: SPAN,
    }
}

#[test]
fn const_then_return_is_valid() {
    assert_eq!(verify(function(vec![constant(0, 1), ret(0)])), Ok(()));
}

#[test]
fn a_binary_over_two_constants_is_valid() {
    assert_eq!(
        verify(function(vec![
            constant(0, 1),
            constant(1, 2),
            binary(2, ast::BinOp::Add, 0, 1),
            ret(2),
        ])),
        Ok(())
    );
}

#[test]
fn a_right_leaning_tree_is_valid() {
    // `1 + 2 * 3`, which the definition-order invariant used to reject.
    assert_eq!(
        verify(function(vec![
            constant(0, 1),
            constant(1, 2),
            constant(2, 3),
            binary(3, ast::BinOp::Mul, 1, 2),
            binary(4, ast::BinOp::Add, 0, 3),
            ret(4),
        ])),
        Ok(())
    );
}

#[test]
fn swapped_operands_are_rejected() {
    assert_eq!(
        verify(function(vec![
            constant(0, 1),
            constant(1, 2),
            binary(2, ast::BinOp::Sub, 1, 0),
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
        verify(function(vec![
            constant(0, 1),
            binary(1, ast::BinOp::Add, 0, 0),
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
        verify(function(vec![constant(0, 1), constant(1, 2), ret(1)])),
        Err(Violation::ValuesLeftOnStack { count: 1 })
    );
}

#[test]
fn using_a_value_that_is_not_on_top_is_rejected() {
    assert_eq!(
        verify(function(vec![constant(0, 1), constant(1, 2), ret(0)])),
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
        verify(function(vec![constant(1, 1), ret(1)])),
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
        verify(function(vec![ret(0)])),
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
        verify(function(vec![constant(0, 1)])),
        Err(Violation::MissingReturn)
    );
}

#[test]
fn return_that_is_not_last_is_rejected() {
    assert_eq!(
        verify(function(vec![constant(0, 1), ret(0), constant(1, 2)])),
        Err(Violation::ReturnNotLast { index: 1 })
    );
}

#[test]
fn storing_and_loading_a_slot_is_valid() {
    assert_eq!(
        verify(framed(
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
        verify(framed(
            2,
            vec![
                constant(0, 1),
                constant(1, 2),
                binary(2, ast::BinOp::Add, 0, 1),
                store(0, 2),
                load(3, 0),
                constant(4, 3),
                binary(5, ast::BinOp::Mul, 3, 4),
                store(1, 5),
                load(6, 1),
                load(7, 0),
                binary(8, ast::BinOp::Sub, 6, 7),
                ret(8),
            ]
        )),
        Ok(())
    );
}

#[test]
fn storing_a_value_that_is_not_on_top_is_rejected() {
    assert_eq!(
        verify(framed(
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
        verify(framed(1, vec![store(0, 0), ret(0)])),
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
        verify(function(vec![load(0, 0), ret(0)])),
        Err(Violation::LocalOutOfRange {
            index: 0,
            local: typed_ast::LocalId(0),
            count: 0,
        })
    );
}

#[test]
fn storing_past_the_frame_is_rejected() {
    assert_eq!(
        verify(framed(
            2,
            vec![constant(0, 1), store(2, 0), constant(1, 2), ret(1)]
        )),
        Err(Violation::LocalOutOfRange {
            index: 1,
            local: typed_ast::LocalId(2),
            count: 2,
        })
    );
}

#[test]
fn a_parameter_load_has_the_parameters_type() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Uint64,
            vec![typed_ast::Type::Bool],
            vec![],
            vec![
                load_param(0, 0),
                constant(1, 1),
                binary(2, ast::BinOp::Add, 0, 1),
                ret(2),
            ]
        )),
        Err(Violation::OperandType {
            index: 2,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}

#[test]
fn a_parameter_out_of_range_is_a_violation() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Uint64,
            vec![typed_ast::Type::Uint64],
            vec![],
            vec![load_param(0, 1), ret(0)]
        )),
        Err(Violation::ParamOutOfRange {
            index: 0,
            param: typed_ast::ParamId(1),
            count: 1,
        })
    );
}

#[test]
fn too_many_parameters_is_a_violation() {
    let count = typed_ast::ParamId::CAPACITY + 1;
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Uint64,
            vec![typed_ast::Type::Uint64; count],
            vec![],
            vec![constant(0, 1), ret(0)]
        )),
        Err(Violation::TooManyParams { count })
    );
}

#[test]
fn a_frame_past_the_capacity_is_rejected() {
    let count = typed_ast::LocalId::CAPACITY + 1;
    assert_eq!(
        verify(framed(count, vec![constant(0, 1), ret(0)])),
        Err(Violation::FrameTooLarge { count })
    );
}

#[test]
fn a_frame_at_the_capacity_is_valid() {
    assert_eq!(
        verify(framed(
            typed_ast::LocalId::CAPACITY,
            vec![constant(0, 1), ret(0)]
        )),
        Ok(())
    );
}

#[test]
fn a_bool_constant_is_returned() {
    for value in [0, 1] {
        assert_eq!(
            verify(shaped(
                typed_ast::Type::Bool,
                vec![],
                vec![],
                vec![constant_of(0, typed_ast::Type::Bool, value), ret(0)]
            )),
            Ok(())
        );
    }
}

#[test]
fn a_bool_constant_outside_its_range_is_rejected() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![constant_of(0, typed_ast::Type::Bool, 2), ret(0)]
        )),
        Err(Violation::BoolOutOfRange { index: 0, value: 2 })
    );
}

#[test]
fn returning_a_uint64_as_a_bool_is_rejected() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![constant(0, 1), ret(0)]
        )),
        Err(Violation::OperandType {
            index: 1,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Uint64,
            expected: typed_ast::Type::Bool,
        })
    );
}

#[test]
fn returning_a_bool_as_a_uint64_is_rejected() {
    assert_eq!(
        verify(function(vec![
            constant_of(0, typed_ast::Type::Bool, 1),
            ret(0)
        ])),
        Err(Violation::OperandType {
            index: 1,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}

#[test]
fn storing_and_loading_a_bool_slot_is_valid() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![typed_ast::Type::Bool],
            vec![
                constant_of(0, typed_ast::Type::Bool, 1),
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
        verify(framed(
            1,
            vec![
                constant_of(0, typed_ast::Type::Bool, 1),
                store(0, 0),
                load(1, 0),
                ret(1),
            ]
        )),
        Err(Violation::OperandType {
            index: 1,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}

#[test]
fn a_load_carries_its_slots_type() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Uint64,
            vec![],
            vec![typed_ast::Type::Bool],
            vec![
                constant_of(0, typed_ast::Type::Bool, 1),
                store(0, 0),
                load(1, 0),
                ret(1),
            ]
        )),
        Err(Violation::OperandType {
            index: 3,
            position: 0,
            value: ValueId(1),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}

#[test]
fn a_bool_as_the_left_operand_of_a_binary_is_rejected() {
    assert_eq!(
        verify(function(vec![
            constant_of(0, typed_ast::Type::Bool, 1),
            constant(1, 2),
            binary(2, ast::BinOp::Add, 0, 1),
            ret(2),
        ])),
        Err(Violation::OperandType {
            index: 2,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}

#[test]
fn a_bool_as_the_right_operand_of_a_binary_is_rejected() {
    assert_eq!(
        verify(function(vec![
            constant(0, 1),
            constant_of(1, typed_ast::Type::Bool, 1),
            binary(2, ast::BinOp::Add, 0, 1),
            ret(2),
        ])),
        Err(Violation::OperandType {
            index: 2,
            position: 1,
            value: ValueId(1),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}

#[test]
fn an_operand_out_of_order_is_rejected_before_its_type() {
    assert_eq!(
        verify(framed(
            1,
            vec![
                constant(0, 1),
                constant_of(1, typed_ast::Type::Bool, 0),
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

/// The six comparisons, which take two `uint64` and produce a `bool`.
const COMPARISONS: [ast::BinOp; 6] = [
    ast::BinOp::Eq,
    ast::BinOp::Ne,
    ast::BinOp::Lt,
    ast::BinOp::Le,
    ast::BinOp::Gt,
    ast::BinOp::Ge,
];

#[test]
fn a_comparison_of_two_uint64s_is_valid() {
    for op in COMPARISONS {
        assert_eq!(
            verify(shaped(
                typed_ast::Type::Bool,
                vec![],
                vec![],
                vec![constant(0, 1), constant(1, 2), binary(2, op, 0, 1), ret(2)]
            )),
            Ok(()),
            "{op:?}"
        );
    }
}

#[test]
fn returning_a_comparison_as_a_uint64_is_rejected() {
    assert_eq!(
        verify(function(vec![
            constant(0, 1),
            constant(1, 2),
            binary(2, ast::BinOp::Lt, 0, 1),
            ret(2),
        ])),
        Err(Violation::OperandType {
            index: 3,
            position: 0,
            value: ValueId(2),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}

#[test]
fn equality_over_uint64s_still_yields_a_bool() {
    assert_eq!(
        verify(function(vec![
            constant(0, 1),
            constant(1, 2),
            binary(2, ast::BinOp::Eq, 0, 1),
            ret(2),
        ])),
        Err(Violation::OperandType {
            index: 3,
            position: 0,
            value: ValueId(2),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}

#[test]
fn equality_over_two_bools_is_valid() {
    for op in [ast::BinOp::Eq, ast::BinOp::Ne] {
        assert_eq!(
            verify(shaped(
                typed_ast::Type::Bool,
                vec![],
                vec![],
                vec![
                    constant_of(0, typed_ast::Type::Bool, 1),
                    constant_of(1, typed_ast::Type::Bool, 0),
                    binary(2, op, 0, 1),
                    ret(2),
                ]
            )),
            Ok(()),
            "{op:?}"
        );
    }
}

#[test]
fn ordering_two_bools_is_rejected() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![
                constant_of(0, typed_ast::Type::Bool, 1),
                constant_of(1, typed_ast::Type::Bool, 0),
                binary(2, ast::BinOp::Lt, 0, 1),
                ret(2),
            ]
        )),
        Err(Violation::OperandType {
            index: 2,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}

#[test]
fn comparing_a_uint64_with_a_bool_is_rejected() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![
                constant(0, 1),
                constant_of(1, typed_ast::Type::Bool, 1),
                binary(2, ast::BinOp::Eq, 0, 1),
                ret(2),
            ]
        )),
        Err(Violation::OperandType {
            index: 2,
            position: 1,
            value: ValueId(1),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}

#[test]
fn comparing_a_bool_with_a_uint64_is_rejected() {
    // The left operand fixes the type the right one must have.
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![
                constant_of(0, typed_ast::Type::Bool, 1),
                constant(1, 1),
                binary(2, ast::BinOp::Eq, 0, 1),
                ret(2),
            ]
        )),
        Err(Violation::OperandType {
            index: 2,
            position: 1,
            value: ValueId(1),
            found: typed_ast::Type::Uint64,
            expected: typed_ast::Type::Bool,
        })
    );
}

#[test]
fn an_equality_without_enough_live_values_is_rejected() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![
                constant_of(0, typed_ast::Type::Bool, 1),
                binary(1, ast::BinOp::Eq, 0, 1),
                ret(2),
            ]
        )),
        Err(Violation::StackUnderflow {
            index: 1,
            needed: 2,
            available: 1,
        })
    );
}

#[test]
fn logic_over_two_bools_is_valid() {
    for op in [ast::BinOp::And, ast::BinOp::Or] {
        assert_eq!(
            verify(shaped(
                typed_ast::Type::Bool,
                vec![],
                vec![],
                vec![
                    constant_of(0, typed_ast::Type::Bool, 1),
                    constant_of(1, typed_ast::Type::Bool, 0),
                    binary(2, op, 0, 1),
                    ret(2),
                ]
            )),
            Ok(()),
            "{op:?}"
        );
    }
}

#[test]
fn a_uint64_as_an_operand_of_logic_is_rejected() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![
                constant(0, 1),
                constant_of(1, typed_ast::Type::Bool, 1),
                binary(2, ast::BinOp::And, 0, 1),
                ret(2),
            ]
        )),
        Err(Violation::OperandType {
            index: 2,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Uint64,
            expected: typed_ast::Type::Bool,
        })
    );
}

#[test]
fn negating_a_bool_is_valid() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![
                constant_of(0, typed_ast::Type::Bool, 1),
                unary(1, ast::UnOp::Not, 0),
                ret(1),
            ]
        )),
        Ok(())
    );
}

#[test]
fn negating_a_uint64_is_rejected() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![constant(0, 1), unary(1, ast::UnOp::Not, 0), ret(1)]
        )),
        Err(Violation::OperandType {
            index: 1,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Uint64,
            expected: typed_ast::Type::Bool,
        })
    );
}

#[test]
fn negating_without_a_live_value_is_rejected() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![unary(0, ast::UnOp::Not, 0), ret(1)]
        )),
        Err(Violation::StackUnderflow {
            index: 0,
            needed: 1,
            available: 0,
        })
    );
}

#[test]
fn negating_a_value_that_is_not_on_top_is_rejected() {
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![
                constant_of(0, typed_ast::Type::Bool, 1),
                constant_of(1, typed_ast::Type::Bool, 0),
                unary(2, ast::UnOp::Not, 0),
                ret(2),
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
fn a_comparison_joined_by_logic_is_valid() {
    // `!(1 < 2 && true)`.
    assert_eq!(
        verify(shaped(
            typed_ast::Type::Bool,
            vec![],
            vec![],
            vec![
                constant(0, 1),
                constant(1, 2),
                binary(2, ast::BinOp::Lt, 0, 1),
                constant_of(3, typed_ast::Type::Bool, 1),
                binary(4, ast::BinOp::And, 2, 3),
                unary(5, ast::UnOp::Not, 4),
                ret(5),
            ]
        )),
        Ok(())
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
            local: typed_ast::LocalId(2),
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
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        }
        .to_string(),
        "well typed: instruction 2 uses %0 of type bool as operand 0, expected uint64"
    );
    assert_eq!(
        Violation::BoolOutOfRange { index: 0, value: 2 }.to_string(),
        "well typed: instruction 0 defines a bool constant of 2, expected 0 or 1"
    );
}

#[test]
fn a_call_consumes_its_arguments_in_order() {
    let insts = vec![
        constant(0, 1),
        constant(1, 2),
        call(2, 1, vec![0, 1]),
        ret(2),
    ];
    assert_eq!(
        verify_calling(
            function(insts),
            vec![callee(
                typed_ast::Type::Uint64,
                vec![typed_ast::Type::Uint64; 2]
            )]
        ),
        Ok(())
    );

    let swapped = vec![
        constant(0, 1),
        constant(1, 2),
        call(2, 1, vec![1, 0]),
        ret(2),
    ];
    assert_eq!(
        verify_calling(
            function(swapped),
            vec![callee(
                typed_ast::Type::Uint64,
                vec![typed_ast::Type::Uint64; 2]
            )]
        ),
        Err(Violation::UseOutOfOrder {
            index: 2,
            position: 0,
            value: ValueId(1),
            expected: ValueId(0),
        })
    );
}

#[test]
fn a_call_argument_of_the_wrong_type_is_a_violation() {
    assert_eq!(
        verify_calling(
            function(vec![constant(0, 1), call(1, 1, vec![0]), ret(1)]),
            vec![callee(typed_ast::Type::Uint64, vec![typed_ast::Type::Bool])]
        ),
        Err(Violation::OperandType {
            index: 1,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Uint64,
            expected: typed_ast::Type::Bool,
        })
    );
}

#[test]
fn a_call_with_too_few_live_values_is_a_violation() {
    assert_eq!(
        verify_calling(
            function(vec![constant(0, 1), call(1, 1, vec![0, 1]), ret(1)]),
            vec![callee(
                typed_ast::Type::Uint64,
                vec![typed_ast::Type::Uint64; 2]
            )]
        ),
        Err(Violation::StackUnderflow {
            index: 1,
            needed: 2,
            available: 1,
        })
    );
}

#[test]
fn an_unknown_callee_is_a_violation() {
    assert_eq!(
        verify(function(vec![constant(0, 1), call(1, 1, vec![0]), ret(1)])),
        Err(Violation::UnknownCallee {
            index: 1,
            callee: typed_ast::FuncId(1),
        })
    );
}

#[test]
fn a_call_defines_the_callees_return_type() {
    assert_eq!(
        verify_calling(
            function(vec![call(0, 1, vec![]), ret(0)]),
            vec![callee(typed_ast::Type::Bool, vec![])]
        ),
        Err(Violation::OperandType {
            index: 1,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        })
    );
}
