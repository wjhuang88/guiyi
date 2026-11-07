# ENG-030 Talos playable-slice proof harness

This example exercises the **real `talos-runtime = 0.10.1` RuntimeBuilder / RuntimeHandle and the existing GUIYI live Tool adapter**. It does not introduce a GUIYI-owned Agent loop or model provider.

## Deterministic CI witness

Run from the workspace root:

```bash
cargo run -p guiyi-example-talos-playable-slice
```

The binary creates an isolated sandboxed `ProjectStorage` fixture containing only a reusable actor-definition document, then drives a Talos SDK session with a **deterministic, result-aware fixture `LanguageModel`**. It requires real GUIYI Tool feedback before selecting a missing-spawn repair, invokes `tactical.make_stage_playable` in dry-run and apply modes, authors Stage / Encounter / Dialogue through semantic Commands, then calls three separately registered typed operational Tools:

- `demo.project.validate` — delegates to existing GUIYI tactical validation and reference checks; requires `Read` and `RunValidation`.
- `demo.project.build` — delegates to the strict GUIYI `BuildPipeline`; requires `Read` and `RunBuild`, no partial artifacts, and all three required tactical artifact types.
- `demo.stage.preview` — loads the matching current-state Stage artifact via `StageRuntimeManager`, twice, then checks StageOwned cleanup; requires `Read` and `RunPreview`.

The three Tools are **demo-scoped Talos `AgentTool` adapters**, not new core GUIYI Tool IDs, generic Tool kinds, or another persistence/graph truth. They reread the authoritative `ProjectStorage` state; a restricted working set is denied before any project-wide scan. A latest-build hash guards against previewing stale artifacts. Existing GUIYI AgentHost/CommandExecutor retains authoring, schema validation, permissions, transaction, audit, persistence and graph ownership.

The executable prints a derived review packet with live Tool schema metadata, observed Tool IDs, safe Tool result/diagnostic summaries, intent dry-run/applied transaction diffs, build artifact IDs/hashes, preview result, persisted document IDs and redacted durable audit entries. Raw full-document Query results, model credentials and private session transcript text are not copied into the packet. The packet is **not** an authoritative replacement for `ProjectStorage` audit records.

The full workspace tests also exercise missing permissions, non-empty working sets, invalid operation input, incomplete/invalid build, zero partial artifacts, missing current build, corrupted artifact hashes, and diagnostic-code-dependent fixture decisions.

Additional ENG-030 CI assertions cover **rejection continuation** and **restart semantics**. The deterministic Talos session deliberately requests `demo.project.validate` with an invalid parameter, receives `status: rejected` and `DEMO_INVALID_INPUT`, and then continues to valid semantic authoring and successful build/preview. Expected domain/authorization rejections from the **demo-scoped** Tools are returned as machine-readable Tool data so Talos can consume them; they remain rejections, never authorization successes. Genuine Talos transport errors are distinct and may end a turn. A newly requested build invalidates the previous derived build cache on **any** rejected/failed attempt; preview then fails closed instead of reusing its earlier success. After reopening ProjectStorage, the harness checks Stage/Encounter/Dialogue presence, Stage-owned external references, Encounter→Stage reference edges and all document-reference targets; it exports only aggregate counts as `persistence_summary`.


## Real-provider product acceptance — **HUMAN REQUIRED / BLOCKED**

**Operator handoff:** [ENG-030-MANUAL-ACCEPTANCE.md](../../docs/validation/ENG-030-MANUAL-ACCEPTANCE.md) contains the separate-session checklist, required evidence, pass/fail matrix, redaction rules and the post-acceptance merge gate. The model-led run has **not** been executed; the fixture is not a replacement.

`guiyi-example-talos-playable-slice` exports `run_with_provider(root, provider, context)`. (Rust crate name: `guiyi_example_talos_playable_slice`.) A separate **operator-owned** Talos embedding application must supply a genuine `Arc<dyn guiyi_talos_runtime::sdk::LanguageModel>` and an already initialized GUIYI project root. Construct/configure that provider and its credentials using Talos/operator infrastructure **outside GUIYI**. Do not implement another model client in engine/toolkit crates.

An embedding calls the public function approximately as follows, with `provider` and `project_root` supplied by the operator:

```rust
use guiyi_engine_core::{AgentSessionId, PermissionSet};
use guiyi_talos_runtime::GuiyiExecutionContext;
use guiyi_example_talos_playable_slice::run_with_provider;

// provider: Arc<dyn guiyi_talos_runtime::sdk::LanguageModel>
// project_root: &std::path::Path
let context = GuiyiExecutionContext::new(
    AgentSessionId::from_static("session.talos.real-proof"),
    PermissionSet::content_author(),
);
let review_packet = run_with_provider(project_root, provider, context).await?;
```

For a real project, make sure the authoritative manifest and any required immutable actor definitions exist before the session; normal Stage/Encounter/Dialogue authoring must occur through the live semantic Commands. This example's short task prompt describes the goal but **does not pre-select the model's Tool sequence**. Project-wide validation/build/preview require unrestricted document scope and the distinct permissions above. The agent must observe a real GUIYI diagnostic, review dry-run evidence and apply a semantic repair before invoking the operational Tools.

**Passing the deterministic CI binary is not evidence of a genuine model-driven product session.** ENG-030 / Issue #72 requires a separately executed, sanitized real-provider Talos session record including successful tool choice, diagnosed defect, dry-run/apply transactions, strict artifacts, preview and audit. Until that is obtained, the Story remains open and PR #74 must stay unmerged.

## Migration and rollback

This example uses only the existing Talos SDK dependency through `guiyi-talos-runtime::sdk` and existing engine/toolkit crates. There is no Stage/Encounter/Dialogue serialization migration or engine-core graph/persistence change. Rollback removes the example, demo operational adapters, new CI command, SDK re-export, tests and documentation; ENG-014 through ENG-023 remain intact.
