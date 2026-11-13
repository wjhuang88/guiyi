# Quality gates

## Global

```bash
cargo metadata --locked --format-version 1 >/dev/null
cargo fmt --all -- --check
cargo check --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo doc --workspace --no-deps --locked
python3 scripts/check_architecture.py
python3 scripts/check_docs.py
python3 scripts/validate_repository.py
python3 scripts/static_rust_check.py
python3 scripts/check_reproducibility.py --self-test
python3 scripts/check_reproducibility.py
```

The local toolchain is pinned to Rust `1.99.1`; update it together with the CI Rust install action under reviewed PR and exact-head gates. Cargo `--locked` requires the checked-in `Cargo.lock` to remain authoritative.

## AI tool gate

Every tool must declare:

- Stable ID.
- Purpose.
- Input and output schemas.
- Required permissions.
- Side effects.
- Related tools.
- Structured failure behavior.

Every mutating tool must support atomic execution and dry-run through the command executor.

## Runtime gate

Runtime work requires lifecycle, repeat-entry, error-content rejection, and headless integration tests.

## Content gate

Schema changes require round-trip tests, invalid-input diagnostics, migration impact, and golden data.
