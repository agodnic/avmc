//! Tests for the type checking pass.

use super::pass;
use crate::ast;
use crate::diag;
use crate::testing;
use crate::typed_ast;

/// Checks `source`, asserting that it produced no diagnostics.
fn check_ok(source: &str) -> typed_ast::Program {
    let mut diags = diag::Sink::default();
    let program = pass::check(&testing::lex_parse(source), &mut diags);
    assert!(diags.is_empty());
    program.expect("checking succeeded")
}

/// Checks `source`, asserting that checking produced nothing, and
/// returning the diagnostics in the order they were reported.
fn check_err(source: &str) -> Vec<diag::Entry> {
    let mut diags = diag::Sink::default();
    assert_eq!(pass::check(&testing::lex_parse(source), &mut diags), None);
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
                params: vec![],
                ret: typed_ast::Type::Uint64,
                body: vec![typed_ast::Stmt::Return {
                    expr: typed_ast::Expr {
                        kind: typed_ast::ExprKind::IntLit(1),
                        ty: typed_ast::Type::Uint64,
                        span: literal,
                    },
                    span: diag::Span {
                        start: return_start,
                        end: literal.end,
                    },
                }],
                span: diag::Span { start, end },
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
        let span = diag::Span {
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
                    ast::BinOp::Add,
                    uint64(typed_ast::ExprKind::IntLit(1), one),
                    uint64(typed_ast::ExprKind::IntLit(2), two),
                ),
                span: diag::Span {
                    start: first_var,
                    end: two.end
                },
            },
            typed_ast::Stmt::Var {
                local: typed_ast::LocalId(1),
                ty: typed_ast::Type::Uint64,
                init: binary(
                    ast::BinOp::Mul,
                    uint64(typed_ast::ExprKind::Var(typed_ast::LocalId(0)), x_times),
                    uint64(typed_ast::ExprKind::IntLit(3), three),
                ),
                span: diag::Span {
                    start: second_var,
                    end: three.end
                },
            },
            typed_ast::Stmt::Return {
                expr: binary(
                    ast::BinOp::Sub,
                    uint64(typed_ast::ExprKind::Var(typed_ast::LocalId(1)), y_minus),
                    uint64(typed_ast::ExprKind::Var(typed_ast::LocalId(0)), x_minus),
                ),
                span: diag::Span {
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
        vec![diag::Entry {
            kind: diag::Kind::UndefinedVariable {
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
        vec![diag::Entry {
            kind: diag::Kind::UndefinedVariable {
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
        vec![diag::Entry {
            kind: diag::Kind::DuplicateVariable {
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
        vec![diag::Entry {
            kind: diag::Kind::UnknownType {
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
        vec![diag::Entry {
            kind: diag::Kind::TooManyVariables {
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
        vec![diag::Entry {
            kind: diag::Kind::MissingReturn,
            span: span("approval"),
        }]
    );
}

#[test]
fn a_declaration_after_a_return_is_unreachable() {
    let source = wrap("return 1 var x uint64 = 2");
    assert_eq!(
        check_err(&source),
        vec![diag::Entry {
            kind: diag::Kind::UnreachableStatement,
            span: testing::span_of(&source, "var x uint64 = 2", 0),
        }]
    );
}

/// The `add` function of the parameters milestone.
const ADD: &str = "func add(a uint64, b uint64) uint64 {\n\tvar sum uint64 = a + b\n\t\
                   return sum\n}\n";

/// `func f(<params>) uint64 { <body> }`.
fn taking(params: &str, body: &str) -> String {
    format!("func f({params}) uint64 {{ {body} }}")
}

#[test]
fn a_parameter_reads_as_its_type() {
    let source = ADD;
    let mut span = testing::spans(source);

    span("func");
    span("add");
    let first = testing::name("a", span("a"));
    span("uint64");
    let second = testing::name("b", span("b"));
    span("uint64");
    span("uint64");

    let var = span("var").start;
    span("sum");
    span("uint64");
    let a = span("a");
    let b = span("b");
    let return_start = span("return").start;
    let sum = span("sum");

    let uint64 = |kind, span| typed_ast::Expr {
        kind,
        ty: typed_ast::Type::Uint64,
        span,
    };
    let func = &check_ok(source).funcs[0];

    assert_eq!(
        func.params,
        vec![
            typed_ast::Param {
                name: first,
                ty: typed_ast::Type::Uint64,
            },
            typed_ast::Param {
                name: second,
                ty: typed_ast::Type::Uint64,
            },
        ]
    );
    assert_eq!(
        func.body,
        vec![
            typed_ast::Stmt::Var {
                local: typed_ast::LocalId(0),
                ty: typed_ast::Type::Uint64,
                init: uint64(
                    typed_ast::ExprKind::Binary {
                        op: ast::BinOp::Add,
                        lhs: Box::new(uint64(typed_ast::ExprKind::Param(typed_ast::ParamId(0)), a)),
                        rhs: Box::new(uint64(typed_ast::ExprKind::Param(typed_ast::ParamId(1)), b)),
                    },
                    diag::Span {
                        start: a.start,
                        end: b.end,
                    },
                ),
                span: diag::Span {
                    start: var,
                    end: b.end,
                },
            },
            typed_ast::Stmt::Return {
                expr: uint64(typed_ast::ExprKind::Var(typed_ast::LocalId(0)), sum),
                span: diag::Span {
                    start: return_start,
                    end: sum.end,
                },
            },
        ]
    );
}

#[test]
fn a_parameter_is_in_scope_before_the_first_statement() {
    let source = taking("a uint64", "return a");
    assert_eq!(
        check_ok(&source).funcs[0].body,
        vec![typed_ast::Stmt::Return {
            expr: typed_ast::Expr {
                kind: typed_ast::ExprKind::Param(typed_ast::ParamId(0)),
                ty: typed_ast::Type::Uint64,
                span: testing::span_of(&source, "a", 1),
            },
            span: testing::span_of(&source, "return a", 0),
        }]
    );
}

#[test]
fn a_variable_may_not_share_a_parameters_name() {
    let source = taking("a uint64", "var a uint64 = 1 return a");
    let mut span = testing::spans(&source);
    span("a");
    span("var");

    assert_eq!(
        check_err(&source),
        vec![diag::Entry {
            kind: diag::Kind::DuplicateVariable {
                name: "a".to_string(),
            },
            span: span("a"),
        }]
    );
}

#[test]
fn a_duplicate_parameter_is_reported_at_the_later_one() {
    let source = taking("a uint64, a uint64", "return a");
    let mut span = testing::spans(&source);
    span("a");

    assert_eq!(
        check_err(&source),
        vec![diag::Entry {
            kind: diag::Kind::DuplicateVariable {
                name: "a".to_string(),
            },
            span: span("a"),
        }]
    );
}

#[test]
fn an_unknown_parameter_type_is_reported() {
    // The use of `a` reports nothing more: its declaration reported it.
    let source = taking("a bytes", "return a + 1");
    assert_eq!(
        check_err(&source),
        vec![diag::Entry {
            kind: diag::Kind::UnknownType {
                name: "bytes".to_string(),
            },
            span: testing::span_of(&source, "bytes", 0),
        }]
    );
}

/// `func f(p0 uint64, .. p<count-1> uint64) uint64 { return 1 }`.
fn parameters(count: usize) -> String {
    let params: Vec<String> = (0..count).map(|index| format!("p{index} uint64")).collect();
    taking(&params.join(", "), "return 1")
}

#[test]
fn a_function_may_declare_the_parameter_capacity() {
    let source = parameters(typed_ast::ParamId::CAPACITY);
    let params = &check_ok(&source).funcs[0].params;

    assert_eq!(params.len(), typed_ast::ParamId::CAPACITY);
    assert_eq!(params[typed_ast::ParamId::CAPACITY - 1].name.text, "p127");
}

#[test]
fn one_parameter_past_the_capacity_is_reported() {
    let source = parameters(typed_ast::ParamId::CAPACITY + 1);
    assert_eq!(
        check_err(&source),
        vec![diag::Entry {
            kind: diag::Kind::TooManyParameters {
                max: typed_ast::ParamId::CAPACITY,
            },
            span: testing::span_of(&source, "p128", 0),
        }]
    );
}

#[test]
fn a_parameter_does_not_outlive_its_function() {
    let source = "func a(x uint64) uint64 { return x } func b() uint64 { return x }";
    let mut span = testing::spans(source);
    span("x");
    span("x");

    assert_eq!(
        check_err(source),
        vec![diag::Entry {
            kind: diag::Kind::UndefinedVariable {
                name: "x".to_string(),
            },
            span: span("x"),
        }]
    );
}

#[test]
fn parameters_are_checked_when_the_body_fails() {
    let source = taking("a bytes", "var x uint64 = 1");
    let mut span = testing::spans(&source);
    span("func");
    let name = span("f");

    assert_eq!(
        check_err(&source),
        vec![
            diag::Entry {
                kind: diag::Kind::UnknownType {
                    name: "bytes".to_string(),
                },
                span: testing::span_of(&source, "bytes", 0),
            },
            diag::Entry {
                kind: diag::Kind::MissingReturn,
                span: name,
            },
        ]
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
        vec![diag::Entry {
            kind: diag::Kind::UndefinedVariable {
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
        vec![diag::Entry {
            kind: diag::Kind::UnknownType {
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
        vec![diag::Entry {
            kind: diag::Kind::MissingReturn,
            span: span("f"),
        }]
    );
}

#[test]
fn a_statement_after_a_return_is_reported() {
    let source = "func f() uint64 { return 1 return 2 }";
    assert_eq!(
        check_err(source),
        vec![diag::Entry {
            kind: diag::Kind::UnreachableStatement,
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
            op: ast::BinOp::Mul,
            lhs: Box::new(literal(2, 0)),
            rhs: Box::new(literal(3, 0)),
        },
        testing::span_of(source, "2 * 3", 0),
    );
    let sum = uint64(
        typed_ast::ExprKind::Binary {
            op: ast::BinOp::Add,
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
            diag::Entry {
                kind: diag::Kind::UnknownType {
                    name: "bytes".to_string(),
                },
                span: span("bytes"),
            },
            diag::Entry {
                kind: diag::Kind::MissingReturn,
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
        vec![diag::Entry {
            kind: diag::Kind::DuplicateFunction {
                name: "a".to_string(),
            },
            span: span("a"),
        }]
    );
}

#[test]
fn every_duplicate_is_reported_in_source_order() {
    let source =
        "func a() uint64 { return 1 } func a() uint64 { return 2 } func a() uint64 { return 3 }";
    let mut span = testing::spans(source);
    span("a");

    assert_eq!(
        check_err(source),
        vec![
            diag::Entry {
                kind: diag::Kind::DuplicateFunction {
                    name: "a".to_string(),
                },
                span: span("a"),
            },
            diag::Entry {
                kind: diag::Kind::DuplicateFunction {
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
            diag::Entry {
                kind: diag::Kind::DuplicateFunction {
                    name: "a".to_string(),
                },
                span: duplicate,
            },
            diag::Entry {
                kind: diag::Kind::UnknownType {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
            diag::Entry {
                kind: diag::Kind::TypeMismatch {
                    expected: typed_ast::Type::Bool,
                    found: typed_ast::Type::Uint64,
                },
                span: one,
            },
            diag::Entry {
                kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::UnknownType {
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
                span: diag::Span {
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
                span: diag::Span {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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

    let mismatch = diag::Entry {
        kind: diag::Kind::TypeMismatch {
            expected: typed_ast::Type::Uint64,
            found: typed_ast::Type::Bool,
        },
        span: left,
    };

    assert_eq!(
        check_err(&source),
        vec![
            mismatch.clone(),
            diag::Entry {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
            diag::Entry {
                kind: diag::Kind::UnknownType {
                    name: "bytes".to_string(),
                },
                span: span("bytes"),
            },
            diag::Entry {
                kind: diag::Kind::MissingReturn,
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
                op: ast::BinOp::Lt,
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
            span: diag::Span {
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
                op: ast::BinOp::Eq,
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
            span: diag::Span {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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

    let mismatch = diag::Entry {
        kind: diag::Kind::TypeMismatch {
            expected: typed_ast::Type::Uint64,
            found: typed_ast::Type::Bool,
        },
        span: left,
    };

    assert_eq!(
        check_err(source),
        vec![
            mismatch.clone(),
            diag::Entry {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
                expected: typed_ast::Type::Uint64,
                found: typed_ast::Type::Bool,
            },
            span: testing::span_of(source, "true", 0),
        }]
    );

    let source = "func approval() bool { return true == 1 }";
    assert_eq!(
        check_err(source),
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::UndefinedVariable {
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
                op: ast::UnOp::Not,
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
            ast::BinOp::And,
        ),
        (
            "func approval() bool { return true || false }",
            ast::BinOp::Or,
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
                span: diag::Span {
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
        let expected: Vec<diag::Entry> = reported
            .into_iter()
            .map(|(text, nth)| diag::Entry {
                kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
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
        vec![diag::Entry {
            kind: diag::Kind::UndefinedVariable {
                name: "x".to_string(),
            },
            span: testing::span_of(source, "x", 0),
        }]
    );
}

/// The example program of the calls milestone.
const CALLS: &str = "func approval() uint64 {\n\treturn add(1, double(2))\n}\n\n\
                     func add(a uint64, b uint64) uint64 {\n\treturn a + b\n}\n\n\
                     func double(x uint64) uint64 {\n\treturn x * 2\n}\n";

/// A `uint64` expression, with its type and span.
fn uint64(kind: typed_ast::ExprKind, span: diag::Span) -> typed_ast::Expr {
    typed_ast::Expr {
        kind,
        ty: typed_ast::Type::Uint64,
        span,
    }
}

#[test]
fn a_call_has_the_callees_return_type() {
    let source = CALLS;
    let mut span = testing::spans(source);

    let approval_start = span("func").start;
    let approval = testing::name("approval", span("approval"));
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
    let add = testing::name("add", span("add"));
    let a = testing::name("a", span("a"));
    span("uint64");
    let b = testing::name("b", span("b"));
    span("uint64");
    span("uint64");
    let add_return = span("return").start;
    let a_use = span("a");
    let b_use = span("b");
    let add_end = span("}").end;

    let double_start_decl = span("func").start;
    let double = testing::name("double", span("double"));
    let x = testing::name("x", span("x"));
    span("uint64");
    span("uint64");
    let double_return = span("return").start;
    let x_use = span("x");
    let two_use = span("2");
    let double_end_decl = span("}").end;

    let param = |name: ast::Name| typed_ast::Param {
        name,
        ty: typed_ast::Type::Uint64,
    };

    assert_eq!(
        check_ok(source),
        typed_ast::Program {
            funcs: vec![
                typed_ast::FuncDecl {
                    name: approval,
                    params: vec![],
                    ret: typed_ast::Type::Uint64,
                    body: vec![typed_ast::Stmt::Return {
                        expr: uint64(
                            typed_ast::ExprKind::Call {
                                callee: typed_ast::FuncId(1),
                                args: vec![
                                    uint64(typed_ast::ExprKind::IntLit(1), one),
                                    uint64(
                                        typed_ast::ExprKind::Call {
                                            callee: typed_ast::FuncId(2),
                                            args: vec![uint64(typed_ast::ExprKind::IntLit(2), two)],
                                        },
                                        diag::Span {
                                            start: double_start,
                                            end: double_end,
                                        },
                                    ),
                                ],
                            },
                            diag::Span {
                                start: call_start,
                                end: call_end,
                            },
                        ),
                        span: diag::Span {
                            start: approval_return,
                            end: call_end,
                        },
                    }],
                    span: diag::Span {
                        start: approval_start,
                        end: approval_end,
                    },
                },
                typed_ast::FuncDecl {
                    name: add,
                    params: vec![param(a), param(b)],
                    ret: typed_ast::Type::Uint64,
                    body: vec![typed_ast::Stmt::Return {
                        expr: uint64(
                            typed_ast::ExprKind::Binary {
                                op: ast::BinOp::Add,
                                lhs: Box::new(uint64(
                                    typed_ast::ExprKind::Param(typed_ast::ParamId(0)),
                                    a_use
                                )),
                                rhs: Box::new(uint64(
                                    typed_ast::ExprKind::Param(typed_ast::ParamId(1)),
                                    b_use
                                )),
                            },
                            diag::Span {
                                start: a_use.start,
                                end: b_use.end,
                            },
                        ),
                        span: diag::Span {
                            start: add_return,
                            end: b_use.end,
                        },
                    }],
                    span: diag::Span {
                        start: add_start,
                        end: add_end,
                    },
                },
                typed_ast::FuncDecl {
                    name: double,
                    params: vec![param(x)],
                    ret: typed_ast::Type::Uint64,
                    body: vec![typed_ast::Stmt::Return {
                        expr: uint64(
                            typed_ast::ExprKind::Binary {
                                op: ast::BinOp::Mul,
                                lhs: Box::new(uint64(
                                    typed_ast::ExprKind::Param(typed_ast::ParamId(0)),
                                    x_use
                                )),
                                rhs: Box::new(uint64(typed_ast::ExprKind::IntLit(2), two_use)),
                            },
                            diag::Span {
                                start: x_use.start,
                                end: two_use.end,
                            },
                        ),
                        span: diag::Span {
                            start: double_return,
                            end: two_use.end,
                        },
                    }],
                    span: diag::Span {
                        start: double_start_decl,
                        end: double_end_decl,
                    },
                },
            ]
        }
    );
}

/// The function the `index`th function of `source` calls in its `return`.
fn returned_callee(source: &str, index: usize) -> typed_ast::FuncId {
    let program = check_ok(source);
    let func = program.funcs.into_iter().nth(index).expect("the function");
    let Some(typed_ast::Stmt::Return { expr, .. }) = func.body.into_iter().next() else {
        panic!("a return statement")
    };
    let typed_ast::ExprKind::Call { callee, .. } = expr.kind else {
        panic!("a call")
    };
    callee
}

#[test]
fn a_call_may_precede_the_declaration() {
    let source = "func approval() uint64 { return f() } func f() uint64 { return 1 }";
    assert_eq!(returned_callee(source, 0), typed_ast::FuncId(1));
}

#[test]
fn a_call_may_follow_the_declaration() {
    let source = "func f() uint64 { return 1 } func approval() uint64 { return f() }";
    assert_eq!(returned_callee(source, 1), typed_ast::FuncId(0));
}

#[test]
fn an_undefined_function_is_reported() {
    let source = wrap("return f()");
    let mut span = testing::spans(&source);
    span("return");

    assert_eq!(
        check_err(&source),
        vec![diag::Entry {
            kind: diag::Kind::UndefinedFunction {
                name: "f".to_string(),
            },
            span: span("f"),
        }]
    );
}

#[test]
fn a_variable_is_not_a_function() {
    let source = wrap("var f uint64 = 1 return f()");
    let mut span = testing::spans(&source);
    span("var");
    span("f");

    assert_eq!(
        check_err(&source),
        vec![diag::Entry {
            kind: diag::Kind::UndefinedFunction {
                name: "f".to_string(),
            },
            span: span("f"),
        }]
    );
}

#[test]
fn a_function_is_not_a_variable() {
    let source = "func approval() uint64 { return f } func f() uint64 { return 1 }";
    let mut span = testing::spans(source);
    span("return");

    assert_eq!(
        check_err(source),
        vec![diag::Entry {
            kind: diag::Kind::UndefinedVariable {
                name: "f".to_string(),
            },
            span: span("f"),
        }]
    );
}

/// `func approval() uint64 { return <call> }`, then `add`, taking two
/// `uint64`s.
fn calling(call: &str) -> String {
    format!(
        "func approval() uint64 {{ return {call} }} \
         func add(a uint64, b uint64) uint64 {{ return a + b }}"
    )
}

#[test]
fn too_few_arguments_are_reported() {
    let source = calling("add(1)");
    assert_eq!(
        check_err(&source),
        vec![diag::Entry {
            kind: diag::Kind::WrongArgumentCount {
                name: "add".to_string(),
                expected: 2,
                found: 1,
            },
            span: testing::span_of(&source, "add(1)", 0),
        }]
    );
}

#[test]
fn too_many_arguments_are_reported() {
    let source = calling("add(1, 2, 3)");
    assert_eq!(
        check_err(&source),
        vec![diag::Entry {
            kind: diag::Kind::WrongArgumentCount {
                name: "add".to_string(),
                expected: 2,
                found: 3,
            },
            span: testing::span_of(&source, "add(1, 2, 3)", 0),
        }]
    );
}

#[test]
fn an_argument_must_have_the_parameters_type() {
    let source = calling("add(1, true)");
    assert_eq!(
        check_err(&source),
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
                expected: typed_ast::Type::Uint64,
                found: typed_ast::Type::Bool,
            },
            span: testing::span_of(&source, "true", 0),
        }]
    );
}

#[test]
fn every_argument_is_reported_in_source_order() {
    let source = calling("add(true, false)");
    let mismatch = |span| diag::Entry {
        kind: diag::Kind::TypeMismatch {
            expected: typed_ast::Type::Uint64,
            found: typed_ast::Type::Bool,
        },
        span,
    };
    assert_eq!(
        check_err(&source),
        vec![
            mismatch(testing::span_of(&source, "true", 0)),
            mismatch(testing::span_of(&source, "false", 0)),
        ]
    );
}

#[test]
fn a_miscounted_call_still_checks_its_arguments() {
    let source = calling("add(x)");
    assert_eq!(
        check_err(&source),
        vec![
            diag::Entry {
                kind: diag::Kind::WrongArgumentCount {
                    name: "add".to_string(),
                    expected: 2,
                    found: 1,
                },
                span: testing::span_of(&source, "add(x)", 0),
            },
            diag::Entry {
                kind: diag::Kind::UndefinedVariable {
                    name: "x".to_string(),
                },
                span: testing::span_of(&source, "x", 0),
            },
        ]
    );
}

#[test]
fn an_unresolved_parameter_type_leaves_the_argument_silent() {
    let source = "func approval() uint64 { return f(true) } func f(a bytes) uint64 { return 1 }";
    assert_eq!(
        check_err(source),
        vec![diag::Entry {
            kind: diag::Kind::UnknownType {
                name: "bytes".to_string(),
            },
            span: testing::span_of(source, "bytes", 0),
        }]
    );
}

#[test]
fn an_unresolved_return_type_leaves_the_call_silent() {
    let source = "func approval() uint64 { return f() } func f() bytes { return 1 }";
    assert_eq!(
        check_err(source),
        vec![diag::Entry {
            kind: diag::Kind::UnknownType {
                name: "bytes".to_string(),
            },
            span: testing::span_of(source, "bytes", 0),
        }]
    );
}

#[test]
fn a_call_returning_bool_is_a_boolean_operand() {
    let source = "func approval() bool { return !f() } func f() bool { return true }";
    assert_eq!(check_ok(source).funcs.len(), 2);

    let source = "func approval() bool { return !f() } func f() uint64 { return 1 }";
    assert_eq!(
        check_err(source),
        vec![diag::Entry {
            kind: diag::Kind::TypeMismatch {
                expected: typed_ast::Type::Bool,
                found: typed_ast::Type::Uint64,
            },
            span: testing::span_of(source, "f()", 0),
        }]
    );
}

#[test]
fn the_entry_point_may_be_called() {
    let source = "func approval() uint64 { return 1 } func f() uint64 { return approval() }";
    assert_eq!(returned_callee(source, 1), typed_ast::FuncId(0));
}

#[test]
fn a_signature_with_an_unknown_type_is_reported_once() {
    let called = "func approval() uint64 { return f(1) } func f(a bytes) uint64 { return 1 }";
    let uncalled = "func approval() uint64 { return 1 } func f(a bytes) uint64 { return 1 }";

    for source in [called, uncalled] {
        assert_eq!(
            check_err(source),
            vec![diag::Entry {
                kind: diag::Kind::UnknownType {
                    name: "bytes".to_string(),
                },
                span: testing::span_of(source, "bytes", 0),
            }],
            "{source}"
        );
    }
}
