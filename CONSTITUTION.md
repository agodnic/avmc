# avmc — Constitution

This document is the governing charter of `avmc`, a compiler targeting the
Algorand Virtual Machine (AVM).

**The decisions in §2 and the rules in §4 are frozen.** They change only
through the process in §5, and that process is the reason this document is
separate from [ARCHITECTURE.md](ARCHITECTURE.md): everything here is meant to
sit still, and everything there is meant to move. Contributors — human or agent
— are expected to read this before writing code, and to treat a conflict
between this document and the code as a bug in the code.

For how the compiler is actually built — the pipeline, the stage contracts, the
correctness strategy, the repository layout — see
**[ARCHITECTURE.md](ARCHITECTURE.md)**.

---

## 1. The machine we are compiling for

Every decision below derives from the target's constraints, so they come first.

| Property | Value |
|---|---|
| Machine shape | Stack machine, maximum stack depth **1000** |
| Value types | `uint64` and `[]byte` (max **4096** bytes). Nothing else. |
| Scratch space | **256** slots, one value per slot |
| Heap | **None.** No allocator, no pointers, no addressable linear memory |
| Persistent storage | Global state, local state, boxes (**32 KB** each) — accessed via opcodes, not memory |
| Compute budget | **700** ops per application call, pooled across a transaction group; **20,000** for signature mode |
| Program size | **2 KB** for approval + clear programs combined, extendable in 2 KB steps to **8 KB** |
| Integer arithmetic | `+`, `-`, `*` **fail the transaction** on overflow/underflow. No wrapping. No floats. |
| Error model | Any failure aborts the entire transaction. No exceptions, no recovery, no unwinding. |

Sources: [AVM specification](https://developer.algorand.org/docs/get-details/dapps/avm/teal/specification/),
[AVM concepts](https://dev.algorand.co/concepts/smart-contracts/avm/).

Three consequences drive the whole design:

1. **No heap means no general-purpose source language.** Closures, dynamic data
   structures, recursion of unbounded depth, and garbage collection are not
   merely slow here — they have nowhere to live.
2. **The compute budget is a correctness property, not a performance concern.**
   A program that exceeds 700 ops does not run slowly; it fails. Cost is
   therefore something the compiler must *analyse and reject*, not something a
   profiler discovers later.
3. **Failure is total.** There is no partial execution to recover from, which
   makes the semantics simpler — and makes a miscompilation maximally
   expensive, since it is discovered on-chain with real value at stake.

---

## 2. The three frozen decisions

### 2.1 Target: TEAL, emitted directly

**We emit TEAL assembly text**, version-pinned with an explicit `#pragma
version N`, and hand it to the existing Algorand assembler (`goal clerk
compile` / algod's compile endpoint) to produce bytecode.

**We do not target LLVM IR or WebAssembly.** The usual argument for them — a
free backend and free differential testing — does not hold for the AVM:

- **LLVM IR** is organised around `load`/`store` against a flat address space
  with pointers. The AVM has no address space. Running LLVM output would
  require emulating linear memory over 256 scratch slots or byte-slice
  concatenation, spending multiple opcodes per synthetic memory access against
  a 700-opcode budget. There is no AVM backend in LLVM, and writing one is a
  larger project than this compiler.
- **WebAssembly** has the same linear-memory mismatch plus an i32-centric type
  system that does not correspond to `uint64`/`[]byte`. A Wasm→TEAL translator
  is a research project in its own right.

Neither provides a free backend, because neither has a backend that reaches the
AVM. What they would provide is a permanent impedance mismatch.

TEAL text is a good target on its merits: it is human-readable, so every
codegen change is visible in a diff; it is stable and versioned; and the
reference assembler and interpreter already exist and are the consensus
implementation.

**We define our own IR.** Not LLVM's — a typed, single-assignment IR designed
around `uint64`/`[]byte` and the absence of a heap. It begins as a flat
instruction list and grows a control-flow graph when the language grows control
flow. Analysis runs on it. See [ARCHITECTURE.md](ARCHITECTURE.md) §2.2.

### 2.2 Source language: our own, designed and frozen

**We design the source language and freeze it early.** Working name: **Ava**
(file extension `.ava`; the name is provisional and tracked in the
issue tracker).

We rejected adopting an existing specified language (a C subset, Lua, Scheme, a
small ML). Their value is their conformance suites — but those suites
predominantly test heap features: `malloc` and pointer arithmetic, tables and
closures, `call/cc` and heap-allocated pairs, algebraic datatypes. On a machine
with no heap we would implement an allocator to fail most of the suite anyway,
spending the project on emulation rather than compilation.

This is not a shortcut. Every serious AVM language — Algorand Python (Puya),
TEALScript, PyTeal — independently converged on a restricted, first-order,
heap-free subset. The constraint is real.

**Ava v0 is:**

- statically typed, with no inference beyond local `let` bindings;
- first-order — no closures, no function values;
- non-recursive — whether depth-bounded recursion is admissible depends on
  the AVM's call-stack limit, which is tracked as an open question;
- built on `uint64`, `bytes`, `bool`, plus structs and fixed-size arrays;
- explicit about on-chain state: storage is *declared* as a resource, never
  ambient;
- explicit about failure: the operations that can abort the transaction are
  visible in the source.

We deliberately do **not** adopt Python-like syntax. Algorand Python
demonstrates the cost: a surface that looks like a familiar language but
supports a small fraction of it generates permanent confusion. Ava should look
like what it is — a small language with real constraints.

**The freeze has teeth.** See rule **R4** in §4: a language change is not a code
change. It is a specification edit plus a conformance test, landed *before* any
implementation. This is the rule that prevents the language from being quietly
redesigned by whoever is implementing the type checker that week.

### 2.3 Implementation: Rust, with a differential-testing oracle

**The compiler is written in Rust.** Sum types with exhaustive matching for the
AST and IR, no GC in analysis passes, `insta` for snapshot-testing stage
boundaries, `proptest` for generative testing of the frontend and the full
pipeline.

**The differential-testing oracle is the real AVM**, reached in two phases:

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
throughput becomes the binding constraint. The oracle interface in
`avmc-oracle` is therefore defined as a trait from day one, with the HTTP client
as its first implementation, so adding the sidecar is a new implementation
rather than a refactor.

**The oracle is never a reimplementation of the AVM.** An AVM we wrote
ourselves would share our own misconceptions and would be worthless as an
independent check. The oracle is always the consensus implementation, wrapped.

We rejected writing the whole compiler in Go: it would trade sum types across
the AST, IR, every pass, and diagnostics — the ~90% of the codebase that gets
refactored continuously — to avoid a process boundary in the test harness.

---

## 3. Non-goals

Stated explicitly so that "should we support X?" has a written answer.

- **No heap, no allocator, no garbage collection.** Not in v0, not later.
- **No closures or first-class functions.**
- **No floating point.** The AVM has none; we will not emulate it.
- **No exceptions, `try`, or recovery.** Failure aborts the transaction; that is
  the only failure mode the language exposes.
- **No dynamic code loading or `eval`.**
- **We are not a TEAL assembler.** We emit TEAL text and delegate assembly to
  the reference implementation.
- **We are not a general-purpose language.** Ava exists to compile to the AVM.
  A feature that cannot be lowered to efficient TEAL does not belong in it.
- **No hand-written TEAL templates** outside the emitter (rule **R7**).

---

## 4. Binding rules

These are the invariants agents and contributors must not violate. Each has a
short identifier so review comments can cite it.

- **R1 — Stages are pure functions.** Every pipeline stage has the shape
  `fn(Input, &mut Diagnostics) -> Option<Output>`. No file I/O, no network, no
  environment access, no global mutable state inside a stage. All I/O lives in
  the driver module and `avmc-cli`.
- **R2 — Spans are threaded end to end.** Every token, AST node, IR
  instruction, and emitted opcode carries a source span. A diagnostic without a
  span is a bug.
- **R3 — Errors never silently degrade.** A stage that reports an error
  produces no output that a later stage will consume. We never emit "best
  effort" TEAL. Recovery for the purpose of reporting *more* diagnostics is
  encouraged; recovery that produces artifacts is forbidden.
- **R4 — The language freeze.** Changing the syntax or static semantics of Ava
  requires, in this order: (1) an edit to `spec/language.md`, (2) a conformance
  test in `tests/conformance/` that fails, (3) the implementation. A pull
  request that changes language behaviour without touching the spec is rejected
  on sight.
- **R5 — Determinism.** For a fixed compiler version, input, and target TEAL
  version, output is byte-identical. No hash-map iteration order, no
  timestamps, no absolute paths, no parallelism-dependent ordering in emitted
  code.
- **R6 — The TEAL version is an explicit input.** It is a required compilation
  parameter, never inferred from the source and never silently upgraded. Using
  an opcode unavailable in the target version is a compile error, not a runtime
  surprise.
- **R7 — TEAL text is written in exactly one place.** Only the emitter
  produces TEAL. No other module — and no ABI/ARC-4 support layer — emits
  assembly text. Higher-level constructs are lowered into IR and go through the
  same emitter as everything else.
- **R8 — The IR verifier runs at every IR boundary** in debug and test builds:
  after lowering, and after each pass once passes exist. What it checks grows
  with the IR — type correctness and single assignment from the start,
  dominance and CFG well-formedness once there is control flow. Invariants are
  checked, not assumed.
- **R9 — No panics on user input.** Malformed source produces diagnostics. In
  crates that process untrusted input, `unwrap`/`expect`/`panic!` are permitted
  only for conditions the IR verifier has already established.
- **R10 — Every diagnostic has a stable code** (`E0001`, `W0001`, …) and an
  entry in the diagnostics index. Codes are never reused for a different
  meaning.
- **R11 — Cost bounds are checked, not estimated.** Where the cost analyser
  reports a bound, differential tests assert that the AVM's measured cost does
  not exceed it. An analyser that can under-report is a broken analyser.


---

## 5. Amendment

The friction here is deliberate and narrowly scoped. Implementation details
should move freely; the shape of the compiler and the definition of the
language should move only on purpose.

**This document.** A pull request that modifies `CONSTITUTION.md` **modifies no
other file**. It states what is changing and why, and is reviewed on its own
merits. This is a mechanical rule precisely so that it can be checked
mechanically — a diff either touches this file alone or it does not touch it at
all. Bundling a constitutional change into a feature branch is the failure mode
the rule exists to prevent.

**The language.** Governed by **R4**, and stricter still: an edit to
`spec/language.md`, then a failing conformance test, then the implementation —
in that order, and the spec edit is its own pull request.

**Everything else** — the pipeline, stage internals, pass lists, repository
layout — lives in [ARCHITECTURE.md](ARCHITECTURE.md) and is amended in the same
pull request as the code that changes it. No ceremony.

**Work in progress** — milestones, open design questions, decisions not yet
made — lives in the issue tracker, not in either document. A question that
needs a decision needs an owner and a closing condition, and a bullet in a
markdown file gives it neither.
