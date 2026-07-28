# Lessons extracted from the wuxia prototype

The original wuxia repository was used as a requirements and failure-mode sample. It is not linked or embedded in this workspace.

## Reused concepts

- Bevy plugin-oriented runtime composition.
- Hex coordinate mathematics.
- Data-driven Stage and story direction.
- Logic/event/application separation in combat architecture.
- Automated Rust unit testing culture.

## Reimplemented boundaries

- Stage lifecycle is explicit and repeatable.
- Authoring content is separated from runtime instances.
- Persistent identity does not use ECS entities.
- Content errors are structured and block compilation.
- Commands are transactional and support dry-run.
- Toolkit concepts are separated from game-specific mechanics.

## Deliberately not extracted

- Spirit altar, lamp oil, lifespan, medicine-body, wuxia attributes, concrete characters, story, and world rules.
- Legacy scene/story loader implementation.
- Existing world-state ownership and exploration lifecycle.
- Existing save format.

## Regression requirements derived from prototype defects

- Re-entering a Stage must not reset persistent world state.
- Loading/unloading repeatedly must not duplicate players, cameras, enemies, or lights.
- Multi-entity state must remain keyed by stable target identity.
- Default sample content must be reachable and previewable.
- Runtime death or combat state must be written through one authoritative identity model.
