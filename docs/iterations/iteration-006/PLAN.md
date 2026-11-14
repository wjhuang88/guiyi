# Iteration 006 — ENG-028 implementation-grade ADR evidence

**Status:** docs-only Ready promotion proposal. Do not implement before this Ready PR has full CI and is squash-merged.

## Primary outcome

Expand **ten** priority accepted/historical ADRs into implementation-grade, test-linked guidance while preserving original decisions, statuses and product direction. This is a P2 documentation quality track in parallel with ENG-030's still-blocked P0 actual Talos model proof; no new engine or Talos feature is authorized.

**Owner:** GUIYI engine platform architecture documentation. Primary Issue [ENG-028 #18](https://github.com/wjhuang88/guiyi/issues/18). [Ready contract and per-ADR baseline](../../validation/ENG-028-READY-CONTRACT.md).

## Day 0: baseline and boundaries

Read `AGENTS.md`, current DIRECTION-GOVERNANCE/DEVELOPMENT/QUALITY-GATES SOPs, ADR-0002/0003/0007/0008/0009/0010/0011/0012/0013/0014 and ADR-0015/0016, Project Storage/Graph, Command-Query, Agent Sessions and Runtime Artifact reference docs.

Confirm: `ADR-0009` is **Superseded** by `ADR-0016`; `ENG-018` audited cross-session Undo/Redo is **Proposed**, `ENG-024` human Workbench is **Proposed**, and `ENG-030` external real-model validation is **HUMAN REQUIRED**. Never convert a proposed behavior into a current invariant in a clarified ADR.

## Implementation deliverables (only after Ready merge)

1. Append specific context, rejected alternatives with tradeoffs, current invariants, known gaps and failure/recovery policy to **each** of the ten ADRs, retaining accepted date, decision and status metadata.
2. Link cited implementation paths and concrete executable test names for each ADR; mark any untested assertion as a **gap** and refer it to its owner, not a newly invented guarantee.
3. Specify compatibility/versioning and migration impact, unknown scaling envelopes and supersession conditions without inventing thresholds or replacement strategies.
4. Publish `docs/validation/ENG-028.md` with per-ADR evidence matrix, code-backed facts, exact-head automated CI and the **separate human architecture review form**.

## Gates

- Full project CI (fmt/check/strict clippy/test/doc using current pinned Rust and lock; Python architecture/docs/repository/static/reproducibility gates; all three mainline examples).
- Docs link consistency and Backlog/ADR history review; **no production code or Tool schema modifications**.
- **HUMAN REQUIRED / BLOCKED until a reviewer actually verifies** independent implementability, correctness, non-retroactive treatment and consistency with ADR-0016. Record reviewer/date/PASS, verified head and evidence URL in Issue #18/implementation PR. CI alone never authorizes Done.

## Rollback and incompatibility

No migrations or API changes. Roll back ENG-028 explanatory additions through a reviewed docs PR, leaving original accepted/superseded decisions. If implementing an ADR gap would require a product or runtime change, stop and create the separately authorized Story. Never resolve ENG-030 human proof by claiming generic tooling/CI as a real model.
