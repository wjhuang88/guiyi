#!/usr/bin/env bash
set -euo pipefail

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
cargo run -p guiyi-engine-cli --locked -- doctor --project sample_projects/agent_tactical_demo --json
cargo run -p guiyi-engine-cli --locked -- validate --project sample_projects/agent_tactical_demo --json
cargo run -p guiyi-example-minimal-project --locked
cargo run -p guiyi-example-tactical-demo --locked
cargo run -p guiyi-example-mock-agent-workflow --locked
