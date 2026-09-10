//! The `formatter` command line interface: a source file in, formatted source
//! text out.

use std::io::{self, Write};
use std::process::ExitCode;

use avmc::diag;
use avmc::driver;
use avmc::formatter;

/// The usage line the binary reports for any bad argument list.
const USAGE: &str = "usage: formatter <file>";

/// Exit code for a usage, read, or write failure.
const FAILURE: u8 = 2;
/// Exit code for a source file that does not lex or parse.
const SOURCE_ERRORS: u8 = 1;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = parse_args(&args) else {
        report(USAGE);
        return ExitCode::from(FAILURE);
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            report(&format!("formatter: cannot read {path}: {error}"));
            return ExitCode::from(FAILURE);
        }
    };

    let mut diags = diag::Sink::default();
    let formatted = formatter::format(&source, &mut diags);
    // Warnings are reported for a source file that formats too.
    for diagnostic in diags.iter() {
        report(&driver::render(diagnostic, &path, &source));
    }
    let Some(formatted) = formatted else {
        return ExitCode::from(SOURCE_ERRORS);
    };

    match io::stdout().write_all(formatted.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(&format!("formatter: cannot write to stdout: {error}"));
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
