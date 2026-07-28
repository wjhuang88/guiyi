#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
python3 scripts/check_architecture.py
python3 scripts/check_docs.py
python3 scripts/validate_repository.py
python3 scripts/static_rust_check.py
cargo run -p guiyi-engine-cli -- doctor --project sample_projects/agent_tactical_demo --json
cargo run -p guiyi-engine-cli -- validate --project sample_projects/agent_tactical_demo --json
cargo run -p guiyi-example-tactical-demo
cargo run -p guiyi-example-mock-agent-workflow
