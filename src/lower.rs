//! Lowering: a typed AST to IR.

use crate::diagnostics::Diagnostics;
use crate::ir::{Function, Inst, Program, ValueId};
use crate::typed_ast::{self, Expr, ExprKind, Stmt};

/// Lowers every function in `program`, in source order.
pub fn lower(program: &typed_ast::Program, _diags: &mut Diagnostics) -> Option<Program> {
    let funcs: Vec<Function> = program.funcs.iter().map(lower_func).collect();

    #[cfg(debug_assertions)]
    for func in &funcs {
        // Only a compiler bug can reach this.
        #[expect(clippy::panic, reason = "a verifier failure is a compiler bug")]
        if let Err(violation) = crate::ir::verify(func) {
            panic!("{violation}");
        }
    }

    Some(Program { funcs })
}

/// Lowers one function. `ValueId`s restart at 0.
fn lower_func(func: &typed_ast::FuncDecl) -> Function {
    let mut insts = Vec::new();
    // The number of values defined so far, which keeps definitions dense.
    let mut next_value = 0;

    for stmt in &func.body {
        let Stmt::Return { expr, span } = stmt;
        let value = lower_expr(expr, &mut insts, &mut next_value);
        insts.push(Inst::Return { value, span: *span });
    }

    Function {
        name: func.name.text.clone(),
        ret: func.ret,
        locals: Vec::new(),
        insts,
        span: func.span,
    }
}

/// Lowers one expression in post-order, appending its instructions to `insts`
/// and yielding the value it produces. `next_value` is advanced past it.
fn lower_expr(expr: &Expr, insts: &mut Vec<Inst>, next_value: &mut u32) -> ValueId {
    match &expr.kind {
        ExprKind::IntLit(value) => {
            let dest = next_value_id(next_value);
            insts.push(Inst::Const {
                dest,
                value: *value,
                span: expr.span,
            });
            dest
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let lhs = lower_expr(lhs, insts, next_value);
            let rhs = lower_expr(rhs, insts, next_value);
            let dest = next_value_id(next_value);
            insts.push(Inst::Binary {
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
fn next_value_id(next_value: &mut u32) -> ValueId {
    let dest = ValueId(*next_value);
    *next_value += 1;
    dest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::BinaryOp;
    use crate::testing::{lex_parse_check, span_of};
    use crate::typed_ast::Type;

    /// Lowers `source`, asserting that it produced no diagnostics.
    fn lower_ok(source: &str) -> Program {
        let mut diags = Diagnostics::default();
        let program = lower(&lex_parse_check(source), &mut diags);
        assert!(diags.is_empty());
        program.expect("lowering succeeded")
    }

    #[test]
    fn example_program() {
        let source = "func approval() uint64 { return 1 }";
        assert_eq!(
            lower_ok(source),
            Program {
                funcs: vec![Function {
                    name: "approval".to_string(),
                    ret: Type::Uint64,
                    locals: vec![],
                    insts: vec![
                        Inst::Const {
                            dest: ValueId(0),
                            value: 1,
                            span: span_of(source, "1", 0),
                        },
                        Inst::Return {
                            value: ValueId(0),
                            span: span_of(source, "return 1", 0),
                        },
                    ],
                    span: span_of(source, "func approval() uint64 { return 1 }", 0),
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
                Inst::Const {
                    dest: ValueId(0),
                    value: 1,
                    span: span_of(source, "1", 0),
                },
                Inst::Const {
                    dest: ValueId(1),
                    value: 2,
                    span: span_of(source, "2", 0),
                },
                Inst::Const {
                    dest: ValueId(2),
                    value: 3,
                    span: span_of(source, "3", 0),
                },
                Inst::Binary {
                    dest: ValueId(3),
                    op: BinaryOp::Mul,
                    lhs: ValueId(1),
                    rhs: ValueId(2),
                    span: span_of(source, "2 * 3", 0),
                },
                Inst::Binary {
                    dest: ValueId(4),
                    op: BinaryOp::Add,
                    lhs: ValueId(0),
                    rhs: ValueId(3),
                    span: span_of(source, "1 + 2 * 3", 0),
                },
                Inst::Return {
                    value: ValueId(4),
                    span: span_of(source, "return 1 + 2 * 3", 0),
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
                Inst::Const {
                    dest: ValueId(0),
                    value: 1,
                    span: span_of(source, "1", 0),
                },
                Inst::Const {
                    dest: ValueId(1),
                    value: 2,
                    span: span_of(source, "2", 0),
                },
                Inst::Binary {
                    dest: ValueId(2),
                    op: BinaryOp::Add,
                    lhs: ValueId(0),
                    rhs: ValueId(1),
                    span: span_of(source, "(1 + 2)", 0),
                },
                Inst::Const {
                    dest: ValueId(3),
                    value: 3,
                    span: span_of(source, "3", 0),
                },
                Inst::Binary {
                    dest: ValueId(4),
                    op: BinaryOp::Mul,
                    lhs: ValueId(2),
                    rhs: ValueId(3),
                    span: span_of(source, "(1 + 2) * 3", 0),
                },
                Inst::Const {
                    dest: ValueId(5),
                    value: 4,
                    span: span_of(source, "4", 1),
                },
                Inst::Const {
                    dest: ValueId(6),
                    value: 5,
                    span: span_of(source, "5", 0),
                },
                Inst::Binary {
                    dest: ValueId(7),
                    op: BinaryOp::Div,
                    lhs: ValueId(5),
                    rhs: ValueId(6),
                    span: span_of(source, "4 / 5", 0),
                },
                Inst::Binary {
                    dest: ValueId(8),
                    op: BinaryOp::Sub,
                    lhs: ValueId(4),
                    rhs: ValueId(7),
                    span: span_of(source, "(1 + 2) * 3 - 4 / 5", 0),
                },
                Inst::Return {
                    value: ValueId(8),
                    span: span_of(source, "return (1 + 2) * 3 - 4 / 5", 0),
                },
            ]
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(lower_ok(""), Program { funcs: vec![] });
    }

    #[test]
    fn each_function_numbers_its_own_values() {
        let source = "func a() uint64 { return 1 } func b() uint64 { return 2 }";
        assert_eq!(
            lower_ok(source),
            Program {
                funcs: vec![
                    Function {
                        name: "a".to_string(),
                        ret: Type::Uint64,
                        locals: vec![],
                        insts: vec![
                            Inst::Const {
                                dest: ValueId(0),
                                value: 1,
                                span: span_of(source, "1", 0),
                            },
                            Inst::Return {
                                value: ValueId(0),
                                span: span_of(source, "return 1", 0),
                            },
                        ],
                        span: span_of(source, "func a() uint64 { return 1 }", 0),
                    },
                    Function {
                        name: "b".to_string(),
                        ret: Type::Uint64,
                        locals: vec![],
                        insts: vec![
                            Inst::Const {
                                dest: ValueId(0),
                                value: 2,
                                span: span_of(source, "2", 0),
                            },
                            Inst::Return {
                                value: ValueId(0),
                                span: span_of(source, "return 2", 0),
                            },
                        ],
                        span: span_of(source, "func b() uint64 { return 2 }", 0),
                    },
                ],
            }
        );
    }
}
