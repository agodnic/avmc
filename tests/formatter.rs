//! End-to-end tests of the `formatter` binary.

// The test helpers here are neither `#[test]` functions nor a `cfg(test)`
// module, so `clippy.toml` does not exempt them: a panic in test setup is a
// failing test.
#![expect(clippy::expect_used, reason = "test setup")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A program written with every whitespace decision made wrongly.
const MESSY: &str = "func approval ( ) uint64 {\n\
                     \n\
                     \x20 // one more than two\n\
                     \x20 var x uint64 = ( 1+2 )    // trailing\n\
                     \x20 return x*3\n\
                     \n\
                     \n\
                     }\n\
                     func f() bool { return !(true) }\n";

/// The same program, formatted.
const CANONICAL: &str = "func approval() uint64 {\n\
                         \t// one more than two\n\
                         \tvar x uint64 = (1 + 2) // trailing\n\
                         \treturn x * 3\n\
                         }\n\
                         \n\
                         func f() bool {\n\
                         \treturn !(true)\n\
                         }\n";

/// The usage line the binary reports for any bad argument list.
const USAGE: &str = "usage: formatter <file>\n";

/// A source file that lives for as long as one test, named after it so that
/// tests running in parallel never share a path.
struct SourceFile {
    path: PathBuf,
}

impl SourceFile {
    fn new(name: &str, source: &str) -> Self {
        let path = std::env::temp_dir().join(format!("avmc-fmt-{}-{name}.txt", std::process::id()));
        std::fs::write(&path, source).expect("writing the source file");
        Self { path }
    }

    fn path(&self) -> &str {
        self.path.to_str().expect("a UTF-8 temporary path")
    }
}

impl Drop for SourceFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Runs the binary with `args`.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_formatter"))
        .args(args)
        .output()
        .expect("running the formatter")
}

/// The exit code of `output`, which is never a signal here.
fn code(output: &Output) -> i32 {
    output.status.code().expect("an exit code")
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("UTF-8 stdout")
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("UTF-8 stderr")
}

#[test]
fn formats_a_file() {
    let file = SourceFile::new("formats_a_file", MESSY);
    let output = run(&[file.path()]);

    assert_eq!(stdout(&output), CANONICAL);
    assert_eq!(stderr(&output), "");
    assert_eq!(code(&output), 0);
}

#[test]
fn a_canonical_file_is_unchanged() {
    let file = SourceFile::new("a_canonical_file_is_unchanged", CANONICAL);
    let output = run(&[file.path()]);

    assert_eq!(stdout(&output), CANONICAL);
    assert_eq!(stderr(&output), "");
    assert_eq!(code(&output), 0);
}

#[test]
fn reports_a_parse_error() {
    let file = SourceFile::new("reports_a_parse_error", "func f() uint64 { return }");
    let output = run(&[file.path()]);

    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        format!(
            "{}:1:26: error[E0002]: expected a literal, an identifier, `!`, or `(`, found `}}`\n",
            file.path()
        )
    );
    assert_eq!(code(&output), 1);
}

#[test]
fn reports_a_lexing_error() {
    let file = SourceFile::new(
        "reports_a_lexing_error",
        "func approval() uint64 {\n  return @\n}\n",
    );
    let output = run(&[file.path()]);

    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        format!("{}:2:10: error[E0001]: unexpected character\n", file.path())
    );
    assert_eq!(code(&output), 1);
}

#[test]
fn rejects_a_missing_argument() {
    let output = run(&[]);

    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), USAGE);
    assert_eq!(code(&output), 2);
}

#[test]
fn rejects_two_arguments() {
    let file = SourceFile::new("rejects_two_arguments", CANONICAL);
    let output = run(&[file.path(), file.path()]);

    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), USAGE);
    assert_eq!(code(&output), 2);
}

#[test]
fn reports_an_unreadable_file() {
    let missing = std::env::temp_dir().join(format!(
        "avmc-fmt-{}-reports_an_unreadable_file.txt",
        std::process::id()
    ));
    assert!(!Path::new(&missing).exists());
    let path = missing.to_str().expect("a UTF-8 temporary path");
    let output = run(&[path]);

    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).starts_with("formatter: cannot read "),
        "unexpected stderr: {}",
        stderr(&output)
    );
    assert_eq!(code(&output), 2);
}
