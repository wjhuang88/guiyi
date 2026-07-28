.PHONY: check test docs gates

check:
	cargo fmt --all -- --check
	cargo check --workspace --all-features
	cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
	cargo test --workspace --all-features

docs:
	cargo doc --workspace --no-deps

gates: check test docs
	python3 scripts/check_architecture.py
	python3 scripts/check_docs.py
