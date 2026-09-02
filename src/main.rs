//! The `avmc` command line interface: a source file in, TEAL text out.

use std::io::{self, Write};
use std::process::ExitCode;

use avmc::diagnostics::Diagnostics;
use avmc::driver::{compile, render};
use avmc::emit::TealVersion;

/// Exit code for a usage, read, or write failure.
const FAILURE: u8 = 2;
/// Exit code for a source file that does not compile.
const COMPILE_ERRORS: u8 = 1;

const USAGE: &str = "usage: avmc <file> --teal-version <N>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((path, version)) = parse_args(&args) else {
        report(USAGE);
        return ExitCode::from(FAILURE);
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            report(&format!("avmc: cannot read {path}: {error}"));
            return ExitCode::from(FAILURE);
        }
    };

    let mut diags = Diagnostics::default();
    let Some(teal) = compile(&source, version, &mut diags) else {
        for diagnostic in diags.iter() {
            report(&render(diagnostic, &path, &source));
        }
        return ExitCode::from(COMPILE_ERRORS);
    };

    match io::stdout().write_all(teal.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(&format!("avmc: cannot write to stdout: {error}"));
            ExitCode::from(FAILURE)
        }
    }
}

/// The file path and target version, or `None` for any other argument list.
///
/// The path is taken as given: it is never canonicalised and its extension is
/// never inspected.
fn parse_args(args: &[String]) -> Option<(String, TealVersion)> {
    let mut path: Option<&String> = None;
    let mut version: Option<u8> = None;

    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--teal-version" {
            if version.is_some() {
                return None;
            }
            version = Some(args.next()?.parse().ok()?);
        } else {
            if path.is_some() {
                return None;
            }
            path = Some(arg);
        }
    }

    Some((path?.clone(), TealVersion(version?)))
}

/// Writes one line to stderr, ignoring a stderr that cannot be written to.
fn report(line: &str) {
    let _ = writeln!(io::stderr(), "{line}");
}
