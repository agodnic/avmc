//! Emission: IR to TEAL text, in a single linear pass.

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

/// Emits the TEAL text of `program`'s entry point.
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

    let mut teal = format!("#pragma version {}\n", version.0);
    for inst in &entry.insts {
        teal.push_str(&line(inst));
        teal.push('\n');
    }
    Some(teal)
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

/// Reports every instruction whose opcode is newer than the target version,
/// returning `None` if there was one.
fn check_versions(func: &Function, version: TealVersion, diags: &mut Diagnostics) -> Option<()> {
    let mut ok = true;

    for inst in &func.insts {
        let min = min_version(inst);
        if min > version.0 {
            diags.push(Diagnostic {
                kind: DiagnosticKind::OpcodeUnavailable {
                    opcode: opcode(inst),
                    min,
                    target: version.0,
                },
                span: span(inst),
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
        Inst::Return { .. } => 2,
    }
}

/// The opcode an instruction emits.
fn opcode(inst: &Inst) -> &'static str {
    match inst {
        Inst::Const { .. } => "pushint",
        Inst::Return { .. } => "return",
    }
}

/// The line an instruction emits, without its terminator.
///
/// `ValueId`s are not consulted: every value has exactly one use, and uses
/// follow definitions in order, so each operand is already on top of the
/// stack for its consumer.
fn line(inst: &Inst) -> String {
    match inst {
        Inst::Const { value, .. } => format!("{} {value}", opcode(inst)),
        Inst::Return { .. } => opcode(inst).to_string(),
    }
}

/// The source an instruction came from.
fn span(inst: &Inst) -> Span {
    match inst {
        Inst::Const { span, .. } | Inst::Return { span, .. } => *span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::lower::lower;
    use crate::parser::parse;
    use crate::typeck::check;

    /// The example program of the v0 milestone.
    const EXAMPLE: &str = "func approval() uint64 { return 1 }";

    /// Compiles `source` for `version`, asserting that it produced no
    /// diagnostics.
    fn emit_ok(source: &str, version: u8) -> String {
        let mut diags = Diagnostics::default();
        let teal = pipeline(source, version, &mut diags);
        assert!(diags.is_empty());
        teal.expect("emission succeeded")
    }

    /// Compiles `source` for `version`, asserting that it emitted nothing, and
    /// returning the diagnostics in the order they were reported.
    fn emit_err(source: &str, version: u8) -> Vec<Diagnostic> {
        let mut diags = Diagnostics::default();
        assert_eq!(pipeline(source, version, &mut diags), None);
        diags.iter().cloned().collect()
    }

    fn pipeline(source: &str, version: u8, diags: &mut Diagnostics) -> Option<String> {
        let tokens = lex(source, diags).expect("lexing succeeded");
        let parsed = parse(source, &tokens, diags).expect("parsing succeeded");
        let checked = check(&parsed, diags).expect("checking succeeded");
        let ir = lower(&checked, diags).expect("lowering succeeded");
        let version = TealVersion::new(version).expect("a supported version");
        emit(&ir, version, diags)
    }

    /// The span of the `nth` occurrence of `text` in `source`, counting from 0.
    fn span_of(source: &str, text: &str, nth: usize) -> Span {
        let start = source
            .match_indices(text)
            .nth(nth)
            .expect("text in source")
            .0;
        Span {
            start,
            end: start + text.len(),
        }
    }

    #[test]
    fn example_program() {
        assert_eq!(
            emit_ok(EXAMPLE, 10),
            "#pragma version 10\npushint 1\nreturn\n"
        );
    }

    #[test]
    fn zero() {
        assert_eq!(
            emit_ok("func approval() uint64 { return 0 }", 10),
            "#pragma version 10\npushint 0\nreturn\n"
        );
    }

    #[test]
    fn largest_uint64() {
        assert_eq!(
            emit_ok("func approval() uint64 { return 18446744073709551615 }", 10),
            "#pragma version 10\npushint 18446744073709551615\nreturn\n"
        );
    }

    #[test]
    fn only_the_entry_point_is_emitted() {
        assert_eq!(
            emit_ok(
                "func f() uint64 { return 2 } func approval() uint64 { return 1 }",
                10
            ),
            "#pragma version 10\npushint 1\nreturn\n"
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
            emit_ok(EXAMPLE, 3),
            "#pragma version 3\npushint 1\nreturn\n"
        );
    }

    #[test]
    fn opcode_newer_than_the_target() {
        assert_eq!(
            emit_err(EXAMPLE, 2),
            vec![Diagnostic {
                kind: DiagnosticKind::OpcodeUnavailable {
                    opcode: "pushint",
                    min: 3,
                    target: 2,
                },
                span: span_of(EXAMPLE, "1", 0),
            }]
        );
    }
}
