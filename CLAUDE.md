
# Programming style
- Code must be easy to comprehend
- Readable is better than clever
- Design for correctness
- Leverage strong typing
- Start with few or even a single rust module, then split into new modules/crates as needed as the software grows
- Avoid unnecessary complexity
- Avoid feature scope creep

# Building
`cargo test` is the entry point. CI runs `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, and `cargo test` on every push to
`main` and every pull request. The toolchain is pinned in `rust-toolchain.toml`.
