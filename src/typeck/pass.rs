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

/// Checks one function. The rules are independent: a return type that does
/// not resolve still leaves the parameters and the body checked.
fn check_func(func: &ast::FuncDecl, diags: &mut diag::Sink) -> Option<typed_ast::FuncDecl> {
    let mut scope = Scope::default();
    let params = check_params(func, &mut scope, diags);
    let ret = resolve_type(&func.ret, diags);
    let body = check_body(func, ret, &mut scope, diags);

    Some(typed_ast::FuncDecl {
        name: func.name.clone(),
        params: params?,
        ret: ret?,
        body: body?,
        span: func.span,
    })
}

/// A function's scope: its parameters, then the variables its body declares.
/// They share one namespace, and a binding's index in its own list is its id.
/// A type of `None` is one that did not resolve.
#[derive(Default)]
struct Scope<'a> {
    params: Vec<(&'a str, Option<typed_ast::Type>)>,
    locals: Vec<(&'a str, Option<typed_ast::Type>)>,
}

impl Scope<'_> {
    /// Whether the scope already declares `name`.
    fn holds(&self, name: &str) -> bool {
        self.params
            .iter()
            .chain(&self.locals)
            .any(|(declared, _)| *declared == name)
    }
}

/// Checks a function's parameters, declaring each one. They are checked in
/// order, and each is reported whether or not another failed.
fn check_params<'a>(
    func: &'a ast::FuncDecl,
    scope: &mut Scope<'a>,
    diags: &mut diag::Sink,
) -> Option<Vec<typed_ast::Param>> {
    let mut params = Vec::new();
    let mut ok = true;

    for param in &func.params {
        let ty = resolve_type(&param.ty, diags);
        let declared = declare_param(param, ty, scope, diags).is_some();
        match ty.filter(|_| declared) {
            Some(ty) => params.push(typed_ast::Param {
                name: param.name.clone(),
                ty,
            }),
            None => ok = false,
        }
    }

    ok.then_some(params)
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

/// Checks a function body: every statement in it, in the scope its
/// parameters opened. The body must end with a `return`, and nothing may
/// follow one — of which only the first is reported.
fn check_body<'a>(
    func: &'a ast::FuncDecl,
    ret: Option<typed_ast::Type>,
    scope: &mut Scope<'a>,
    diags: &mut diag::Sink,
) -> Option<Vec<typed_ast::Stmt>> {
    let mut stmts = Vec::new();
    let mut ok = true;
    let mut returned = false;
    let mut unreachable = None;

    for stmt in &func.body {
        let (ast::Stmt::Var { span, .. } | ast::Stmt::Return { span, .. }) = stmt;
        if returned && unreachable.is_none() {
            unreachable = Some(*span);
        }
        match check_stmt(stmt, ret, scope, diags) {
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
    scope: &mut Scope<'a>,
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
            let init = check_expr(init, scope, diags);
            let ty = resolve_type(ty, diags);
            let agrees = check_type(init.as_ref(), ty, diags);
            // The name is declared with its written type even when the
            // initializer disagreed, so that later uses check against the
            // declaration.
            let local = declare(name, ty, scope, diags);
            agrees.then_some(())?;
            Some(typed_ast::Stmt::Var {
                local: local?,
                ty: ty?,
                init: init?,
                span: *span,
            })
        }
        ast::Stmt::Return { expr, span } => {
            let expr = check_expr(expr, scope, diags)?;
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

/// Declares a variable, reporting it if the scope already holds its name or
/// the frame has no room for it. A name that is reported is not declared.
fn declare<'a>(
    name: &'a ast::Name,
    ty: Option<typed_ast::Type>,
    scope: &mut Scope<'a>,
    diags: &mut diag::Sink,
) -> Option<typed_ast::LocalId> {
    let kind = if scope.holds(&name.text) {
        diag::Kind::DuplicateVariable {
            name: name.text.clone(),
        }
    } else if let Some(local) = typed_ast::LocalId::new(scope.locals.len()) {
        scope.locals.push((&name.text, ty));
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

/// Declares a parameter, reporting it if the scope already holds its name or
/// the function takes too many. A parameter that is reported is not declared.
fn declare_param<'a>(
    param: &'a ast::Param,
    ty: Option<typed_ast::Type>,
    scope: &mut Scope<'a>,
    diags: &mut diag::Sink,
) -> Option<typed_ast::ParamId> {
    let kind = if scope.holds(&param.name.text) {
        diag::Kind::DuplicateVariable {
            name: param.name.text.clone(),
        }
    } else if let Some(id) = typed_ast::ParamId::new(scope.params.len()) {
        scope.params.push((&param.name.text, ty));
        return Some(id);
    } else {
        diag::Kind::TooManyParameters {
            max: typed_ast::ParamId::CAPACITY,
        }
    };

    diags.push(diag::Entry {
        kind,
        span: param.name.span,
    });
    None
}

/// Checks one expression, giving it its type. A name that resolves to nothing
/// in scope and an operand of arithmetic that is not a `uint64` are what can
/// fail.
fn check_expr(expr: &ast::Expr, scope: &Scope, diags: &mut diag::Sink) -> Option<typed_ast::Expr> {
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
            let lhs = check_expr(lhs, scope, diags);
            let rhs = check_expr(rhs, scope, diags);
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
            let operand = check_expr(operand, scope, diags);
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
            let (kind, ty) = resolve_var(name, scope, diags)?;
            (kind, ty, *span)
        }
    };
    Some(typed_ast::Expr { kind, ty, span })
}

/// Resolves a name to the expression reading it and its type, reporting it if
/// it names neither a parameter nor a variable. A binding whose declared type
/// did not resolve is `None` without a report: the declaration reported it.
fn resolve_var(
    name: &ast::Name,
    scope: &Scope,
    diags: &mut diag::Sink,
) -> Option<(typed_ast::ExprKind, typed_ast::Type)> {
    if let Some((index, ty)) = lookup(&scope.params, name) {
        let id = typed_ast::ParamId::new(index)?;
        return Some((typed_ast::ExprKind::Param(id), ty?));
    }
    if let Some((index, ty)) = lookup(&scope.locals, name) {
        let id = typed_ast::LocalId::new(index)?;
        return Some((typed_ast::ExprKind::Var(id), ty?));
    }

    diags.push(diag::Entry {
        kind: diag::Kind::UndefinedVariable {
            name: name.text.clone(),
        },
        span: name.span,
    });
    None
}

/// The position and declared type of `name` among `bindings`, if they hold it.
fn lookup(
    bindings: &[(&str, Option<typed_ast::Type>)],
    name: &ast::Name,
) -> Option<(usize, Option<typed_ast::Type>)> {
    bindings
        .iter()
        .enumerate()
        .find(|(_, (declared, _))| *declared == name.text)
        .map(|(index, (_, ty))| (index, *ty))
}
