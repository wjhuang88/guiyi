# Iteration 001 validation

## Generation environment

The package-generation environment did not contain `rustc`, `cargo`, `rustfmt`, or `clippy`, and outbound network access was unavailable. Cargo-based gates could not be truthfully executed in that environment.

## Generation-time gates

All of the following completed successfully on 2026-07-28:

```text
python3 scripts/check_architecture.py
  architecture gate passed for 24 packages

python3 scripts/check_docs.py
  documentation link gate passed

python3 scripts/validate_repository.py
  repository gate passed: 24 workspace members

python3 scripts/static_rust_check.py
  offline Rust checks passed for 24 files
```

The offline Rust checker validates delimiter integrity and direct dependency declarations. It is deliberately not presented as a substitute for the Rust compiler.

## Package inventory

- Workspace packages: 24
- Rust source files at package generation: 24
- Rust source lines at package generation: approximately 4,100
- Markdown documents at package generation: 55
- Total project files before ZIP packaging: 121

## ENG-006 compiler-backed acceptance

Iteration 001 received compiler-backed acceptance on 2026-07-30 through pull request #20, branch `agent/ENG-006-cargo-ci-baseline`, and CI run `30552961077` (run number 32).

The final `rust` job completed every configured command successfully with exit code 0:

```text
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo run -p guiyi-example-minimal-project
cargo run -p guiyi-example-tactical-demo
cargo run -p guiyi-example-mock-agent-workflow
```

The final `repository-gates` job also completed every configured command successfully with exit code 0:

```text
python3 scripts/check_architecture.py
python3 scripts/check_docs.py
python3 scripts/validate_repository.py
python3 scripts/static_rust_check.py
```

The first real Cargo resolution produced and committed `Cargo.lock`.

## Repairs made during acceptance

- Applied canonical `cargo fmt` output across the workspace.
- Corrected ownership in the semantic reference query so the nested iterator owns per-document query targets without moving the full request into the first iteration.
- Updated Stage lifecycle tests to assert the engine-owned `StageOwned` population and preservation of a recorded `GlobalPersistent` entity instead of relying on Bevy's total internal entity count.

## Acceptance result

ENG-003, ENG-004, ENG-005, and ENG-006 satisfy their Iteration 001 acceptance gates. Iteration 001 is accepted as compiler-, lint-, test-, documentation-, example-, architecture-, and repository-gate green on the recorded commit.
