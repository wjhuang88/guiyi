# ENG-030 — Operator handoff: real-model acceptance (HUMAN REQUIRED)

> **Status: BLOCKED / NOT EXECUTED — 2026-11-04.** This is an **operator acceptance checklist**, **not** proof that a live model passed. Keep ENG-030 **Ready**, Issue #72 **Open** and PR #74 **Draft / unmerged** until the requirements below are objectively satisfied. No simulated provider or deterministic CI run can sign this off.

**Story:** [ENG-030 / Issue #72](https://github.com/wjhuang88/guiyi/issues/72)  
**Implementation:** [Draft PR #74](https://github.com/wjhuang88/guiyi/pull/74), branch `feature/ENG-030-talos-playable-slice`  
**Executable/injection contract:** [example README](../../examples/talos_playable_slice/README.md)  
**Machine-only validation record:** [ENG-030.md](ENG-030.md)

## Responsibility and sequencing

- **Human/operator or a separate guided acceptance session (required):** choose and configure a **genuine external model/provider**, inject it through the published Talos `LanguageModel` seam, initialize an isolated test project, execute one complete Talos model-driven session, inspect the resulting sanitized evidence, and explicitly record PASS/FAIL against the contract. Secrets/provider selection remain outside GUIYI. The model must choose Tool calls based on returned structured data; operator may set task and project fixture, but may **not** manually replay the Tool sequence or edit authored files during the session.
- **Implementation/automated work (no operator needed):** GUIYI code, contract tests, deterministic CI witness, security/failure tests, typed Tool schema discovery, artifact hashing and audit checks, doc consistency, independent CI execution. These tasks can continue now without pretending to close the acceptance gate.
- **Final integrator (only after human proof):** reference immutable real-provider evidence, rerun the complete suite on the **final exact implementation HEAD**, review the diff, then squash-merge PR #74 and mark ENG-030 Done / Issue #72 closed. This order must not be reversed.

## Operator preflight (do not expose credentials)

1. Use a disposable workspace and an **authorized copy** of PR #74 code, recording `git rev-parse HEAD`, Talos runtime SDK version `0.10.1`, external provider/model identification (non-secret), run date and environment. Do not use a production GUIYI project.
2. Configure a real `Arc<dyn guiyi_talos_runtime::sdk::LanguageModel>` in an **operator-owned embedding application** via the Talos SDK/provider infrastructure; **do not add** a provider client/secret loader to GUIYI. Consult [README](../../examples/talos_playable_slice/README.md#real-provider-product-acceptance--still-required) for the `run_with_provider(project_root, provider, context)` invocation. This API returns a machine-readable review packet if all its minimum runtime assertions succeed.
3. Create an isolated GUIYI `ProjectStorage` project with manifest/actor-definition input as necessary **before** the session. Ensure an unrestricted document working set and explicit `Read`, `EditContent`, `RunValidation`, `RunBuild`, `RunPreview` permissions for the accepted end-to-end path. Store the pre-run manifest/document IDs or hashes for later reconciliation.
4. Use the harness prompt as the task; do not supply the model with a pre-written Tool-call tape. Capture only an access-controlled sanitized event/Tool-result summary and resulting review packet; **never** commit API keys, model messages containing private prompts, raw document Query results, filesystem secrets, or authentication headers.

## Mandatory observations and pass criteria

| Step | Required observable evidence | Pass condition |
| --- | --- | --- |
| 1. Genuine provider | Operator provenance, non-secret model/provider metadata; Talos Runtime session and observed live Tool input schemas | An **actual model** selects Tools; the deterministic fixture is explicitly excluded |
| 2. Inspect and author | `project.documents.list`, typed `stage.create` (or applicable semantic change), `encounter.*`, `dialogue.*` Tool calls and applied transaction IDs | Stage, Encounter participant, Dialogue graph are authored by GUIYI semantic Commands, not private file edits |
| 3. Observe defect | A model-visible `stage.validate` result with authentic `STAGE_NO_SPAWN_POINT` or approved equivalent | Defect diagnostic result **precedes** the model's choice to repair |
| 4. Repair | `tactical.make_stage_playable` dry-run status `previewed`, then status `applied`; before/after diagnostics and matching transaction diffs | No direct patch; previewed transaction did **not** persist; applied transaction **did** persist |
| 5. Validate | Model-requested `demo.project.validate` Tool and successful structured report | Authorized full-project validation; no blocking error |
| 6. Build | Model-requested `demo.project.build` succeeds with Stage + Encounter + Dialogue artifacts, IDs and source/integrity hashes | Exactly the required three tactical artifact classes from current authoritative persisted documents; no hidden post-session host-only build |
| 7. Preview | Model-requested `demo.stage.preview` references the Stage artifact built in step 6 | Artifact integrity confirmed; nonzero expected entity count; **two** load/unload cycles with zero leaked Stage-owned objects |
| 8. Persistence and security | Reopen `ProjectStorage`, compare manifest/references, applied audit transaction IDs; negative permission/working-set test and invalid-operation result | Persisted contents match applied Commands only; no secrets/private data in exported evidence; rejection does not bypass scope or mutate state |
| 9. Completion | Talos terminal event, review packet, operator result and exact tested HEAD | Session completed with all required machine assertions; **human PASS recorded separately** |

If any point fails, record **FAIL / BLOCKED**, concrete diagnostic codes and safe repro instructions; do not declare this Story Done. If there is no real model/provider access, record **NOT RUN / BLOCKED** rather than interpreting fixture CI as PASS.

## Evidence handoff form — complete only after a real run

Copy into an **appropriately access-controlled** review record (Issue #72 / PR #74 link, or sanitized repository document if approved). The checkboxes are intentionally unchecked.

- [ ] Provider genuinely called (not fixture), model/provider ID (non-secret): **NOT RUN**
- [ ] Test date (UTC), exact implementation commit SHA: **NOT RUN**
- [ ] Isolated project fixture, pre-run state hash and operator embedding reference: **NOT RUN**
- [ ] Live schema discovery and ordered model-selected Tool calls: **NOT RUN**
- [ ] Diagnostic observed before dry-run/apply; transaction IDs and safe diff hashes: **NOT RUN**
- [ ] Agent-invoked validate/build/preview, three artifact IDs/hashes and matching preview: **NOT RUN**
- [ ] Persistence/reopen audit reconciliation; dry-run and rejection have no persisted effect: **NOT RUN**
- [ ] Authorization/working-set/invalid-input negative continuation: **NOT RUN**
- [ ] Redaction review (no credentials, full documents, raw prompts or secret paths): **NOT RUN**
- [ ] Operator outcome (PASS / FAIL / BLOCKED), signer/reviewer and evidence link: **BLOCKED**

**Do not overwrite the NOT RUN markers until the actual evidence exists.** A green run of `cargo run -p guiyi-example-talos-playable-slice` is a regression witness only.

## After operator PASS: controlled merge

1. Attach sanitized evidence and the acceptance matrix to Issue #72 and PR #74; keep a reference to the actual **provider/model** rather than calling the fixture a live model.
2. Verify no new changes invalidated the evidence. For any behavior-affecting code change after the run, repeat affected real-model proof before acceptance.
3. Run `cargo fmt --all -- --check`, workspace check/strict clippy/tests/docs, four Python repository gates and **all four example executables** through GitHub Actions at **the exact final PR head**. See [validation record](ENG-030.md) for full commands.
4. Only with operator PASS + final green exact-head CI + review of boundaries/security/ADR may PR #74 leave draft, squash merge and Issue #72/Backlog become Done. Until then maintain Ready/Open/Draft.

## Non-goals / boundaries

The separate operator conversation is a **human verification lane**, not authorization to implement ENG-024 or any other Proposed Story. Talos owns provider/planning/turn lifecycle. GUIYI owns semantic Commands, Query, security, audit, persistence, tactical validation/build/runtime. Never create a second Editor, graph, storage or private authoring path solely to satisfy the acceptance demonstration.
