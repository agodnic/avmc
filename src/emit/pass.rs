use crate::ast;
use crate::diag;
use crate::ir;
use crate::typed_ast;

/// The TEAL version the output targets: the one MainNet runs.
const TEAL_VERSION: u8 = 13;

/// The function the program starts at.
pub(super) const ENTRY_POINT: &str = "approval";

/// Emits the TEAL text of `program`: a call to its entry point, then the
/// entry point's subroutine.
///
/// Any other function is dead code — nothing can call it yet — and is not
/// emitted.
pub fn emit(program: &ir::Program, diags: &mut diag::Sink) -> Option<String> {
    let func = entry_point(program, diags)?;

    let mut teal = format!("#pragma version {TEAL_VERSION}\ncallsub {ENTRY_POINT}\nreturn\n");
    teal.push_str(&function(func));
    Some(teal)
}

/// The subroutine for `func`: its label, its frame, and its body.
pub(super) fn function(func: &ir::Function) -> String {
    let params = func.params.len();
    let mut teal = format!("{}:\nproto {params} 1\n", func.name);
    for ty in &func.locals {
        teal.push_str(placeholder(*ty));
        teal.push('\n');
    }
    for inst in &func.insts {
        teal.push_str(&line(inst, params));
        teal.push('\n');
    }
    teal
}

/// The line that allocates a frame slot of type `ty`.
fn placeholder(ty: typed_ast::Type) -> &'static str {
    match ty {
        typed_ast::Type::Uint64 => "pushint 0",
        typed_ast::Type::Bool => "pushint 0",
    }
}

/// Finds the entry point, reporting it if there is none, or if it takes
/// parameters: it is called with nothing on the stack.
fn entry_point<'a>(program: &'a ir::Program, diags: &mut diag::Sink) -> Option<&'a ir::Function> {
    let Some(func) = program.funcs.iter().find(|func| func.name == ENTRY_POINT) else {
        diags.push(diag::Entry {
            kind: diag::Kind::MissingEntryPoint { name: ENTRY_POINT },
            // There is no token to point at.
            span: diag::Span { start: 0, end: 0 },
        });
        return None;
    };

    if !func.params.is_empty() {
        diags.push(diag::Entry {
            kind: diag::Kind::EntryPointTakesParameters { name: ENTRY_POINT },
            span: func.span,
        });
        return None;
    }
    Some(func)
}

/// The opcode an instruction emits.
fn opcode(inst: &ir::Inst) -> &'static str {
    match inst {
        ir::Inst::Const { .. } => "pushint",
        ir::Inst::Binary { op, .. } => match op {
            ast::BinOp::Add => "+",
            ast::BinOp::Sub => "-",
            ast::BinOp::Mul => "*",
            ast::BinOp::Div => "/",
            ast::BinOp::Mod => "%",
            ast::BinOp::Eq => "==",
            ast::BinOp::Ne => "!=",
            ast::BinOp::Lt => "<",
            ast::BinOp::Le => "<=",
            ast::BinOp::Gt => ">",
            ast::BinOp::Ge => ">=",
            ast::BinOp::And => "&&",
            ast::BinOp::Or => "||",
        },
        ir::Inst::Unary { op, .. } => match op {
            ast::UnOp::Not => "!",
        },
        ir::Inst::Store { .. } => "frame_bury",
        ir::Inst::Load { .. } | ir::Inst::LoadParam { .. } => "frame_dig",
        ir::Inst::Return { .. } => "retsub",
    }
}

/// The line an instruction emits, without its terminator. `params` is how
/// many parameters the enclosing function takes.
///
/// `ValueId`s are not consulted: every instruction consumes its operands from
/// the top of the stack, where the instructions that defined them left them.
fn line(inst: &ir::Inst, params: usize) -> String {
    match inst {
        ir::Inst::Const { value, .. } => format!("{} {value}", opcode(inst)),
        ir::Inst::Store { local, .. } | ir::Inst::Load { local, .. } => {
            format!("{} {}", opcode(inst), local.0)
        }
        ir::Inst::LoadParam { param, .. } => {
            format!("{} {}", opcode(inst), offset(*param, params))
        }
        ir::Inst::Binary { .. } | ir::Inst::Unary { .. } | ir::Inst::Return { .. } => {
            opcode(inst).to_string()
        }
    }
}

/// The `frame_dig` offset of `param` in a function taking `params` of them:
/// with `proto`, the arguments sit below the frame pointer, the first at
/// `-params` and the last at `-1`.
fn offset(param: typed_ast::ParamId, params: usize) -> i64 {
    i64::from(param.0) - i64::try_from(params).unwrap_or(i64::MAX)
}
