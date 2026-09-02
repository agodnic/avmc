//! Lowering: a typed AST to IR (see `ARCHITECTURE.md` §2.2).

use crate::diagnostics::{Diagnostic, Diagnostics};
use crate::ir::{Function, Inst, Program, ValueId};
use crate::typed_ast::{self, Expr, ExprKind, Stmt};
use std::collections::HashSet;

/// Lowers every function in `program`, in source order.
///
/// Reports every duplicate name and returns `None` if it found any (R3).
pub fn lower(program: &typed_ast::Program, diags: &mut Diagnostics) -> Option<Program> {
    check_duplicates(program, diags)?;

    let funcs: Vec<Function> = program.funcs.iter().map(lower_func).collect();

    #[cfg(debug_assertions)]
    for func in &funcs {
        // Only a compiler bug can reach this (R6, R7).
        if let Err(message) = crate::ir::verify(func) {
            panic!("{message}");
        }
    }

    Some(Program { funcs })
}

/// Reports E0007 for every declaration whose name was already declared,
/// returning `None` if there was one.
fn check_duplicates(program: &typed_ast::Program, diags: &mut Diagnostics) -> Option<()> {
    let mut seen = HashSet::new();
    let mut ok = true;

    for func in &program.funcs {
        if !seen.insert(func.name.text.as_str()) {
            diags.push(Diagnostic {
                code: "E0007",
                message: format!("duplicate function `{}`", func.name.text),
                span: func.name.span,
            });
            ok = false;
        }
    }

    ok.then_some(())
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
        name: func.name.clone(),
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
/// which keeps definitions dense (the IR's rule 1).
fn next_value(insts: &[Inst]) -> ValueId {
    let defined = insts
        .iter()
        .filter(|inst| matches!(inst, Inst::Const { .. }))
        .count();
    ValueId(defined as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Name;
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

    /// Lexes, parses, checks and lowers `source`, asserting that lowering
    /// produced nothing, and returning the diagnostics in the order they were
    /// reported.
    fn lower_err(source: &str) -> Vec<Diagnostic> {
        let mut diags = Diagnostics::default();
        assert_eq!(pipeline(source, &mut diags), None);
        diags.iter().cloned().collect()
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

    fn name(source: &str, text: &str, nth: usize) -> Name {
        Name {
            text: text.to_string(),
            span: span_of(source, text, nth),
        }
    }

    #[test]
    fn example_program() {
        let source = "func approval() uint64 { return 1 }";
        assert_eq!(
            lower_ok(source),
            Program {
                funcs: vec![Function {
                    name: name(source, "approval", 0),
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
                        name: name(source, "a", 0),
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
                        name: name(source, "b", 0),
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

    #[test]
    fn duplicate_name_is_reported_at_the_later_declaration() {
        let source = "func a() uint64 { return 1 } func a() uint64 { return 2 }";
        assert_eq!(
            lower_err(source),
            vec![Diagnostic {
                code: "E0007",
                message: "duplicate function `a`".to_string(),
                span: span_of(source, "a", 1),
            }]
        );
    }

    #[test]
    fn every_duplicate_is_reported_in_source_order() {
        let source = "func a() uint64 { return 1 } func a() uint64 { return 2 } func a() uint64 { return 3 }";
        assert_eq!(
            lower_err(source),
            vec![
                Diagnostic {
                    code: "E0007",
                    message: "duplicate function `a`".to_string(),
                    span: span_of(source, "a", 1),
                },
                Diagnostic {
                    code: "E0007",
                    message: "duplicate function `a`".to_string(),
                    span: span_of(source, "a", 2),
                },
            ]
        );
    }
}
