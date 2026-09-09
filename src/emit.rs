//! Emission: IR to TEAL text, in a single linear pass.

use crate::ast::{BinaryOp, UnaryOp};
use crate::diagnostics::{Diagnostic, DiagnosticKind, Diagnostics, Span};
use crate::ir::{self, Function, Inst};
use crate::typed_ast::Type;

/// The TEAL version the output targets.
///
/// A value of this type is always a version the AVM accepts: it can only be
/// built by [`TealVersion::new`], which bounds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TealVersion(u8);

impl TealVersion {
    /// The oldest TEAL version the AVM accepts.
    pub const MIN: u8 = 1;
    /// The newest TEAL version the AVM accepts.
    pub const MAX: u8 = 11;

    /// `version`, or `None` if it is outside the supported range.
    pub fn new(version: u8) -> Option<Self> {
        (Self::MIN..=Self::MAX)
            .contains(&version)
            .then_some(Self(version))
    }
}

/// The function the program starts at.
const ENTRY_POINT: &str = "approval";

/// The opcodes every program uses regardless of its instructions, with the
/// version each first appeared in, in emission order.
const FIXED: [(&str, u8); 3] = [("callsub", 4), ("return", 2), ("proto", 8)];

/// Emits the TEAL text of `program`: a call to its entry point, then the
/// entry point's subroutine.
///
/// Any other function is dead code — nothing can call it yet — and is not
/// emitted.
pub fn emit(
    program: &ir::Program,
    version: TealVersion,
    diags: &mut Diagnostics,
) -> Option<String> {
    let entry = entry_point(program, diags)?;
    check_versions(entry, version, diags)?;

    let mut teal = format!(
        "#pragma version {}\ncallsub {ENTRY_POINT}\nreturn\n",
        version.0
    );
    teal.push_str(&function(entry));
    Some(teal)
}

/// The subroutine for `func`: its label, its frame, and its body.
fn function(func: &Function) -> String {
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
///
/// Placeholders are not version-checked: `proto` precedes them and requires a
/// newer version than any of them.
fn placeholder(ty: Type) -> &'static str {
    match ty {
        Type::Uint64 => "pushint 0",
        Type::Bool => "pushint 0",
    }
}

/// Finds the entry point, reporting it if there is none.
fn entry_point<'a>(program: &'a ir::Program, diags: &mut Diagnostics) -> Option<&'a Function> {
    let entry = program.funcs.iter().find(|func| func.name == ENTRY_POINT);

    if entry.is_none() {
        diags.push(Diagnostic {
            kind: DiagnosticKind::MissingEntryPoint { name: ENTRY_POINT },
            // There is no token to point at.
            span: Span { start: 0, end: 0 },
        });
    }
    entry
}

/// Reports every opcode newer than the target version, returning `None` if
/// there was one.
///
/// The fixed opcodes come first, at the span of the whole function, then the
/// instructions in order.
fn check_versions(func: &Function, version: TealVersion, diags: &mut Diagnostics) -> Option<()> {
    let fixed = FIXED.iter().map(|&(opcode, min)| (opcode, min, func.span));
    let insts = func
        .insts
        .iter()
        .map(|inst| (opcode(inst), min_version(inst), inst.span()));

    let mut ok = true;
    for (opcode, min, span) in fixed.chain(insts) {
        if min > version.0 {
            diags.push(Diagnostic {
                kind: DiagnosticKind::OpcodeUnavailable {
                    opcode,
                    min,
                    target: version.0,
                },
                span,
            });
            ok = false;
        }
    }

    ok.then_some(())
}

/// The TEAL version an instruction's opcode first appeared in.
fn min_version(inst: &Inst) -> u8 {
    match inst {
        Inst::Const { .. } => 3,
        Inst::Binary { .. } | Inst::Unary { .. } => 1,
        Inst::Store { .. } | Inst::Load { .. } => 8,
        Inst::Return { .. } => 4,
    }
}

/// The opcode an instruction emits.
fn opcode(inst: &Inst) -> &'static str {
    match inst {
        Inst::Const { .. } => "pushint",
        Inst::Binary { op, .. } => match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        },
        Inst::Unary { op, .. } => match op {
            UnaryOp::Not => "!",
        },
        Inst::Store { .. } => "frame_bury",
        Inst::Load { .. } => "frame_dig",
        Inst::Return { .. } => "retsub",
    }
}

/// The line an instruction emits, without its terminator.
///
/// `ValueId`s are not consulted: every instruction consumes its operands from
/// the top of the stack, where the instructions that defined them left them.
fn line(inst: &Inst) -> String {
    match inst {
        Inst::Const { value, .. } => format!("{} {value}", opcode(inst)),
        Inst::Store { local, .. } | Inst::Load { local, .. } => {
            format!("{} {}", opcode(inst), local.0)
        }
        Inst::Binary { .. } | Inst::Unary { .. } | Inst::Return { .. } => opcode(inst).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ValueId;
    use crate::lower::lower;
    use crate::testing::{EXAMPLE, lex_parse_check, span_of};
    use crate::typed_ast::LocalId;

    /// The span every hand-built instruction carries.
    const ZERO: Span = Span { start: 0, end: 0 };

    /// Emits `source` for `version`, asserting that it produced no
    /// diagnostics.
    fn emit_ok(source: &str, version: u8) -> String {
        let mut diags = Diagnostics::default();
        let teal = pipeline(source, version, &mut diags);
        assert!(diags.is_empty());
        teal.expect("emission succeeded")
    }

    /// Emits `source` for `version`, asserting that it emitted nothing, and
    /// returning the diagnostics in the order they were reported.
    fn emit_err(source: &str, version: u8) -> Vec<Diagnostic> {
        let mut diags = Diagnostics::default();
        assert_eq!(pipeline(source, version, &mut diags), None);
        diags.iter().cloned().collect()
    }

    fn pipeline(source: &str, version: u8, diags: &mut Diagnostics) -> Option<String> {
        let ir = lower(&lex_parse_check(source), diags).expect("lowering succeeded");
        let version = TealVersion::new(version).expect("a supported version");
        emit(&ir, version, diags)
    }

    /// Emits a hand-built program for `version`: one function named
    /// `approval` returning `ret`, with `locals` as its frame and `insts` as
    /// its body, at the zero span throughout. `emit_ok` and `emit_err` go
    /// through source, which cannot produce a frame yet.
    fn hand_built(
        ret: Type,
        locals: Vec<Type>,
        insts: Vec<Inst>,
        version: u8,
        diags: &mut Diagnostics,
    ) -> Option<String> {
        let program = ir::Program {
            funcs: vec![Function {
                name: ENTRY_POINT.to_string(),
                ret,
                locals,
                insts,
                span: ZERO,
            }],
        };
        let version = TealVersion::new(version).expect("a supported version");
        emit(&program, version, diags)
    }

    /// Emits a hand-built program, asserting that it produced no diagnostics.
    fn hand_built_ok(ret: Type, locals: Vec<Type>, insts: Vec<Inst>, version: u8) -> String {
        let mut diags = Diagnostics::default();
        let teal = hand_built(ret, locals, insts, version, &mut diags);
        assert!(diags.is_empty());
        teal.expect("emission succeeded")
    }

    /// Emits a hand-built program, asserting that it emitted nothing, and
    /// returning the diagnostics in the order they were reported.
    fn hand_built_err(
        ret: Type,
        locals: Vec<Type>,
        insts: Vec<Inst>,
        version: u8,
    ) -> Vec<Diagnostic> {
        let mut diags = Diagnostics::default();
        assert_eq!(hand_built(ret, locals, insts, version, &mut diags), None);
        diags.iter().cloned().collect()
    }

    fn constant(dest: u32, value: u64) -> Inst {
        constant_of(dest, Type::Uint64, value)
    }

    fn constant_of(dest: u32, ty: Type, value: u64) -> Inst {
        Inst::Const {
            dest: ValueId(dest),
            ty,
            value,
            span: ZERO,
        }
    }

    fn binary(dest: u32, op: BinaryOp, lhs: u32, rhs: u32) -> Inst {
        Inst::Binary {
            dest: ValueId(dest),
            op,
            lhs: ValueId(lhs),
            rhs: ValueId(rhs),
            span: ZERO,
        }
    }

    fn unary(dest: u32, op: UnaryOp, operand: u32) -> Inst {
        Inst::Unary {
            dest: ValueId(dest),
            op,
            operand: ValueId(operand),
            span: ZERO,
        }
    }

    fn store(local: u8, value: u32) -> Inst {
        Inst::Store {
            local: LocalId(local),
            value: ValueId(value),
            span: ZERO,
        }
    }

    fn load(dest: u32, local: u8) -> Inst {
        Inst::Load {
            dest: ValueId(dest),
            local: LocalId(local),
            span: ZERO,
        }
    }

    fn ret(value: u32) -> Inst {
        Inst::Return {
            value: ValueId(value),
            span: ZERO,
        }
    }

    /// `var x uint64 = 1; return x`, as the next slice will lower it.
    fn one_slot() -> Vec<Inst> {
        vec![constant(0, 1), store(0, 0), load(1, 0), ret(1)]
    }

    #[test]
    fn a_frame_slot_is_allocated_and_addressed() {
        assert_eq!(
            hand_built_ok(Type::Uint64, vec![Type::Uint64], one_slot(), 10),
            "#pragma version 10\n\
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
            binary(2, BinaryOp::Add, 0, 1),
            store(0, 2),
            load(3, 0),
            constant(4, 3),
            binary(5, BinaryOp::Mul, 3, 4),
            store(1, 5),
            load(6, 1),
            load(7, 0),
            binary(8, BinaryOp::Sub, 6, 7),
            ret(8),
        ];
        assert_eq!(
            hand_built_ok(Type::Uint64, vec![Type::Uint64; 2], insts, 10),
            "#pragma version 10\n\
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
            hand_built_ok(
                Type::Bool,
                vec![],
                vec![constant_of(0, Type::Bool, 1), ret(0)],
                10
            ),
            "#pragma version 10\n\
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
            emit_ok("func approval() bool { return true }", 10),
            "#pragma version 10\n\
             callsub approval\n\
             return\n\
             approval:\n\
             proto 0 1\n\
             pushint 1\n\
             retsub\n"
        );
        assert_eq!(
            emit_ok("func approval() bool { return false }", 10),
            "#pragma version 10\n\
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
            constant_of(0, Type::Bool, 1),
            store(0, 0),
            load(1, 0),
            ret(1),
        ];
        assert_eq!(
            hand_built_ok(Type::Bool, vec![Type::Bool], insts, 10),
            "#pragma version 10\n\
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
    fn the_frame_instructions_need_version_8() {
        let unavailable = |opcode| Diagnostic {
            kind: DiagnosticKind::OpcodeUnavailable {
                opcode,
                min: 8,
                target: 7,
            },
            span: ZERO,
        };
        assert_eq!(
            hand_built_err(Type::Uint64, vec![Type::Uint64], one_slot(), 7),
            vec![
                unavailable("proto"),
                unavailable("frame_bury"),
                unavailable("frame_dig"),
            ]
        );
    }

    /// The five lines every hand-built program starts with, at version 10.
    const PROLOGUE: &str = "#pragma version 10\n\
                            callsub approval\n\
                            return\n\
                            approval:\n\
                            proto 0 1\n";

    #[test]
    fn a_comparison_emits_its_mnemonic() {
        let cases = [
            (BinaryOp::Eq, "=="),
            (BinaryOp::Ne, "!="),
            (BinaryOp::Lt, "<"),
            (BinaryOp::Le, "<="),
            (BinaryOp::Gt, ">"),
            (BinaryOp::Ge, ">="),
        ];
        for (op, mnemonic) in cases {
            let insts = vec![constant(0, 1), constant(1, 2), binary(2, op, 0, 1), ret(2)];
            assert_eq!(
                hand_built_ok(Type::Bool, vec![], insts, 10),
                format!("{PROLOGUE}pushint 1\npushint 2\n{mnemonic}\nretsub\n"),
                "{op:?}"
            );
        }
    }

    #[test]
    fn logic_emits_its_mnemonic() {
        for (op, mnemonic) in [(BinaryOp::And, "&&"), (BinaryOp::Or, "||")] {
            let insts = vec![
                constant_of(0, Type::Bool, 1),
                constant_of(1, Type::Bool, 0),
                binary(2, op, 0, 1),
                ret(2),
            ];
            assert_eq!(
                hand_built_ok(Type::Bool, vec![], insts, 10),
                format!("{PROLOGUE}pushint 1\npushint 0\n{mnemonic}\nretsub\n"),
                "{op:?}"
            );
        }
    }

    #[test]
    fn negation_emits_its_mnemonic() {
        let insts = vec![
            constant_of(0, Type::Bool, 1),
            unary(1, UnaryOp::Not, 0),
            ret(1),
        ];
        assert_eq!(
            hand_built_ok(Type::Bool, vec![], insts, 10),
            format!("{PROLOGUE}pushint 1\n!\nretsub\n")
        );
    }

    #[test]
    fn a_comparison_joined_by_logic() {
        // `!(1 < 2 && true)`.
        let insts = vec![
            constant(0, 1),
            constant(1, 2),
            binary(2, BinaryOp::Lt, 0, 1),
            constant_of(3, Type::Bool, 1),
            binary(4, BinaryOp::And, 2, 3),
            unary(5, UnaryOp::Not, 4),
            ret(5),
        ];
        assert_eq!(
            hand_built_ok(Type::Bool, vec![], insts, 10),
            "#pragma version 10\n\
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
    fn a_comparison_is_as_old_as_the_avm() {
        // Only the opcodes around it are newer than version 2.
        let unavailable = |opcode, min| Diagnostic {
            kind: DiagnosticKind::OpcodeUnavailable {
                opcode,
                min,
                target: 2,
            },
            span: ZERO,
        };
        let insts = vec![
            constant(0, 1),
            constant(1, 2),
            binary(2, BinaryOp::Lt, 0, 1),
            ret(2),
        ];
        assert_eq!(
            hand_built_err(Type::Bool, vec![], insts, 2),
            vec![
                unavailable("callsub", 4),
                unavailable("proto", 8),
                unavailable("pushint", 3),
                unavailable("pushint", 3),
                unavailable("retsub", 4),
            ]
        );
    }

    #[test]
    fn example_program() {
        assert_eq!(
            emit_ok(EXAMPLE, 10),
            "#pragma version 10\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 1\nretsub\n"
        );
    }

    #[test]
    fn zero() {
        assert_eq!(
            emit_ok("func approval() uint64 { return 0 }", 10),
            "#pragma version 10\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 0\nretsub\n"
        );
    }

    #[test]
    fn largest_uint64() {
        assert_eq!(
            emit_ok("func approval() uint64 { return 18446744073709551615 }", 10),
            "#pragma version 10\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 18446744073709551615\nretsub\n"
        );
    }

    #[test]
    fn arithmetic() {
        assert_eq!(
            emit_ok("func approval() uint64 { return (1 + 2) * 3 - 4 / 5 }", 10),
            "#pragma version 10\n\
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
            emit_ok("func approval() uint64 { return 7 % 4 }", 10),
            "#pragma version 10\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 7\npushint 4\n%\nretsub\n"
        );
    }

    #[test]
    fn only_the_entry_point_is_emitted() {
        assert_eq!(
            emit_ok(
                "func f() uint64 { return 2 } func approval() uint64 { return 1 }",
                10
            ),
            "#pragma version 10\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 1\nretsub\n"
        );
    }

    #[test]
    fn missing_entry_point() {
        assert_eq!(
            emit_err("func f() uint64 { return 1 }", 10),
            vec![Diagnostic {
                kind: DiagnosticKind::MissingEntryPoint { name: "approval" },
                span: Span { start: 0, end: 0 },
            }]
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(
            emit_err("", 10),
            vec![Diagnostic {
                kind: DiagnosticKind::MissingEntryPoint { name: "approval" },
                span: Span { start: 0, end: 0 },
            }]
        );
    }

    #[test]
    fn the_bounds_are_supported_versions() {
        assert_eq!(
            TealVersion::new(TealVersion::MIN),
            Some(TealVersion(TealVersion::MIN))
        );
        assert_eq!(
            TealVersion::new(TealVersion::MAX),
            Some(TealVersion(TealVersion::MAX))
        );
    }

    #[test]
    fn a_version_outside_the_bounds_is_rejected() {
        assert_eq!(TealVersion::new(TealVersion::MIN - 1), None);
        assert_eq!(TealVersion::new(TealVersion::MAX + 1), None);
        assert_eq!(TealVersion::new(u8::MAX), None);
    }

    #[test]
    fn the_target_version_is_the_one_requested() {
        assert_eq!(
            emit_ok(EXAMPLE, 8),
            "#pragma version 8\ncallsub approval\nreturn\napproval:\nproto 0 1\npushint 1\nretsub\n"
        );
    }

    #[test]
    fn opcode_newer_than_the_target() {
        let unavailable = |opcode, min, span| Diagnostic {
            kind: DiagnosticKind::OpcodeUnavailable {
                opcode,
                min,
                target: 2,
            },
            span,
        };
        let whole = span_of(EXAMPLE, EXAMPLE, 0);
        assert_eq!(
            emit_err(EXAMPLE, 2),
            vec![
                unavailable("callsub", 4, whole),
                unavailable("proto", 8, whole),
                unavailable("pushint", 3, span_of(EXAMPLE, "1", 0)),
                unavailable("retsub", 4, span_of(EXAMPLE, "return 1", 0)),
            ]
        );
    }

    #[test]
    fn the_frame_needs_version_8() {
        assert_eq!(
            emit_err(EXAMPLE, 7),
            vec![Diagnostic {
                kind: DiagnosticKind::OpcodeUnavailable {
                    opcode: "proto",
                    min: 8,
                    target: 7,
                },
                span: span_of(EXAMPLE, EXAMPLE, 0),
            }]
        );
    }
}
