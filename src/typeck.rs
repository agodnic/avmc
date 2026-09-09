//! The type checker: an AST to a typed AST, resolving every type it names.

use crate::ast;
use crate::diagnostics::{Diagnostic, DiagnosticKind, Diagnostics};
use crate::typed_ast::{Expr, ExprKind, FuncDecl, LocalId, Program, Stmt, Type};
use std::collections::HashSet;

/// The one type name the language has.
const UINT64: &str = "uint64";

/// Checks `program`: its declared names, then every function in source order.
///
/// Reports every problem it finds, and returns `None` if it found any.
pub fn check(program: &ast::Program, diags: &mut Diagnostics) -> Option<Program> {
    let mut ok = check_duplicates(program, diags);
    let mut funcs = Vec::new();

    for func in &program.funcs {
        match check_func(func, diags) {
            Some(func) => funcs.push(func),
            None => ok = false,
        }
    }

    ok.then_some(Program { funcs })
}

/// Reports every declaration whose name was already declared, returning false
/// if there was one.
fn check_duplicates(program: &ast::Program, diags: &mut Diagnostics) -> bool {
    let mut seen = HashSet::new();
    let mut ok = true;

    for func in &program.funcs {
        if !seen.insert(func.name.text.as_str()) {
            diags.push(Diagnostic {
                kind: DiagnosticKind::DuplicateFunction {
                    name: func.name.text.clone(),
                },
                span: func.name.span,
            });
            ok = false;
        }
    }

    ok
}

/// Checks one function. The three rules are independent: a return type that
/// does not resolve still leaves the body checked.
fn check_func(func: &ast::FuncDecl, diags: &mut Diagnostics) -> Option<FuncDecl> {
    let ret = resolve_type(&func.ret, diags);
    let body = check_body(func, diags);

    Some(FuncDecl {
        name: func.name.clone(),
        ret: ret?,
        body: body?,
        span: func.span,
    })
}

/// Resolves a written type name, reporting it if it names no type.
fn resolve_type(ret: &ast::TypeRef, diags: &mut Diagnostics) -> Option<Type> {
    if ret.name.text == UINT64 {
        return Some(Type::Uint64);
    }
    diags.push(Diagnostic {
        kind: DiagnosticKind::UnknownType {
            name: ret.name.text.clone(),
        },
        span: ret.name.span,
    });
    None
}

/// Checks a function body: every statement in it, in a scope of its own. The
/// body must end with a `return`, and nothing may follow one — of which only
/// the first is reported.
fn check_body(func: &ast::FuncDecl, diags: &mut Diagnostics) -> Option<Vec<Stmt>> {
    let mut stmts = Vec::new();
    // The variables declared so far, in order: a name's index is its slot.
    let mut locals = Vec::new();
    let mut ok = true;
    let mut returned = false;
    let mut unreachable = None;

    for stmt in &func.body {
        let (ast::Stmt::Var { span, .. } | ast::Stmt::Return { span, .. }) = stmt;
        if returned && unreachable.is_none() {
            unreachable = Some(*span);
        }
        match check_stmt(stmt, &mut locals, diags) {
            Some(stmt) => stmts.push(stmt),
            None => ok = false,
        }
        returned |= matches!(stmt, ast::Stmt::Return { .. });
    }

    if !returned {
        diags.push(Diagnostic {
            kind: DiagnosticKind::MissingReturn,
            span: func.name.span,
        });
        return None;
    }
    if let Some(span) = unreachable {
        diags.push(Diagnostic {
            kind: DiagnosticKind::UnreachableStatement,
            span,
        });
        return None;
    }
    ok.then_some(stmts)
}

/// Checks one statement, adding what it declares to `locals`.
///
/// A declaration's initializer, type and name are independent: each is
/// checked, and reported, whether or not another failed.
fn check_stmt<'a>(
    stmt: &'a ast::Stmt,
    locals: &mut Vec<&'a str>,
    diags: &mut Diagnostics,
) -> Option<Stmt> {
    match stmt {
        ast::Stmt::Var {
            name,
            ty,
            init,
            span,
        } => {
            // The initializer cannot see the name being declared.
            let init = check_expr(init, locals, diags);
            let ty = resolve_type(ty, diags);
            let local = declare(name, locals, diags);
            Some(Stmt::Var {
                local: local?,
                ty: ty?,
                init: init?,
                span: *span,
            })
        }
        ast::Stmt::Return { expr, span } => Some(Stmt::Return {
            expr: check_expr(expr, locals, diags)?,
            span: *span,
        }),
    }
}

/// Declares `name`, reporting it if the scope already holds it or has no room
/// for it. A name that is reported is not declared.
fn declare<'a>(
    name: &'a ast::Name,
    locals: &mut Vec<&'a str>,
    diags: &mut Diagnostics,
) -> Option<LocalId> {
    let kind = if locals.contains(&name.text.as_str()) {
        DiagnosticKind::DuplicateVariable {
            name: name.text.clone(),
        }
    } else if let Some(local) = LocalId::new(locals.len()) {
        locals.push(&name.text);
        return Some(local);
    } else {
        DiagnosticKind::TooManyVariables {
            max: LocalId::CAPACITY,
        }
    };

    diags.push(Diagnostic {
        kind,
        span: name.span,
    });
    None
}

/// Checks one expression. A name that resolves to no variable is the one
/// thing that can fail.
fn check_expr(expr: &ast::Expr, locals: &[&str], diags: &mut Diagnostics) -> Option<Expr> {
    let (kind, span) = match expr {
        ast::Expr::IntLit { value, span } => (ExprKind::IntLit(*value), *span),
        ast::Expr::Binary { op, lhs, rhs, span } => {
            // Both operands are checked, so both report.
            let lhs = check_expr(lhs, locals, diags);
            let rhs = check_expr(rhs, locals, diags);
            (
                ExprKind::Binary {
                    op: *op,
                    lhs: Box::new(lhs?),
                    rhs: Box::new(rhs?),
                },
                *span,
            )
        }
        ast::Expr::Var { name, span } => (ExprKind::Var(resolve_var(name, locals, diags)?), *span),
    };
    Some(Expr {
        kind,
        ty: Type::Uint64,
        span,
    })
}

/// Resolves a name to its frame slot, reporting it if it names no variable.
fn resolve_var(name: &ast::Name, locals: &[&str], diags: &mut Diagnostics) -> Option<LocalId> {
    let local = locals
        .iter()
        .position(|declared| *declared == name.text)
        .and_then(LocalId::new);

    if local.is_none() {
        diags.push(Diagnostic {
            kind: DiagnosticKind::UndefinedVariable {
                name: name.text.clone(),
            },
            span: name.span,
        });
    }
    local
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Span;
    use crate::testing::{lex_parse, name, span_of, spans};

    /// Checks `source`, asserting that it produced no diagnostics.
    fn check_ok(source: &str) -> Program {
        let mut diags = Diagnostics::default();
        let program = check(&lex_parse(source), &mut diags);
        assert!(diags.is_empty());
        program.expect("checking succeeded")
    }

    /// Checks `source`, asserting that checking produced nothing, and
    /// returning the diagnostics in the order they were reported.
    fn check_err(source: &str) -> Vec<Diagnostic> {
        let mut diags = Diagnostics::default();
        assert_eq!(check(&lex_parse(source), &mut diags), None);
        diags.iter().cloned().collect()
    }

    #[test]
    fn checks_the_approval_program() {
        let source = "func approval() uint64 {\n  return 1\n}\n";
        let mut span = spans(source);

        let start = span("func").start;
        let func_name = name("approval", span("approval"));
        span("uint64");
        let return_start = span("return").start;
        let literal = span("1");
        let end = span("}").end;

        assert_eq!(
            check_ok(source),
            Program {
                funcs: vec![FuncDecl {
                    name: func_name,
                    ret: Type::Uint64,
                    body: vec![Stmt::Return {
                        expr: Expr {
                            kind: ExprKind::IntLit(1),
                            ty: Type::Uint64,
                            span: literal,
                        },
                        span: Span {
                            start: return_start,
                            end: literal.end,
                        },
                    }],
                    span: Span { start, end },
                }]
            }
        );
    }

    /// The example program of the variables milestone.
    const VARIABLES: &str = "func approval() uint64 {\n  var x uint64 = 1 + 2\n  \
                             var y uint64 = x * 3\n  return y - x\n}\n";

    /// `func approval() uint64 { <body> }`.
    fn wrap(body: &str) -> String {
        format!("func approval() uint64 {{ {body} }}")
    }

    #[test]
    fn checks_the_variables_program() {
        let source = VARIABLES;
        let mut span = spans(source);

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

        let uint64 = |kind, span| Expr {
            kind,
            ty: Type::Uint64,
            span,
        };
        let binary = |op, lhs: Expr, rhs: Expr| {
            let span = Span {
                start: lhs.span.start,
                end: rhs.span.end,
            };
            uint64(
                ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            )
        };

        assert_eq!(
            check_ok(source).funcs[0].body,
            vec![
                Stmt::Var {
                    local: LocalId(0),
                    ty: Type::Uint64,
                    init: binary(
                        ast::BinaryOp::Add,
                        uint64(ExprKind::IntLit(1), one),
                        uint64(ExprKind::IntLit(2), two),
                    ),
                    span: Span {
                        start: first_var,
                        end: two.end
                    },
                },
                Stmt::Var {
                    local: LocalId(1),
                    ty: Type::Uint64,
                    init: binary(
                        ast::BinaryOp::Mul,
                        uint64(ExprKind::Var(LocalId(0)), x_times),
                        uint64(ExprKind::IntLit(3), three),
                    ),
                    span: Span {
                        start: second_var,
                        end: three.end
                    },
                },
                Stmt::Return {
                    expr: binary(
                        ast::BinaryOp::Sub,
                        uint64(ExprKind::Var(LocalId(1)), y_minus),
                        uint64(ExprKind::Var(LocalId(0)), x_minus),
                    ),
                    span: Span {
                        start: return_start,
                        end: x_minus.end
                    },
                },
            ]
        );
    }

    #[test]
    fn an_undefined_variable_is_reported() {
        let source = wrap("return x");
        assert_eq!(
            check_err(&source),
            vec![Diagnostic {
                kind: DiagnosticKind::UndefinedVariable {
                    name: "x".to_string(),
                },
                span: span_of(&source, "x", 0),
            }]
        );
    }

    #[test]
    fn an_initializer_cannot_see_the_name_it_declares() {
        let source = wrap("var x uint64 = x return x");
        let mut span = spans(&source);
        span("x");

        assert_eq!(
            check_err(&source),
            vec![Diagnostic {
                kind: DiagnosticKind::UndefinedVariable {
                    name: "x".to_string(),
                },
                span: span("x"),
            }]
        );
    }

    #[test]
    fn a_duplicate_declaration_is_reported_at_the_later_one() {
        let source = wrap("var x uint64 = 1 var x uint64 = 2 return x");
        let mut span = spans(&source);
        span("x");

        assert_eq!(
            check_err(&source),
            vec![Diagnostic {
                kind: DiagnosticKind::DuplicateVariable {
                    name: "x".to_string(),
                },
                span: span("x"),
            }]
        );
    }

    #[test]
    fn an_unknown_declared_type_is_reported() {
        let source = wrap("var x bytes = 1 return x");
        assert_eq!(
            check_err(&source),
            vec![Diagnostic {
                kind: DiagnosticKind::UnknownType {
                    name: "bytes".to_string(),
                },
                span: span_of(&source, "bytes", 0),
            }]
        );
    }

    /// `var v0 uint64 = 0 .. var v<count-1> uint64 = 0 return v0`.
    fn declarations(count: usize) -> String {
        let mut body = String::new();
        for index in 0..count {
            body.push_str(&format!("var v{index} uint64 = 0 "));
        }
        body.push_str("return v0");
        wrap(&body)
    }

    #[test]
    fn a_function_may_declare_the_frame_capacity() {
        let source = declarations(LocalId::CAPACITY);
        let body = &check_ok(&source).funcs[0].body;

        assert_eq!(body.len(), LocalId::CAPACITY + 1);
        assert!(matches!(
            body[LocalId::CAPACITY - 1],
            Stmt::Var {
                local: LocalId(127),
                ..
            }
        ));
    }

    #[test]
    fn one_declaration_past_the_capacity_is_reported() {
        let source = declarations(LocalId::CAPACITY + 1);
        assert_eq!(
            check_err(&source),
            vec![Diagnostic {
                kind: DiagnosticKind::TooManyVariables {
                    max: LocalId::CAPACITY,
                },
                span: span_of(&source, "v128", 0),
            }]
        );
    }

    #[test]
    fn a_body_of_declarations_alone_is_missing_a_return() {
        let source = wrap("var x uint64 = 1");
        let mut span = spans(&source);
        span("func");

        assert_eq!(
            check_err(&source),
            vec![Diagnostic {
                kind: DiagnosticKind::MissingReturn,
                span: span("approval"),
            }]
        );
    }

    #[test]
    fn a_declaration_after_a_return_is_unreachable() {
        let source = wrap("return 1 var x uint64 = 2");
        assert_eq!(
            check_err(&source),
            vec![Diagnostic {
                kind: DiagnosticKind::UnreachableStatement,
                span: span_of(&source, "var x uint64 = 2", 0),
            }]
        );
    }

    #[test]
    fn a_variable_does_not_outlive_its_function() {
        let source = "func a() uint64 { var x uint64 = 1 return x } func b() uint64 { return x }";
        let mut span = spans(source);
        span("x");
        span("x");

        assert_eq!(
            check_err(source),
            vec![Diagnostic {
                kind: DiagnosticKind::UndefinedVariable {
                    name: "x".to_string(),
                },
                span: span("x"),
            }]
        );
    }

    #[test]
    fn empty_input_produces_no_functions() {
        assert_eq!(check_ok(""), Program { funcs: Vec::new() });
    }

    #[test]
    fn an_unknown_return_type_is_reported() {
        let source = "func f() bytes { return 1 }";
        assert_eq!(
            check_err(source),
            vec![Diagnostic {
                kind: DiagnosticKind::UnknownType {
                    name: "bytes".to_string(),
                },
                span: spans(source)("bytes"),
            }]
        );
    }

    #[test]
    fn a_body_without_a_return_is_reported() {
        let source = "func f() uint64 {}";
        let mut span = spans(source);
        span("func");

        assert_eq!(
            check_err(source),
            vec![Diagnostic {
                kind: DiagnosticKind::MissingReturn,
                span: span("f"),
            }]
        );
    }

    #[test]
    fn a_statement_after_a_return_is_reported() {
        let source = "func f() uint64 { return 1 return 2 }";
        assert_eq!(
            check_err(source),
            vec![Diagnostic {
                kind: DiagnosticKind::UnreachableStatement,
                span: spans(source)("return 2"),
            }]
        );
    }

    #[test]
    fn checks_arithmetic() {
        let source = "func approval() uint64 { return 1 + 2 * 3 }";
        let uint64 = |kind, span| Expr {
            kind,
            ty: Type::Uint64,
            span,
        };
        let literal = |value: u64, nth| {
            uint64(
                ExprKind::IntLit(value),
                span_of(source, &value.to_string(), nth),
            )
        };

        let product = uint64(
            ExprKind::Binary {
                op: ast::BinaryOp::Mul,
                lhs: Box::new(literal(2, 0)),
                rhs: Box::new(literal(3, 0)),
            },
            span_of(source, "2 * 3", 0),
        );
        let sum = uint64(
            ExprKind::Binary {
                op: ast::BinaryOp::Add,
                lhs: Box::new(literal(1, 0)),
                rhs: Box::new(product),
            },
            span_of(source, "1 + 2 * 3", 0),
        );

        assert_eq!(
            check_ok(source).funcs[0].body,
            vec![Stmt::Return {
                expr: sum,
                span: span_of(source, "return 1 + 2 * 3", 0),
            }]
        );
    }

    #[test]
    fn every_function_is_checked() {
        let source = "func a() bytes { return 1 } func b() uint64 {}";
        let mut span = spans(source);

        assert_eq!(
            check_err(source),
            vec![
                Diagnostic {
                    kind: DiagnosticKind::UnknownType {
                        name: "bytes".to_string(),
                    },
                    span: span("bytes"),
                },
                Diagnostic {
                    kind: DiagnosticKind::MissingReturn,
                    span: span("b"),
                },
            ]
        );
    }

    #[test]
    fn duplicate_name_is_reported_at_the_later_declaration() {
        let source = "func a() uint64 { return 1 } func a() uint64 { return 2 }";
        let mut span = spans(source);
        span("a");

        assert_eq!(
            check_err(source),
            vec![Diagnostic {
                kind: DiagnosticKind::DuplicateFunction {
                    name: "a".to_string(),
                },
                span: span("a"),
            }]
        );
    }

    #[test]
    fn every_duplicate_is_reported_in_source_order() {
        let source = "func a() uint64 { return 1 } func a() uint64 { return 2 } func a() uint64 { return 3 }";
        let mut span = spans(source);
        span("a");

        assert_eq!(
            check_err(source),
            vec![
                Diagnostic {
                    kind: DiagnosticKind::DuplicateFunction {
                        name: "a".to_string(),
                    },
                    span: span("a"),
                },
                Diagnostic {
                    kind: DiagnosticKind::DuplicateFunction {
                        name: "a".to_string(),
                    },
                    span: span("a"),
                },
            ]
        );
    }

    #[test]
    fn a_duplicate_and_a_type_error_are_reported_together() {
        let source = "func a() uint64 { return 1 } func a() bytes { return 2 }";
        let mut span = spans(source);
        span("a");
        let duplicate = span("a");

        assert_eq!(
            check_err(source),
            vec![
                Diagnostic {
                    kind: DiagnosticKind::DuplicateFunction {
                        name: "a".to_string(),
                    },
                    span: duplicate,
                },
                Diagnostic {
                    kind: DiagnosticKind::UnknownType {
                        name: "bytes".to_string(),
                    },
                    span: span("bytes"),
                },
            ]
        );
    }

    #[test]
    fn the_rules_are_independent() {
        let source = "func f() bytes {}";
        let mut span = spans(source);
        span("func");
        let func_name = span("f");

        assert_eq!(
            check_err(source),
            vec![
                Diagnostic {
                    kind: DiagnosticKind::UnknownType {
                        name: "bytes".to_string(),
                    },
                    span: span("bytes"),
                },
                Diagnostic {
                    kind: DiagnosticKind::MissingReturn,
                    span: func_name,
                },
            ]
        );
    }
}
