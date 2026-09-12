use crate::ast;
use crate::diag;
use crate::ir;
use crate::typed_ast;

/// The TEAL version the output targets: the one MainNet runs.
const TEAL_VERSION: u8 = 13;

/// The function the program starts at.
pub(super) const ENTRY_POINT: &str = "approval";

/// Emits the TEAL text of `program`: a call to its entry point, then the
/// entry point's subroutine, then those of every function it reaches,
/// directly or transitively, in source order, each once.
///
/// A function nothing reaches is dead code and is not emitted.
pub fn emit(program: &ir::Program, diags: &mut diag::Sink) -> Option<String> {
    let entry = entry_point(program, diags)?;

    let mut teal = format!("#pragma version {TEAL_VERSION}\ncallsub {ENTRY_POINT}\nreturn\n");
    for func in reached(program, entry) {
        teal.push_str(&function(program, func));
    }
    Some(teal)
}

/// The functions the entry point reaches, in the order they are emitted: the
/// entry point, then the rest in source order.
fn reached(program: &ir::Program, entry: usize) -> Vec<&ir::Function> {
    let mut reached = vec![false; program.funcs.len()];
    let mut pending = vec![entry];

    while let Some(index) = pending.pop() {
        let (Some(func), Some(seen)) = (program.funcs.get(index), reached.get_mut(index)) else {
            continue;
        };
        if std::mem::replace(seen, true) {
            continue;
        }
        pending.extend(func.insts.iter().filter_map(callee_index));
    }

    let rest = reached
        .iter()
        .enumerate()
        .filter(|&(index, &reached)| reached && index != entry)
        .map(|(index, _)| index);
    std::iter::once(entry)
        .chain(rest)
        .filter_map(|index| program.funcs.get(index))
        .collect()
}

/// The position of the function `inst` calls, if it is a call.
fn callee_index(inst: &ir::Inst) -> Option<usize> {
    let ir::Inst::Call { callee, .. } = inst else {
        return None;
    };
    usize::try_from(callee.0).ok()
}

/// The subroutine for `func`: its label, its frame, and its body. `program`
/// is the unit it belongs to, which names the functions it calls.
pub(super) fn function(program: &ir::Program, func: &ir::Function) -> String {
    let params = func.params.len();
    let mut teal = format!("{}:\nproto {params} 1\n", func.name);
    for ty in &func.locals {
        teal.push_str(placeholder(*ty));
        teal.push('\n');
    }
    for inst in &func.insts {
        teal.push_str(&line(program, inst, params));
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

/// Finds the entry point's position, reporting it if there is none, or if it
/// takes parameters: it is called with nothing on the stack.
fn entry_point(program: &ir::Program, diags: &mut diag::Sink) -> Option<usize> {
    let Some((index, func)) = program
        .funcs
        .iter()
        .enumerate()
        .find(|(_, func)| func.name == ENTRY_POINT)
    else {
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
    Some(index)
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
        ir::Inst::Call { .. } => "callsub",
        ir::Inst::Store { .. } => "frame_bury",
        ir::Inst::Load { .. } | ir::Inst::LoadParam { .. } => "frame_dig",
        ir::Inst::Return { .. } => "retsub",
    }
}

/// The line an instruction emits, without its terminator. `params` is how
/// many parameters the enclosing function takes, and `program` names the
/// function a call calls.
///
/// `ValueId`s are not consulted: every instruction consumes its operands from
/// the top of the stack, where the instructions that defined them left them.
fn line(program: &ir::Program, inst: &ir::Inst, params: usize) -> String {
    match inst {
        ir::Inst::Const { value, .. } => format!("{} {}", opcode(inst), value.word()),
        ir::Inst::Store { local, .. } | ir::Inst::Load { local, .. } => {
            format!("{} {}", opcode(inst), local.0)
        }
        ir::Inst::LoadParam { param, .. } => {
            format!("{} {}", opcode(inst), offset(*param, params))
        }
        ir::Inst::Call { .. } => format!("{} {}", opcode(inst), callee_name(program, inst)),
        ir::Inst::Binary { .. } | ir::Inst::Unary { .. } | ir::Inst::Return { .. } => {
            opcode(inst).to_string()
        }
    }
}

/// The name of the function a call calls, empty if the program does not
/// define it — which the verifier rejects.
fn callee_name<'a>(program: &'a ir::Program, inst: &ir::Inst) -> &'a str {
    callee_index(inst)
        .and_then(|index| program.funcs.get(index))
        .map_or("", |func| func.name.as_str())
}

/// The `frame_dig` offset of `param` in a function taking `params` of them:
/// with `proto`, the arguments sit below the frame pointer, the first at
/// `-params` and the last at `-1`.
fn offset(param: typed_ast::ParamId, params: usize) -> i64 {
    i64::from(param.0) - i64::try_from(params).unwrap_or(i64::MAX)
}
