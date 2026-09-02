# avmc — Architecture

How `avmc` is built: the compilation pipeline, the contracts between its
stages, and the correctness strategy.

Read [CONSTITUTION.md](CONSTITUTION.md) first. This document assumes it.

## 1. Design posture

**Start at the simplest thing that is honestly end-to-end, and grow each stage
only when a language feature forces it.**

§2 is the pipeline as it exists. Nothing is added to it speculatively — if a
stage has no work to do, it does not exist yet.

The thing that makes this safe rather than reckless is §3. A strong oracle
means the middle of the compiler can be torn out and replaced with confidence
that no program changed meaning. "Simplify now, generalise later" is a bad plan
without differential testing and a good one with it.

---

## 2. The pipeline

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
   │  lower   │  typed AST ─────────► IR  (flat single-assignment list)
   ├──────────┤
   │   emit   │  IR ────────────────► TEAL text
   └────┬─────┘
        │
   TEAL ──► (external assembler) ──► bytecode
```

Six stages, each of which does real work. There is no optimiser, no stack
scheduler, no scratch allocator, and no constant pooler, because at the current
language level none of them would have anything to do.

### 2.1 Frontend

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
also enforces the AVM-derived static rules: byte-length bounds and the absence
of constructs the machine cannot support.

### 2.2 IR

A typed, single-assignment **flat instruction list**. Not yet a control-flow
graph — the language has no control flow, so a CFG would be one block, and a
one-block CFG is an expression tree wearing a costume.

The IR is deliberately *not* LLVM-shaped: there are no `alloca`, `load`, or
`store` instructions, because there is no memory to address. Storage access is
an explicit effectful instruction corresponding to an AVM opcode family
(global state, local state, boxes), which keeps the analysis honest about what
is a cheap stack operation and what is an expensive state access.

The IR exists at v0 even though the AST could be emitted from directly, because
it is the seam that lets the backend grow without the frontend noticing. The
emitter consumes IR and nothing else, so extending the backend is a change to
the IR and the emitter, not to the parser or the type checker.

**The v0 invariant is what makes emission trivial:** every value has **exactly
one use**, and uses appear in the order values are defined. Lowering an
expression tree in post-order produces exactly this. The verifier (**R8**)
enforces it, along with type correctness and single assignment.

`spec/ir.md` is the normative definition.

**Lowering** translates the typed AST to IR and is where all desugaring lives.
Later, ARC-4 method routing and ABI encoding land here too (**R7**: as
generated IR, never as TEAL templates).

### 2.3 Emission

A single linear pass over the IR. Because every value has exactly one use and
uses follow definitions in order, each instruction emits its opcodes and leaves
its result on the stack for the next consumer. No `dup`, no `cover`, no
`uncover`, no scratch traffic, no scheduling algorithm — a post-order traversal
of an expression tree *is* optimal stack code.

**R7** confines TEAL text to this stage.

### 2.4 Stage contracts

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
  it receives is the previous stage's output type.

---

## 3. Correctness strategy

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
  random source program
     ├─► reference interpreter over the IR ──────────► result A
     └─► compile to TEAL ──► execute on real AVM ────► result B
  assert A == B
```

Two independently derived answers to "what does this program mean". Any
disagreement is a bug in the compiler or the interpreter — either way, a bug.
`proptest` generates programs and shrinks failures to minimal reproducers
automatically.

This is what catches the bug class that matters: TEAL that assembles cleanly,
passes every snapshot test, and means something subtly different from the
source. It is the technique that found hundreds of bugs in GCC and LLVM, and it
is the reason the oracle is the real AVM rather than something we wrote.
