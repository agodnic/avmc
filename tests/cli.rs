//! End-to-end tests of the `avmc` binary.

// The test helpers here are neither `#[test]` functions nor a `cfg(test)`
// module, so `clippy.toml` does not exempt them: a panic in test setup is a
// failing test.
#![expect(clippy::expect_used, reason = "test setup")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The example program of the v0 milestone.
const EXAMPLE: &str = "func approval() uint64 { return 1 }";

/// The TEAL the example program compiles to for version 10.
const EXAMPLE_TEAL: &str = "#pragma version 10\npushint 1\nreturn\n";

/// A source file that lives for as long as one test, named after it so that
/// tests running in parallel never share a path.
struct SourceFile {
    path: PathBuf,
}

impl SourceFile {
    fn new(name: &str, source: &str) -> Self {
        let path = std::env::temp_dir().join(format!("avmc-cli-{}-{name}.txt", std::process::id()));
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
    Command::new(env!("CARGO_BIN_EXE_avmc"))
        .args(args)
        .output()
        .expect("running avmc")
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
fn compiles_a_file_to_teal() {
    let file = SourceFile::new("compiles_a_file_to_teal", EXAMPLE);
    let output = run(&[file.path(), "--teal-version", "10"]);

    assert_eq!(stdout(&output), EXAMPLE_TEAL);
    assert_eq!(stderr(&output), "");
    assert_eq!(code(&output), 0);
}

#[test]
fn accepts_the_flag_before_the_path() {
    let file = SourceFile::new("accepts_the_flag_before_the_path", EXAMPLE);
    let output = run(&["--teal-version", "10", file.path()]);

    assert_eq!(stdout(&output), EXAMPLE_TEAL);
    assert_eq!(stderr(&output), "");
    assert_eq!(code(&output), 0);
}

#[test]
fn reports_a_lexing_error() {
    let file = SourceFile::new(
        "reports_a_lexing_error",
        "func approval() uint64 {\n  return @\n}\n",
    );
    let output = run(&[file.path(), "--teal-version", "10"]);

    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        format!("{}:2:10: error[E0001]: unexpected character\n", file.path())
    );
    assert_eq!(code(&output), 1);
}

#[test]
fn reports_a_missing_entry_point() {
    let file = SourceFile::new(
        "reports_a_missing_entry_point",
        "func f() uint64 { return 1 }",
    );
    let output = run(&[file.path(), "--teal-version", "10"]);

    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        format!(
            "{}:1:1: error[E0008]: missing entry point `approval`\n",
            file.path()
        )
    );
    assert_eq!(code(&output), 1);
}

#[test]
fn rejects_a_missing_version() {
    let file = SourceFile::new("rejects_a_missing_version", EXAMPLE);
    let output = run(&[file.path()]);

    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "usage: avmc <file> --teal-version <N>\n");
    assert_eq!(code(&output), 2);
}

#[test]
fn rejects_a_version_that_is_not_a_number() {
    let file = SourceFile::new("rejects_a_version_that_is_not_a_number", EXAMPLE);
    let output = run(&[file.path(), "--teal-version", "abc"]);

    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "usage: avmc <file> --teal-version <N>\n");
    assert_eq!(code(&output), 2);
}

#[test]
fn reports_a_file_it_cannot_read() {
    let missing = std::env::temp_dir().join(format!(
        "avmc-cli-{}-reports_a_file_it_cannot_read.txt",
        std::process::id()
    ));
    assert!(!Path::new(&missing).exists());
    let path = missing.to_str().expect("a UTF-8 temporary path");
    let output = run(&[path, "--teal-version", "10"]);

    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).starts_with("avmc: cannot read "),
        "unexpected stderr: {}",
        stderr(&output)
    );
    assert_eq!(code(&output), 2);
}
