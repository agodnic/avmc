//! Tests for the emission pass.

use super::pass;
use crate::ast;
use crate::diag;
use crate::ir;
use crate::lower;
use crate::testing;
use crate::typed_ast;

/// The span every hand-built instruction carries.
const ZERO: diag::Span = diag::Span { start: 0, end: 0 };

/// Emits `source`, asserting that it produced no diagnostics.
fn emit_ok(source: &str) -> String {
    let mut diags = diag::Sink::default();
    let teal = pipeline(source, &mut diags);
    assert!(diags.is_empty());
    teal.expect("emission succeeded")
}

/// Emits `source`, asserting that it emitted nothing, and returning the
/// diagnostics in the order they were reported.
fn emit_err(source: &str) -> Vec<diag::Entry> {
    let mut diags = diag::Sink::default();
    assert_eq!(pipeline(source, &mut diags), None);
    diags.iter().cloned().collect()
}

fn pipeline(source: &str, diags: &mut diag::Sink) -> Option<String> {
    let ir = lower::lower(&testing::lex_parse_check(source), diags).expect("lowering succeeded");
    pass::emit(&ir, diags)
}

/// Emits a hand-built program: one function named `approval` returning
/// `ret`, with `locals` as its frame and `insts` as its body, at the zero
/// span throughout. `emit_ok` and `emit_err` go through source, which
/// cannot produce a frame yet.
fn hand_built(ret: typed_ast::Type, locals: Vec<typed_ast::Type>, insts: Vec<ir::Inst>) -> String {
    let program = ir::Program {
        funcs: vec![ir::Function {
            name: pass::ENTRY_POINT.to_string(),
            ret,
            params: vec![],
            locals,
            insts,
            span: ZERO,
        }],
    };
    let mut diags = diag::Sink::default();
    let teal = pass::emit(&program, &mut diags);
    assert!(diags.is_empty());
    teal.expect("emission succeeded")
}

fn constant(dest: u32, value: u64) -> ir::Inst {
    ir::Inst::Const {
        dest: ir::ValueId(dest),
        value: ir::ConstValue::Uint64(value),
        span: ZERO,
    }
}

fn constant_bool(dest: u32, value: bool) -> ir::Inst {
    ir::Inst::Const {
        dest: ir::ValueId(dest),
        value: ir::ConstValue::Bool(value),
        span: ZERO,
    }
}

fn binary(dest: u32, op: ast::BinOp, lhs: u32, rhs: u32) -> ir::Inst {
    ir::Inst::Binary {
        dest: ir::ValueId(dest),
        op,
        lhs: ir::ValueId(lhs),
        rhs: ir::ValueId(rhs),
        span: ZERO,
    }
}

fn unary(dest: u32, op: ast::UnOp, operand: u32) -> ir::Inst {
    ir::Inst::Unary {
        dest: ir::ValueId(dest),
        op,
        operand: ir::ValueId(operand),
        span: ZERO,
    }
}

fn store(local: u8, value: u32) -> ir::Inst {
    ir::Inst::Store {
        local: typed_ast::LocalId(local),
        value: ir::ValueId(value),
        span: ZERO,
    }
}

fn load(dest: u32, local: u8) -> ir::Inst {
    ir::Inst::Load {
        dest: ir::ValueId(dest),
        local: typed_ast::LocalId(local),
        span: ZERO,
    }
}

fn load_param(dest: u32, param: u8) -> ir::Inst {
    ir::Inst::LoadParam {
        dest: ir::ValueId(dest),
        param: typed_ast::ParamId(param),
        span: ZERO,
    }
}

fn ret(value: u32) -> ir::Inst {
    ir::Inst::Return {
        value: ir::ValueId(value),
        span: ZERO,
    }
}

/// `var x uint64 = 1; return x`, as the next slice will lower it.
fn one_slot() -> Vec<ir::Inst> {
    vec![constant(0, 1), store(0, 0), load(1, 0), ret(1)]
}

#[test]
fn a_frame_slot_is_allocated_and_addressed() {
    assert_eq!(
        hand_built(
            typed_ast::Type::Uint64,
            vec![typed_ast::Type::Uint64],
            one_slot()
        ),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 0\n\
         pushint 1\n\
         frame_bury 0\n\
         frame_dig 0\n\
         retsub\n"
    );
}

#[test]
fn two_frame_slots() {
    // `var x uint64 = 1 + 2; var y uint64 = x * 3; return y - x`.
    let insts = vec![
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
    ];
    assert_eq!(
        hand_built(
            typed_ast::Type::Uint64,
            vec![typed_ast::Type::Uint64; 2],
            insts
        ),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 0\n\
         pushint 0\n\
         pushint 1\n\
         pushint 2\n\
         +\n\
         frame_bury 0\n\
         frame_dig 0\n\
         pushint 3\n\
         *\n\
         frame_bury 1\n\
         frame_dig 1\n\
         frame_dig 0\n\
         -\n\
         retsub\n"
    );
}

#[test]
fn a_bool_constant_is_returned() {
    assert_eq!(
        hand_built(
            typed_ast::Type::Bool,
            vec![],
            vec![constant_bool(0, true), ret(0)]
        ),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 1\n\
         retsub\n"
    );
}

#[test]
fn a_boolean_literal_is_compiled() {
    assert_eq!(
        emit_ok("func approval() bool { return true }"),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 1\n\
         retsub\n"
    );
    assert_eq!(
        emit_ok("func approval() bool { return false }"),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 0\n\
         retsub\n"
    );
}

#[test]
fn a_bool_slot_starts_as_false() {
    // `var ok bool = true; return ok`, as a later slice will lower it.
    let insts = vec![constant_bool(0, true), store(0, 0), load(1, 0), ret(1)];
    assert_eq!(
        hand_built(typed_ast::Type::Bool, vec![typed_ast::Type::Bool], insts),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 0\n\
         pushint 1\n\
         frame_bury 0\n\
         frame_dig 0\n\
         retsub\n"
    );
}

/// The five lines every hand-built program starts with.
const PROLOGUE: &str = "#pragma version 13\n\
                        callsub approval\n\
                        return\n\
                        approval:\n\
                        proto 0 1\n";

#[test]
fn a_comparison_emits_its_mnemonic() {
    let cases = [
        (ast::BinOp::Eq, "=="),
        (ast::BinOp::Ne, "!="),
        (ast::BinOp::Lt, "<"),
        (ast::BinOp::Le, "<="),
        (ast::BinOp::Gt, ">"),
        (ast::BinOp::Ge, ">="),
    ];
    for (op, mnemonic) in cases {
        let insts = vec![constant(0, 1), constant(1, 2), binary(2, op, 0, 1), ret(2)];
        assert_eq!(
            hand_built(typed_ast::Type::Bool, vec![], insts),
            format!("{PROLOGUE}pushint 1\npushint 2\n{mnemonic}\nretsub\n"),
            "{op:?}"
        );
    }
}

#[test]
fn logic_emits_its_mnemonic() {
    for (op, mnemonic) in [(ast::BinOp::And, "&&"), (ast::BinOp::Or, "||")] {
        let insts = vec![
            constant_bool(0, true),
            constant_bool(1, false),
            binary(2, op, 0, 1),
            ret(2),
        ];
        assert_eq!(
            hand_built(typed_ast::Type::Bool, vec![], insts),
            format!("{PROLOGUE}pushint 1\npushint 0\n{mnemonic}\nretsub\n"),
            "{op:?}"
        );
    }
}

#[test]
fn negation_emits_its_mnemonic() {
    let insts = vec![constant_bool(0, true), unary(1, ast::UnOp::Not, 0), ret(1)];
    assert_eq!(
        hand_built(typed_ast::Type::Bool, vec![], insts),
        format!("{PROLOGUE}pushint 1\n!\nretsub\n")
    );
}

#[test]
fn a_comparison_joined_by_logic() {
    // `!(1 < 2 && true)`.
    let insts = vec![
        constant(0, 1),
        constant(1, 2),
        binary(2, ast::BinOp::Lt, 0, 1),
        constant_bool(3, true),
        binary(4, ast::BinOp::And, 2, 3),
        unary(5, ast::UnOp::Not, 4),
        ret(5),
    ];
    assert_eq!(
        hand_built(typed_ast::Type::Bool, vec![], insts),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 1\n\
         pushint 2\n\
         <\n\
         pushint 1\n\
         &&\n\
         !\n\
         retsub\n"
    );
}

#[test]
fn example_program() {
    assert_eq!(
        emit_ok(testing::EXAMPLE),
        "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 1\nretsub\n"
    );
}

#[test]
fn zero() {
    assert_eq!(
        emit_ok("func approval() uint64 { return 0 }"),
        "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 0\nretsub\n"
    );
}

#[test]
fn largest_uint64() {
    assert_eq!(
        emit_ok("func approval() uint64 { return 18446744073709551615 }"),
        "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 18446744073709551615\nretsub\n"
    );
}

#[test]
fn arithmetic() {
    assert_eq!(
        emit_ok("func approval() uint64 { return (1 + 2) * 3 - 4 / 5 }"),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 1\n\
         pushint 2\n\
         +\n\
         pushint 3\n\
         *\n\
         pushint 4\n\
         pushint 5\n\
         /\n\
         -\n\
         retsub\n"
    );
}

#[test]
fn the_remainder_opcode() {
    assert_eq!(
        emit_ok("func approval() uint64 { return 7 % 4 }"),
        "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 7\npushint 4\n%\nretsub\n"
    );
}

#[test]
fn an_unreached_function_is_not_emitted() {
    assert_eq!(
        emit_ok("func f(a uint64) uint64 { return a } func approval() uint64 { return 1 }"),
        "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 1\nretsub\n"
    );
}

/// The subroutine for `func`, as the only function of its program.
fn subroutine(func: ir::Function) -> String {
    let program = ir::Program { funcs: vec![func] };
    let func = program.funcs.first().expect("the function under test");
    pass::function(&program, func)
}

#[test]
fn the_subroutine_of_a_function_with_parameters() {
    // `func add(a uint64, b uint64) uint64 { var sum uint64 = a + b
    //  return sum }`.
    let func = ir::Function {
        name: "add".to_string(),
        ret: typed_ast::Type::Uint64,
        params: vec![typed_ast::Type::Uint64; 2],
        locals: vec![typed_ast::Type::Uint64],
        insts: vec![
            load_param(0, 0),
            load_param(1, 1),
            binary(2, ast::BinOp::Add, 0, 1),
            store(0, 2),
            load(3, 0),
            ret(3),
        ],
        span: ZERO,
    };
    assert_eq!(
        subroutine(func),
        "add:\n\
         proto 2 1\n\
         pushint 0\n\
         frame_dig -2\n\
         frame_dig -1\n\
         +\n\
         frame_bury 0\n\
         frame_dig 0\n\
         retsub\n"
    );
}

#[test]
fn an_entry_point_with_parameters_is_reported() {
    let source = "func approval(a uint64) uint64 { return a }";
    assert_eq!(
        emit_err(source),
        vec![diag::Entry {
            kind: diag::Kind::EntryPointTakesParameters { name: "approval" },
            span: testing::span_of(source, source, 0),
        }]
    );
}

#[test]
fn missing_entry_point() {
    assert_eq!(
        emit_err("func f() uint64 { return 1 }"),
        vec![diag::Entry {
            kind: diag::Kind::MissingEntryPoint { name: "approval" },
            span: diag::Span { start: 0, end: 0 },
        }]
    );
}

#[test]
fn empty_input() {
    assert_eq!(
        emit_err(""),
        vec![diag::Entry {
            kind: diag::Kind::MissingEntryPoint { name: "approval" },
            span: diag::Span { start: 0, end: 0 },
        }]
    );
}

/// The example program of the comparison milestone.
const COMPARISONS: &str = "func approval() bool {\n  var x uint64 = 1 + 2\n  \
                           var big bool = x * 2 >= 6\n  return big == (x != 4)\n}\n";

#[test]
fn emits_the_comparisons_program() {
    assert_eq!(
        emit_ok(COMPARISONS),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 0\n\
         pushint 0\n\
         pushint 1\n\
         pushint 2\n\
         +\n\
         frame_bury 0\n\
         frame_dig 0\n\
         pushint 2\n\
         *\n\
         pushint 6\n\
         >=\n\
         frame_bury 1\n\
         frame_dig 1\n\
         frame_dig 0\n\
         pushint 4\n\
         !=\n\
         ==\n\
         retsub\n"
    );
}

/// The example program of the comparison-and-logic milestone.
const LOGIC: &str = "func approval() bool {\n  var x uint64 = 1 + 2\n  \
                     var odd bool = x % 2 == 1\n  \
                     return !(x > 5) && (odd || x == 4)\n}\n";

#[test]
fn emits_the_logic_program() {
    assert_eq!(
        emit_ok(LOGIC),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 0\n\
         pushint 0\n\
         pushint 1\n\
         pushint 2\n\
         +\n\
         frame_bury 0\n\
         frame_dig 0\n\
         pushint 2\n\
         %\n\
         pushint 1\n\
         ==\n\
         frame_bury 1\n\
         frame_dig 0\n\
         pushint 5\n\
         >\n\
         !\n\
         frame_dig 1\n\
         frame_dig 0\n\
         pushint 4\n\
         ==\n\
         ||\n\
         &&\n\
         retsub\n"
    );
}

/// The example program of the calls milestone.
const CALLS: &str = "func approval() uint64 {\n\treturn add(1, double(2))\n}\n\n\
                     func add(a uint64, b uint64) uint64 {\n\treturn a + b\n}\n\n\
                     func double(x uint64) uint64 {\n\treturn x * 2\n}\n";

#[test]
fn a_call_emits_callsub() {
    assert_eq!(
        emit_ok(CALLS),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         pushint 1\n\
         pushint 2\n\
         callsub double\n\
         callsub add\n\
         retsub\n\
         add:\n\
         proto 2 1\n\
         frame_dig -2\n\
         frame_dig -1\n\
         +\n\
         retsub\n\
         double:\n\
         proto 1 1\n\
         frame_dig -1\n\
         pushint 2\n\
         *\n\
         retsub\n"
    );
}

#[test]
fn reached_functions_follow_the_entry_point_in_source_order() {
    let source = "func b() uint64 { return 1 } \
                  func approval() uint64 { return a() } \
                  func a() uint64 { return b() }";
    assert_eq!(
        emit_ok(source),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         callsub a\n\
         retsub\n\
         b:\n\
         proto 0 1\n\
         pushint 1\n\
         retsub\n\
         a:\n\
         proto 0 1\n\
         callsub b\n\
         retsub\n"
    );
}

#[test]
fn a_function_reached_twice_is_emitted_once() {
    let source = "func approval() uint64 { return f() + f() } func f() uint64 { return 1 }";
    assert_eq!(
        emit_ok(source),
        "#pragma version 13\n\
         callsub approval\n\
         return\n\
         approval:\n\
         proto 0 1\n\
         callsub f\n\
         callsub f\n\
         +\n\
         retsub\n\
         f:\n\
         proto 0 1\n\
         pushint 1\n\
         retsub\n"
    );
}
