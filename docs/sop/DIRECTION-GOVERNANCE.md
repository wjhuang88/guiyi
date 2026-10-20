# Product direction governance

This SOP prevents technically reasonable work from pulling GUIYI away from its product boundary. It applies before a Story is promoted to `Ready` and whenever implementation discovers a scope/ownership conflict.

## North star

The current product proof is:

> A real Talos session can discover GUIYI capabilities, understand a project semantically, make typed transactional game-content changes, repair structured diagnostics, validate, build, preview, and leave a reviewable audit trail for a playable tactical-RPG slice.

Near-term work should remove a concrete blocker to this proof, close a P0/P1 safety/reliability gap that would invalidate it, or provide a required compatibility/migration foundation.

## Mandatory ownership test

Every Story proposed for `Ready` must name exactly one primary ownership layer:

1. **Talos runtime** — model/provider integration, conversation/turn lifecycle, planning, context management, generic memory/Skills, Tool-choice policy, retry, stopping, and generic Agent UX/runtime behavior.
2. **GUIYI engine platform** — reusable project/content models, Tool contracts, Commands, Queries, transactions, diagnostics, validation, build, artifacts, preview, Bevy runtime, and domain execution/security boundaries.
3. **GUIYI toolkit** — reusable tactical-RPG semantics, validators, compilers, runtime behavior, and intent-level Tools.
4. **Game repository** — title-specific mechanics, lore, content IDs, progression, balance, assets, and save semantics.
5. **Human client** — review, visualization, approval, rollback, inspection, and other UI that consumes the same GUIYI APIs.

If ownership is ambiguous, the Story is not Ready. Resolve the boundary first, normally through an ADR for architectural ambiguity.

## Talos boundary gate

For AI-related scope ask, in order:

1. Is this generic Agent behavior rather than game-development semantics?
2. Does the published `talos-runtime` SDK already provide the capability or a supported extension point?
3. Can GUIYI express the need as an `AgentTool`, structured Tool result, permission/access rule, diagnostic, or domain execution context instead of implementing Agent logic?

If the answer to the first question is yes, the default owner is Talos.

GUIYI must not add a generic provider abstraction, model client, prompt planner, conversation store, context compressor, generic memory system, retry engine, stopping policy, or competing Agent loop unless an accepted ADR explicitly supersedes ADR-0016.

## Talos dependency gate

The default first-party integration contract is:

- depend on the published crates.io `talos-runtime` SDK;
- keep Talos dependencies at the outer integration boundary;
- use the smallest feature set required;
- prefer SDK re-exports and `RuntimeBuilder` extension points;
- expose GUIYI capabilities as real Talos `AgentTool` adapters with their own Tool IDs and schemas;
- resolve dependency versions through the committed `Cargo.lock`;
- treat pre-1.0 Talos minor-version changes as reviewed compatibility changes.

Do not add direct dependencies on Talos implementation crates for convenience. If `talos-runtime` lacks a required supported capability, record the gap and prefer improving the Talos SDK. A direct lower-level Talos dependency requires an explicit Story rationale and architectural review.

Do not commit a normal dependency on Talos `main`. Git/path overrides are temporary development mechanisms only.

## Platform-expansion test

Before adding a new engine-core subsystem, answer all of the following in the Story:

- Which current north-star workflow is blocked without it?
- Why can the need not be satisfied by an existing Command, Query, Tool descriptor, diagnostic, compiler, validator, adapter, or toolkit feature?
- Why does the capability belong in reusable GUIYI engine infrastructure instead of Talos, the tactical toolkit, a game repository, or a human client?
- What is the smallest vertical acceptance scenario that proves value?

If those answers are weak, keep the idea Proposed. Do not create a foundation subsystem merely because it may be useful later.

## Semantic Tool preference

Prefer increasing game-semantic Tool density over exposing lower-level mutation primitives.

Preferred progression:

```text
primitive/document mechanism
    ↓
domain Command or Query
    ↓
intent-level toolkit Tool
```

High-level Tools must still delegate through GUIYI validation, transaction, permission and audit boundaries.

## Sequence control

The current default sequence is:

```text
ENG-029
→ ENG-014
→ ENG-012 + ENG-013
→ ENG-020..023
→ ENG-030
→ ENG-024
```

Parallel governance/security/reproducibility work is allowed when it protects this sequence without replacing the primary product outcome.

Changing the primary sequence requires:

1. a Backlog change explaining the blocker or new evidence;
2. updates to the roadmap if the direction materially changes;
3. an ADR when responsibility/dependency direction changes;
4. explicit migration/rollback notes for in-progress work that is displaced.

## Story Ready checklist

A Story may become `Ready` only when it records:

- primary ownership layer;
- north-star contribution or mandatory safety/compatibility reason;
- explicit non-goals;
- required ADR/reference reads;
- dependency direction impact;
- machine-facing acceptance scenario;
- validation evidence required for `Done`;
- migration/rollback requirements;
- any Talos SDK/version/feature impact when AI integration is involved.

## Scope correction during implementation

When implementation reveals that a requirement belongs to another ownership layer:

1. stop expanding the active patch;
2. keep already-valid work within the active Story;
3. create or refine the correct Story in the owning project/layer;
4. update the current Story's dependency or non-goal;
5. update/supersede an ADR if the ownership boundary itself changed.

Do not solve an ownership mistake by adding a convenience abstraction in the wrong layer.

## Human UI gate

Before the Talos-driven production demo is credible, human-client work should be limited to functionality required for review, debugging, inspection, or approval of the machine-facing workflow. A GUI-only authoring capability is not an acceptable shortcut around missing Commands, Queries, diagnostics, or Tool schemas.

## Completion review

At Story completion, ask:

- Did this change strengthen the Talos → GUIYI → Bevy path?
- Did GUIYI accidentally acquire generic Agent-runtime responsibility?
- Is every new public capability discoverable and callable headlessly?
- Is the authoritative state still owned by the correct layer?
- Did any dependency or API choice create a second source of truth?

Any negative answer is a completion blocker or must be captured as an explicitly accepted limitation with a follow-up Story.
