use crate::diag;
use crate::ir;
use crate::typed_ast;

/// Lowers every function in `program`, in source order.
pub fn lower(program: &typed_ast::Program, _diags: &mut diag::Sink) -> Option<ir::Program> {
    let lowered = ir::Program {
        funcs: program.funcs.iter().map(lower_func).collect(),
    };

    #[cfg(debug_assertions)]
    for func in &lowered.funcs {
        // Only a compiler bug can reach this.
        #[expect(clippy::panic, reason = "a verifier failure is a compiler bug")]
        if let Err(violation) = crate::ir::verify(&lowered, func) {
            panic!("{violation}");
        }
    }

    Some(lowered)
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
        params: func.params.iter().map(|param| param.ty).collect(),
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
        typed_ast::ExprKind::Param(param) => {
            let dest = next_value_id(next_value);
            insts.push(ir::Inst::LoadParam {
                dest,
                param: *param,
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
        typed_ast::ExprKind::Call { callee, args } => {
            let args = args
                .iter()
                .map(|arg| lower_expr(arg, insts, next_value))
                .collect();
            let dest = next_value_id(next_value);
            insts.push(ir::Inst::Call {
                dest,
                callee: *callee,
                args,
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
