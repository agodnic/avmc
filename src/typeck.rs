//! The type checker: an AST to a typed AST, resolving every type it names.

use crate::ast;
use crate::diagnostics;
use crate::typed_ast;
use std::collections::HashSet;

/// Checks `program`: its declared names, then every function in source order.
///
/// Reports every problem it finds, and returns `None` if it found any.
pub fn check(
    program: &ast::Program,
    diags: &mut diagnostics::Diagnostics,
) -> Option<typed_ast::Program> {
    let mut ok = check_duplicates(program, diags);
    let mut funcs = Vec::new();

    for func in &program.funcs {
        match check_func(func, diags) {
            Some(func) => funcs.push(func),
            None => ok = false,
        }
    }

    ok.then_some(typed_ast::Program { funcs })
}

/// Reports every declaration whose name was already declared, returning false
/// if there was one.
fn check_duplicates(program: &ast::Program, diags: &mut diagnostics::Diagnostics) -> bool {
    let mut seen = HashSet::new();
    let mut ok = true;

    for func in &program.funcs {
        if !seen.insert(func.name.text.as_str()) {
            diags.push(diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::DuplicateFunction {
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
fn check_func(
    func: &ast::FuncDecl,
    diags: &mut diagnostics::Diagnostics,
) -> Option<typed_ast::FuncDecl> {
    let ret = resolve_type(&func.ret, diags);
    let body = check_body(func, ret, diags);

    Some(typed_ast::FuncDecl {
        name: func.name.clone(),
        ret: ret?,
        body: body?,
        span: func.span,
    })
}

/// Resolves a written type name, reporting it if it names no type.
fn resolve_type(
    ret: &ast::TypeRef,
    diags: &mut diagnostics::Diagnostics,
) -> Option<typed_ast::Type> {
    match ret.name.text.as_str() {
        "uint64" => Some(typed_ast::Type::Uint64),
        "bool" => Some(typed_ast::Type::Bool),
        name => {
            diags.push(diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnknownType {
                    name: name.to_string(),
                },
                span: ret.name.span,
            });
            None
        }
    }
}

/// Checks a function body: every statement in it, in a scope of its own. The
/// body must end with a `return`, and nothing may follow one — of which only
/// the first is reported.
fn check_body(
    func: &ast::FuncDecl,
    ret: Option<typed_ast::Type>,
    diags: &mut diagnostics::Diagnostics,
) -> Option<Vec<typed_ast::Stmt>> {
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
        match check_stmt(stmt, ret, &mut locals, diags) {
            Some(stmt) => stmts.push(stmt),
            None => ok = false,
        }
        returned |= matches!(stmt, ast::Stmt::Return { .. });
    }

    if !returned {
        diags.push(diagnostics::Diagnostic {
            kind: diagnostics::DiagnosticKind::MissingReturn,
            span: func.name.span,
        });
        return None;
    }
    if let Some(span) = unreachable {
        diags.push(diagnostics::Diagnostic {
            kind: diagnostics::DiagnosticKind::UnreachableStatement,
            span,
        });
        return None;
    }
    ok.then_some(stmts)
}

/// Checks one statement, adding what it declares to `locals`. `ret` is the
/// enclosing function's return type, which a `return` must match.
///
/// A declaration's initializer, type and name are independent: each is
/// checked, and reported, whether or not another failed.
fn check_stmt<'a>(
    stmt: &'a ast::Stmt,
    ret: Option<typed_ast::Type>,
    locals: &mut Vec<(&'a str, Option<typed_ast::Type>)>,
    diags: &mut diagnostics::Diagnostics,
) -> Option<typed_ast::Stmt> {
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
            let agrees = check_type(init.as_ref(), ty, diags);
            // The name is declared with its written type even when the
            // initializer disagreed, so that later uses check against the
            // declaration.
            let local = declare(name, ty, locals, diags);
            agrees.then_some(())?;
            Some(typed_ast::Stmt::Var {
                local: local?,
                ty: ty?,
                init: init?,
                span: *span,
            })
        }
        ast::Stmt::Return { expr, span } => {
            let expr = check_expr(expr, locals, diags)?;
            check_type(Some(&expr), ret, diags).then_some(())?;
            Some(typed_ast::Stmt::Return { expr, span: *span })
        }
    }
}

/// Reports `expr` if it has a type other than `expected`, returning false if
/// it did. An expression or an expectation that did not resolve reports
/// nothing: the problem is already reported.
fn check_type(
    expr: Option<&typed_ast::Expr>,
    expected: Option<typed_ast::Type>,
    diags: &mut diagnostics::Diagnostics,
) -> bool {
    let (Some(expr), Some(expected)) = (expr, expected) else {
        return true;
    };
    if expr.ty == expected {
        return true;
    }
    diags.push(diagnostics::Diagnostic {
        kind: diagnostics::DiagnosticKind::TypeMismatch {
            expected,
            found: expr.ty,
        },
        span: expr.span,
    });
    false
}

/// Declares `name`, reporting it if the scope already holds it or has no room
/// for it. A name that is reported is not declared.
fn declare<'a>(
    name: &'a ast::Name,
    ty: Option<typed_ast::Type>,
    locals: &mut Vec<(&'a str, Option<typed_ast::Type>)>,
    diags: &mut diagnostics::Diagnostics,
) -> Option<typed_ast::LocalId> {
    let kind = if locals.iter().any(|(declared, _)| *declared == name.text) {
        diagnostics::DiagnosticKind::DuplicateVariable {
            name: name.text.clone(),
        }
    } else if let Some(local) = typed_ast::LocalId::new(locals.len()) {
        locals.push((&name.text, ty));
        return Some(local);
    } else {
        diagnostics::DiagnosticKind::TooManyVariables {
            max: typed_ast::LocalId::CAPACITY,
        }
    };

    diags.push(diagnostics::Diagnostic {
        kind,
        span: name.span,
    });
    None
}

/// Checks one expression, giving it its type. A name that resolves to no
/// variable and an operand of arithmetic that is not a `uint64` are what can
/// fail.
fn check_expr(
    expr: &ast::Expr,
    locals: &[(&str, Option<typed_ast::Type>)],
    diags: &mut diagnostics::Diagnostics,
) -> Option<typed_ast::Expr> {
    let (kind, ty, span) = match expr {
        ast::Expr::IntLit { value, span } => (
            typed_ast::ExprKind::IntLit(*value),
            typed_ast::Type::Uint64,
            *span,
        ),
        ast::Expr::BoolLit { value, span } => (
            typed_ast::ExprKind::BoolLit(*value),
            typed_ast::Type::Bool,
            *span,
        ),
        ast::Expr::Binary { op, lhs, rhs, span } => {
            // Both operands are checked, so both report.
            let lhs = check_expr(lhs, locals, diags);
            let rhs = check_expr(rhs, locals, diags);
            // Equality takes any one type: the right operand must match the
            // left.
            let lhs_expected = typed_ast::operand_type(*op);
            let rhs_expected = lhs_expected.or(lhs.as_ref().map(|lhs| lhs.ty));
            let lhs_agrees = check_type(lhs.as_ref(), lhs_expected, diags);
            let rhs_agrees = check_type(rhs.as_ref(), rhs_expected, diags);
            (lhs_agrees && rhs_agrees).then_some(())?;
            (
                typed_ast::ExprKind::Binary {
                    op: *op,
                    lhs: Box::new(lhs?),
                    rhs: Box::new(rhs?),
                },
                typed_ast::result_type(*op),
                *span,
            )
        }
        ast::Expr::Unary { op, operand, span } => {
            let operand = check_expr(operand, locals, diags);
            check_type(operand.as_ref(), Some(typed_ast::Type::Bool), diags).then_some(())?;
            (
                typed_ast::ExprKind::Unary {
                    op: *op,
                    operand: Box::new(operand?),
                },
                typed_ast::Type::Bool,
                *span,
            )
        }
        ast::Expr::Var { name, span } => {
            let (local, ty) = resolve_var(name, locals, diags)?;
            (typed_ast::ExprKind::Var(local), ty, *span)
        }
    };
    Some(typed_ast::Expr { kind, ty, span })
}

/// Resolves a name to its frame slot and type, reporting it if it names no
/// variable. A variable whose declared type did not resolve is `None` without
/// a report: the declaration reported it.
fn resolve_var(
    name: &ast::Name,
    locals: &[(&str, Option<typed_ast::Type>)],
    diags: &mut diagnostics::Diagnostics,
) -> Option<(typed_ast::LocalId, typed_ast::Type)> {
    let Some((index, (_, ty))) = locals
        .iter()
        .enumerate()
        .find(|(_, (declared, _))| *declared == name.text)
    else {
        diags.push(diagnostics::Diagnostic {
            kind: diagnostics::DiagnosticKind::UndefinedVariable {
                name: name.text.clone(),
            },
            span: name.span,
        });
        return None;
    };
    Some((typed_ast::LocalId::new(index)?, (*ty)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    /// Checks `source`, asserting that it produced no diagnostics.
    fn check_ok(source: &str) -> typed_ast::Program {
        let mut diags = diagnostics::Diagnostics::default();
        let program = check(&testing::lex_parse(source), &mut diags);
        assert!(diags.is_empty());
        program.expect("checking succeeded")
    }

    /// Checks `source`, asserting that checking produced nothing, and
    /// returning the diagnostics in the order they were reported.
    fn check_err(source: &str) -> Vec<diagnostics::Diagnostic> {
        let mut diags = diagnostics::Diagnostics::default();
        assert_eq!(check(&testing::lex_parse(source), &mut diags), None);
        diags.iter().cloned().collect()
    }

    #[test]
    fn checks_the_approval_program() {
        let source = "func approval() uint64 {\n  return 1\n}\n";
        let mut span = testing::spans(source);

        let start = span("func").start;
        let func_name = testing::name("approval", span("approval"));
        span("uint64");
        let return_start = span("return").start;
        let literal = span("1");
        let end = span("}").end;

        assert_eq!(
            check_ok(source),
            typed_ast::Program {
                funcs: vec![typed_ast::FuncDecl {
                    name: func_name,
                    ret: typed_ast::Type::Uint64,
                    body: vec![typed_ast::Stmt::Return {
                        expr: typed_ast::Expr {
                            kind: typed_ast::ExprKind::IntLit(1),
                            ty: typed_ast::Type::Uint64,
                            span: literal,
                        },
                        span: diagnostics::Span {
                            start: return_start,
                            end: literal.end,
                        },
                    }],
                    span: diagnostics::Span { start, end },
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

        let uint64 = |kind, span| typed_ast::Expr {
            kind,
            ty: typed_ast::Type::Uint64,
            span,
        };
        let binary = |op, lhs: typed_ast::Expr, rhs: typed_ast::Expr| {
            let span = diagnostics::Span {
                start: lhs.span.start,
                end: rhs.span.end,
            };
            uint64(
                typed_ast::ExprKind::Binary {
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
                typed_ast::Stmt::Var {
                    local: typed_ast::LocalId(0),
                    ty: typed_ast::Type::Uint64,
                    init: binary(
                        ast::BinaryOp::Add,
                        uint64(typed_ast::ExprKind::IntLit(1), one),
                        uint64(typed_ast::ExprKind::IntLit(2), two),
                    ),
                    span: diagnostics::Span {
                        start: first_var,
                        end: two.end
                    },
                },
                typed_ast::Stmt::Var {
                    local: typed_ast::LocalId(1),
                    ty: typed_ast::Type::Uint64,
                    init: binary(
                        ast::BinaryOp::Mul,
                        uint64(typed_ast::ExprKind::Var(typed_ast::LocalId(0)), x_times),
                        uint64(typed_ast::ExprKind::IntLit(3), three),
                    ),
                    span: diagnostics::Span {
                        start: second_var,
                        end: three.end
                    },
                },
                typed_ast::Stmt::Return {
                    expr: binary(
                        ast::BinaryOp::Sub,
                        uint64(typed_ast::ExprKind::Var(typed_ast::LocalId(1)), y_minus),
                        uint64(typed_ast::ExprKind::Var(typed_ast::LocalId(0)), x_minus),
                    ),
                    span: diagnostics::Span {
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
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UndefinedVariable {
                    name: "x".to_string(),
                },
                span: testing::span_of(&source, "x", 0),
            }]
        );
    }

    #[test]
    fn an_initializer_cannot_see_the_name_it_declares() {
        let source = wrap("var x uint64 = x return x");
        let mut span = testing::spans(&source);
        span("x");

        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UndefinedVariable {
                    name: "x".to_string(),
                },
                span: span("x"),
            }]
        );
    }

    #[test]
    fn a_duplicate_declaration_is_reported_at_the_later_one() {
        let source = wrap("var x uint64 = 1 var x uint64 = 2 return x");
        let mut span = testing::spans(&source);
        span("x");

        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::DuplicateVariable {
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
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnknownType {
                    name: "bytes".to_string(),
                },
                span: testing::span_of(&source, "bytes", 0),
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
        let source = declarations(typed_ast::LocalId::CAPACITY);
        let body = &check_ok(&source).funcs[0].body;

        assert_eq!(body.len(), typed_ast::LocalId::CAPACITY + 1);
        assert!(matches!(
            body[typed_ast::LocalId::CAPACITY - 1],
            typed_ast::Stmt::Var {
                local: typed_ast::LocalId(127),
                ..
            }
        ));
    }

    #[test]
    fn one_declaration_past_the_capacity_is_reported() {
        let source = declarations(typed_ast::LocalId::CAPACITY + 1);
        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TooManyVariables {
                    max: typed_ast::LocalId::CAPACITY,
                },
                span: testing::span_of(&source, "v128", 0),
            }]
        );
    }

    #[test]
    fn a_body_of_declarations_alone_is_missing_a_return() {
        let source = wrap("var x uint64 = 1");
        let mut span = testing::spans(&source);
        span("func");

        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::MissingReturn,
                span: span("approval"),
            }]
        );
    }

    #[test]
    fn a_declaration_after_a_return_is_unreachable() {
        let source = wrap("return 1 var x uint64 = 2");
        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnreachableStatement,
                span: testing::span_of(&source, "var x uint64 = 2", 0),
            }]
        );
    }

    #[test]
    fn a_variable_does_not_outlive_its_function() {
        let source = "func a() uint64 { var x uint64 = 1 return x } func b() uint64 { return x }";
        let mut span = testing::spans(source);
        span("x");
        span("x");

        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UndefinedVariable {
                    name: "x".to_string(),
                },
                span: span("x"),
            }]
        );
    }

    #[test]
    fn empty_input_produces_no_functions() {
        assert_eq!(check_ok(""), typed_ast::Program { funcs: Vec::new() });
    }

    #[test]
    fn an_unknown_return_type_is_reported() {
        let source = "func f() bytes { return 1 }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnknownType {
                    name: "bytes".to_string(),
                },
                span: testing::spans(source)("bytes"),
            }]
        );
    }

    #[test]
    fn a_body_without_a_return_is_reported() {
        let source = "func f() uint64 {}";
        let mut span = testing::spans(source);
        span("func");

        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::MissingReturn,
                span: span("f"),
            }]
        );
    }

    #[test]
    fn a_statement_after_a_return_is_reported() {
        let source = "func f() uint64 { return 1 return 2 }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnreachableStatement,
                span: testing::spans(source)("return 2"),
            }]
        );
    }

    #[test]
    fn checks_arithmetic() {
        let source = "func approval() uint64 { return 1 + 2 * 3 }";
        let uint64 = |kind, span| typed_ast::Expr {
            kind,
            ty: typed_ast::Type::Uint64,
            span,
        };
        let literal = |value: u64, nth| {
            uint64(
                typed_ast::ExprKind::IntLit(value),
                testing::span_of(source, &value.to_string(), nth),
            )
        };

        let product = uint64(
            typed_ast::ExprKind::Binary {
                op: ast::BinaryOp::Mul,
                lhs: Box::new(literal(2, 0)),
                rhs: Box::new(literal(3, 0)),
            },
            testing::span_of(source, "2 * 3", 0),
        );
        let sum = uint64(
            typed_ast::ExprKind::Binary {
                op: ast::BinaryOp::Add,
                lhs: Box::new(literal(1, 0)),
                rhs: Box::new(product),
            },
            testing::span_of(source, "1 + 2 * 3", 0),
        );

        assert_eq!(
            check_ok(source).funcs[0].body,
            vec![typed_ast::Stmt::Return {
                expr: sum,
                span: testing::span_of(source, "return 1 + 2 * 3", 0),
            }]
        );
    }

    #[test]
    fn every_function_is_checked() {
        let source = "func a() bytes { return 1 } func b() uint64 {}";
        let mut span = testing::spans(source);

        assert_eq!(
            check_err(source),
            vec![
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::UnknownType {
                        name: "bytes".to_string(),
                    },
                    span: span("bytes"),
                },
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::MissingReturn,
                    span: span("b"),
                },
            ]
        );
    }

    #[test]
    fn duplicate_name_is_reported_at_the_later_declaration() {
        let source = "func a() uint64 { return 1 } func a() uint64 { return 2 }";
        let mut span = testing::spans(source);
        span("a");

        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::DuplicateFunction {
                    name: "a".to_string(),
                },
                span: span("a"),
            }]
        );
    }

    #[test]
    fn every_duplicate_is_reported_in_source_order() {
        let source = "func a() uint64 { return 1 } func a() uint64 { return 2 } func a() uint64 { return 3 }";
        let mut span = testing::spans(source);
        span("a");

        assert_eq!(
            check_err(source),
            vec![
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::DuplicateFunction {
                        name: "a".to_string(),
                    },
                    span: span("a"),
                },
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::DuplicateFunction {
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
        let mut span = testing::spans(source);
        span("a");
        let duplicate = span("a");

        assert_eq!(
            check_err(source),
            vec![
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::DuplicateFunction {
                        name: "a".to_string(),
                    },
                    span: duplicate,
                },
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::UnknownType {
                        name: "bytes".to_string(),
                    },
                    span: span("bytes"),
                },
            ]
        );
    }

    #[test]
    fn a_returned_literal_must_have_the_return_type() {
        let source = "func approval() bool { return 1 }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Bool,
                    found: typed_ast::Type::Uint64,
                },
                span: testing::span_of(source, "1", 0),
            }]
        );
    }

    #[test]
    fn a_returned_variable_must_have_the_return_type() {
        let source = "func approval() bool { var x uint64 = 1 return x }";
        let mut span = testing::spans(source);
        span("x");

        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Bool,
                    found: typed_ast::Type::Uint64,
                },
                span: span("x"),
            }]
        );
    }

    #[test]
    fn a_failed_initializer_leaves_the_name_declared_as_written() {
        let source = wrap("var x bool = 1 return x");
        let mut span = testing::spans(&source);
        span("x");
        let one = span("1");

        assert_eq!(
            check_err(&source),
            vec![
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::TypeMismatch {
                        expected: typed_ast::Type::Bool,
                        found: typed_ast::Type::Uint64,
                    },
                    span: one,
                },
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::TypeMismatch {
                        expected: typed_ast::Type::Uint64,
                        found: typed_ast::Type::Bool,
                    },
                    span: span("x"),
                },
            ]
        );
    }

    #[test]
    fn a_declaration_agreeing_with_the_return_type_reports_only_its_initializer() {
        let source = "func approval() bool { var x bool = 1 return x }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Bool,
                    found: typed_ast::Type::Uint64,
                },
                span: testing::span_of(source, "1", 0),
            }]
        );
    }

    #[test]
    fn an_initializer_must_have_the_declared_type() {
        let source = wrap("var x uint64 = 1 var y bool = x return x");
        let mut span = testing::spans(&source);
        span("x");
        span("1");
        span("y");

        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Bool,
                    found: typed_ast::Type::Uint64,
                },
                span: span("x"),
            }]
        );
    }

    #[test]
    fn a_use_of_an_untyped_name_is_silent_inside_a_binary() {
        let source = wrap("var x bytes = 1 var y uint64 = x + 1 return y");
        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UnknownType {
                    name: "bytes".to_string(),
                },
                span: testing::span_of(&source, "bytes", 0),
            }]
        );
    }

    /// The example program of the booleans milestone.
    const BOOLEANS: &str = "func approval() bool {\n  var ok bool = true\n  return ok\n}\n";

    #[test]
    fn checks_the_booleans_program() {
        let source = BOOLEANS;
        let mut span = testing::spans(source);

        span("func");
        span("approval");
        span("bool");

        let var_start = span("var").start;
        span("ok");
        span("bool");
        let literal = span("true");

        let return_start = span("return").start;
        let returned = span("ok");

        assert_eq!(
            check_ok(source).funcs[0].body,
            vec![
                typed_ast::Stmt::Var {
                    local: typed_ast::LocalId(0),
                    ty: typed_ast::Type::Bool,
                    init: typed_ast::Expr {
                        kind: typed_ast::ExprKind::BoolLit(true),
                        ty: typed_ast::Type::Bool,
                        span: literal,
                    },
                    span: diagnostics::Span {
                        start: var_start,
                        end: literal.end,
                    },
                },
                typed_ast::Stmt::Return {
                    expr: typed_ast::Expr {
                        kind: typed_ast::ExprKind::Var(typed_ast::LocalId(0)),
                        ty: typed_ast::Type::Bool,
                        span: returned,
                    },
                    span: diagnostics::Span {
                        start: return_start,
                        end: returned.end,
                    },
                },
            ]
        );
        assert_eq!(check_ok(source).funcs[0].ret, typed_ast::Type::Bool);
    }

    #[test]
    fn a_returned_boolean_literal_may_be_the_return_type() {
        let source = "func approval() bool { return false }";
        assert_eq!(
            check_ok(source).funcs[0].body,
            vec![typed_ast::Stmt::Return {
                expr: typed_ast::Expr {
                    kind: typed_ast::ExprKind::BoolLit(false),
                    ty: typed_ast::Type::Bool,
                    span: testing::span_of(source, "false", 0),
                },
                span: testing::span_of(source, "return false", 0),
            }]
        );
    }

    #[test]
    fn a_returned_boolean_literal_must_have_the_return_type() {
        let source = wrap("return true");
        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Uint64,
                    found: typed_ast::Type::Bool,
                },
                span: testing::span_of(&source, "true", 0),
            }]
        );
    }

    #[test]
    fn a_boolean_left_operand_of_arithmetic_is_reported() {
        let source = wrap("return true + 1");
        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Uint64,
                    found: typed_ast::Type::Bool,
                },
                span: testing::span_of(&source, "true", 0),
            }]
        );
    }

    #[test]
    fn a_boolean_right_operand_of_arithmetic_is_reported() {
        let source = wrap("return 1 + true");
        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Uint64,
                    found: typed_ast::Type::Bool,
                },
                span: testing::span_of(&source, "true", 0),
            }]
        );
    }

    #[test]
    fn both_operands_of_arithmetic_are_reported_in_source_order() {
        let source = wrap("return true * false");
        let mut span = testing::spans(&source);
        let left = span("true");
        let right = span("false");

        let mismatch = diagnostics::Diagnostic {
            kind: diagnostics::DiagnosticKind::TypeMismatch {
                expected: typed_ast::Type::Uint64,
                found: typed_ast::Type::Bool,
            },
            span: left,
        };

        assert_eq!(
            check_err(&source),
            vec![
                mismatch.clone(),
                diagnostics::Diagnostic {
                    span: right,
                    ..mismatch
                },
            ]
        );
    }

    #[test]
    fn a_failed_operand_reports_no_return_mismatch() {
        let source = "func approval() bool { return true + 1 }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Uint64,
                    found: typed_ast::Type::Bool,
                },
                span: testing::span_of(source, "true", 0),
            }]
        );
    }

    #[test]
    fn a_boolean_variable_in_arithmetic_is_reported() {
        let source = wrap("var x bool = true var y uint64 = x + 1 return y");
        let mut span = testing::spans(&source);
        span("x");
        span("true");

        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Uint64,
                    found: typed_ast::Type::Bool,
                },
                span: span("x"),
            }]
        );
    }

    #[test]
    fn bool_is_a_type_only_where_a_type_is_expected() {
        let source = wrap("var bool uint64 = 1 return bool");
        let body = &check_ok(&source).funcs[0].body;

        assert!(matches!(
            body[0],
            typed_ast::Stmt::Var {
                local: typed_ast::LocalId(0),
                ty: typed_ast::Type::Uint64,
                ..
            }
        ));
        assert!(matches!(
            body[1],
            typed_ast::Stmt::Return {
                expr: typed_ast::Expr {
                    kind: typed_ast::ExprKind::Var(typed_ast::LocalId(0)),
                    ty: typed_ast::Type::Uint64,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn the_rules_are_independent() {
        let source = "func f() bytes {}";
        let mut span = testing::spans(source);
        span("func");
        let func_name = span("f");

        assert_eq!(
            check_err(source),
            vec![
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::UnknownType {
                        name: "bytes".to_string(),
                    },
                    span: span("bytes"),
                },
                diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::MissingReturn,
                    span: func_name,
                },
            ]
        );
    }

    /// The typed `Expr` of the one `return` in `source`.
    fn returned(source: &str) -> typed_ast::Expr {
        let program = check_ok(source);
        let funcs = program.funcs;
        assert_eq!(funcs.len(), 1);
        let mut body = funcs.into_iter().flat_map(|func| func.body);
        let Some(typed_ast::Stmt::Return { expr, .. }) = body.next() else {
            panic!("one return statement")
        };
        assert!(body.next().is_none());
        expr
    }

    #[test]
    fn a_comparison_of_integers_is_a_bool() {
        let source = "func approval() bool { return 1 < 2 }";
        let mut span = testing::spans(source);
        span("bool");
        let one = span("1");
        let two = span("2");

        assert_eq!(
            returned(source),
            typed_ast::Expr {
                kind: typed_ast::ExprKind::Binary {
                    op: ast::BinaryOp::Lt,
                    lhs: Box::new(typed_ast::Expr {
                        kind: typed_ast::ExprKind::IntLit(1),
                        ty: typed_ast::Type::Uint64,
                        span: one,
                    }),
                    rhs: Box::new(typed_ast::Expr {
                        kind: typed_ast::ExprKind::IntLit(2),
                        ty: typed_ast::Type::Uint64,
                        span: two,
                    }),
                },
                ty: typed_ast::Type::Bool,
                span: diagnostics::Span {
                    start: one.start,
                    end: two.end,
                },
            }
        );
    }

    #[test]
    fn equality_takes_booleans() {
        let source = "func approval() bool { return true == false }";
        let mut span = testing::spans(source);
        span("bool");
        let left = span("true");
        let right = span("false");

        assert_eq!(
            returned(source),
            typed_ast::Expr {
                kind: typed_ast::ExprKind::Binary {
                    op: ast::BinaryOp::Eq,
                    lhs: Box::new(typed_ast::Expr {
                        kind: typed_ast::ExprKind::BoolLit(true),
                        ty: typed_ast::Type::Bool,
                        span: left,
                    }),
                    rhs: Box::new(typed_ast::Expr {
                        kind: typed_ast::ExprKind::BoolLit(false),
                        ty: typed_ast::Type::Bool,
                        span: right,
                    }),
                },
                ty: typed_ast::Type::Bool,
                span: diagnostics::Span {
                    start: left.start,
                    end: right.end,
                },
            }
        );
    }

    #[test]
    fn a_comparison_of_arithmetic_checks() {
        let source = "func approval() bool { return 1 + 2 < 3 * 4 }";
        assert_eq!(returned(source).ty, typed_ast::Type::Bool);
    }

    #[test]
    fn a_returned_comparison_must_have_the_return_type() {
        let source = wrap("return 1 < 2");
        assert_eq!(
            check_err(&source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Uint64,
                    found: typed_ast::Type::Bool,
                },
                span: testing::span_of(&source, "1 < 2", 0),
            }]
        );
    }

    #[test]
    fn a_boolean_operand_of_an_ordering_is_reported() {
        let source = "func approval() bool { return 1 < true }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Uint64,
                    found: typed_ast::Type::Bool,
                },
                span: testing::span_of(source, "true", 0),
            }]
        );
    }

    #[test]
    fn both_boolean_operands_of_an_ordering_are_reported() {
        let source = "func approval() bool { return true < false }";
        let mut span = testing::spans(source);
        span("bool");
        let left = span("true");
        let right = span("false");

        let mismatch = diagnostics::Diagnostic {
            kind: diagnostics::DiagnosticKind::TypeMismatch {
                expected: typed_ast::Type::Uint64,
                found: typed_ast::Type::Bool,
            },
            span: left,
        };

        assert_eq!(
            check_err(source),
            vec![
                mismatch.clone(),
                diagnostics::Diagnostic {
                    span: right,
                    ..mismatch
                },
            ]
        );
    }

    #[test]
    fn equality_takes_the_type_of_its_left_operand() {
        let source = "func approval() bool { return 1 == true }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Uint64,
                    found: typed_ast::Type::Bool,
                },
                span: testing::span_of(source, "true", 0),
            }]
        );

        let source = "func approval() bool { return true == 1 }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Bool,
                    found: typed_ast::Type::Uint64,
                },
                span: testing::span_of(source, "1", 0),
            }]
        );
    }

    #[test]
    fn a_comparison_is_a_boolean_operand_of_equality() {
        let source = "func approval() bool { return (1 == 2) == 3 }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Bool,
                    found: typed_ast::Type::Uint64,
                },
                span: testing::span_of(source, "3", 0),
            }]
        );
    }

    #[test]
    fn an_unresolved_left_operand_leaves_the_right_one_unchecked() {
        let source = "func approval() bool { return x == 1 }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UndefinedVariable {
                    name: "x".to_string(),
                },
                span: testing::span_of(source, "x", 0),
            }]
        );
    }

    #[test]
    fn a_negation_is_a_bool() {
        let source = "func approval() bool { return !true }";
        let literal = testing::span_of(source, "true", 0);
        let negation = testing::span_of(source, "!true", 0);

        assert_eq!(
            returned(source),
            typed_ast::Expr {
                kind: typed_ast::ExprKind::Unary {
                    op: ast::UnaryOp::Not,
                    operand: Box::new(typed_ast::Expr {
                        kind: typed_ast::ExprKind::BoolLit(true),
                        ty: typed_ast::Type::Bool,
                        span: literal,
                    }),
                },
                ty: typed_ast::Type::Bool,
                span: negation,
            }
        );
    }

    #[test]
    fn logic_takes_booleans() {
        for (source, op) in [
            (
                "func approval() bool { return true && false }",
                ast::BinaryOp::And,
            ),
            (
                "func approval() bool { return true || false }",
                ast::BinaryOp::Or,
            ),
        ] {
            let mut span = testing::spans(source);
            span("bool");
            let yes = span("true");
            let no = span("false");

            assert_eq!(
                returned(source),
                typed_ast::Expr {
                    kind: typed_ast::ExprKind::Binary {
                        op,
                        lhs: Box::new(typed_ast::Expr {
                            kind: typed_ast::ExprKind::BoolLit(true),
                            ty: typed_ast::Type::Bool,
                            span: yes,
                        }),
                        rhs: Box::new(typed_ast::Expr {
                            kind: typed_ast::ExprKind::BoolLit(false),
                            ty: typed_ast::Type::Bool,
                            span: no,
                        }),
                    },
                    ty: typed_ast::Type::Bool,
                    span: diagnostics::Span {
                        start: yes.start,
                        end: no.end,
                    },
                },
                "{source}"
            );
        }
    }

    #[test]
    fn an_operand_of_logic_that_is_not_a_bool_is_reported() {
        // The source, and the integer literals reported in it, in order.
        let cases = [
            ("func approval() bool { return !1 }", vec![("1", 0)]),
            ("func approval() bool { return 1 && true }", vec![("1", 0)]),
            ("func approval() bool { return true || 1 }", vec![("1", 0)]),
            (
                "func approval() bool { return 1 || 2 }",
                vec![("1", 0), ("2", 0)],
            ),
        ];

        for (source, reported) in cases {
            let expected: Vec<diagnostics::Diagnostic> = reported
                .into_iter()
                .map(|(text, nth)| diagnostics::Diagnostic {
                    kind: diagnostics::DiagnosticKind::TypeMismatch {
                        expected: typed_ast::Type::Bool,
                        found: typed_ast::Type::Uint64,
                    },
                    span: testing::span_of(source, text, nth),
                })
                .collect();
            assert_eq!(check_err(source), expected, "{source}");
        }
    }

    #[test]
    fn a_negation_returned_from_a_uint64_function_is_reported() {
        let source = "func approval() uint64 { return !true }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::TypeMismatch {
                    expected: typed_ast::Type::Uint64,
                    found: typed_ast::Type::Bool,
                },
                span: testing::span_of(source, "!true", 0),
            }]
        );
    }

    #[test]
    fn a_negation_of_an_undefined_name_reports_only_the_name() {
        let source = "func approval() bool { return !x }";
        assert_eq!(
            check_err(source),
            vec![diagnostics::Diagnostic {
                kind: diagnostics::DiagnosticKind::UndefinedVariable {
                    name: "x".to_string(),
                },
                span: testing::span_of(source, "x", 0),
            }]
        );
    }
}
