# avmc

A compiler targeting the Algorand Virtual Machine (AVM).

`avmc` compiles a small, statically typed, heap-free language — designed for the
AVM's constraints, and not yet named — to TEAL.

## Status

Pre-implementation. The architecture is settled; the code is not written yet.

The current milestone is a thin vertical slice: a variable-free signature-mode
program compiled end to end and executed against a real AVM.

## Start here

Two documents, split by how often they change.

**[CONSTITUTION.md](CONSTITUTION.md)** — the governing charter. Read it before
writing code.

- the AVM constraints every decision derives from
- the three frozen decisions — target, source language, implementation
- non-goals, stated explicitly
- the binding rules (`R1`–`R10`) that contributions are reviewed against

**[ARCHITECTURE.md](ARCHITECTURE.md)** — how the compiler is built.

- the pipeline, stage by stage, and the contracts between stages
- the correctness strategy, in particular differential testing against a real AVM

## The short version

- **Target:** TEAL, emitted directly. Not LLVM IR, not WebAssembly — neither has
  a backend that reaches the AVM, and both assume a flat memory the AVM does not
  have.
- **Source:** our own language, designed and frozen early. Existing specified
  languages bring conformance suites that predominantly test heap features,
  which a machine with no heap cannot support.
- **Implementation:** Rust, cross-checked by running emitted TEAL on a real AVM.

## Licence

See [LICENSE](LICENSE).
