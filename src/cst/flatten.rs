use super::node::{Expr, FuncDecl, Program, Stmt};
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
    tokens.extend([
        func.func,
        func.name,
        func.lparen,
        func.rparen,
        func.ret,
        func.lbrace,
    ]);
    for stmt in &func.body {
        push_stmt(tokens, stmt);
    }
    tokens.push(func.rbrace);
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
    }
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
