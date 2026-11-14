# ENG-028 — Independent human architecture review handoff

> **HUMAN REQUIRED / BLOCKED / NOT RUN.** This is a **review checklist**, not evidence of human approval. Do not mark ENG-028 Done or merge its Draft ADR implementation PR until the independent review is completed and PASS is explicitly recorded. Automated docs/CI are necessary, **not sufficient**.

**Issue:** [ENG-028 / #18](https://github.com/wjhuang88/guiyi/issues/18)  
**Ready authorization:** [docs-only PR #81](https://github.com/wjhuang88/guiyi/pull/81), merged after CI run 382  
**Source baseline and goals:** [ENG-028 Ready contract](ENG-028-READY-CONTRACT.md)  
**Implementation evidence:** [ENG-028 validation](ENG-028.md)  
**Authoritative direction:** [ADR-0016 Talos ownership](../decisions/ADR-0016-TALOS-AGENT-RUNTIME.md) and [product governance](../sop/DIRECTION-GOVERNANCE.md)

## Who reviews and what to capture

An operator/architecture reviewer separate from the documentation drafting pass should read the ten ADRs on the **exact implementation PR HEAD**, cross-check their linked Rust code/test paths, and verify no policy was quietly invented. Do not assume an assistant-written document or a passing CI test has approved an architectural interpretation.

Capture (no secrets): reviewer identity or accountable review role, review date (UTC), exact tested Git SHA, PR URL, ten-item PASS/FAIL matrix, explicit policy-gap Issues, overall PASS/FAIL, and sanitized evidence link. A review based on an old head becomes stale after material ADR edits; re-review affected files.

## Ten-ADR review matrix

| Decision | Independent reviewer must check | Current outcome |
| --- | --- | --- |
| [0002](../decisions/ADR-0002-DOCUMENT-ARTIFACT-RUNTIME.md) | Document vs compiled Artifact vs short-lived Bevy Runtime authority, type/version/hash load rejection, unload cleanup, content/save migration explicitly not implied | NOT RUN |
| [0003](../decisions/ADR-0003-STABLE-IDS.md) | Stable Document/Object/Artifact IDs, authoritative references, missing-target rejection and no ECS `Entity` persistence or invented migration | NOT RUN |
| [0007](../decisions/ADR-0007-TYPED-COMMANDS.md) | Registered typed schemas, pre-mutation validation, actual working-set diff, dry-run vs apply, audit/persistence boundary and no raw patch/Editor-only bypass | NOT RUN |
| [0008](../decisions/ADR-0008-TRANSACTIONS.md) | In-memory clone/diff/preview vs durable ProjectStorage journal; pre/postcommit recovery; nested/concurrent transactions **not** falsely promised; ENG-018 audited undo still Proposed | NOT RUN |
| [0009](../decisions/ADR-0009-AGENT-LOOP-DECOUPLING.md) | Status still **Superseded** by ADR-0016; historical loop fixtures do not authorize a competing Talos runtime | NOT RUN |
| [0010](../decisions/ADR-0010-DIAGNOSTICS.md) | Structured code/severity/location fields, JSON Pointer / optional suggestions, stable code vs changing prose and secure error visibility | NOT RUN |
| [0011](../decisions/ADR-0011-PROJECT-GRAPH.md) | Envelope reference source of truth, derived cache refresh/rebuild, dangling targets, hidden-node traversal isolation | NOT RUN |
| [0012](../decisions/ADR-0012-LEAST-PRIVILEGE.md) | Session/budget/working-set/permission checks, actual changed-document scope, no implicit trusted-local authority or misleading multi-tenant guarantee | NOT RUN |
| [0013](../decisions/ADR-0013-HEADLESS-FIRST.md) | Real CLI/Preview/Workbench process tests, stdout/stderr, JSON/error and no GUI requirement; distinguishes fixture headless CI from live-model proof | NOT RUN |
| [0014](../decisions/ADR-0014-UI-CLIENT.md) | Agent-first client of same APIs, existing JSONL Workbench vs ENG-024 Proposed visual UI, no second project truth | NOT RUN |

## Acceptance procedure

1. Verify main's Ready authorization was the **docs-only PR #81** (not a direct code change on Proposed), and record implementation PR number and exact Git head SHA. Check that the diff is confined to the ten ADRs, this validation packet and any approved docs-only references.
2. For each ADR read **context/forces, meaningful rejected alternative, present invariants, known unsupported gaps, failure/recovery, version/compatibility/migration, costs/scaling and supersession condition**. Check actual implementation/test names. Reject unsupported statements such as “all concurrent writes serialize”, “Undo/Redo is already audited on disk”, “all fatal CLI errors are JSON”, “the visual Editor owns content”, or “scripted Talos fixture proves a real model”.
3. Check consistency across ADR-0015 schema authority, accepted ADR-0016 Talos ownership, product governance, and the Backlog statuses of ENG-018, ENG-024/025, ENG-030.
4. Inspect exact final-head GitHub Actions CI (all required Rust, docs, repository Python checks, reproducibility gate and three examples) and the immutable PR diff. A green CI does not replace this review.
5. If inconsistent: record **FAIL**, cite conflicting lines, observed Rust behavior and any remediating Issue/Story; keep PR Draft/Open and Issue #18 Open. Do not implement code fixes under ENG-028 to make prose true.
6. Only if **all ten items pass**, attach a sanitized signed review record to Issue #18 and the implementation PR and explicitly authorize its final exact-head merge. Re-run affected docs/CI if anything changed after review.

## Operator evidence form — fill only after actual review

- [ ] Reviewer name/role: **NOT RUN**
- [ ] Date/time UTC: **NOT RUN**
- [ ] Implementation PR URL, final exact head SHA: **NOT RUN**
- [ ] All ten ADR PASS/FAIL dispositions, with links/line references: **NOT RUN**
- [ ] Cross-ADR consistency / original status and supersession history: **NOT RUN**
- [ ] Real implementation and test evidence checked (not merely linked): **NOT RUN**
- [ ] Known Proposed gaps distinguished from current behavior: **NOT RUN**
- [ ] Final exact-head GitHub CI: **NOT RUN**
- [ ] All major policy conflicts resolved or assigned to separate authorized Story: **NOT RUN**
- [ ] Overall decision **PASS / FAIL / BLOCKED**, review evidence URL: **BLOCKED**

**Separate operator lane:** ENG-030's actual Talos model-provider acceptance has its **own** [operator checklist](https://github.com/wjhuang88/guiyi/blob/feature/ENG-030-talos-playable-slice/docs/validation/ENG-030-MANUAL-ACCEPTANCE.md). This architecture review is not a substitute for that validation and must not modify its outcome.
