//! Emission: IR to TEAL text, in a single linear pass.

use crate::ast;
use crate::diag;
use crate::ir;
use crate::typed_ast;

/// The TEAL version the output targets: the one MainNet runs.
const TEAL_VERSION: u8 = 13;

/// The function the program starts at.
const ENTRY_POINT: &str = "approval";

/// Emits the TEAL text of `program`: a call to its entry point, then the
/// entry point's subroutine.
///
/// Any other function is dead code — nothing can call it yet — and is not
/// emitted.
pub fn emit(program: &ir::Program, diags: &mut diag::Sink) -> Option<String> {
    let entry = entry_point(program, diags)?;

    let mut teal = format!("#pragma version {TEAL_VERSION}\ncallsub {ENTRY_POINT}\nreturn\n");
    teal.push_str(&function(entry));
    Some(teal)
}

/// The subroutine for `func`: its label, its frame, and its body.
fn function(func: &ir::Function) -> String {
    let mut teal = format!("{}:\nproto 0 1\n", func.name);
    for ty in &func.locals {
        teal.push_str(placeholder(*ty));
        teal.push('\n');
    }
    for inst in &func.insts {
        teal.push_str(&line(inst));
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

/// Finds the entry point, reporting it if there is none.
fn entry_point<'a>(program: &'a ir::Program, diags: &mut diag::Sink) -> Option<&'a ir::Function> {
    let entry = program.funcs.iter().find(|func| func.name == ENTRY_POINT);

    if entry.is_none() {
        diags.push(diag::Entry {
            kind: diag::Kind::MissingEntryPoint { name: ENTRY_POINT },
            // There is no token to point at.
            span: diag::Span { start: 0, end: 0 },
        });
    }
    entry
}

/// The opcode an instruction emits.
fn opcode(inst: &ir::Inst) -> &'static str {
    match inst {
        ir::Inst::Const { .. } => "pushint",
        ir::Inst::Binary { op, .. } => match op {
            ast::BinaryOp::Add => "+",
            ast::BinaryOp::Sub => "-",
            ast::BinaryOp::Mul => "*",
            ast::BinaryOp::Div => "/",
            ast::BinaryOp::Mod => "%",
            ast::BinaryOp::Eq => "==",
            ast::BinaryOp::Ne => "!=",
            ast::BinaryOp::Lt => "<",
            ast::BinaryOp::Le => "<=",
            ast::BinaryOp::Gt => ">",
            ast::BinaryOp::Ge => ">=",
            ast::BinaryOp::And => "&&",
            ast::BinaryOp::Or => "||",
        },
        ir::Inst::Unary { op, .. } => match op {
            ast::UnaryOp::Not => "!",
        },
        ir::Inst::Store { .. } => "frame_bury",
        ir::Inst::Load { .. } => "frame_dig",
        ir::Inst::Return { .. } => "retsub",
    }
}

/// The line an instruction emits, without its terminator.
///
/// `ValueId`s are not consulted: every instruction consumes its operands from
/// the top of the stack, where the instructions that defined them left them.
fn line(inst: &ir::Inst) -> String {
    match inst {
        ir::Inst::Const { value, .. } => format!("{} {value}", opcode(inst)),
        ir::Inst::Store { local, .. } | ir::Inst::Load { local, .. } => {
            format!("{} {}", opcode(inst), local.0)
        }
        ir::Inst::Binary { .. } | ir::Inst::Unary { .. } | ir::Inst::Return { .. } => {
            opcode(inst).to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower;
    use crate::testing;

    /// The span every hand-built instruction carries.
    const ZERO: diag::Span = diag::Span { start: 0, end: 0 };

    /// Emits `source`, asserting that it produced no diagnostics.
    fn emit_ok(source: &str) -> String {
        let mut diags = diag::Sink::default();
        let teal = pipeline(source, &mut diags);
        assert!(diags.is_empty());
        teal.expect("emission succeeded")
    }

    /// Emits `source`, asserting that it emitted nothing, and returning the
    /// diagnostics in the order they were reported.
    fn emit_err(source: &str) -> Vec<diag::Entry> {
        let mut diags = diag::Sink::default();
        assert_eq!(pipeline(source, &mut diags), None);
        diags.iter().cloned().collect()
    }

    fn pipeline(source: &str, diags: &mut diag::Sink) -> Option<String> {
        let ir =
            lower::lower(&testing::lex_parse_check(source), diags).expect("lowering succeeded");
        emit(&ir, diags)
    }

    /// Emits a hand-built program: one function named `approval` returning
    /// `ret`, with `locals` as its frame and `insts` as its body, at the zero
    /// span throughout. `emit_ok` and `emit_err` go through source, which
    /// cannot produce a frame yet.
    fn hand_built(
        ret: typed_ast::Type,
        locals: Vec<typed_ast::Type>,
        insts: Vec<ir::Inst>,
    ) -> String {
        let program = ir::Program {
            funcs: vec![ir::Function {
                name: ENTRY_POINT.to_string(),
                ret,
                locals,
                insts,
                span: ZERO,
            }],
        };
        let mut diags = diag::Sink::default();
        let teal = emit(&program, &mut diags);
        assert!(diags.is_empty());
        teal.expect("emission succeeded")
    }

    fn constant(dest: u32, value: u64) -> ir::Inst {
        constant_of(dest, typed_ast::Type::Uint64, value)
    }

    fn constant_of(dest: u32, ty: typed_ast::Type, value: u64) -> ir::Inst {
        ir::Inst::Const {
            dest: ir::ValueId(dest),
            ty,
            value,
            span: ZERO,
        }
    }

    fn binary(dest: u32, op: ast::BinaryOp, lhs: u32, rhs: u32) -> ir::Inst {
        ir::Inst::Binary {
            dest: ir::ValueId(dest),
            op,
            lhs: ir::ValueId(lhs),
            rhs: ir::ValueId(rhs),
            span: ZERO,
        }
    }

    fn unary(dest: u32, op: ast::UnaryOp, operand: u32) -> ir::Inst {
        ir::Inst::Unary {
            dest: ir::ValueId(dest),
            op,
            operand: ir::ValueId(operand),
            span: ZERO,
        }
    }

    fn store(local: u8, value: u32) -> ir::Inst {
        ir::Inst::Store {
            local: typed_ast::LocalId(local),
            value: ir::ValueId(value),
            span: ZERO,
        }
    }

    fn load(dest: u32, local: u8) -> ir::Inst {
        ir::Inst::Load {
            dest: ir::ValueId(dest),
            local: typed_ast::LocalId(local),
            span: ZERO,
        }
    }

    fn ret(value: u32) -> ir::Inst {
        ir::Inst::Return {
            value: ir::ValueId(value),
            span: ZERO,
        }
    }

    /// `var x uint64 = 1; return x`, as the next slice will lower it.
    fn one_slot() -> Vec<ir::Inst> {
        vec![constant(0, 1), store(0, 0), load(1, 0), ret(1)]
    }

    #[test]
    fn a_frame_slot_is_allocated_and_addressed() {
        assert_eq!(
            hand_built(
                typed_ast::Type::Uint64,
                vec![typed_ast::Type::Uint64],
                one_slot()
            ),
            "#pragma version 13\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 0\n\
             pushint 1\n\
             frame_bury 0\n\
             frame_dig 0\n\
             retsub\n"
        );
    }

    #[test]
    fn two_frame_slots() {
        // `var x uint64 = 1 + 2; var y uint64 = x * 3; return y - x`.
        let insts = vec![
            constant(0, 1),
            constant(1, 2),
            binary(2, ast::BinaryOp::Add, 0, 1),
            store(0, 2),
            load(3, 0),
            constant(4, 3),
            binary(5, ast::BinaryOp::Mul, 3, 4),
            store(1, 5),
            load(6, 1),
            load(7, 0),
            binary(8, ast::BinaryOp::Sub, 6, 7),
            ret(8),
        ];
        assert_eq!(
            hand_built(
                typed_ast::Type::Uint64,
                vec![typed_ast::Type::Uint64; 2],
                insts
            ),
            "#pragma version 13\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 0\n\
             pushint 0\n\
             pushint 1\n\
             pushint 2\n\
             +\n\
             frame_bury 0\n\
             frame_dig 0\n\
             pushint 3\n\
             *\n\
             frame_bury 1\n\
             frame_dig 1\n\
             frame_dig 0\n\
             -\n\
             retsub\n"
        );
    }

    #[test]
    fn a_bool_constant_is_returned() {
        assert_eq!(
            hand_built(
                typed_ast::Type::Bool,
                vec![],
                vec![constant_of(0, typed_ast::Type::Bool, 1), ret(0)]
            ),
            "#pragma version 13\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 1\n\
             retsub\n"
        );
    }

    #[test]
    fn a_boolean_literal_is_compiled() {
        assert_eq!(
            emit_ok("func approval() bool { return true }"),
            "#pragma version 13\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 1\n\
             retsub\n"
        );
        assert_eq!(
            emit_ok("func approval() bool { return false }"),
            "#pragma version 13\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 0\n\
             retsub\n"
        );
    }

    #[test]
    fn a_bool_slot_starts_as_false() {
        // `var ok bool = true; return ok`, as a later slice will lower it.
        let insts = vec![
            constant_of(0, typed_ast::Type::Bool, 1),
            store(0, 0),
            load(1, 0),
            ret(1),
        ];
        assert_eq!(
            hand_built(typed_ast::Type::Bool, vec![typed_ast::Type::Bool], insts),
            "#pragma version 13\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 0\n\
             pushint 1\n\
             frame_bury 0\n\
             frame_dig 0\n\
             retsub\n"
        );
    }

    /// The five lines every hand-built program starts with.
    const PROLOGUE: &str = "#pragma version 13\n\
                            callsub approval\n\
                            return\n\
                            approval:\n\
                            proto 0 1\n";

    #[test]
    fn a_comparison_emits_its_mnemonic() {
        let cases = [
            (ast::BinaryOp::Eq, "=="),
            (ast::BinaryOp::Ne, "!="),
            (ast::BinaryOp::Lt, "<"),
            (ast::BinaryOp::Le, "<="),
            (ast::BinaryOp::Gt, ">"),
            (ast::BinaryOp::Ge, ">="),
        ];
        for (op, mnemonic) in cases {
            let insts = vec![constant(0, 1), constant(1, 2), binary(2, op, 0, 1), ret(2)];
            assert_eq!(
                hand_built(typed_ast::Type::Bool, vec![], insts),
                format!("{PROLOGUE}pushint 1\npushint 2\n{mnemonic}\nretsub\n"),
                "{op:?}"
            );
        }
    }

    #[test]
    fn logic_emits_its_mnemonic() {
        for (op, mnemonic) in [(ast::BinaryOp::And, "&&"), (ast::BinaryOp::Or, "||")] {
            let insts = vec![
                constant_of(0, typed_ast::Type::Bool, 1),
                constant_of(1, typed_ast::Type::Bool, 0),
                binary(2, op, 0, 1),
                ret(2),
            ];
            assert_eq!(
                hand_built(typed_ast::Type::Bool, vec![], insts),
                format!("{PROLOGUE}pushint 1\npushint 0\n{mnemonic}\nretsub\n"),
                "{op:?}"
            );
        }
    }

    #[test]
    fn negation_emits_its_mnemonic() {
        let insts = vec![
            constant_of(0, typed_ast::Type::Bool, 1),
            unary(1, ast::UnaryOp::Not, 0),
            ret(1),
        ];
        assert_eq!(
            hand_built(typed_ast::Type::Bool, vec![], insts),
            format!("{PROLOGUE}pushint 1\n!\nretsub\n")
        );
    }

    #[test]
    fn a_comparison_joined_by_logic() {
        // `!(1 < 2 && true)`.
        let insts = vec![
            constant(0, 1),
            constant(1, 2),
            binary(2, ast::BinaryOp::Lt, 0, 1),
            constant_of(3, typed_ast::Type::Bool, 1),
            binary(4, ast::BinaryOp::And, 2, 3),
            unary(5, ast::UnaryOp::Not, 4),
            ret(5),
        ];
        assert_eq!(
            hand_built(typed_ast::Type::Bool, vec![], insts),
            "#pragma version 13\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 1\n\
             pushint 2\n\
             <\n\
             pushint 1\n\
             &&\n\
             !\n\
             retsub\n"
        );
    }

    #[test]
    fn example_program() {
        assert_eq!(
            emit_ok(testing::EXAMPLE),
            "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 1\nretsub\n"
        );
    }

    #[test]
    fn zero() {
        assert_eq!(
            emit_ok("func approval() uint64 { return 0 }"),
            "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 0\nretsub\n"
        );
    }

    #[test]
    fn largest_uint64() {
        assert_eq!(
            emit_ok("func approval() uint64 { return 18446744073709551615 }"),
            "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 18446744073709551615\nretsub\n"
        );
    }

    #[test]
    fn arithmetic() {
        assert_eq!(
            emit_ok("func approval() uint64 { return (1 + 2) * 3 - 4 / 5 }"),
            "#pragma version 13\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 1\n\
             pushint 2\n\
             +\n\
             pushint 3\n\
             *\n\
             pushint 4\n\
             pushint 5\n\
             /\n\
             -\n\
             retsub\n"
        );
    }

    #[test]
    fn the_remainder_opcode() {
        assert_eq!(
            emit_ok("func approval() uint64 { return 7 % 4 }"),
            "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 7\npushint 4\n%\nretsub\n"
        );
    }

    #[test]
    fn only_the_entry_point_is_emitted() {
        assert_eq!(
            emit_ok("func f() uint64 { return 2 } func approval() uint64 { return 1 }"),
            "#pragma version 13\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 1\nretsub\n"
        );
    }

    #[test]
    fn missing_entry_point() {
        assert_eq!(
            emit_err("func f() uint64 { return 1 }"),
            vec![diag::Entry {
                kind: diag::Kind::MissingEntryPoint { name: "approval" },
                span: diag::Span { start: 0, end: 0 },
            }]
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(
            emit_err(""),
            vec![diag::Entry {
                kind: diag::Kind::MissingEntryPoint { name: "approval" },
                span: diag::Span { start: 0, end: 0 },
            }]
        );
    }

    /// The example program of the comparison milestone.
    const COMPARISONS: &str = "func approval() bool {\n  var x uint64 = 1 + 2\n  \
                               var big bool = x * 2 >= 6\n  return big == (x != 4)\n}\n";

    #[test]
    fn emits_the_comparisons_program() {
        assert_eq!(
            emit_ok(COMPARISONS),
            "#pragma version 13\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 0\n\
             pushint 0\n\
             pushint 1\n\
             pushint 2\n\
             +\n\
             frame_bury 0\n\
             frame_dig 0\n\
             pushint 2\n\
             *\n\
             pushint 6\n\
             >=\n\
             frame_bury 1\n\
             frame_dig 1\n\
             frame_dig 0\n\
             pushint 4\n\
             !=\n\
             ==\n\
             retsub\n"
        );
    }

    /// The example program of the comparison-and-logic milestone.
    const LOGIC: &str = "func approval() bool {\n  var x uint64 = 1 + 2\n  \
                         var odd bool = x % 2 == 1\n  \
                         return !(x > 5) && (odd || x == 4)\n}\n";

    #[test]
    fn emits_the_logic_program() {
        assert_eq!(
            emit_ok(LOGIC),
            "#pragma version 13\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 0\n\
             pushint 0\n\
             pushint 1\n\
             pushint 2\n\
             +\n\
             frame_bury 0\n\
             frame_dig 0\n\
             pushint 2\n\
             %\n\
             pushint 1\n\
             ==\n\
             frame_bury 1\n\
             frame_dig 0\n\
             pushint 5\n\
             >\n\
             !\n\
             frame_dig 1\n\
             frame_dig 0\n\
             pushint 4\n\
             ==\n\
             ||\n\
             &&\n\
             retsub\n"
        );
    }
}
