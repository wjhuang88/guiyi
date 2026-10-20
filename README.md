# GUIYI Engine

GUIYI Engine is an AI-native, headless-first game development infrastructure built on Bevy. Its first domain toolkit targets tactical RPGs with staged exploration, encounters, dialogue, triggers, and data-driven content.

The primary first-party AI client is **Talos**, using the published `talos-runtime` Rust SDK. GUIYI does not implement a competing generic agent framework: Talos owns model/provider, conversation, planning, context, memory and generic Agent-loop behavior, while GUIYI owns game-world semantics, typed Tools, transactions, diagnostics, validation, build, preview and Bevy runtime behavior.

Human-facing tools and direct machine clients consume the same GUIYI command/query execution surface rather than a separate source of truth.

## Design principles

- **AI-first:** every core capability is discoverable and callable without a GUI.
- **Talos-first integration:** first-party AI uses the published `talos-runtime` SDK at an outer adapter boundary.
- **Typed changes:** content mutations use registered commands, never ad-hoc file edits.
- **Transactional:** changes support dry-run, diff, commit, audit, and rollback.
- **Headless-first:** validation, compilation, preview, and tests run without a window.
- **Layered:** Talos integration, engine core, genre toolkit, and game-specific extensions have strict dependency direction.
- **Document → Artifact → Runtime:** authoring data, compiled output, and ECS instances are separate.
- **No game-specific concepts in core:** the wuxia repository informed requirements but is not a dependency.

## First-party AI boundary

```text
Talos / talos-runtime
        ↓
GUIYI Talos adapter
        ↓
GUIYI Tool catalog + execution boundary
        ↓
Queries / Commands / Transactions / Diagnostics
        ↓
Validation / Build / Preview / Artifacts
        ↓
Bevy ECS
```

The near-term product proof is a real Talos session that can discover GUIYI capabilities, edit a tactical-RPG project, repair structured diagnostics, validate, build, preview, and leave a reviewable transaction trail.

## Workspace map

- `crates/engine_*`: reusable engine infrastructure.
- `toolkits/tactical_rpg/*`: tactical RPG domain types, runtime decoration, Tools, and validation.
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

Run the direct machine-facing Workbench:

```bash
cargo run -p guiyi-engine-workbench -- --project ./my-game
```

It accepts newline-delimited JSON Tool calls on stdin and returns structured Tool results on stdout. This direct client remains useful for tests and automation; ENG-014 adds the first-party Talos runtime adapter over the same GUIYI capability boundary.

## Quality gates

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

See [the documentation index](docs/README.md), [the product backlog](docs/backlog/PRODUCT-BACKLOG.md), and [ADR-0016](docs/decisions/ADR-0016-TALOS-AGENT-RUNTIME.md).
