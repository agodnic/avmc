use crate::ast;
use crate::diag;
use crate::typed_ast;
use std::collections::HashSet;

/// Checks `program`: its declared names, then every function in source order.
///
/// Reports every problem it finds, and returns `None` if it found any.
pub fn check(program: &ast::Program, diags: &mut diag::Sink) -> Option<typed_ast::Program> {
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
fn check_duplicates(program: &ast::Program, diags: &mut diag::Sink) -> bool {
    let mut seen = HashSet::new();
    let mut ok = true;

    for func in &program.funcs {
        if !seen.insert(func.name.text.as_str()) {
            diags.push(diag::Entry {
                kind: diag::Kind::DuplicateFunction {
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
fn check_func(func: &ast::FuncDecl, diags: &mut diag::Sink) -> Option<typed_ast::FuncDecl> {
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
fn resolve_type(ret: &ast::TypeRef, diags: &mut diag::Sink) -> Option<typed_ast::Type> {
    match ret.name.text.as_str() {
        "uint64" => Some(typed_ast::Type::Uint64),
        "bool" => Some(typed_ast::Type::Bool),
        name => {
            diags.push(diag::Entry {
                kind: diag::Kind::UnknownType {
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
    diags: &mut diag::Sink,
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
        diags.push(diag::Entry {
            kind: diag::Kind::MissingReturn,
            span: func.name.span,
        });
        return None;
    }
    if let Some(span) = unreachable {
        diags.push(diag::Entry {
            kind: diag::Kind::UnreachableStatement,
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
    diags: &mut diag::Sink,
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
    diags: &mut diag::Sink,
) -> bool {
    let (Some(expr), Some(expected)) = (expr, expected) else {
        return true;
    };
    if expr.ty == expected {
        return true;
    }
    diags.push(diag::Entry {
        kind: diag::Kind::TypeMismatch {
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
    diags: &mut diag::Sink,
) -> Option<typed_ast::LocalId> {
    let kind = if locals.iter().any(|(declared, _)| *declared == name.text) {
        diag::Kind::DuplicateVariable {
            name: name.text.clone(),
        }
    } else if let Some(local) = typed_ast::LocalId::new(locals.len()) {
        locals.push((&name.text, ty));
        return Some(local);
    } else {
        diag::Kind::TooManyVariables {
            max: typed_ast::LocalId::CAPACITY,
        }
    };

    diags.push(diag::Entry {
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
    diags: &mut diag::Sink,
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
    diags: &mut diag::Sink,
) -> Option<(typed_ast::LocalId, typed_ast::Type)> {
    let Some((index, (_, ty))) = locals
        .iter()
        .enumerate()
        .find(|(_, (declared, _))| *declared == name.text)
    else {
        diags.push(diag::Entry {
            kind: diag::Kind::UndefinedVariable {
                name: name.text.clone(),
            },
            span: name.span,
        });
        return None;
    };
    Some((typed_ast::LocalId::new(index)?, (*ty)?))
}
