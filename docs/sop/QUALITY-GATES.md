# Quality gates

## Global

```bash
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
python3 scripts/check_architecture.py
python3 scripts/check_docs.py
python3 scripts/validate_repository.py
```

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
