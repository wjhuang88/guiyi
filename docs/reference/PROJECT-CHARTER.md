# Project charter

## Mission

Build an independent Bevy infrastructure project for AI-driven tactical RPG development. The engine exposes deterministic, discoverable, transactional, and machine-verifiable capabilities. A human interface is optional and uses the same APIs as agents.

## Product layers

1. Engine domain and content core.
2. Command, query, validation, build, runtime, and asset platform.
3. Agent protocol, session host, permission model, and tool catalog.
4. Tactical RPG toolkit.
5. Game-specific extensions maintained outside this repository.

## Alpha success criteria

- A project can be initialized through CLI.
- An agent can discover capabilities without a static mega-prompt.
- An agent can dry-run, apply, validate, query, and audit Stage changes.
- Stage documents compile into artifacts and load into Bevy ECS headlessly.
- Repeated Stage load/unload has an automated lifecycle test.
- Engine core has no tactical RPG or game-specific dependency.
- CI executes formatting, compilation, linting, tests, docs, architecture, and link gates.

## Non-goals before 1.0

- General-purpose DCC replacement.
- General animation, shader, or behavior-tree authoring.
- Rust dynamic-library plugins.
- Multiplayer collaborative editing.
- A full visual editor before command/query APIs stabilize.
- Embedding one model vendor or one agent-loop implementation in core.
