# avmc — Architecture

How `avmc` is built: the compilation pipeline, the contracts between its
stages, the correctness strategy, the growth path, and the repository layout.

Read [CONSTITUTION.md](CONSTITUTION.md) first. This document assumes it.

## 1. Design posture

**Start at the simplest thing that is honestly end-to-end, and grow each stage
only when a language feature forces it.**

§2 is the pipeline as it exists. §4 is the schedule by which it grows, and the
feature that forces each step. Nothing is added to §2 speculatively — if a
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
   ├──────────┤
   │   cost   │  TEAL ──────────────► cost bound  (rejects over-budget code)
   └────┬─────┘
        │
   TEAL ──► (external assembler) ──► bytecode
```

Seven stages, each of which does real work. There is no optimiser, no stack
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
emitter consumes IR and nothing else; every step in §4 is a change to the IR
and the emitter, not to the parser or the type checker.

**The v0 invariant is what makes emission trivial:** every value has **exactly
one use**, and uses appear in the order values are defined. Lowering an
expression tree in post-order produces exactly this. The verifier (**R8**)
enforces it, along with type correctness and single assignment.

That invariant is also the growth schedule in miniature. v1 relaxes
"exactly one use" and scratch slots appear. v2 relaxes "flat" and basic blocks
appear. Each relaxation is a verifier clause being removed and a corresponding
capability being added to the emitter — a legible, reviewable change rather
than a rewrite.

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

This is the stage that grows the most over §4, and the reason **R7** confines
TEAL text to it.

### 2.4 Cost analysis

Computes a cost bound and **rejects programs that can exceed the budget** (700
for application mode, 20,000 for signature mode; see CONSTITUTION.md §1).

At v0 this is a sum over the emitted opcodes — genuinely a few dozen lines, and
exact, because straight-line code has one path. It runs on emitted TEAL rather
than on the IR so that it needs no second opcode-cost table to drift out of
sync with the emitter. It moves onto the IR at v2, when branches make it a
worst-case-path problem that wants a CFG.

It is here at v0 despite being trivial, for two reasons. The **R11**
differential assertion — measured cost never exceeds the predicted bound — is
free to establish while cost is a sum, and expensive to retrofit trust into
later. And static cost rejection is the capability that distinguishes this
compiler from the existing AVM toolchains; a differentiating feature built last
tends not to get built.

### 2.5 Stage contracts

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
source. At v0 that is mostly operand order and overflow behaviour. As the
backend grows along §4 it becomes clobbered scratch slots, operands transposed
by a `cover` sequence, and overflow checks removed by an optimiser — which is
the other reason to have the harness at full strength from the start, well
before the bugs it is built for can occur. It is the technique that found
hundreds of bugs in GCC and LLVM, and it is the reason the oracle is the real
AVM rather than something we wrote.

---


---

## 4. Growth path

Each row names the language feature that **forces** the machinery in it.
Nothing on this list is built before its forcing feature lands. The point of
writing it down is not to schedule the work — it is to keep "grow as needed"
from decaying into "never architect anything", by fixing in advance what
counts as needing it.

| Step | Language feature | What it forces |
|---|---|---|
| **v0** | Expressions: arithmetic, comparison, `&&`/`\|\|`, transaction field access | Nothing beyond §2. The whole backend is a post-order walk. |
| **v1** | `let` bindings | Scratch slots — and only for values with more than one use; single-use values still stay on the stack. Drops the IR's one-use invariant. |
| **v2** | `if` / conditional expressions | Basic blocks, terminators, labels. The IR becomes a CFG and the verifier gains dominance checks. Cost analysis moves onto the IR and becomes worst-case-path. |
| **v3** | Loops | Back edges. Loop bound annotations, and a real answer for programs whose cost cannot be bounded. |
| **v4** | Functions | A calling convention over `callsub`/`retsub`; enough scratch pressure to need liveness-based allocation with slot reuse rather than one slot per value. |
| **later** | — | Optimisation passes, once there is something to optimise. Constant pooling, once program size actually approaches the 2 KB page limit. Stack scheduling proper, once values have multiple uses across branches. Application mode and ARC-4 routing. |

Note that v0 is not a toy. TEAL's `&&` and `||` are ordinary
non-short-circuiting opcodes, so a useful signature-mode predicate over
transaction fields needs no control flow at all.

---

## 5. Repository layout

```
crates/
  avmc/             the compiler library
    span/           source ids, spans, source map
    diag/           diagnostics, codes, rendering
    lexer/
    ast/
    parser/
    sema/           name resolution, type checking
    ir/             IR types, verifier, printer, reference interpreter
    lower/          typed AST -> IR
    backend/        emission (R7: the only place TEAL text is written)
    cost/           budget analysis
    driver/         pipeline orchestration; the only module with I/O
  avmc-cli/         the `avmc` binary
  avmc-oracle/      test-only: the Oracle trait + algod HTTP implementation
spec/
  language.md       normative Ava grammar and static semantics
  ir.md             normative IR definition
tests/
  conformance/      executable language specification
  golden/           end-to-end source -> TEAL snapshots
  differential/     generative cross-checking against the AVM
```

Three crates, not fourteen. The pipeline stages are **modules inside one
library crate**, because a fourteen-crate workspace is a lot of ceremony for a
compiler that does not exist yet, and modules can be split into crates later by
moving directories.

The two that are separate are separate for a reason. `avmc-cli` keeps the
library usable as a library. `avmc-oracle` is test-only and carries an HTTP
client; isolating it keeps those dependencies out of the compiler's dependency
graph entirely.

The cost of this is real and worth stating: with one crate, "the module graph
is a DAG matching the pipeline order" is a **review rule** rather than
something the compiler enforces, since Rust prevents cycles between crates but
not between modules. A dependency that runs backwards along the pipeline is an
architecture violation either way. If that rule turns out to need mechanical
enforcement, the modules split back into crates.
