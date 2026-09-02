
# Programming style
The items in this section are not hard rules - they can be broken if there is a good reason.
- Code must be easy to comprehend
- Readable is better than clever
- Design for correctness
- Leverage strong typing
- Start with few or even a single rust module, then split into new modules/crates as needed as the software grows
- Avoid unnecessary complexity
- Avoid feature scope creep
- Keep doc comments small
- Avoid adding redundant information to doc comments that can be obtained somewhere else
- Try to avoid cross-references in comments. These tend to get stale, or need constant updating as the code changes. It is also a form of duplicate information.

# Building
`cargo test` is the entry point. CI runs `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, and `cargo test` on every push to
`main` and every pull request. The toolchain is pinned in `rust-toolchain.toml`.
