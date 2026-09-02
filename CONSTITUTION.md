# avmc — Constitution

This document is the governing charter of `avmc`, a compiler targeting the
Algorand Virtual Machine (AVM).

Contributors — human or agent — are expected to read this before writing code,
and to treat a conflict between this document and the code as a bug in the
code.

For how the compiler is actually built — the pipeline, the stage contracts, the
correctness strategy, the invariants — see
**[ARCHITECTURE.md](ARCHITECTURE.md)**.

---

## 1. The two frozen decisions

### 1.1 Source language: our own, designed and frozen

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

### 1.2 Implementation: Rust, cross-checked against the real AVM

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
