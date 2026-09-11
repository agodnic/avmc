use crate::ast;
use crate::diag;
use crate::emit;
use crate::lexer;
use crate::lower;
use crate::parser;
use crate::typeck;

/// Compiles `source` to TEAL text, stopping at the first stage that fails.
pub fn compile(source: &str, diags: &mut diag::Sink) -> Option<String> {
    let tokens = lexer::lex(source, diags)?;
    let parsed = parser::parse(&tokens, diags)?;
    let program = ast::from_cst(source, &parsed, diags)?;
    let checked = typeck::check(&program, diags)?;
    let ir = lower::lower(&checked, diags)?;
    emit::emit(&ir, diags)
}
