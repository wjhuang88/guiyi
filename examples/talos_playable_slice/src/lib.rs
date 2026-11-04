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
use guiyi_engine_runtime::{StageOwned, StageRuntimeManager};
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
    // A well-formed project-operation invocation always returns a structured
    // GUIYI status. Expected validation/permission/input rejections are
    // semantic results, not Talos transport errors: the model must receive
    // the rejection and have a chance to select a corrective Tool next.
    TalosToolResult::success(payload.to_string())
}

// Packet projection is intentionally allowlisted: Tool outputs can contain
// private authored content and diagnostic messages can contain user data.
fn diagnostic_codes(value: &Value) -> Value {
    Value::Array(
        value
            .as_array()
            .into_iter()
            .flatten()
            .map(|entry| {
                json!({
                    "code": entry["code"],
                    "severity": entry["severity"],
                })
            })
            .collect(),
    )
}

fn safe_document_diffs(value: &Value) -> Value {
    Value::Array(
        value
            .as_array()
            .into_iter()
            .flatten()
            .map(|diff| {
                json!({
                    "document_id": diff["document_id"],
                    "change": diff["change"],
                    "before_hash": diff["before_hash"],
                    "after_hash": diff["after_hash"],
                })
            })
            .collect(),
    )
}

fn safe_output(tool: Option<&str>, output: &Value) -> Value {
    match tool {
        Some("stage.validate") => {
            json!({"diagnostics": diagnostic_codes(&output["diagnostics"])})
        }
        Some("tactical.make_stage_playable") => json!({
            "stage_id": output["stage_id"],
            "playable_before": output["playable_before"],
            "playable_after": output["playable_after"],
            "diagnostics_before": diagnostic_codes(&output["diagnostics_before"]),
            "diagnostics_after": diagnostic_codes(&output["diagnostics_after"]),
            "plan": output["plan"].as_array().map(|plans| plans.iter().map(|plan| {
                json!({
                    "command": plan["command"],
                    "reason_codes": plan["reason_codes"]
                })
            }).collect::<Vec<_>>()),
        }),
        Some("demo.project.validate") => json!({
            "ok": output["ok"],
            "summary": output["summary"],
            "error": {"code": output["error"]["code"]},
        }),
        Some("demo.project.build") => json!({
            "ok": output["ok"],
            "error": {"code": output["error"]["code"]},
            "skipped_documents": output["skipped_documents"],
            "artifacts": output["artifacts"].as_array().map(|artifacts| artifacts.iter().map(|artifact| {
                json!({
                    "id": artifact["id"],
                    "artifact_type": artifact["artifact_type"],
                    "source_document": artifact["source_document"],
                    "source_hash": artifact["source_hash"],
                    "artifact_hash": artifact["artifact_hash"],
                })
            }).collect::<Vec<_>>()),
        }),
        Some("demo.stage.preview") => json!({
            "status": output["status"],
            "stage_id": output["stage_id"],
            "artifact_id": output["artifact_id"],
            "source_hash": output["source_hash"],
            "artifact_hash": output["artifact_hash"],
            "entity_count": output["entity_count"],
            "load_unload_cycles": output["load_unload_cycles"],
            "error": {"code": output["error"]["code"]},
        }),
        _ => Value::Null,
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

// Reopen the persisted project and prove the semantic slice's reference
// edges survived the journal, not only its document IDs. Envelope references
// remain authoritative; this is a verification report, not a second graph.
fn persisted_slice_summary(store: &DocumentStore) -> Result<Value, String> {
    let stages: BTreeSet<String> = store
        .iter()
        .filter(|(_, document)| document.header.type_id.as_str() == STAGE_DOCUMENT_TYPE)
        .map(|(_, document)| document.header.id.to_string())
        .collect();
    let mut encounters = 0usize;
    let mut dialogues = 0usize;
    let mut reference_count = 0usize;
    let mut stage_reference_count = 0usize;
    let mut encounter_stage_edges = 0usize;
    for (_, document) in store.iter() {
        for reference in &document.references {
            if store.get(&reference.target_document).is_err() {
                return Err("reopened ProjectStorage has an unresolved semantic reference".into());
            }
            reference_count += 1;
        }
        match document.header.type_id.as_str() {
            STAGE_DOCUMENT_TYPE => stage_reference_count += document.references.len(),
            ENCOUNTER_DOCUMENT_TYPE => {
                encounters += 1;
                encounter_stage_edges += document
                    .references
                    .iter()
                    .filter(|reference| {
                        stages.contains(reference.target_document.as_str())
                            && reference.kind == "encounter_stage"
                    })
                    .count();
            }
            DIALOGUE_DOCUMENT_TYPE => dialogues += 1,
            _ => {}
        }
    }
    if stages.is_empty()
        || encounters == 0
        || dialogues == 0
        || stage_reference_count == 0
        || encounter_stage_edges == 0
    {
        return Err("reopened ProjectStorage lacks the referenced tactical slice".into());
    }
    Ok(json!({
        "stage_count": stages.len(),
        "encounter_count": encounters,
        "dialogue_count": dialogues,
        "authoritative_reference_count": reference_count,
        "encounter_stage_reference_count": encounter_stage_edges,
    }))
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
        // A newly requested build supersedes the last derived build witness.
        // No rejected or failed build may leave an earlier artifact available
        // for a subsequent preview claim, including malformed/denied calls.
        if matches!(self.operation, Operation::Build) {
            let mut cache = match self.shared.cache.lock() {
                Ok(cache) => cache,
                Err(_) => return denied("DEMO_BUILD_CACHE_FAILED", "build cache lock poisoned"),
            };
            *cache = BuildCache::default();
        }
        if !self.shared.permissions.contains(Permission::Read)
            || !self.shared.permissions.contains(self.needed_permission())
        {
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
                let required_types = [
                    "tactical.stage.artifact",
                    "tactical.encounter.artifact",
                    "tactical.dialogue.artifact",
                ];
                if ok
                    && !required_types.iter().all(|required| {
                        report
                            .artifacts
                            .iter()
                            .any(|artifact| artifact.artifact_type.as_str() == *required)
                    })
                {
                    if let Ok(mut cache) = self.shared.cache.lock() {
                        *cache = BuildCache::default();
                    }
                    return result(
                        false,
                        json!({
                            "ok": false,
                            "error": {"code": "DEMO_BUILD_INCOMPLETE_SLICE"},
                            "artifacts": [],
                        }),
                        json!(report.diagnostics.diagnostics),
                    );
                }
                let artifacts = if ok {
                    report
                        .artifacts
                        .iter()
                        .map(|artifact| {
                            json!({
                                "id": artifact.id,
                                "artifact_type": artifact.artifact_type,
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
                    // Count live owned entities, not Bevy's monotonically
                    // allocated entity slots across repeated load/unload.
                    entity_count = world.query::<&StageOwned>().iter(&world).count();
                    if let Err(error) = manager.unload(&mut world, &instance) {
                        return denied("DEMO_PREVIEW_RUNTIME_FAILED", &error.to_string());
                    }
                    let remaining_owned = world.query::<&StageOwned>().iter(&world).count();
                    if manager.active_count() != 0 || remaining_owned != 0 {
                        return denied(
                            "DEMO_PREVIEW_CLEANUP_FAILED",
                            "Stage-owned ECS resources leaked across repeated previews",
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
                let tool_name = self
                    .calls
                    .iter()
                    .find(|call| call["id"].as_str() == Some(result.tool_use_id.as_str()))
                    .and_then(|call| call["tool"].as_str());
                // Retain only inspection-safe evidence. Raw document/query payloads
                // and model transcript content are never copied into the packet.
                let safe_output = safe_output(tool_name, &value["output"]);
                let transaction = value
                    .get("transaction")
                    .filter(|item| item.is_object())
                    .map(|item| {
                        json!({
                            "transaction_id": item["transaction_id"],
                            "command": item["command"],
                            "status": item["status"],
                            "document_diffs": safe_document_diffs(&item["document_diffs"]),
                        })
                    });
                self.results.push(json!({
                    "tool_use_id": result.tool_use_id,
                    "status": value.get("status"),
                    "diagnostics": diagnostic_codes(&value["diagnostics"]),
                    "transaction": transaction,
                    "output": safe_output,
                }));
            }
            _ => {}
        }
    }
}

// Only completed results count. Linking by Tool use ID prevents unrelated
// responses from satisfying a gate; the result order must show causal progress.
fn evidence_result_index(
    evidence: &EventEvidence,
    tool: &str,
    predicate: fn(&Value) -> bool,
) -> Option<usize> {
    evidence
        .results
        .iter()
        .enumerate()
        .find_map(|(index, result)| {
            let matching_call = evidence
                .calls
                .iter()
                .any(|call| call["id"] == result["tool_use_id"] && call["tool"] == tool);
            (matching_call && result["status"] == "ok" && predicate(result)).then_some(index)
        })
}

fn verify_evidence(evidence: &EventEvidence) -> Result<(), String> {
    let diagnose = evidence_result_index(evidence, "stage.validate", |result| {
        result["output"]["diagnostics"]
            .as_array()
            .is_some_and(|items| {
                items
                    .iter()
                    .any(|entry| entry["code"] == "STAGE_NO_SPAWN_POINT")
            })
    })
    .ok_or("model did not observe an authentic STAGE_NO_SPAWN_POINT diagnostic")?;
    let dry_run = evidence_result_index(evidence, "tactical.make_stage_playable", |result| {
        result["transaction"]["status"] == "previewed" && result["output"]["playable_after"] == true
    })
    .ok_or("successful intent dry-run evidence missing")?;
    let apply = evidence_result_index(evidence, "tactical.make_stage_playable", |result| {
        result["transaction"]["status"] == "applied" && result["output"]["playable_after"] == true
    })
    .ok_or("successful applied intent evidence missing")?;
    let validate = evidence_result_index(evidence, "demo.project.validate", |result| {
        result["output"]["ok"] == true
    })
    .ok_or("successful agent-requested project validation evidence missing")?;
    let build = evidence_result_index(evidence, "demo.project.build", |result| {
        result["output"]["ok"] == true
            && result["output"]["artifacts"]
                .as_array()
                .is_some_and(|items| {
                    items.len() == 3
                        && [
                            "tactical.stage.artifact",
                            "tactical.encounter.artifact",
                            "tactical.dialogue.artifact",
                        ]
                        .iter()
                        .all(|kind| items.iter().any(|item| item["artifact_type"] == *kind))
                })
    })
    .ok_or("successful three-artifact strict build evidence missing")?;
    let preview = evidence_result_index(evidence, "demo.stage.preview", |result| {
        result["output"]["status"] == "ready"
            && result["output"]["load_unload_cycles"] == 2
            && result["output"]["entity_count"]
                .as_u64()
                .is_some_and(|count| count > 0)
    })
    .ok_or("successful two-cycle Stage preview evidence missing")?;
    if !(diagnose < dry_run
        && dry_run < apply
        && apply < validate
        && validate < build
        && build < preview)
    {
        return Err("Talos evidence is not ordered: diagnose -> dry-run -> apply -> validate -> build -> preview".into());
    }
    let build_artifacts = evidence.results[build]["output"]["artifacts"]
        .as_array()
        .ok_or("missing build artifacts")?;
    let preview_output = &evidence.results[preview]["output"];
    if !build_artifacts.iter().any(|artifact| {
        artifact["artifact_type"] == "tactical.stage.artifact"
            && artifact["id"] == preview_output["artifact_id"]
            && artifact["source_hash"] == preview_output["source_hash"]
            && artifact["artifact_hash"] == preview_output["artifact_hash"]
            && artifact["source_document"] == preview_output["stage_id"]
    }) {
        return Err("preview evidence does not match the successful Stage build artifact".into());
    }
    Ok(())
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
    let mut capability_schemas = adapter
        .agent_tools()
        .iter()
        .map(|tool| json!({"tool": tool.name(), "input_schema": tool.parameters()}))
        .collect::<Vec<_>>();
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
        let tool = Arc::new(OperationalTool {
            operation,
            shared: Arc::clone(&ops),
        });
        capability_schemas.push(json!({
            "tool": tool.name(),
            "input_schema": tool.parameters(),
        }));
        builder = builder.tool(tool);
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
    verify_evidence(&evidence)?;
    let storage = ProjectStorage::open(root).map_err(|error| error.to_string())?;
    let audits = storage.audit_records().map_err(|error| error.to_string())?;
    let persisted = project_state(root)?;
    let persistence_summary = persisted_slice_summary(&persisted)?;
    let ids = persisted
        .iter()
        .map(|(id, _)| id.to_string())
        .collect::<Vec<_>>();
    let durable_audits = audits
        .iter()
        .filter(|record| record.session_id == session_id)
        .map(|record| {
            json!({
                "sequence": record.sequence,
                "transaction_id": record.transaction_id,
                "command": record.report["command"],
                "status": record.report["status"],
                "document_diffs": safe_document_diffs(&record.report["document_diffs"]),
            })
        })
        .collect::<Vec<_>>();
    if durable_audits.is_empty() {
        return Err("no durable GUIYI applied transaction audit was recorded".into());
    }
    // Check the runtime evidence against a *freshly reopened* durable journal.
    // A dry-run must not appear as an applied mutation, while the successful
    // intent repair's exact transaction identity must appear in the journal.
    let intent_transaction = |status: &str| {
        evidence.results.iter().find_map(|result| {
            if result["transaction"]["command"] == "tactical.make_stage_playable"
                && result["transaction"]["status"] == status
            {
                result["transaction"]["transaction_id"].as_str()
            } else {
                None
            }
        })
    };
    let dry_run_id =
        intent_transaction("previewed").ok_or("missing dry-run transaction identity")?;
    let applied_id = intent_transaction("applied").ok_or("missing applied transaction identity")?;
    if durable_audits
        .iter()
        .any(|record| record["transaction_id"] == dry_run_id)
    {
        return Err("GUIYI persisted a dry-run transaction in the applied audit".into());
    }
    if !durable_audits.iter().any(|record| {
        record["transaction_id"] == applied_id
            && record["command"] == "tactical.make_stage_playable"
            && record["status"] == "applied"
    }) {
        return Err("the applied intent transaction is missing from durable GUIYI audit".into());
    }
    if validators(&persisted).has_errors() {
        return Err("persisted GUIYI project is invalid after reopening storage".into());
    }
    Ok(json!({
        "status": "complete",
        "session_id": session_id,
        "model_proof": "injected-provider; independent real-provider evidence still required",
        "talos_completed": evidence.complete,
        "capability_schemas": capability_schemas,
        "calls": evidence.calls,
        "results": evidence.results,
        "persisted_documents": ids,
        "persistence_summary": persistence_summary,
        "durable_applied_audits": durable_audits,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_content::{ProjectManifest, ProjectPath, ProjectTransaction};
    use guiyi_engine_core::{AgentSessionId, EngineVersion, ObjectId, ProjectId};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tactical_rpg_content::{HexCoord, StageObject, StageObjectKind};

    fn shared(
        root: &Path,
        permissions: PermissionSet,
        working_set: Vec<DocumentId>,
    ) -> Arc<OperationalContext> {
        Arc::new(OperationalContext {
            root: root.to_path_buf(),
            permissions,
            working_set,
            cache: Mutex::new(BuildCache::default()),
        })
    }

    fn invoke(shared: &Arc<OperationalContext>, operation: Operation, input: Value) -> Value {
        let tool = OperationalTool {
            operation,
            shared: Arc::clone(shared),
        };
        serde_json::from_str(&tool.execute_sync(input).content).unwrap()
    }

    fn make_project(stage: Option<StageDocument>) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "guiyi-eng030-negative-{}-{nonce}",
            std::process::id()
        ));
        let storage = ProjectStorage::create(&root).unwrap();
        let mut manifest = ProjectManifest {
            project_id: ProjectId::from_static("project.eng030-negative"),
            name: "ENG-030 negative cases".into(),
            engine_api_version: EngineVersion::CURRENT.to_string(),
            content_schema_version: 1,
            enabled_extensions: vec!["tactical_rpg".into()],
            documents: Vec::new(),
        };
        let mut transaction = ProjectTransaction::generated(
            "eng030-negative",
            AgentSessionId::from_static("session.eng030-fixture"),
            "fixture.create",
            json!({"kind": "immutable_test_fixture"}),
        )
        .unwrap();
        if let Some(stage) = stage {
            let path = ProjectPath::new("content/stages/eng030.stage.json").unwrap();
            transaction
                .write_json(path.clone(), &stage.to_envelope().unwrap())
                .unwrap();
            manifest.documents.push(path);
        }
        transaction
            .write_manifest_json(ProjectPath::new("engine-project.json").unwrap(), &manifest)
            .unwrap();
        storage.commit(transaction).unwrap();
        root
    }

    fn stage_with_spawn(position: HexCoord) -> StageDocument {
        let mut stage =
            StageDocument::new_hex(DocumentId::from_static("stage.eng030"), "ENG-030", 4, 4);
        stage.objects.push(StageObject {
            id: ObjectId::from_static("spawn.player"),
            position,
            object: StageObjectKind::SpawnPoint {
                profile: "player".into(),
            },
            properties: json!({}),
        });
        stage
    }

    #[test]
    fn review_packet_drops_private_document_and_diagnostic_fields() {
        let source = json!({
            "diagnostics": [{
                "code": "STAGE_NO_SPAWN_POINT",
                "severity": "warning",
                "message": "secret user text",
                "location": {"field_path": "/private/path"}
            }],
            "private_document": "not-for-export",
        });
        let projected = safe_output(Some("stage.validate"), &source);
        assert_eq!(projected["diagnostics"][0]["code"], "STAGE_NO_SPAWN_POINT");
        assert!(!projected.to_string().contains("secret"));
        assert!(!projected.to_string().contains("/private/path"));
        let diffs = safe_document_diffs(&json!([{
            "document_id": "stage.test",
            "change": "modified",
            "after_hash": "hash-only",
            "payload": "top secret"
        }]));
        assert_eq!(diffs[0]["after_hash"], "hash-only");
        assert!(!diffs.to_string().contains("top secret"));
        let intent = safe_output(
            Some("tactical.make_stage_playable"),
            &json!({
                "stage_id": "stage.test",
                "playable_before": false,
                "playable_after": true,
                "plan": [{
                    "command": "stage.create_spawn",
                    "reason_codes": ["STAGE_NO_SPAWN_POINT"],
                    "input": {"private": "secret authoring text"}
                }]
            }),
        );
        assert_eq!(intent["plan"][0]["command"], "stage.create_spawn");
        assert!(!intent.to_string().contains("secret authoring text"));
        assert_eq!(
            safe_output(Some("project.documents.list"), &source),
            Value::Null
        );
    }

    fn ordered_fixture_evidence() -> EventEvidence {
        let artifacts = json!([
            {"artifact_type": "tactical.stage.artifact", "id": "artifact.stage", "source_document": "stage.test", "source_hash": "source", "artifact_hash": "compiled"},
            {"artifact_type": "tactical.encounter.artifact", "id": "artifact.encounter"},
            {"artifact_type": "tactical.dialogue.artifact", "id": "artifact.dialogue"}
        ]);
        let steps = [
            (
                "stage.validate",
                json!({"diagnostics": [{"code": "STAGE_NO_SPAWN_POINT"}]}),
                Value::Null,
            ),
            (
                "tactical.make_stage_playable",
                json!({"playable_after": true}),
                json!({"status": "previewed"}),
            ),
            (
                "tactical.make_stage_playable",
                json!({"playable_after": true}),
                json!({"status": "applied"}),
            ),
            ("demo.project.validate", json!({"ok": true}), Value::Null),
            (
                "demo.project.build",
                json!({"ok": true, "artifacts": artifacts}),
                Value::Null,
            ),
            (
                "demo.stage.preview",
                json!({"status": "ready", "stage_id": "stage.test", "artifact_id": "artifact.stage", "source_hash": "source", "artifact_hash": "compiled", "load_unload_cycles": 2, "entity_count": 1}),
                Value::Null,
            ),
        ];
        let mut evidence = EventEvidence::default();
        for (index, (name, output, transaction)) in steps.into_iter().enumerate() {
            let id = format!("call-{index}");
            evidence.calls.push(json!({"id": id, "tool": name}));
            evidence.results.push(json!({
                "tool_use_id": id, "status": "ok", "output": output, "transaction": transaction
            }));
        }
        evidence
    }

    #[test]
    fn evidence_requires_causal_tool_order_and_matching_preview_artifact() {
        let mut evidence = ordered_fixture_evidence();
        assert!(verify_evidence(&evidence).is_ok());
        evidence.results.swap(0, 1);
        assert!(verify_evidence(&evidence)
            .unwrap_err()
            .contains("not ordered"));
        evidence.results.swap(0, 1);
        evidence.results[5]["output"]["artifact_hash"] = json!("different");
        assert!(verify_evidence(&evidence)
            .unwrap_err()
            .contains("does not match"));
    }

    #[test]
    fn operational_permissions_and_working_set_are_not_bypassed() {
        let root = Path::new("not-a-real-eng030-project");
        for operation in [Operation::Validate, Operation::Build, Operation::Preview] {
            let input = if matches!(operation, Operation::Preview) {
                json!({"stage_id": "stage.eng030"})
            } else {
                json!({})
            };
            let denied = invoke(
                &shared(root, PermissionSet::read_only(), Vec::new()),
                operation,
                input.clone(),
            );
            assert_eq!(denied["output"]["error"]["code"], "AGENT_PERMISSION_DENIED");

            let restricted = invoke(
                &shared(
                    root,
                    PermissionSet::content_author(),
                    vec![DocumentId::from_static("stage.visible")],
                ),
                operation,
                input,
            );
            assert_eq!(
                restricted["output"]["error"]["code"],
                "AGENT_WORKING_SET_DENIED"
            );
        }
        let validation_only = invoke(
            &shared(
                root,
                PermissionSet::new([Permission::RunValidation]),
                Vec::new(),
            ),
            Operation::Validate,
            json!({}),
        );
        assert_eq!(
            validation_only["output"]["error"]["code"],
            "AGENT_PERMISSION_DENIED"
        );
    }

    #[test]
    fn empty_slice_and_preview_before_build_fail_closed() {
        let root = make_project(None);
        let shared = shared(&root, PermissionSet::content_author(), Vec::new());

        let malformed = invoke(&shared, Operation::Validate, json!({"unexpected": true}));
        assert_eq!(malformed["output"]["error"]["code"], "DEMO_INVALID_INPUT");

        let incomplete = invoke(&shared, Operation::Build, json!({}));
        assert_eq!(
            incomplete["output"]["error"]["code"],
            "DEMO_BUILD_INCOMPLETE_SLICE"
        );
        assert_eq!(incomplete["output"]["artifacts"], json!([]));

        let preview = invoke(
            &shared,
            Operation::Preview,
            json!({"stage_id": "stage.eng030"}),
        );
        assert_eq!(
            preview["output"]["error"]["code"],
            "DEMO_PREVIEW_STALE_BUILD"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_stage_produces_no_partial_artifacts_or_preview() {
        let root = make_project(Some(stage_with_spawn(HexCoord::new(99, 99))));
        let shared = shared(&root, PermissionSet::content_author(), Vec::new());

        let failed_build = invoke(&shared, Operation::Build, json!({}));
        assert_eq!(
            failed_build["output"]["error"]["code"],
            "DEMO_BUILD_VALIDATION_FAILED"
        );
        assert_eq!(failed_build["output"]["artifacts"], json!([]));
        assert!(failed_build["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item["code"] == "STAGE_OBJECT_OUT_OF_BOUNDS" }));

        let preview = invoke(
            &shared,
            Operation::Preview,
            json!({"stage_id": "stage.eng030"}),
        );
        assert_eq!(
            preview["output"]["error"]["code"],
            "DEMO_PREVIEW_STALE_BUILD"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reopened_slice_requires_semantic_stage_and_encounter_references() {
        let root = make_project(Some(stage_with_spawn(HexCoord::new(0, 0))));
        let store = project_state(&root).unwrap();
        assert!(persisted_slice_summary(&store)
            .unwrap_err()
            .contains("lacks the referenced tactical slice"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejected_build_invalidates_previous_derived_preview() {
        let root = make_project(Some(stage_with_spawn(HexCoord::new(0, 0))));
        let store = project_state(&root).unwrap();
        let report = strict_build(&store, &root).unwrap();
        assert!(report.succeeded());
        let shared = shared(&root, PermissionSet::content_author(), Vec::new());
        *shared.cache.lock().unwrap() = BuildCache {
            document_hash: project_hash(&store).unwrap(),
            artifacts: report.artifacts,
        };
        // A previous successful build is previewable, but an attempted
        // malformed build must revoke it rather than reuse stale success.
        let preview_input = json!({"stage_id": "stage.eng030"});
        assert_eq!(
            invoke(&shared, Operation::Preview, preview_input.clone())["status"],
            "ok"
        );
        let invalid = invoke(&shared, Operation::Build, json!({"unexpected": true}));
        assert_eq!(invalid["output"]["error"]["code"], "DEMO_INVALID_INPUT");
        let preview = invoke(&shared, Operation::Preview, preview_input);
        assert_eq!(
            preview["output"]["error"]["code"],
            "DEMO_PREVIEW_STALE_BUILD"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_compiled_artifact_is_rejected_by_existing_runtime_integrity_gate() {
        let root = make_project(Some(stage_with_spawn(HexCoord::new(0, 0))));
        let store = project_state(&root).unwrap();
        let report = strict_build(&store, &root).unwrap();
        assert!(report.succeeded());
        let mut artifact = report.artifacts.into_iter().next().unwrap();
        artifact.artifact_hash = "corrupt".into();

        let shared = shared(&root, PermissionSet::content_author(), Vec::new());
        *shared.cache.lock().unwrap() = BuildCache {
            document_hash: project_hash(&store).unwrap(),
            artifacts: vec![artifact],
        };
        let preview = invoke(
            &shared,
            Operation::Preview,
            json!({"stage_id": "stage.eng030"}),
        );
        assert_eq!(
            preview["output"]["error"]["code"],
            "DEMO_PREVIEW_RUNTIME_FAILED"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
