//! The `avmc` command line interface: a source file in, TEAL text out.

use std::io::{self, Write};
use std::process::ExitCode;

use avmc::diagnostics::Diagnostics;
use avmc::driver::{compile, render};

/// The usage line the binary reports for any bad argument list.
const USAGE: &str = "usage: avmc <file>";

/// Exit code for a usage, read, or write failure.
const FAILURE: u8 = 2;
/// Exit code for a source file that does not compile.
const COMPILE_ERRORS: u8 = 1;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = parse_args(&args) else {
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
    let teal = compile(&source, &mut diags);
    // Warnings are reported for a source file that compiles too.
    for diagnostic in diags.iter() {
        report(&render(diagnostic, &path, &source));
    }
    let Some(teal) = teal else {
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

/// The file path, or `None` for any argument list that is not exactly one.
///
/// The path is taken as given: it is never canonicalised and its extension is
/// never inspected.
fn parse_args(args: &[String]) -> Option<String> {
    match args {
        [path] => Some(path.clone()),
        _ => None,
    }
}

/// Writes one line to stderr, ignoring a stderr that cannot be written to.
fn report(line: &str) {
    let _ = writeln!(io::stderr(), "{line}");
}
