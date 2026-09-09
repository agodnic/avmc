//! Emission: IR to TEAL text, in a single linear pass.

use crate::ast::BinaryOp;
use crate::diagnostics::{Diagnostic, DiagnosticKind, Diagnostics, Span};
use crate::ir::{self, Function, Inst};

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
    for inst in &func.insts {
        teal.push_str(&line(inst));
        teal.push('\n');
    }
    teal
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
        Inst::Binary { .. } => 1,
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
        },
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
        Inst::Binary { .. } | Inst::Return { .. } => opcode(inst).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::lower;
    use crate::testing::{EXAMPLE, lex_parse_check, span_of};

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
