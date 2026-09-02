# avmc — Architecture

How `avmc` is built: the compilation pipeline, the contracts between its
stages, the correctness strategy, and the repository layout.

This document is **living**. It is amended in the ordinary course of work, in
the same pull request that changes the code it describes.

Its companion, **[CONSTITUTION.md](CONSTITUTION.md)**, is not. That document
holds the AVM constraints every decision here derives from (§1), the frozen
decisions about target, source language, and implementation (§2), and the
binding rules **R1**–**R11** (§4) that this document cites throughout. Changing
those requires a dedicated pull request; changing this one does not.

Read the constitution first. This document assumes it.

---

## 1. Pipeline architecture

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

### 1.1 Frontend stages

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

### 1.2 IR

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

### 1.3 Cost analysis

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

### 1.4 Stack scheduling

Turning SSA values into stack-machine code. A value consumed once, in order,
can stay on the stack; anything else must be stored to a scratch slot and
reloaded.

Doing this well — using `dup`, `swap`, `cover`, `uncover`, and `dig` in place
of naive store/load pairs — is the difference between mediocre and good code
generation, and it is where the interesting engineering is. Prior art worth
studying: WebAssembly's "stackification" of a CFG, Koopman's *Stack Computers*,
and the RVSDG literature on regionalised dataflow.

### 1.5 Scratch allocation

Register allocation, with 256 registers and no cheap spill target. Standard
approaches (linear scan, graph colouring) apply, with one difference from a
conventional backend: **running out of slots is a compile error**, not a
performance cliff, because the only place to spill to is boxes, at a cost that
would usually blow the opcode budget anyway.

### 1.6 Constant pooling and emission

TEAL's `intcblock`/`bytecblock` mechanism makes constant pooling a genuine size
optimisation rather than a cleanup: `intc_0` is one byte where `pushint` is
variable-length, and program size is a hard 2 KB-per-page limit. Pooling
selects which constants earn a block slot.

Emission then produces TEAL text, and the linking step handles page splitting
across the approval and clear programs within the size limit.

### 1.7 Stage contracts

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

## 2. Correctness strategy

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

## 3. Repository layout

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
