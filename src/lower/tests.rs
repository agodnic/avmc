//! Tests for the lowering pass.

use super::pass;
use crate::ast;
use crate::diag;
use crate::ir;
use crate::testing;
use crate::typed_ast;

/// Lowers `source`, asserting that it produced no diagnostics.
fn lower_ok(source: &str) -> ir::Program {
    let mut diags = diag::Sink::default();
    let program = pass::lower(&testing::lex_parse_check(source), &mut diags);
    assert!(diags.is_empty());
    program.expect("lowering succeeded")
}

#[test]
fn example_program() {
    let source = "func approval() uint64 { return 1 }";
    assert_eq!(
        lower_ok(source),
        ir::Program {
            funcs: vec![ir::Function {
                name: "approval".to_string(),
                ret: typed_ast::Type::Uint64,
                params: vec![],
                locals: vec![],
                insts: vec![
                    ir::Inst::Const {
                        dest: ir::ValueId(0),
                        ty: typed_ast::Type::Uint64,
                        value: 1,
                        span: testing::span_of(source, "1", 0),
                    },
                    ir::Inst::Return {
                        value: ir::ValueId(0),
                        span: testing::span_of(source, "return 1", 0),
                    },
                ],
                span: testing::span_of(source, "func approval() uint64 { return 1 }", 0),
            }],
        }
    );
}

#[test]
fn arithmetic_lowers_in_post_order() {
    let source = "func approval() uint64 { return 1 + 2 * 3 }";
    assert_eq!(
        lower_ok(source).funcs[0].insts,
        vec![
            ir::Inst::Const {
                dest: ir::ValueId(0),
                ty: typed_ast::Type::Uint64,
                value: 1,
                span: testing::span_of(source, "1", 0),
            },
            ir::Inst::Const {
                dest: ir::ValueId(1),
                ty: typed_ast::Type::Uint64,
                value: 2,
                span: testing::span_of(source, "2", 0),
            },
            ir::Inst::Const {
                dest: ir::ValueId(2),
                ty: typed_ast::Type::Uint64,
                value: 3,
                span: testing::span_of(source, "3", 0),
            },
            ir::Inst::Binary {
                dest: ir::ValueId(3),
                op: ast::BinOp::Mul,
                lhs: ir::ValueId(1),
                rhs: ir::ValueId(2),
                span: testing::span_of(source, "2 * 3", 0),
            },
            ir::Inst::Binary {
                dest: ir::ValueId(4),
                op: ast::BinOp::Add,
                lhs: ir::ValueId(0),
                rhs: ir::ValueId(3),
                span: testing::span_of(source, "1 + 2 * 3", 0),
            },
            ir::Inst::Return {
                value: ir::ValueId(4),
                span: testing::span_of(source, "return 1 + 2 * 3", 0),
            },
        ]
    );
}

#[test]
fn parenthesised_arithmetic() {
    let source = "func approval() uint64 { return (1 + 2) * 3 - 4 / 5 }";
    assert_eq!(
        lower_ok(source).funcs[0].insts,
        vec![
            ir::Inst::Const {
                dest: ir::ValueId(0),
                ty: typed_ast::Type::Uint64,
                value: 1,
                span: testing::span_of(source, "1", 0),
            },
            ir::Inst::Const {
                dest: ir::ValueId(1),
                ty: typed_ast::Type::Uint64,
                value: 2,
                span: testing::span_of(source, "2", 0),
            },
            ir::Inst::Binary {
                dest: ir::ValueId(2),
                op: ast::BinOp::Add,
                lhs: ir::ValueId(0),
                rhs: ir::ValueId(1),
                span: testing::span_of(source, "1 + 2", 0),
            },
            ir::Inst::Const {
                dest: ir::ValueId(3),
                ty: typed_ast::Type::Uint64,
                value: 3,
                span: testing::span_of(source, "3", 0),
            },
            ir::Inst::Binary {
                dest: ir::ValueId(4),
                op: ast::BinOp::Mul,
                lhs: ir::ValueId(2),
                rhs: ir::ValueId(3),
                span: diag::Span {
                    start: testing::span_of(source, "1 + 2", 0).start,
                    end: testing::span_of(source, "3", 0).end,
                },
            },
            ir::Inst::Const {
                dest: ir::ValueId(5),
                ty: typed_ast::Type::Uint64,
                value: 4,
                span: testing::span_of(source, "4", 1),
            },
            ir::Inst::Const {
                dest: ir::ValueId(6),
                ty: typed_ast::Type::Uint64,
                value: 5,
                span: testing::span_of(source, "5", 0),
            },
            ir::Inst::Binary {
                dest: ir::ValueId(7),
                op: ast::BinOp::Div,
                lhs: ir::ValueId(5),
                rhs: ir::ValueId(6),
                span: testing::span_of(source, "4 / 5", 0),
            },
            ir::Inst::Binary {
                dest: ir::ValueId(8),
                op: ast::BinOp::Sub,
                lhs: ir::ValueId(4),
                rhs: ir::ValueId(7),
                span: diag::Span {
                    start: testing::span_of(source, "1 + 2", 0).start,
                    end: testing::span_of(source, "5", 0).end,
                },
            },
            ir::Inst::Return {
                value: ir::ValueId(8),
                span: testing::span_of(source, "return (1 + 2) * 3 - 4 / 5", 0),
            },
        ]
    );
}

#[test]
fn a_boolean_literal_lowers_to_a_bool_constant() {
    let source = "func approval() bool { return true }";
    assert_eq!(
        lower_ok(source).funcs[0],
        ir::Function {
            name: "approval".to_string(),
            ret: typed_ast::Type::Bool,
            params: vec![],
            locals: vec![],
            insts: vec![
                ir::Inst::Const {
                    dest: ir::ValueId(0),
                    ty: typed_ast::Type::Bool,
                    value: 1,
                    span: testing::span_of(source, "true", 0),
                },
                ir::Inst::Return {
                    value: ir::ValueId(0),
                    span: testing::span_of(source, "return true", 0),
                },
            ],
            span: testing::span_of(source, source, 0),
        }
    );

    let source = "func approval() bool { return false }";
    assert_eq!(
        lower_ok(source).funcs[0].insts[0],
        ir::Inst::Const {
            dest: ir::ValueId(0),
            ty: typed_ast::Type::Bool,
            value: 0,
            span: testing::span_of(source, "false", 0),
        }
    );
}

#[test]
fn the_booleans_program_lowers_to_the_frame() {
    let source = "func approval() bool {\n  var ok bool = true\n  return ok\n}\n";
    let mut span = testing::spans(source);

    span("func");
    span("approval");
    span("bool");

    let var_start = span("var").start;
    span("ok");
    span("bool");
    let literal = span("true");

    let return_start = span("return").start;
    let loaded = span("ok");

    assert_eq!(
        lower_ok(source).funcs[0],
        ir::Function {
            name: "approval".to_string(),
            ret: typed_ast::Type::Bool,
            params: vec![],
            locals: vec![typed_ast::Type::Bool],
            insts: vec![
                ir::Inst::Const {
                    dest: ir::ValueId(0),
                    ty: typed_ast::Type::Bool,
                    value: 1,
                    span: literal,
                },
                ir::Inst::Store {
                    local: typed_ast::LocalId(0),
                    value: ir::ValueId(0),
                    span: diag::Span {
                        start: var_start,
                        end: literal.end,
                    },
                },
                ir::Inst::Load {
                    dest: ir::ValueId(1),
                    local: typed_ast::LocalId(0),
                    span: loaded,
                },
                ir::Inst::Return {
                    value: ir::ValueId(1),
                    span: diag::Span {
                        start: return_start,
                        end: loaded.end,
                    },
                },
            ],
            span: testing::span_of(
                source,
                "func approval() bool {\n  var ok bool = true\n  return ok\n}",
                0
            ),
        }
    );
}

#[test]
fn variables_lower_to_the_frame() {
    let source = "func approval() uint64 {\n  var x uint64 = 1 + 2\n  \
                  var y uint64 = x * 3\n  return y - x\n}\n";
    let mut span = testing::spans(source);

    span("func");
    span("approval");
    span("uint64");

    let first_var = span("var").start;
    span("x");
    span("uint64");
    let one = span("1");
    let two = span("2");

    let second_var = span("var").start;
    span("y");
    span("uint64");
    let x_times = span("x");
    let three = span("3");

    let return_start = span("return").start;
    let y_minus = span("y");
    let x_minus = span("x");

    assert_eq!(
        lower_ok(source).funcs[0],
        ir::Function {
            name: "approval".to_string(),
            ret: typed_ast::Type::Uint64,
            params: vec![],
            locals: vec![typed_ast::Type::Uint64, typed_ast::Type::Uint64],
            insts: vec![
                ir::Inst::Const {
                    dest: ir::ValueId(0),
                    ty: typed_ast::Type::Uint64,
                    value: 1,
                    span: one,
                },
                ir::Inst::Const {
                    dest: ir::ValueId(1),
                    ty: typed_ast::Type::Uint64,
                    value: 2,
                    span: two,
                },
                ir::Inst::Binary {
                    dest: ir::ValueId(2),
                    op: ast::BinOp::Add,
                    lhs: ir::ValueId(0),
                    rhs: ir::ValueId(1),
                    span: diag::Span {
                        start: one.start,
                        end: two.end
                    },
                },
                ir::Inst::Store {
                    local: typed_ast::LocalId(0),
                    value: ir::ValueId(2),
                    span: diag::Span {
                        start: first_var,
                        end: two.end
                    },
                },
                ir::Inst::Load {
                    dest: ir::ValueId(3),
                    local: typed_ast::LocalId(0),
                    span: x_times,
                },
                ir::Inst::Const {
                    dest: ir::ValueId(4),
                    ty: typed_ast::Type::Uint64,
                    value: 3,
                    span: three,
                },
                ir::Inst::Binary {
                    dest: ir::ValueId(5),
                    op: ast::BinOp::Mul,
                    lhs: ir::ValueId(3),
                    rhs: ir::ValueId(4),
                    span: diag::Span {
                        start: x_times.start,
                        end: three.end
                    },
                },
                ir::Inst::Store {
                    local: typed_ast::LocalId(1),
                    value: ir::ValueId(5),
                    span: diag::Span {
                        start: second_var,
                        end: three.end
                    },
                },
                ir::Inst::Load {
                    dest: ir::ValueId(6),
                    local: typed_ast::LocalId(1),
                    span: y_minus,
                },
                ir::Inst::Load {
                    dest: ir::ValueId(7),
                    local: typed_ast::LocalId(0),
                    span: x_minus,
                },
                ir::Inst::Binary {
                    dest: ir::ValueId(8),
                    op: ast::BinOp::Sub,
                    lhs: ir::ValueId(6),
                    rhs: ir::ValueId(7),
                    span: diag::Span {
                        start: y_minus.start,
                        end: x_minus.end
                    },
                },
                ir::Inst::Return {
                    value: ir::ValueId(8),
                    span: diag::Span {
                        start: return_start,
                        end: x_minus.end
                    },
                },
            ],
            span: testing::span_of(source, source.trim_end(), 0),
        }
    );
}

#[test]
fn empty_input() {
    assert_eq!(lower_ok(""), ir::Program { funcs: vec![] });
}

#[test]
fn a_parameter_lowers_to_a_load() {
    let source = "func f(a uint64, b uint64) uint64 { return a + b } \
                  func approval() uint64 { return 1 }";
    let mut span = testing::spans(source);

    let f_start = span("func").start;
    span("f");
    span("a");
    span("uint64");
    span("b");
    span("uint64");
    span("uint64");
    let f_return = span("return").start;
    let a = span("a");
    let b = span("b");
    let f_end = span("}").end;

    let approval_start = span("func").start;
    span("approval");
    span("uint64");
    let approval_return = span("return").start;
    let one = span("1");
    let approval_end = span("}").end;

    assert_eq!(
        lower_ok(source),
        ir::Program {
            funcs: vec![
                ir::Function {
                    name: "f".to_string(),
                    ret: typed_ast::Type::Uint64,
                    params: vec![typed_ast::Type::Uint64; 2],
                    locals: vec![],
                    insts: vec![
                        ir::Inst::LoadParam {
                            dest: ir::ValueId(0),
                            param: typed_ast::ParamId(0),
                            span: a,
                        },
                        ir::Inst::LoadParam {
                            dest: ir::ValueId(1),
                            param: typed_ast::ParamId(1),
                            span: b,
                        },
                        ir::Inst::Binary {
                            dest: ir::ValueId(2),
                            op: ast::BinOp::Add,
                            lhs: ir::ValueId(0),
                            rhs: ir::ValueId(1),
                            span: diag::Span {
                                start: a.start,
                                end: b.end,
                            },
                        },
                        ir::Inst::Return {
                            value: ir::ValueId(2),
                            span: diag::Span {
                                start: f_return,
                                end: b.end,
                            },
                        },
                    ],
                    span: diag::Span {
                        start: f_start,
                        end: f_end,
                    },
                },
                ir::Function {
                    name: "approval".to_string(),
                    ret: typed_ast::Type::Uint64,
                    params: vec![],
                    locals: vec![],
                    insts: vec![
                        ir::Inst::Const {
                            dest: ir::ValueId(0),
                            ty: typed_ast::Type::Uint64,
                            value: 1,
                            span: one,
                        },
                        ir::Inst::Return {
                            value: ir::ValueId(0),
                            span: diag::Span {
                                start: approval_return,
                                end: one.end,
                            },
                        },
                    ],
                    span: diag::Span {
                        start: approval_start,
                        end: approval_end,
                    },
                },
            ],
        }
    );
}

#[test]
fn each_function_numbers_its_own_values() {
    let source = "func a() uint64 { return 1 } func b() uint64 { return 2 }";
    assert_eq!(
        lower_ok(source),
        ir::Program {
            funcs: vec![
                ir::Function {
                    name: "a".to_string(),
                    ret: typed_ast::Type::Uint64,
                    params: vec![],
                    locals: vec![],
                    insts: vec![
                        ir::Inst::Const {
                            dest: ir::ValueId(0),
                            ty: typed_ast::Type::Uint64,
                            value: 1,
                            span: testing::span_of(source, "1", 0),
                        },
                        ir::Inst::Return {
                            value: ir::ValueId(0),
                            span: testing::span_of(source, "return 1", 0),
                        },
                    ],
                    span: testing::span_of(source, "func a() uint64 { return 1 }", 0),
                },
                ir::Function {
                    name: "b".to_string(),
                    ret: typed_ast::Type::Uint64,
                    params: vec![],
                    locals: vec![],
                    insts: vec![
                        ir::Inst::Const {
                            dest: ir::ValueId(0),
                            ty: typed_ast::Type::Uint64,
                            value: 2,
                            span: testing::span_of(source, "2", 0),
                        },
                        ir::Inst::Return {
                            value: ir::ValueId(0),
                            span: testing::span_of(source, "return 2", 0),
                        },
                    ],
                    span: testing::span_of(source, "func b() uint64 { return 2 }", 0),
                },
            ],
        }
    );
}

#[test]
fn a_comparison_lowers_like_arithmetic() {
    let source = "func approval() bool { return 1 < 2 }";
    assert_eq!(
        lower_ok(source).funcs[0].insts,
        vec![
            ir::Inst::Const {
                dest: ir::ValueId(0),
                ty: typed_ast::Type::Uint64,
                value: 1,
                span: testing::span_of(source, "1", 0),
            },
            ir::Inst::Const {
                dest: ir::ValueId(1),
                ty: typed_ast::Type::Uint64,
                value: 2,
                span: testing::span_of(source, "2", 0),
            },
            ir::Inst::Binary {
                dest: ir::ValueId(2),
                op: ast::BinOp::Lt,
                lhs: ir::ValueId(0),
                rhs: ir::ValueId(1),
                span: testing::span_of(source, "1 < 2", 0),
            },
            ir::Inst::Return {
                value: ir::ValueId(2),
                span: testing::span_of(source, "return 1 < 2", 0),
            },
        ]
    );
}

#[test]
fn negation_lowers_after_its_operand() {
    let source = "func approval() bool { return !true }";
    assert_eq!(
        lower_ok(source).funcs[0].insts,
        vec![
            ir::Inst::Const {
                dest: ir::ValueId(0),
                ty: typed_ast::Type::Bool,
                value: 1,
                span: testing::span_of(source, "true", 0),
            },
            ir::Inst::Unary {
                dest: ir::ValueId(1),
                op: ast::UnOp::Not,
                operand: ir::ValueId(0),
                span: testing::span_of(source, "!true", 0),
            },
            ir::Inst::Return {
                value: ir::ValueId(1),
                span: testing::span_of(source, "return !true", 0),
            },
        ]
    );
}

#[test]
fn logic_lowers_in_post_order() {
    let source = "func approval() bool { return true && false }";
    assert_eq!(
        lower_ok(source).funcs[0].insts,
        vec![
            ir::Inst::Const {
                dest: ir::ValueId(0),
                ty: typed_ast::Type::Bool,
                value: 1,
                span: testing::span_of(source, "true", 0),
            },
            ir::Inst::Const {
                dest: ir::ValueId(1),
                ty: typed_ast::Type::Bool,
                value: 0,
                span: testing::span_of(source, "false", 0),
            },
            ir::Inst::Binary {
                dest: ir::ValueId(2),
                op: ast::BinOp::And,
                lhs: ir::ValueId(0),
                rhs: ir::ValueId(1),
                span: testing::span_of(source, "true && false", 0),
            },
            ir::Inst::Return {
                value: ir::ValueId(2),
                span: testing::span_of(source, "return true && false", 0),
            },
        ]
    );
}

/// The example program of the calls milestone.
const CALLS: &str = "func approval() uint64 {\n\treturn add(1, double(2))\n}\n\n\
                     func add(a uint64, b uint64) uint64 {\n\treturn a + b\n}\n\n\
                     func double(x uint64) uint64 {\n\treturn x * 2\n}\n";

#[test]
fn a_call_lowers_to_its_arguments_then_a_call() {
    let source = CALLS;
    let mut span = testing::spans(source);

    let approval_start = span("func").start;
    span("approval");
    span("uint64");
    let approval_return = span("return").start;
    let call_start = span("add").start;
    let one = span("1");
    let double_start = span("double").start;
    let two = span("2");
    let double_end = span(")").end;
    let call_end = span(")").end;
    let approval_end = span("}").end;

    let add_start = span("func").start;
    span("add");
    span("a");
    span("uint64");
    span("b");
    span("uint64");
    span("uint64");
    let add_return = span("return").start;
    let a_use = span("a");
    let b_use = span("b");
    let add_end = span("}").end;

    let double_decl_start = span("func").start;
    span("double");
    span("x");
    span("uint64");
    span("uint64");
    let double_return = span("return").start;
    let x_use = span("x");
    let two_use = span("2");
    let double_decl_end = span("}").end;

    assert_eq!(
        lower_ok(source),
        ir::Program {
            funcs: vec![
                ir::Function {
                    name: "approval".to_string(),
                    ret: typed_ast::Type::Uint64,
                    params: vec![],
                    locals: vec![],
                    insts: vec![
                        ir::Inst::Const {
                            dest: ir::ValueId(0),
                            ty: typed_ast::Type::Uint64,
                            value: 1,
                            span: one,
                        },
                        ir::Inst::Const {
                            dest: ir::ValueId(1),
                            ty: typed_ast::Type::Uint64,
                            value: 2,
                            span: two,
                        },
                        ir::Inst::Call {
                            dest: ir::ValueId(2),
                            callee: typed_ast::FuncId(2),
                            args: vec![ir::ValueId(1)],
                            span: diag::Span {
                                start: double_start,
                                end: double_end,
                            },
                        },
                        ir::Inst::Call {
                            dest: ir::ValueId(3),
                            callee: typed_ast::FuncId(1),
                            args: vec![ir::ValueId(0), ir::ValueId(2)],
                            span: diag::Span {
                                start: call_start,
                                end: call_end,
                            },
                        },
                        ir::Inst::Return {
                            value: ir::ValueId(3),
                            span: diag::Span {
                                start: approval_return,
                                end: call_end,
                            },
                        },
                    ],
                    span: diag::Span {
                        start: approval_start,
                        end: approval_end,
                    },
                },
                ir::Function {
                    name: "add".to_string(),
                    ret: typed_ast::Type::Uint64,
                    params: vec![typed_ast::Type::Uint64; 2],
                    locals: vec![],
                    insts: vec![
                        ir::Inst::LoadParam {
                            dest: ir::ValueId(0),
                            param: typed_ast::ParamId(0),
                            span: a_use,
                        },
                        ir::Inst::LoadParam {
                            dest: ir::ValueId(1),
                            param: typed_ast::ParamId(1),
                            span: b_use,
                        },
                        ir::Inst::Binary {
                            dest: ir::ValueId(2),
                            op: ast::BinOp::Add,
                            lhs: ir::ValueId(0),
                            rhs: ir::ValueId(1),
                            span: diag::Span {
                                start: a_use.start,
                                end: b_use.end,
                            },
                        },
                        ir::Inst::Return {
                            value: ir::ValueId(2),
                            span: diag::Span {
                                start: add_return,
                                end: b_use.end,
                            },
                        },
                    ],
                    span: diag::Span {
                        start: add_start,
                        end: add_end,
                    },
                },
                ir::Function {
                    name: "double".to_string(),
                    ret: typed_ast::Type::Uint64,
                    params: vec![typed_ast::Type::Uint64],
                    locals: vec![],
                    insts: vec![
                        ir::Inst::LoadParam {
                            dest: ir::ValueId(0),
                            param: typed_ast::ParamId(0),
                            span: x_use,
                        },
                        ir::Inst::Const {
                            dest: ir::ValueId(1),
                            ty: typed_ast::Type::Uint64,
                            value: 2,
                            span: two_use,
                        },
                        ir::Inst::Binary {
                            dest: ir::ValueId(2),
                            op: ast::BinOp::Mul,
                            lhs: ir::ValueId(0),
                            rhs: ir::ValueId(1),
                            span: diag::Span {
                                start: x_use.start,
                                end: two_use.end,
                            },
                        },
                        ir::Inst::Return {
                            value: ir::ValueId(2),
                            span: diag::Span {
                                start: double_return,
                                end: two_use.end,
                            },
                        },
                    ],
                    span: diag::Span {
                        start: double_decl_start,
                        end: double_decl_end,
                    },
                },
            ],
        }
    );
}

#[test]
fn a_call_inside_an_expression() {
    let source = "func approval() uint64 { return 1 + double(2) } \
                  func double(x uint64) uint64 { return x * 2 }";
    let mut span = testing::spans(source);
    let return_start = span("return").start;
    let one = span("1");
    let call_start = span("double").start;
    let two = span("2");
    let call_end = span(")").end;

    assert_eq!(
        lower_ok(source).funcs[0].insts,
        vec![
            ir::Inst::Const {
                dest: ir::ValueId(0),
                ty: typed_ast::Type::Uint64,
                value: 1,
                span: one,
            },
            ir::Inst::Const {
                dest: ir::ValueId(1),
                ty: typed_ast::Type::Uint64,
                value: 2,
                span: two,
            },
            ir::Inst::Call {
                dest: ir::ValueId(2),
                callee: typed_ast::FuncId(1),
                args: vec![ir::ValueId(1)],
                span: diag::Span {
                    start: call_start,
                    end: call_end,
                },
            },
            ir::Inst::Binary {
                dest: ir::ValueId(3),
                op: ast::BinOp::Add,
                lhs: ir::ValueId(0),
                rhs: ir::ValueId(2),
                span: diag::Span {
                    start: one.start,
                    end: call_end,
                },
            },
            ir::Inst::Return {
                value: ir::ValueId(3),
                span: diag::Span {
                    start: return_start,
                    end: call_end,
                },
            },
        ]
    );
}
