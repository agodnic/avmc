# avmc — Constitution

This document is the governing charter of `avmc`, a compiler targeting the
Algorand Virtual Machine (AVM).

Contributors — human or agent — are expected to read this before writing code,
and to treat a conflict between this document and the code as a bug in the
code.

For how the compiler is actually built — the pipeline, the stage contracts, the
correctness strategy — see **[ARCHITECTURE.md](ARCHITECTURE.md)**.

---

## 1. The three frozen decisions

### 1.1 Target: TEAL, emitted directly

**We emit TEAL assembly text**, version-pinned with an explicit `#pragma
version N`, and hand it to the existing Algorand assembler (`goal clerk
compile` / algod's compile endpoint) to produce bytecode.

**We define our own IR.** Not LLVM's — a typed, single-assignment IR designed
around `uint64`/`[]byte` and the absence of a heap. It begins as a flat
instruction list and grows a control-flow graph when the language grows control
flow. Analysis runs on it. See [ARCHITECTURE.md](ARCHITECTURE.md) §2.2.

### 1.2 Source language: our own, designed and frozen

**We design the source language and freeze it early.** It has no name yet.

**The v0 language is:**

- statically typed, with no inference beyond local `let` bindings;
- first-order — no closures, no function values;
- non-recursive — whether depth-bounded recursion is admissible depends on
  the AVM's call-stack limit, which is tracked as an open question;
- built on `uint64`, `bytes`, `bool`, plus structs and fixed-size arrays;
- explicit about on-chain state: storage is *declared* as a resource, never
  ambient;
- explicit about failure: the operations that can abort the transaction are
  visible in the source.

### 1.3 Implementation: Rust, cross-checked against the real AVM

**The compiler is written in Rust.** Sum types with exhaustive matching for the
AST and IR, no GC in analysis passes, `insta` for snapshot-testing stage
boundaries, `proptest` for generative testing of the frontend and the full
pipeline.

**Differential testing runs against the real AVM**, reached in two phases:

- **v0 — algod over HTTP.** The test harness compiles to TEAL and executes it
  against AlgoKit LocalNet via algod's compile and `simulate` endpoints, which
  return execution result, per-opcode cost, and stack traces. One toolchain,
  simple CI, adequate for a curated test corpus.
- **Later — a Go sidecar.** A small (~200 line) Go binary linking
  `go-algorand/data/transactions/logic` directly, speaking newline-delimited
  JSON over stdin/stdout: `{teal, mode, args}` in, `{approved, cost, error,
  final_stack}` out. Per-case overhead drops from a network round-trip to tens
  of microseconds, which is what makes large generative campaigns practical.

The sidecar is deferred, not abandoned. It is added when generative-testing
throughput becomes the binding constraint. The interface to the AVM runner is
therefore defined as a trait from day one, with the HTTP client as its first
implementation, so adding the sidecar is a new implementation rather than a
refactor.

**We never reimplement the AVM to test against.** An AVM we wrote ourselves
would share our own misconceptions and would be worthless as an independent
check. What we run against is always the consensus implementation, wrapped.

We rejected writing the whole compiler in Go: it would trade sum types across
the AST, IR, every pass, and diagnostics — the ~90% of the codebase that gets
refactored continuously — to avoid a process boundary in the test harness.

---

## 2. Non-goals

Stated explicitly so that "should we support X?" has a written answer.

- **No heap, no allocator, no garbage collection.** Not in v0, not later.
- **No closures or first-class functions.**
- **No floating point.** The AVM has none; we will not emulate it.
- **No exceptions, `try`, or recovery.** Failure aborts the transaction; that is
  the only failure mode the language exposes.
- **No dynamic code loading or `eval`.**
- **We are not a TEAL assembler.** We emit TEAL text and delegate assembly to
  the reference implementation.
- **We are not a general-purpose language.** The language exists to compile to
  the AVM.
  A feature that cannot be lowered to efficient TEAL does not belong in it.
- **No hand-written TEAL templates** (**R7** — single emitter).

---

## 3. Binding rules

These are the invariants agents and contributors must not violate. Each has a
number for citation and a short tag naming what it requires; references
elsewhere carry both.

- **R1 — pure stages.** Stages are pure functions. Every pipeline stage has the
  shape `fn(Input, &mut Diagnostics) -> Option<Output>`. No file I/O, no
  network, no environment access, no global mutable state inside a stage. All
  I/O lives in the driver module and `avmc-cli`.
- **R2 — spans everywhere.** Spans are threaded end to end. Every token, AST
  node, IR instruction, and emitted opcode carries a source span. A diagnostic
  without a span is a bug.
- **R3 — no degraded output.** Errors never silently degrade. A stage that
  reports an error produces no output that a later stage will consume. We never
  emit "best effort" TEAL. Recovery for the purpose of reporting *more*
  diagnostics is encouraged; recovery that produces artifacts is forbidden.
- **R4 — language freeze.** Changing the syntax or static semantics of the
  language requires, in this order: (1) an edit to `spec/language.md`, (2) a
  conformance test in `tests/conformance/` that fails, (3) the implementation.
  A pull request that changes language behaviour without touching the spec is
  rejected on sight.
- **R5 — determinism.** For a fixed compiler version, input, and target TEAL
  version, output is byte-identical. No hash-map iteration order, no
  timestamps, no absolute paths, no parallelism-dependent ordering in emitted
  code.
- **R6 — explicit TEAL version.** The target version is a required compilation
  parameter, never inferred from the source and never silently upgraded. Using
  an opcode unavailable in the target version is a compile error, not a runtime
  surprise.
- **R7 — single emitter.** TEAL text is written in exactly one place. Only the
  emitter produces TEAL. No other module — and no ABI/ARC-4 support layer —
  emits assembly text. Higher-level constructs are lowered into IR and go
  through the same emitter as everything else.
- **R8 — verify the IR.** The verifier runs at every IR boundary in debug and
  test builds: after lowering, and after each pass once passes exist. What it
  checks grows with the IR — type correctness and single assignment from the
  start, dominance and CFG well-formedness once there is control flow.
  Invariants are checked, not assumed.
- **R9 — no panics.** Malformed source produces diagnostics, never a panic. In
  crates that process untrusted input, `unwrap`/`expect`/`panic!` are permitted
  only for conditions the IR verifier has already established.
- **R10 — stable diagnostic codes.** Every diagnostic has a stable code
  (`E0001`, `W0001`, …) and an entry in the diagnostics index. Codes are never
  reused for a different meaning.
