# avmc — Architecture

The compilation pipeline, the contracts between its stages, and the invariants
all of it holds to.

**Design posture: start at the simplest thing that is honestly end-to-end, and
grow each stage only when a language feature forces it.** The pipeline below is
what exists, not what is planned, and this file describes its shape rather than
the features built on it — a new feature is not a reason to extend it.

## 1. The pipeline

```
   source text
        │
   ┌────▼─────┐
   │  lexer   │  text ──────────────► tokens + spans
   ├──────────┤
   │  parser  │  tokens ────────────► AST
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

**Lexer and parser.** Both hand-written, chosen over generated ones for
error-message quality. The AST mirrors the surface syntax; desugaring happens
in lowering, not here.

**Type checking.** Produces a typed AST in which every expression has a
resolved type. Types are checked, not inferred. This stage
also enforces the AVM-derived static rules: byte-length bounds and the absence
of constructs the machine cannot support.

**The IR** is a typed, single-assignment flat instruction list — not a
control-flow graph, because the language has no control flow, and a one-block
CFG is an expression tree wearing a costume. Its v0 invariant is what makes
emission trivial: every value has **exactly one use**, and an instruction's
operands are the values most recently defined and not yet consumed, in operand
order — stack order. Lowering an expression tree in post-order produces exactly
that, so emission is one linear pass with no stack shuffling, no scratch
traffic, and no scheduling algorithm.

**The verifier** enforces that invariant, along with type correctness and
single assignment, at every IR boundary in debug and test builds. What it
checks grows with the IR.

**The target TEAL version is a required compilation parameter**, never inferred
from the source and never silently upgraded. Using an opcode unavailable in the
target version is a compile error.

## 2. Stage contracts

Uniform, and enforced by review:

```rust
pub fn stage(input: Input, diags: &mut Diagnostics) -> Option<Output>;
```

- **Stages are pure functions.** No file I/O, no network, no environment
  access, no global mutable state. All I/O lives in the binary. Purity is what
  makes every stage trivially testable in isolation.
- **Errors never silently degrade.** A stage that reports an error produces no
  output that a later stage will consume; `None` is how that is expressed.
  Recovery for the purpose of reporting *more* diagnostics is encouraged;
  recovery that produces artifacts is forbidden.
- A stage cannot reach into another stage's internals, because the only thing
  it receives is the previous stage's output type.

## 3. Invariants

These hold across every stage. Agents and contributors must not violate them.

- **Spans everywhere.** Every token, AST node, IR instruction, and emitted
  opcode carries a source span. A diagnostic without a span is a bug.
- **Determinism.** For a fixed compiler version, input, and target TEAL
  version, output is byte-identical. No hash-map iteration order, no
  timestamps, no absolute paths, no parallelism-dependent ordering.
- **No panics.** Malformed source produces diagnostics, never a panic.
  `unwrap`/`expect`/`panic!` are permitted only for conditions the IR verifier
  has already established.
