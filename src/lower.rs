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
        if let Err(message) = crate::ir::verify(func) {
            panic!("{message}");
        }
    }

    Some(Program { funcs })
}

/// Lowers one function. `ValueId`s restart at 0.
fn lower_func(func: &typed_ast::FuncDecl) -> Function {
    let mut insts = Vec::new();

    for stmt in &func.body {
        let Stmt::Return { expr, span } = stmt;
        let value = lower_expr(expr, &mut insts);
        insts.push(Inst::Return { value, span: *span });
    }

    Function {
        name: func.name.text.clone(),
        ret: func.ret,
        insts,
        span: func.span,
    }
}

/// Lowers one expression in post-order, appending its instructions to `insts`
/// and yielding the value it produces.
fn lower_expr(expr: &Expr, insts: &mut Vec<Inst>) -> ValueId {
    let ExprKind::IntLit(value) = expr.kind;
    let dest = next_value(insts);
    insts.push(Inst::Const {
        dest,
        value,
        span: expr.span,
    });
    dest
}

/// The value the next definition takes: one past the values defined so far,
/// which keeps definitions dense.
fn next_value(insts: &[Inst]) -> ValueId {
    let defined = insts
        .iter()
        .filter(|inst| matches!(inst, Inst::Const { .. }))
        .count();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a function cannot define u32::MAX values"
    )]
    ValueId(defined as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Span;
    use crate::lexer::lex;
    use crate::parser::parse;
    use crate::typeck::check;
    use crate::typed_ast::Type;

    /// Lexes, parses, checks and lowers `source`, asserting that it produced
    /// no diagnostics.
    fn lower_ok(source: &str) -> Program {
        let mut diags = Diagnostics::default();
        let program = pipeline(source, &mut diags);
        assert!(diags.is_empty());
        program.expect("lowering succeeded")
    }

    fn pipeline(source: &str, diags: &mut Diagnostics) -> Option<Program> {
        let tokens = lex(source, diags).expect("lexing succeeded");
        let parsed = parse(source, &tokens, diags).expect("parsing succeeded");
        let checked = check(&parsed, diags).expect("checking succeeded");
        lower(&checked, diags)
    }

    /// The span of the `nth` occurrence of `text` in `source`, counting from 0.
    fn span_of(source: &str, text: &str, nth: usize) -> Span {
        let start = source
            .match_indices(text)
            .nth(nth)
            .expect("text in source")
            .0;
        Span {
            start,
            end: start + text.len(),
        }
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
