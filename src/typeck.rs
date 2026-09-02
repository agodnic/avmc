//! The type checker: an AST to a typed AST, resolving every type it names
//! (see `ARCHITECTURE.md` §2.1).

use crate::ast;
use crate::diagnostics::{Diagnostic, Diagnostics};
use crate::typed_ast::{Expr, ExprKind, FuncDecl, Program, Stmt, Type};

/// The one type name the language has.
const UINT64: &str = "uint64";

/// Checks every function in `program`, in source order.
///
/// Reports every problem it finds, and returns `None` if it found any (R2).
pub fn check(program: &ast::Program, diags: &mut Diagnostics) -> Option<Program> {
    let mut funcs = Vec::new();
    let mut ok = true;

    for func in &program.funcs {
        match check_func(func, diags) {
            Some(func) => funcs.push(func),
            None => ok = false,
        }
    }

    ok.then_some(Program { funcs })
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

/// Resolves a written type name, reporting E0004 if it names no type.
fn resolve_type(ret: &ast::TypeRef, diags: &mut Diagnostics) -> Option<Type> {
    if ret.name.text == UINT64 {
        return Some(Type::Uint64);
    }
    diags.push(Diagnostic {
        code: "E0004",
        message: format!("unknown type `{}`", ret.name.text),
        span: ret.name.span,
    });
    None
}

/// Checks a function body: it must end with a `return` (E0005), and nothing
/// may follow one (E0006, reported for the first such statement only).
fn check_body(func: &ast::FuncDecl, diags: &mut Diagnostics) -> Option<Vec<Stmt>> {
    let mut stmts = Vec::new();
    let mut returned = false;
    let mut unreachable = None;

    for stmt in &func.body {
        let ast::Stmt::Return { expr, span } = stmt;
        if returned && unreachable.is_none() {
            unreachable = Some(*span);
        }
        stmts.push(Stmt::Return {
            expr: check_expr(expr),
            span: *span,
        });
        returned = true;
    }

    if !returned {
        diags.push(Diagnostic {
            code: "E0005",
            message: "missing return".to_string(),
            span: func.name.span,
        });
        return None;
    }
    if let Some(span) = unreachable {
        diags.push(Diagnostic {
            code: "E0006",
            message: "unreachable statement".to_string(),
            span,
        });
        return None;
    }
    Some(stmts)
}

fn check_expr(expr: &ast::Expr) -> Expr {
    let ast::Expr::IntLit { value, span } = expr;
    Expr {
        kind: ExprKind::IntLit(*value),
        ty: Type::Uint64,
        span: *span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Name;
    use crate::diagnostics::Span;
    use crate::lexer::lex;
    use crate::parser::parse;

    /// Lexes, parses and checks `source`, asserting that it produced no
    /// diagnostics.
    fn check_ok(source: &str) -> Program {
        let mut diags = Diagnostics::default();
        let tokens = lex(source, &mut diags).expect("lexing succeeded");
        let parsed = parse(source, &tokens, &mut diags).expect("parsing succeeded");
        let program = check(&parsed, &mut diags);
        assert!(diags.is_empty());
        program.expect("checking succeeded")
    }

    /// Lexes, parses and checks `source`, asserting that checking produced
    /// nothing, and returning the diagnostics in the order they were reported.
    fn check_err(source: &str) -> Vec<Diagnostic> {
        let mut diags = Diagnostics::default();
        let tokens = lex(source, &mut diags).expect("lexing succeeded");
        let parsed = parse(source, &tokens, &mut diags).expect("parsing succeeded");
        assert_eq!(check(&parsed, &mut diags), None);
        diags.iter().cloned().collect()
    }

    /// Returns a closure giving the span of the next occurrence of its
    /// argument, so expected spans are written in source order.
    fn spans(source: &str) -> impl FnMut(&str) -> Span + '_ {
        let mut offset = 0;
        move |text| {
            let start = source[offset..].find(text).expect("text in source") + offset;
            offset = start + text.len();
            Span { start, end: offset }
        }
    }

    fn name(text: &str, span: Span) -> Name {
        Name {
            text: text.to_string(),
            span,
        }
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
                code: "E0004",
                message: "unknown type `bytes`".to_string(),
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
                code: "E0005",
                message: "missing return".to_string(),
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
                code: "E0006",
                message: "unreachable statement".to_string(),
                span: spans(source)("return 2"),
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
                    code: "E0004",
                    message: "unknown type `bytes`".to_string(),
                    span: span("bytes"),
                },
                Diagnostic {
                    code: "E0005",
                    message: "missing return".to_string(),
                    span: span("b"),
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
                    code: "E0004",
                    message: "unknown type `bytes`".to_string(),
                    span: span("bytes"),
                },
                Diagnostic {
                    code: "E0005",
                    message: "missing return".to_string(),
                    span: func_name,
                },
            ]
        );
    }
}
