# First local run

## Prerequisites

- Rust **1.99.1** from the checked-in `rust-toolchain.toml`, with `rustfmt` and `clippy`.
- Python 3.11 or newer.
- Network access for the initial locked Cargo dependency download; `Cargo.lock` is already committed. `--locked` prevents implicit lockfile updates, not network access.

## Execute all gates

```bash
cd guiyi-engine
./scripts/first_run.sh
```

## Exercise the CLI manually

```bash
cargo run --locked -p guiyi-engine-cli -- init --path /tmp/guiyi-game --name "AI Game"
cargo run --locked -p guiyi-engine-cli -- doctor --project /tmp/guiyi-game --json
cargo run --locked -p guiyi-engine-cli -- capabilities --json
cargo run --locked -p guiyi-engine-cli -- validate --project /tmp/guiyi-game --json
cargo run --locked -p guiyi-engine-cli -- compile --project /tmp/guiyi-game --out /tmp/guiyi-game/artifacts --json
cargo run --locked -p guiyi-engine-preview -- --artifact /tmp/guiyi-game/artifacts/stage.demo.artifact.json --json
```

## Exercise the JSONL workbench

```bash
cargo run --locked -p guiyi-engine-workbench -- --project /tmp/guiyi-game < protocol_examples/create_stage.jsonl
```

The sample JSONL creates a separate `stage.ai`; run it against a fresh project or change the IDs when repeating the test.
