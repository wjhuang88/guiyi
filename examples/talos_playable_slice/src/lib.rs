#![forbid(unsafe_code)]

//! ENG-030 proof harness: real Talos Runtime, authoritative GUIYI semantic
//! Commands and demo-scoped operational Tools. Provider construction remains
//! with the Talos/operator embedding application.

use async_trait::async_trait;
use bevy_ecs::world::World;
use guiyi_engine_agent_host::AgentHost;
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_build::{BuildPipeline, BuildProfile};
use guiyi_engine_command::{register_builtin_document_commands, CommandExecutor, CommandRegistry};
use guiyi_engine_content::{
    ArtifactEnvelope, CompileContext, CompilerRegistry, DocumentStore, ProjectStorage,
};
use guiyi_engine_core::{deterministic_hash, DocumentId, EngineTypeId, Permission, PermissionSet};
use guiyi_engine_query::{register_builtin_queries, QueryExecutor, QueryRegistry};
use guiyi_engine_runtime::StageRuntimeManager;
use guiyi_engine_validation::{Diagnostic, DiagnosticBag};
use guiyi_talos_runtime::sdk::{
    AgentEvent, AgentTool, LanguageModel, PermissionDecision, PermissionRule, RuntimeBuilder,
    RuntimeTurnCompletionStatus, SessionEvent, ToolNature, ToolResult as TalosToolResult,
    TurnEventPayload,
};
use guiyi_talos_runtime::{GuiyiExecutionContext, GuiyiTalosAdapter, ProjectStoragePersistence};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tactical_rpg_content::{
    DialogueCompiler, DialogueDocument, EncounterCompiler, EncounterDocument, StageCompiler,
    StageDocument, ACTOR_DEFINITION_TYPE, DIALOGUE_DOCUMENT_TYPE, ENCOUNTER_DOCUMENT_TYPE,
    STAGE_DOCUMENT_TYPE,
};
use tactical_rpg_tools::{register_tactical_commands, register_tactical_queries};
use tactical_rpg_validation::{validate_dialogue, validate_encounter_project, validate_stage};

#[derive(Default)]
struct BuildCache {
    document_hash: String,
    artifacts: Vec<ArtifactEnvelope>,
}

struct OperationalContext {
    root: PathBuf,
    permissions: PermissionSet,
    working_set: Vec<DocumentId>,
    cache: Mutex<BuildCache>,
}

#[derive(Clone, Copy)]
enum Operation {
    Validate,
    Build,
    Preview,
}

struct OperationalTool {
    operation: Operation,
    shared: Arc<OperationalContext>,
}

fn result(ok: bool, output: Value, diagnostics: Value) -> TalosToolResult {
    let payload = json!({
        "status": if ok { "ok" } else { "rejected" },
        "output": output,
        "diagnostics": diagnostics,
        "transaction": null
    });
    if ok {
        TalosToolResult::success(payload.to_string())
    } else {
        TalosToolResult::error(payload.to_string())
    }
}

fn denied(code: &str, message: &str) -> TalosToolResult {
    result(
        false,
        json!({"error": {"code": code, "message": message}}),
        json!([]),
    )
}

fn project_state(root: &Path) -> Result<DocumentStore, String> {
    ProjectStoragePersistence::open(root)
        .map(|(_, state)| state.documents)
        .map_err(|error| error.to_string())
}

fn project_hash(store: &DocumentStore) -> Result<String, String> {
    serde_json::to_vec(store)
        .map(|bytes| deterministic_hash(&bytes))
        .map_err(|error| error.to_string())
}

fn validators(store: &DocumentStore) -> DiagnosticBag {
    let mut diagnostics = DiagnosticBag::default();
    for (_, document) in store.iter() {
        match document.header.type_id.as_str() {
            STAGE_DOCUMENT_TYPE => match StageDocument::from_envelope(document) {
                Ok(stage) => diagnostics.extend(validate_stage(&stage)),
                Err(error) => diagnostics.push(
                    Diagnostic::error("STAGE_PARSE_FAILED", error.to_string())
                        .at_document(document.header.id.clone()),
                ),
            },
            ENCOUNTER_DOCUMENT_TYPE => match EncounterDocument::from_envelope(document) {
                Ok(encounter) => diagnostics.extend(validate_encounter_project(&encounter, store)),
                Err(error) => diagnostics.push(
                    Diagnostic::error("ENCOUNTER_PARSE_FAILED", error.to_string())
                        .at_document(document.header.id.clone()),
                ),
            },
            DIALOGUE_DOCUMENT_TYPE => match DialogueDocument::from_envelope(document) {
                Ok(dialogue) => diagnostics.extend(validate_dialogue(&dialogue)),
                Err(error) => diagnostics.push(
                    Diagnostic::error("DIALOGUE_PARSE_FAILED", error.to_string())
                        .at_document(document.header.id.clone()),
                ),
            },
            _ => {}
        }

        for reference in &document.references {
            if store.get(&reference.target_document).is_err() {
                diagnostics.push(
                    Diagnostic::error(
                        "REFERENCE_NOT_FOUND",
                        format!(
                            "{} reference points to missing document {}",
                            reference.kind, reference.target_document
                        ),
                    )
                    .at_document(document.header.id.clone()),
                );
            }
        }
    }
    diagnostics
}

fn strict_build(
    store: &DocumentStore,
    root: &Path,
) -> Result<guiyi_engine_build::BuildReport, String> {
    let mut compilers = CompilerRegistry::default();
    compilers
        .register(StageCompiler)
        .map_err(|error| error.to_string())?;
    compilers
        .register(EncounterCompiler)
        .map_err(|error| error.to_string())?;
    compilers
        .register(DialogueCompiler)
        .map_err(|error| error.to_string())?;
    let profile = BuildProfile::default()
        .with_authoring_only_type(EngineTypeId::from_static(ACTOR_DEFINITION_TYPE));
    Ok(BuildPipeline::with_profile(compilers, profile).build(
        store,
        &CompileContext {
            project_root: root.to_path_buf(),
            profile: "development".into(),
        },
    ))
}

impl OperationalTool {
    fn needed_permission(&self) -> Permission {
        match self.operation {
            Operation::Validate => Permission::RunValidation,
            Operation::Build => Permission::RunBuild,
            Operation::Preview => Permission::RunPreview,
        }
    }

    fn execute_sync(&self, input: Value) -> TalosToolResult {
        if !self.shared.permissions.contains(self.needed_permission()) {
            return denied(
                "AGENT_PERMISSION_DENIED",
                "required GUIYI project-operation permission is missing",
            );
        }
        // These operations read whole-project content. Never run a project
        // scan from a restricted working set and expose hidden source data.
        if !self.shared.working_set.is_empty() {
            return denied(
                "AGENT_WORKING_SET_DENIED",
                "project-wide operational Tools require unrestricted document scope",
            );
        }
        let store = match project_state(&self.shared.root) {
            Ok(store) => store,
            Err(error) => return denied("DEMO_PROJECT_LOAD_FAILED", &error),
        };
        match self.operation {
            Operation::Validate => {
                if input != json!({}) {
                    return denied(
                        "DEMO_INVALID_INPUT",
                        "validation requires an empty input object",
                    );
                }
                let diagnostics = validators(&store);
                let ok = !diagnostics.has_errors();
                result(
                    ok,
                    json!({"ok": ok, "summary": diagnostics.summary()}),
                    json!(diagnostics.diagnostics),
                )
            }
            Operation::Build => {
                if input != json!({}) {
                    return denied("DEMO_INVALID_INPUT", "build requires an empty input object");
                }
                let diagnostics = validators(&store);
                if diagnostics.has_errors() {
                    if let Ok(mut cache) = self.shared.cache.lock() {
                        *cache = BuildCache::default();
                    }
                    return result(
                        false,
                        json!({
                            "ok": false,
                            "error": {"code": "DEMO_BUILD_VALIDATION_FAILED"},
                            "artifacts": []
                        }),
                        json!(diagnostics.diagnostics),
                    );
                }
                let report = match strict_build(&store, &self.shared.root) {
                    Ok(report) => report,
                    Err(error) => return denied("DEMO_BUILD_SETUP_FAILED", &error),
                };
                let ok = report.succeeded();
                let artifacts = if ok {
                    report
                        .artifacts
                        .iter()
                        .map(|artifact| {
                            json!({
                                "id": artifact.id,
                                "source_document": artifact.source_document,
                                "source_hash": artifact.source_hash,
                                "artifact_hash": artifact.artifact_hash,
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                let document_hash = match project_hash(&store) {
                    Ok(hash) => hash,
                    Err(error) => return denied("DEMO_BUILD_HASH_FAILED", &error),
                };
                let mut cache = match self.shared.cache.lock() {
                    Ok(cache) => cache,
                    Err(_) => {
                        return denied("DEMO_BUILD_CACHE_FAILED", "build cache lock poisoned")
                    }
                };
                *cache = if ok {
                    BuildCache {
                        document_hash,
                        artifacts: report.artifacts,
                    }
                } else {
                    BuildCache::default()
                };
                result(
                    ok,
                    json!({
                        "ok": ok,
                        "artifacts": artifacts,
                        "skipped_documents": report.skipped_documents
                    }),
                    json!(report.diagnostics.diagnostics),
                )
            }
            Operation::Preview => {
                let Some(stage_id) = input.get("stage_id").and_then(Value::as_str) else {
                    return denied("DEMO_INVALID_INPUT", "preview requires a stage_id string");
                };
                if input.as_object().is_none_or(|object| object.len() != 1) {
                    return denied("DEMO_INVALID_INPUT", "preview accepts only stage_id");
                }
                let document_hash = match project_hash(&store) {
                    Ok(hash) => hash,
                    Err(error) => return denied("DEMO_PREVIEW_HASH_FAILED", &error),
                };
                let artifact = {
                    let cache = match self.shared.cache.lock() {
                        Ok(cache) => cache,
                        Err(_) => {
                            return denied("DEMO_BUILD_CACHE_FAILED", "build cache lock poisoned")
                        }
                    };
                    if cache.document_hash != document_hash || cache.artifacts.is_empty() {
                        return denied("DEMO_PREVIEW_STALE_BUILD", "preview requires a successful build of current authoritative documents");
                    }
                    cache
                        .artifacts
                        .iter()
                        .find(|artifact| artifact.source_document.as_str() == stage_id)
                        .cloned()
                };
                let Some(artifact) = artifact else {
                    return denied(
                        "DEMO_PREVIEW_ARTIFACT_NOT_FOUND",
                        "the latest build has no artifact for the requested Stage",
                    );
                };
                let mut world = World::new();
                let mut manager = StageRuntimeManager::default();
                let mut entity_count = 0;
                for _ in 0..2 {
                    let instance = match manager.load(&mut world, &artifact) {
                        Ok(instance) => instance,
                        Err(error) => {
                            return denied("DEMO_PREVIEW_RUNTIME_FAILED", &error.to_string())
                        }
                    };
                    entity_count = world.entities().len();
                    if let Err(error) = manager.unload(&mut world, &instance) {
                        return denied("DEMO_PREVIEW_RUNTIME_FAILED", &error.to_string());
                    }
                    if manager.active_count() != 0 || !world.entities().is_empty() {
                        return denied(
                            "DEMO_PREVIEW_CLEANUP_FAILED",
                            "Stage ECS resources leaked across repeated previews",
                        );
                    }
                }
                result(
                    true,
                    json!({
                        "status": "ready",
                        "stage_id": stage_id,
                        "artifact_id": artifact.id,
                        "source_hash": artifact.source_hash,
                        "artifact_hash": artifact.artifact_hash,
                        "entity_count": entity_count,
                        "load_unload_cycles": 2
                    }),
                    json!([]),
                )
            }
        }
    }
}

#[async_trait]
impl AgentTool for OperationalTool {
    fn name(&self) -> &str {
        match self.operation {
            Operation::Validate => "demo.project.validate",
            Operation::Build => "demo.project.build",
            Operation::Preview => "demo.stage.preview",
        }
    }

    fn description(&self) -> &str {
        match self.operation {
            Operation::Validate => "Validate authoritative GUIYI project content and references, returning structured diagnostics.",
            Operation::Build => "Run GUIYI strict Stage, Encounter and Dialogue compilation against authoritative project content; keep ephemeral artifacts for preview.",
            Operation::Preview => "Headlessly load and unload the latest successfully built Stage artifact, rejecting stale or invalid builds.",
        }
    }

    fn parameters(&self) -> Value {
        match self.operation {
            Operation::Validate | Operation::Build => {
                json!({"type": "object", "additionalProperties": false})
            }
            Operation::Preview => json!({
                "type": "object",
                "required": ["stage_id"],
                "properties": {"stage_id": {"type": "string"}},
                "additionalProperties": false
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        matches!(self.operation, Operation::Validate)
    }

    async fn execute(&self, input: Value) -> TalosToolResult {
        self.execute_sync(input)
    }
}

fn host(store: DocumentStore) -> Result<AgentHost, String> {
    let mut commands = CommandRegistry::default();
    register_builtin_document_commands(&mut commands).map_err(|error| error.to_string())?;
    register_tactical_commands(&mut commands).map_err(|error| error.to_string())?;
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).map_err(|error| error.to_string())?;
    register_tactical_queries(&mut queries).map_err(|error| error.to_string())?;
    let catalog =
        ToolCatalog::from_registries(&commands, &queries).map_err(|error| error.to_string())?;
    Ok(AgentHost::new(
        guiyi_engine_command::EngineState { documents: store },
        CommandExecutor::new(commands),
        QueryExecutor::new(queries),
        catalog,
    ))
}

#[derive(Default)]
struct EventEvidence {
    calls: Vec<Value>,
    results: Vec<Value>,
    complete: bool,
}

impl EventEvidence {
    fn collect(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::ToolCall { call, .. } => {
                self.calls.push(json!({"id": call.id, "tool": call.name}));
            }
            AgentEvent::ToolResult { result } => {
                let value: Value = serde_json::from_str(&result.content).unwrap_or(Value::Null);
                self.results.push(json!({
                    "tool_use_id": result.tool_use_id,
                    "status": value.get("status"),
                    "diagnostics": value.get("diagnostics"),
                    "transaction": value.get("transaction"),
                    "output": value.get("output"),
                }));
            }
            _ => {}
        }
    }
}

/// Execute an actual Talos runtime turn with an externally supplied model.
/// A deterministic CI provider is allowed for regression tests only; ENG-030
/// still requires separate evidence from a real model/provider run before Done.
pub async fn run_with_provider(
    root: &Path,
    provider: Arc<dyn LanguageModel>,
    context: GuiyiExecutionContext,
) -> Result<Value, String> {
    let session_id = context.session_id.clone();
    let (_, state) = ProjectStoragePersistence::open(root).map_err(|error| error.to_string())?;
    let adapter = GuiyiTalosAdapter::with_persistence(
        host(state.documents)?,
        context.clone(),
        ProjectStoragePersistence::open(root)
            .map_err(|error| error.to_string())?
            .0,
    );
    let ops = Arc::new(OperationalContext {
        root: root.to_path_buf(),
        permissions: context.permissions,
        working_set: context.working_set,
        cache: Mutex::new(BuildCache::default()),
    });
    let mut builder = adapter.register(
        RuntimeBuilder::new()
            .provider(provider)
            .workspace_root(root)
            .permission_rule(PermissionRule::new_nature(
                ToolNature::Write,
                None,
                None,
                PermissionDecision::Allow,
            )),
    );
    for operation in [Operation::Validate, Operation::Build, Operation::Preview] {
        builder = builder.tool(Arc::new(OperationalTool {
            operation,
            shared: Arc::clone(&ops),
        }));
    }
    let mut runtime = builder.build().map_err(|error| error.to_string())?;
    runtime.submit(
        "Inspect the GUIYI project. Author a Stage, Encounter and Dialogue through semantic Tools. \
         Observe an authoritative Stage diagnostic before repair, dry-run and apply an intent repair, \
         validate the project, strictly build the tactical content, and headlessly preview the Stage. \
         Report the result only after all Tool evidence is returned.",
    ).await.map_err(|error| error.to_string())?;

    let mut evidence = EventEvidence::default();
    let mut seen_events = 0usize;
    while let Some(event) = runtime.next_event().await {
        seen_events += 1;
        if seen_events > 2000 {
            let _ = runtime.shutdown().await;
            return Err("Talos session exceeded the bounded event limit".into());
        }
        match event {
            SessionEvent::TurnEvent {
                payload: TurnEventPayload::Progress { event },
                ..
            }
            | SessionEvent::StructuredTurnEvent {
                payload: TurnEventPayload::Progress { event },
                ..
            }
            | SessionEvent::AgentEvent { event } => evidence.collect(event),
            SessionEvent::TurnEvent {
                payload: TurnEventPayload::Completed { status },
                ..
            }
            | SessionEvent::StructuredTurnEvent {
                payload: TurnEventPayload::Completed { status },
                ..
            }
            | SessionEvent::TurnCompleted { status, .. } => {
                if !matches!(status, RuntimeTurnCompletionStatus::Success { .. }) {
                    let _ = runtime.shutdown().await;
                    return Err(format!("Talos turn did not succeed: {status:?}"));
                }
                evidence.complete = true;
                break;
            }
            SessionEvent::Error { message } => {
                let _ = runtime.shutdown().await;
                return Err(format!("Talos session error: {message}"));
            }
            _ => {}
        }
    }
    runtime
        .shutdown()
        .await
        .map_err(|error| error.to_string())?;
    if !evidence.complete {
        return Err("Talos runtime ended before a successful completion event".into());
    }
    let names: BTreeSet<&str> = evidence
        .calls
        .iter()
        .filter_map(|call| call["tool"].as_str())
        .collect();
    for required in [
        "project.documents.list",
        "stage.create",
        "stage.validate",
        "tactical.make_stage_playable",
        "encounter.create",
        "encounter.add_participant",
        "dialogue.create",
        "dialogue.add_node",
        "dialogue.add_choice",
        "demo.project.validate",
        "demo.project.build",
        "demo.stage.preview",
    ] {
        if !names.contains(required) {
            return Err(format!("Talos session omitted required Tool: {required}"));
        }
    }
    let diagnostics_seen = evidence.results.iter().any(|r| {
        r["output"]["diagnostics"].as_array().is_some_and(|items| {
            items
                .iter()
                .any(|diagnostic| diagnostic["code"] == "STAGE_NO_SPAWN_POINT")
        })
    });
    if !diagnostics_seen {
        return Err("model did not observe an authentic STAGE_NO_SPAWN_POINT diagnostic".into());
    }
    for status in ["previewed", "applied"] {
        if !evidence.results.iter().any(|r| {
            r["transaction"]["command"] == "tactical.make_stage_playable"
                && r["transaction"]["status"] == status
        }) {
            return Err(format!(
                "intent repair transaction status {status} was not observed"
            ));
        }
    }
    let build_succeeded = evidence.results.iter().any(|r| {
        r["output"]["ok"] == true
            && r["output"]["artifacts"]
                .as_array()
                .is_some_and(|artifacts| artifacts.len() == 3)
    });
    let preview_succeeded = evidence
        .results
        .iter()
        .any(|r| r["output"]["status"] == "ready" && r["output"]["load_unload_cycles"] == 2);
    if !build_succeeded || !preview_succeeded {
        let observed = evidence.results.iter().map(|r| json!({
            "tool_use_id": r["tool_use_id"],
            "status": r["status"],
            "ok": r["output"]["ok"],
            "error_code": r["output"]["error"]["code"],
            "artifact_count": r["output"]["artifacts"].as_array().map(Vec::len),
            "preview_status": r["output"]["status"],
            "load_unload_cycles": r["output"]["load_unload_cycles"],
            "diagnostic_codes": r["diagnostics"].as_array().map(|items| items.iter().map(|d| d["code"].clone()).collect::<Vec<_>>())
        })).collect::<Vec<_>>();
        return Err(format!("Talos build/preview evidence incomplete: {observed:?}"));
    }
    let storage = ProjectStorage::open(root).map_err(|error| error.to_string())?;
    let audits = storage.audit_records().map_err(|error| error.to_string())?;
    let persisted = project_state(root)?;
    let ids = persisted
        .iter()
        .map(|(id, _)| id.to_string())
        .collect::<Vec<_>>();
    let audit_count = audits
        .iter()
        .filter(|record| record.session_id == session_id)
        .count();
    if audit_count == 0 {
        return Err("no durable GUIYI applied transaction audit was recorded".into());
    }
    Ok(json!({
        "status": "complete",
        "session_id": session_id,
        "model_proof": "injected-provider; independent real-provider evidence still required",
        "talos_completed": evidence.complete,
        "calls": evidence.calls,
        "results": evidence.results,
        "persisted_documents": ids,
        "durable_applied_audits": audit_count,
    }))
}
