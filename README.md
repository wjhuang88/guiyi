# GUIYI Engine

GUIYI Engine is an AI-native, headless-first game development infrastructure built on Bevy. Its first domain toolkit targets tactical RPGs with staged exploration, encounters, dialogue, triggers, and data-driven content.

The primary client is an AI agent operating through typed commands, structured queries, transactions, machine-readable diagnostics, and validation gates. Human-facing tools are clients of the same command/query platform rather than a separate source of truth.

## Design principles

- **AI-first:** every core capability is discoverable and callable without a GUI.
- **Typed changes:** content mutations use registered commands, never ad-hoc file edits.
- **Transactional:** changes support dry-run, diff, commit, audit, and rollback.
- **Headless-first:** validation, compilation, preview, and tests run without a window.
- **Layered:** engine core, genre toolkit, and game-specific extensions have strict dependency direction.
- **Document → Artifact → Runtime:** authoring data, compiled output, and ECS instances are separate.
- **No game-specific concepts in core:** the wuxia repository informed requirements but is not a dependency.

## Workspace map

- `crates/engine_*`: reusable engine infrastructure.
- `toolkits/tactical_rpg/*`: tactical RPG domain types, runtime decoration, tools, and validation.
- `examples/*`: standalone consumption and agent workflow examples.
- `docs/`: architecture, ADRs, roadmap, SOP, API and protocol reference.
- `scripts/`: architecture and documentation gates.

## First commands

```bash
cargo run -p guiyi-engine-cli -- init --path ./my-game --name "My Game"
cargo run -p guiyi-engine-cli -- doctor --project ./my-game
cargo run -p guiyi-engine-cli -- capabilities --json
cargo run -p guiyi-engine-cli -- validate --project ./my-game
cargo run -p guiyi-engine-cli -- compile --project ./my-game --out ./my-game/artifacts
cargo run -p guiyi-engine-preview -- --artifact ./my-game/artifacts/stage.demo.artifact.json
```

Run the machine-facing workbench:

```bash
cargo run -p guiyi-engine-workbench -- --project ./my-game
```

It accepts newline-delimited JSON tool calls on stdin and returns structured tool results on stdout.

## Quality gates

```bash
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
python3 scripts/check_architecture.py
python3 scripts/check_docs.py
```

See [the documentation index](docs/README.md) and [Iteration 001 validation report](docs/iterations/iteration-001/VALIDATION.md).
