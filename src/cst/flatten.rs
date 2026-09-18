use super::node::{Arg, Block, Expr, FuncDecl, IfStmt, Param, Program, Stmt};
use crate::lexer;

/// The tokens of `program` in source order, ending with `eof`.
pub fn tokens(program: &Program) -> Vec<lexer::Token> {
    let mut tokens = Vec::new();
    for func in &program.funcs {
        push_func(&mut tokens, func);
    }
    tokens.push(program.eof);
    tokens
}

fn push_func(tokens: &mut Vec<lexer::Token>, func: &FuncDecl) {
    tokens.extend([func.func, func.name, func.lparen]);
    for param in &func.params {
        push_param(tokens, param);
    }
    tokens.extend([func.rparen, func.ret]);
    push_block(tokens, &func.body);
}

fn push_block(tokens: &mut Vec<lexer::Token>, block: &Block) {
    tokens.push(block.lbrace);
    for stmt in &block.stmts {
        push_stmt(tokens, stmt);
    }
    tokens.push(block.rbrace);
}

fn push_param(tokens: &mut Vec<lexer::Token>, param: &Param) {
    tokens.extend([param.name, param.ty]);
    tokens.extend(param.comma);
}

fn push_stmt(tokens: &mut Vec<lexer::Token>, stmt: &Stmt) {
    match stmt {
        Stmt::Var {
            var,
            name,
            ty,
            equals,
            init,
        } => {
            tokens.extend([*var, *name, *ty, *equals]);
            push_expr(tokens, init);
        }
        Stmt::Return { ret, expr } => {
            tokens.push(*ret);
            push_expr(tokens, expr);
        }
        Stmt::If(stmt) => push_if(tokens, stmt),
    }
}

fn push_if(tokens: &mut Vec<lexer::Token>, stmt: &IfStmt) {
    tokens.push(stmt.keyword);
    push_expr(tokens, &stmt.cond);
    push_block(tokens, &stmt.then);
}

fn push_expr(tokens: &mut Vec<lexer::Token>, expr: &Expr) {
    match expr {
        Expr::IntLit(token) | Expr::BoolLit(token) | Expr::Var(token) => tokens.push(*token),
        Expr::Binary { lhs, op, rhs } => {
            push_expr(tokens, lhs);
            tokens.push(*op);
            push_expr(tokens, rhs);
        }
        Expr::Unary { op, operand } => {
            tokens.push(*op);
            push_expr(tokens, operand);
        }
        Expr::Call {
            callee,
            lparen,
            args,
            rparen,
        } => {
            tokens.extend([*callee, *lparen]);
            for arg in args {
                push_arg(tokens, arg);
            }
            tokens.push(*rparen);
        }
        Expr::Paren {
            lparen,
            inner,
            rparen,
        } => {
            tokens.push(*lparen);
            push_expr(tokens, inner);
            tokens.push(*rparen);
        }
    }
}

fn push_arg(tokens: &mut Vec<lexer::Token>, arg: &Arg) {
    push_expr(tokens, &arg.expr);
    tokens.extend(arg.comma);
}
