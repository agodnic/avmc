# avmc

A compiler targeting the Algorand Virtual Machine (AVM).

`avmc` compiles **Ava** — a small, statically typed, heap-free language designed
for the AVM's constraints — to TEAL.

## Status

Pre-implementation. The architecture is settled; the code is not written yet.

The current milestone is a thin vertical slice through every pipeline stage:
a trivial signature-mode program compiled end to end and executed against a
real AVM.

## Start here

**[CONSTITUTION.md](CONSTITUTION.md)** is the governing document. Read it before
writing code. It covers:

- the AVM constraints every decision derives from
- the three frozen decisions — target, source language, implementation
- non-goals, stated explicitly
- the binding rules (`R1`–`R11`) that contributions are reviewed against
- the pipeline architecture and stage contracts
- the correctness strategy, in particular differential testing against a real AVM

## The short version

- **Target:** TEAL, emitted directly. Not LLVM IR, not WebAssembly — neither has
  a backend that reaches the AVM, and both assume a flat memory the AVM does not
  have.
- **Source:** our own language, designed and frozen early. Existing specified
  languages bring conformance suites that predominantly test heap features,
  which a machine with no heap cannot support.
- **Implementation:** Rust, with the real AVM as a differential-testing oracle.

## Licence

See [LICENSE](LICENSE).
