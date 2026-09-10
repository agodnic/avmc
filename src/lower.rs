//! Lowering: a typed AST to IR.

use crate::diagnostics;
use crate::ir;
use crate::typed_ast;

/// Lowers every function in `program`, in source order.
pub fn lower(
    program: &typed_ast::Program,
    _diags: &mut diagnostics::Diagnostics,
) -> Option<ir::Program> {
    let funcs: Vec<ir::Function> = program.funcs.iter().map(lower_func).collect();

    #[cfg(debug_assertions)]
    for func in &funcs {
        // Only a compiler bug can reach this.
        #[expect(clippy::panic, reason = "a verifier failure is a compiler bug")]
        if let Err(violation) = crate::ir::verify(func) {
            panic!("{violation}");
        }
    }

    Some(ir::Program { funcs })
}

/// Lowers one function. `ValueId`s restart at 0.
fn lower_func(func: &typed_ast::FuncDecl) -> ir::Function {
    let mut insts = Vec::new();
    // The frame, which grows with every declaration the body walks past.
    let mut locals = Vec::new();
    // The number of values defined so far, which keeps definitions dense.
    let mut next_value = 0;

    for stmt in &func.body {
        match stmt {
            typed_ast::Stmt::Var {
                local,
                ty,
                init,
                span,
            } => {
                let value = lower_expr(init, &mut insts, &mut next_value);
                insts.push(ir::Inst::Store {
                    local: *local,
                    value,
                    span: *span,
                });
                locals.push(*ty);
            }
            typed_ast::Stmt::Return { expr, span } => {
                let value = lower_expr(expr, &mut insts, &mut next_value);
                insts.push(ir::Inst::Return { value, span: *span });
            }
        }
    }

    ir::Function {
        name: func.name.text.clone(),
        ret: func.ret,
        locals,
        insts,
        span: func.span,
    }
}

/// Lowers one expression in post-order, appending its instructions to `insts`
/// and yielding the value it produces. `next_value` is advanced past it.
fn lower_expr(
    expr: &typed_ast::Expr,
    insts: &mut Vec<ir::Inst>,
    next_value: &mut u32,
) -> ir::ValueId {
    match &expr.kind {
        typed_ast::ExprKind::IntLit(value) => {
            let dest = next_value_id(next_value);
            insts.push(ir::Inst::Const {
                dest,
                ty: typed_ast::Type::Uint64,
                value: *value,
                span: expr.span,
            });
            dest
        }
        typed_ast::ExprKind::BoolLit(value) => {
            let dest = next_value_id(next_value);
            insts.push(ir::Inst::Const {
                dest,
                ty: typed_ast::Type::Bool,
                value: u64::from(*value),
                span: expr.span,
            });
            dest
        }
        typed_ast::ExprKind::Var(local) => {
            let dest = next_value_id(next_value);
            insts.push(ir::Inst::Load {
                dest,
                local: *local,
                span: expr.span,
            });
            dest
        }
        typed_ast::ExprKind::Unary { op, operand } => {
            let operand = lower_expr(operand, insts, next_value);
            let dest = next_value_id(next_value);
            insts.push(ir::Inst::Unary {
                dest,
                op: *op,
                operand,
                span: expr.span,
            });
            dest
        }
        typed_ast::ExprKind::Binary { op, lhs, rhs } => {
            let lhs = lower_expr(lhs, insts, next_value);
            let rhs = lower_expr(rhs, insts, next_value);
            let dest = next_value_id(next_value);
            insts.push(ir::Inst::Binary {
                dest,
                op: *op,
                lhs,
                rhs,
                span: expr.span,
            });
            dest
        }
    }
}

/// The next `ValueId`, advancing `next_value` past it.
fn next_value_id(next_value: &mut u32) -> ir::ValueId {
    let dest = ir::ValueId(*next_value);
    *next_value += 1;
    dest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::testing;

    /// Lowers `source`, asserting that it produced no diagnostics.
    fn lower_ok(source: &str) -> ir::Program {
        let mut diags = diagnostics::Diagnostics::default();
        let program = lower(&testing::lex_parse_check(source), &mut diags);
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
                    op: ast::BinaryOp::Mul,
                    lhs: ir::ValueId(1),
                    rhs: ir::ValueId(2),
                    span: testing::span_of(source, "2 * 3", 0),
                },
                ir::Inst::Binary {
                    dest: ir::ValueId(4),
                    op: ast::BinaryOp::Add,
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
                    op: ast::BinaryOp::Add,
                    lhs: ir::ValueId(0),
                    rhs: ir::ValueId(1),
                    span: testing::span_of(source, "(1 + 2)", 0),
                },
                ir::Inst::Const {
                    dest: ir::ValueId(3),
                    ty: typed_ast::Type::Uint64,
                    value: 3,
                    span: testing::span_of(source, "3", 0),
                },
                ir::Inst::Binary {
                    dest: ir::ValueId(4),
                    op: ast::BinaryOp::Mul,
                    lhs: ir::ValueId(2),
                    rhs: ir::ValueId(3),
                    span: testing::span_of(source, "(1 + 2) * 3", 0),
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
                    op: ast::BinaryOp::Div,
                    lhs: ir::ValueId(5),
                    rhs: ir::ValueId(6),
                    span: testing::span_of(source, "4 / 5", 0),
                },
                ir::Inst::Binary {
                    dest: ir::ValueId(8),
                    op: ast::BinaryOp::Sub,
                    lhs: ir::ValueId(4),
                    rhs: ir::ValueId(7),
                    span: testing::span_of(source, "(1 + 2) * 3 - 4 / 5", 0),
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
                        span: diagnostics::Span {
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
                        span: diagnostics::Span {
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
                        op: ast::BinaryOp::Add,
                        lhs: ir::ValueId(0),
                        rhs: ir::ValueId(1),
                        span: diagnostics::Span {
                            start: one.start,
                            end: two.end
                        },
                    },
                    ir::Inst::Store {
                        local: typed_ast::LocalId(0),
                        value: ir::ValueId(2),
                        span: diagnostics::Span {
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
                        op: ast::BinaryOp::Mul,
                        lhs: ir::ValueId(3),
                        rhs: ir::ValueId(4),
                        span: diagnostics::Span {
                            start: x_times.start,
                            end: three.end
                        },
                    },
                    ir::Inst::Store {
                        local: typed_ast::LocalId(1),
                        value: ir::ValueId(5),
                        span: diagnostics::Span {
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
                        op: ast::BinaryOp::Sub,
                        lhs: ir::ValueId(6),
                        rhs: ir::ValueId(7),
                        span: diagnostics::Span {
                            start: y_minus.start,
                            end: x_minus.end
                        },
                    },
                    ir::Inst::Return {
                        value: ir::ValueId(8),
                        span: diagnostics::Span {
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
    fn each_function_numbers_its_own_values() {
        let source = "func a() uint64 { return 1 } func b() uint64 { return 2 }";
        assert_eq!(
            lower_ok(source),
            ir::Program {
                funcs: vec![
                    ir::Function {
                        name: "a".to_string(),
                        ret: typed_ast::Type::Uint64,
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
                    op: ast::BinaryOp::Lt,
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
                    op: ast::UnaryOp::Not,
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
                    op: ast::BinaryOp::And,
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
}
