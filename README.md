# avmc

A compiler targeting the Algorand Virtual Machine (AVM). It compiles a small,
statically typed, heap-free language to TEAL.

## Building

`cargo test` is the entry point. CI runs `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, and `cargo test` on every push to
`main` and every pull request. The toolchain is pinned in `rust-toolchain.toml`.

## Documents

- [CONSTITUTION.md](CONSTITUTION.md) — frozen decisions and binding rules. Read first.
- [ARCHITECTURE.md](ARCHITECTURE.md) — pipeline, stage contracts, correctness strategy.
- [LICENSE](LICENSE)
