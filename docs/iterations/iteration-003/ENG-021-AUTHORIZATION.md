# ENG-021 authorization

ENG-021 / #62 is the next Ready Gate-3 tactical-RPG Story after ENG-020.

The executable authority remains `docs/backlog/PRODUCT-BACKLOG.md`; this note keeps Iteration 003 synchronized without rewriting historical Story closure sections.

Authorized scope:

- additive `EncounterDocument` / `EncounterParticipant` authoring model and deterministic `EncounterCompiler`;
- authoritative Stage/actor/spawn `DocumentEnvelope.references` consumed by the existing derived Project Graph;
- structured Encounter validation;
- live schema-bearing `encounter.create`, `encounter.add_participant`, and `encounter.remove_participant` Commands;
- CLI/example compiler registration needed to validate/build the new document type through existing engine infrastructure.

The Story remains Agent-first: normal Encounter composition must be possible through semantic Commands and existing Queries without Editor-only hidden state or raw file patching.

ENG-022 and ENG-023 remain Proposed and non-executable until separately promoted.

Issue #62 is the implementation contract. Full repository validation and final clean-head CI are required before ENG-021 may be marked Done or merged.
