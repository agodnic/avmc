# avmc — Constitution

This document is the governing architecture of `avmc`, a compiler targeting the
Algorand Virtual Machine (AVM).

It exists because the expensive mistakes in a compiler project are not bugs —
they are *architectural drift*: the language quietly redesigning itself during
implementation, stages growing back-channels into each other, and codegen that
is never checked against anything but its own previous output.

**The decisions in §2 are frozen.** Everything else in this document is binding
until amended by the process in §9. Contributors — human or agent — are expected
to read this before writing code, and to treat a conflict between this document
and the code as a bug in the code.

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

**We define our own IR.** Not LLVM's — a typed, SSA-form CFG designed around
`uint64`/`[]byte` and the absence of a heap. Optimisation and analysis run on
it. See §5.

### 2.2 Source language: our own, designed and frozen

**We design the source language and freeze it early.** Working name: **Ava**
(file extension `.ava`; the name is provisional, see §10).

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
- non-recursive (see §10 for the bounded-recursion question);
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
  `avmc-driver` and `avmc-cli`.
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
- **R7 — TEAL text is written in exactly one place.** Only `avmc-backend`'s
  emitter produces TEAL. No other crate — and no ABI/ARC-4 support layer —
  emits assembly text. Higher-level constructs are lowered into IR and go
  through the same backend as everything else.
- **R8 — The IR verifier runs after every pass** in debug and test builds. Type
  correctness, SSA dominance, CFG well-formedness, and no-heap invariants are
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

## 5. Pipeline architecture

```
   source text
        │
   ┌────▼─────┐
   │  lexer   │  text ──────────────► tokens + spans
   ├──────────┤
   │  parser  │  tokens ────────────► AST
   ├──────────┤
   │  resolve │  AST ───────────────► names bound to declarations
   ├──────────┤
   │  typeck  │  AST ───────────────► typed AST
   ├──────────┤
   │  lower   │  typed AST ─────────► IR  (SSA over a CFG)
   ├──────────┤
   │   opt    │  IR ────────────────► IR
   ├──────────┤
   │   cost   │  IR ────────────────► cost bounds  (rejects over-budget code)
   ╞══════════╡
   │  sched   │  IR ────────────────► stack-scheduled IR      ┐
   ├──────────┤                                                │ AVM-specific
   │  regall  │  ──────────────────► scratch slot assignment  │ back half
   ├──────────┤                                                │
   │  pool    │  ──────────────────► constant blocks          │
   ├──────────┤                                                │
   │  emit    │  ──────────────────► TEAL text                ┘
   └────┬─────┘
        │
   TEAL ──► (external assembler) ──► bytecode
```

The first half is a conventional compiler frontend. The second half is where
the actual difficulty of this project lives, and it is specific to the AVM.

### 5.1 Frontend stages

**Lexer.** Hand-written, not generated. Produces a token stream with spans,
recovering from unknown characters rather than aborting.

**Parser.** Hand-written recursive descent with Pratt-style expression parsing.
Chosen over a parser generator for error-message quality and error recovery,
both of which matter more than parser code volume. Produces an AST that
mirrors the surface syntax closely — desugaring happens in lowering, not here.

**Name resolution.** Binds every identifier to a declaration; reports shadowing
according to the spec's rules; builds the scope tree consumed by the type
checker.

**Type checking.** Produces a typed AST in which every expression has a
resolved type. Types are checked, not inferred, beyond local `let`. This stage
also enforces the AVM-derived static rules: array indices in range where
statically known, byte-length bounds, and the absence of constructs the machine
cannot support.

### 5.2 IR

A typed, SSA-form control-flow graph. Functions contain basic blocks; blocks
contain instructions and end in a terminator. Block arguments rather than
phi nodes.

The IR is deliberately *not* LLVM-shaped: there are no `alloca`, `load`, or
`store` instructions, because there is no memory to address. Storage access is
an explicit effectful instruction corresponding to an AVM opcode family
(global state, local state, boxes), which keeps the analysis honest about what
is a cheap register move and what is an expensive state access.

`spec/ir.md` is the normative definition. The verifier (**R8**) enforces it.

**Lowering** translates the typed AST to IR and is where all desugaring lives:
control flow to blocks and branches, structs and fixed arrays to their
component values, and — later — ARC-4 method routing and ABI encoding
(**R7**: as generated IR, never as TEAL templates).

**Optimisation** is a set of IR→IR passes. The absence of a heap and of
aliasing makes the classical passes unusually straightforward: constant
folding, dead-code elimination, common-subexpression elimination, copy
propagation, function inlining. Each pass is independently testable, and the
verifier runs after each.

### 5.3 Cost analysis

Computes a worst-case opcode-cost bound per entry point and **rejects programs
that can exceed the budget** (700 for application mode, 20,000 for signature
mode).

Straight-line and branching code give exact bounds. Loops make this a bounds
problem: where a bound is statically inferable it is used; where it is not, the
loop must carry an explicit annotation acknowledging that the bound is
unproven, and the compiler reports the program's cost as unbounded rather than
guessing.

This is the most differentiating capability in the design. A program that
cannot exceed its budget is a program that cannot fail on-chain for the most
common non-logic reason. Rule **R11** keeps the analyser honest by checking it
against the AVM's measured cost on every differential test.

### 5.4 Stack scheduling

Turning SSA values into stack-machine code. A value consumed once, in order,
can stay on the stack; anything else must be stored to a scratch slot and
reloaded.

Doing this well — using `dup`, `swap`, `cover`, `uncover`, and `dig` in place
of naive store/load pairs — is the difference between mediocre and good code
generation, and it is where the interesting engineering is. Prior art worth
studying: WebAssembly's "stackification" of a CFG, Koopman's *Stack Computers*,
and the RVSDG literature on regionalised dataflow.

### 5.5 Scratch allocation

Register allocation, with 256 registers and no cheap spill target. Standard
approaches (linear scan, graph colouring) apply, with one difference from a
conventional backend: **running out of slots is a compile error**, not a
performance cliff, because the only place to spill to is boxes, at a cost that
would usually blow the opcode budget anyway.

### 5.6 Constant pooling and emission

TEAL's `intcblock`/`bytecblock` mechanism makes constant pooling a genuine size
optimisation rather than a cleanup: `intc_0` is one byte where `pushint` is
variable-length, and program size is a hard 2 KB-per-page limit. Pooling
selects which constants earn a block slot.

Emission then produces TEAL text, and the linking step handles page splitting
across the approval and clear programs within the size limit.

### 5.7 Stage contracts

Uniform, and enforced by review:

```rust
pub fn stage(input: Input, diags: &mut Diagnostics) -> Option<Output>;
```

- Pure (**R1**), so every stage is trivially testable in isolation.
- `None` means errors were reported and no artifact is produced (**R3**).
- Every boundary is snapshot-testable, which means a change in any stage
  produces a reviewable diff at that stage's boundary rather than only in final
  TEAL.
- A stage cannot reach into another stage's internals, because the only thing
  it receives is the previous stage's output type. This is the property that
  makes the codebase safe to hand to agents working in parallel.

---

## 6. Correctness strategy

Four layers, in increasing order of strength.

**Unit tests.** Per-stage, per-pass. Ordinary.

**Snapshot tests** (`insta`) at every stage boundary. These catch *changes*.
They do not catch *wrongness* — an incorrect first emission gets snapshotted
and then defended forever. This is the limitation that motivates the next
layer.

**Conformance tests** (`tests/conformance/`). The executable half of
`spec/language.md`. Every language feature has at least one. Under **R4**,
these are written before the implementation they describe.

**Differential tests.** The primary defence against miscompilation:

```
  random Ava program
     ├─► reference interpreter over the IR ──────────► result A
     └─► compile to TEAL ──► execute on real AVM ────► result B
  assert A == B
  assert measured_cost <= predicted_bound          (R11)
```

Two independently derived answers to "what does this program mean". Any
disagreement is a bug in the compiler or the interpreter — either way, a bug.
`proptest` generates programs and shrinks failures to minimal reproducers
automatically.

This is what catches the bug class that matters: TEAL that assembles cleanly,
passes every snapshot test, and means something subtly different from the
source — a clobbered scratch slot, operands transposed by a `cover` sequence,
an overflow check the optimiser removed. It is the technique that found
hundreds of bugs in GCC and LLVM, and it is the reason the oracle is the real
AVM rather than something we wrote.

---

## 7. Repository layout

```
crates/
  avmc-span/        source ids, spans, source map
  avmc-diag/        diagnostics, codes, rendering
  avmc-lexer/
  avmc-ast/
  avmc-parser/
  avmc-sema/        name resolution, type checking
  avmc-ir/          IR types, verifier, printer, reference interpreter
  avmc-lower/       typed AST -> IR
  avmc-opt/         IR -> IR passes
  avmc-cost/        static budget analysis
  avmc-backend/     stack scheduling, scratch allocation, pooling, TEAL emission
  avmc-driver/      pipeline orchestration; the only crate with I/O
  avmc-cli/         the `avmc` binary
  avmc-oracle/      test-only: the Oracle trait + algod HTTP implementation
tools/
  oracle-sidecar/   (future) Go binary linking go-algorand
spec/
  language.md       normative Ava grammar and static semantics
  ir.md             normative IR definition
tests/
  conformance/      executable language specification
  golden/           end-to-end source -> TEAL snapshots
  differential/     generative cross-checking against the AVM
```

The crate graph is a DAG matching the pipeline order. A dependency that runs
backwards along the pipeline is an architecture violation.

---

## 8. v0 milestone

**A thin vertical slice through every stage.**

The first milestone compiles a trivial program — a signature-mode program that
approves based on one `uint64` comparison — end to end: lexer, parser,
resolution, type checking, lowering to IR, the verifier, cost analysis, stack
scheduling, scratch allocation, emission, and execution against a real AVM via
the oracle.

Chosen over "complete the frontend first" deliberately. The unknown risk in
this project is concentrated in the back half — stack scheduling, scratch
allocation, cost analysis. A complete type checker sitting on top of unproven
codegen is a project that has retired none of its risk. The vertical slice
proves the architecture, and every stage afterwards deepens a component whose
interfaces are already exercised.

v0 is complete when a `.ava` source file produces TEAL that the reference
assembler accepts and the AVM executes with the expected result, with the whole
path covered by a golden test and a differential test.

---

## 9. Amendment

**This document.** Changes to §2 (the frozen decisions) or §4 (binding rules)
require an explicit pull request that changes only this file, states what is
being changed and why, and is reviewed on its own. They are never made in
passing as part of a feature branch.

**The language.** Governed by **R4** and stricter: `spec/language.md` first,
then a failing conformance test, then the implementation.

**Everything else** — layout, stage internals, pass lists — may be amended in
the ordinary course of work, updating this document in the same pull request
that changes the code.

The purpose of the friction is narrow. Implementation details should move
freely; the shape of the compiler and the definition of the language should
move only deliberately.

---

## 10. Open questions

Recorded rather than resolved, so they are decided explicitly.

1. **Language name.** "Ava" is provisional. Fix before the spec stabilises.
2. **Bounded recursion.** v0 forbids recursion. The AVM supports subroutines
   via `callsub`/`retsub` with a bounded call stack; whether to allow recursion
   with a statically proven depth bound depends on that limit, which must be
   **verified against the pinned `go-algorand` consensus parameters** rather
   than assumed. Until verified, no depth number appears in the spec.
3. **Application mode in v0.** The v0 slice targets signature mode as the
   simpler entry point. Application mode, state access, and ARC-4 routing
   follow — the pipeline is designed for them (§5.2), but they are not v0.
4. **Loop bound annotations.** Syntax and inference strength for the cost
   analyser's loop bounds (§5.3) are undesigned. This is a language-surface
   question and therefore falls under **R4**.
5. **Error message conventions.** Diagnostic phrasing, code ranges, and
   rendering style need a written guide before the diagnostic count grows.
6. **Consensus parameter pinning.** The limits in §1 are current values. The
   compiler must record which consensus version and TEAL version it targets,
   and the numbers should be verified against a pinned `go-algorand` rather
   than against documentation.
