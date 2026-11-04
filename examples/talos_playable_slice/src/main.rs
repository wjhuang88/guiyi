#![forbid(unsafe_code)]

//! Deterministic offline CI witness for the ENG-030 integration.
//! This fixture provider is NOT a substitute for the required live-model proof.

use async_trait::async_trait;
use guiyi_engine_content::{
    DocumentEnvelope, DocumentHeader, ProjectManifest, ProjectPath, ProjectStorage,
    ProjectTransaction,
};
use guiyi_engine_core::{
    AgentSessionId, DocumentId, EngineTypeId, EngineVersion, PermissionSet, ProjectId,
};
#[path = "lib.rs"]
mod proof;
use self::proof::run_with_provider;
use guiyi_talos_runtime::sdk::{
    AgentEvent, LanguageModel, Message, ProviderResult, Receiver, StopReason,
    ToolCall as TalosToolCall, ToolDefinition, Usage,
};
use guiyi_talos_runtime::GuiyiExecutionContext;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tactical_rpg_content::ACTOR_DEFINITION_TYPE;

#[derive(Default)]
struct FixtureProvider {
    observed_tool_names: Mutex<Vec<String>>,
}

enum Next {
    Call {
        id: &'static str,
        name: &'static str,
        input: Value,
    },
    Text,
}

impl FixtureProvider {
    fn requires_spawn_repair(diagnostics: &Value) -> bool {
        diagnostics.as_array().is_some_and(|items| {
            items
                .iter()
                .any(|diagnostic| diagnostic["code"] == "STAGE_NO_SPAWN_POINT")
        })
    }

    fn choose(&self, messages: &[Message], tools: &[ToolDefinition]) -> Next {
        if let Ok(mut names) = self.observed_tool_names.lock() {
            *names = tools.iter().map(|tool| tool.name.clone()).collect();
        }
        let mut results = BTreeMap::<String, Value>::new();
        for message in messages {
            if let Message::Tool { result } = message {
                if let Ok(value) = serde_json::from_str::<Value>(&result.content) {
                    results.insert(result.tool_use_id.clone(), value);
                }
            }
        }
        let absent = |name: &str| !results.contains_key(name);
        // The fixture examines real returned JSON. It never unconditionally
        // chooses a repair unless the validator explicitly finds that defect.
        if absent("invalid-input") {
            return Next::Call {
                id: "invalid-input",
                name: "demo.project.validate",
                input: json!({"unexpected": true}),
            };
        }
        // Talos must deliver the typed error to the model, then allow it
        // to continue with valid GUIYI Tools instead of ending the turn.
        if results["invalid-input"]["status"] != "rejected"
            || results["invalid-input"]["output"]["error"]["code"] != "DEMO_INVALID_INPUT"
        {
            return Next::Text;
        }
        if absent("inspect") {
            return Next::Call {
                id: "inspect",
                name: "project.documents.list",
                input: json!({}),
            };
        }
        if absent("stage") {
            return Next::Call {
                id: "stage",
                name: "stage.create",
                input: json!({"input": {"id": "stage.demo", "name": "Talos Playable Slice", "width": 8, "height": 8}}),
            };
        }
        if absent("diagnose") {
            return Next::Call {
                id: "diagnose",
                name: "stage.validate",
                input: json!({"stage_id": "stage.demo"}),
            };
        }
        let no_spawn = Self::requires_spawn_repair(&results["diagnose"]["output"]["diagnostics"]);
        if no_spawn && absent("repair-preview") {
            return Next::Call {
                id: "repair-preview",
                name: "tactical.make_stage_playable",
                input: json!({"input": {"stage_id": "stage.demo"}, "dry_run": true}),
            };
        }
        if no_spawn && absent("repair-apply") {
            if results["repair-preview"]["transaction"]["status"] != "previewed" {
                return Next::Text;
            }
            return Next::Call {
                id: "repair-apply",
                name: "tactical.make_stage_playable",
                input: json!({"input": {"stage_id": "stage.demo"}, "dry_run": false}),
            };
        }
        if absent("actor") {
            return Next::Call {
                id: "actor",
                name: "stage.place_actor",
                input: json!({"input": {"stage_id": "stage.demo", "object_id": "actor.guard", "definition": "actor.guard.basic", "q": 4, "r": 3}}),
            };
        }
        if absent("encounter") {
            return Next::Call {
                id: "encounter",
                name: "encounter.create",
                input: json!({"input": {"encounter_id": "encounter.demo", "name": "Talos Encounter", "stage_id": "stage.demo"}}),
            };
        }
        if absent("participant") {
            return Next::Call {
                id: "participant",
                name: "encounter.add_participant",
                input: json!({"input": {
                    "encounter_id": "encounter.demo", "stage_id": "stage.demo",
                    "participant_id": "participant.guard", "actor_definition": "actor.guard.basic",
                    "spawn_point": "spawn.player", "side": "enemy"
                }}),
            };
        }
        if absent("dialogue") {
            return Next::Call {
                id: "dialogue",
                name: "dialogue.create",
                input: json!({"input": {
                    "dialogue_id": "dialogue.demo", "name": "Gate Dialogue",
                    "entry_node_id": "node.entry", "speaker": "Guard", "text": "Halt!"
                }}),
            };
        }
        if absent("node") {
            return Next::Call {
                id: "node",
                name: "dialogue.add_node",
                input: json!({"input": {
                    "dialogue_id": "dialogue.demo", "node_id": "node.end",
                    "speaker": "Guard", "text": "You may enter."
                }}),
            };
        }
        if absent("choice") {
            return Next::Call {
                id: "choice",
                name: "dialogue.add_choice",
                input: json!({"input": {
                    "dialogue_id": "dialogue.demo", "from_node_id": "node.entry",
                    "choice_id": "choice.enter", "text": "Let us pass.",
                    "target_node_id": "node.end", "conditions": [{"type": "always"}],
                    "effects": [{"type": "emit_signal", "signal": "gate.open"}]
                }}),
            };
        }
        if absent("validate") {
            return Next::Call {
                id: "validate",
                name: "demo.project.validate",
                input: json!({}),
            };
        }
        if absent("build") {
            if results["validate"]["output"]["ok"] != true {
                return Next::Text;
            }
            return Next::Call {
                id: "build",
                name: "demo.project.build",
                input: json!({}),
            };
        }
        if absent("preview") {
            if results["build"]["output"]["ok"] != true {
                return Next::Text;
            }
            return Next::Call {
                id: "preview",
                name: "demo.stage.preview",
                input: json!({"stage_id": "stage.demo"}),
            };
        }
        Next::Text
    }

    async fn emit(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> ProviderResult<Receiver<AgentEvent>> {
        let next = self.choose(messages, tools);
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        tokio::spawn(async move {
            let _ = tx.send(AgentEvent::TurnStart).await;
            match next {
                Next::Call { id, name, input } => {
                    let _ = tx
                        .send(AgentEvent::ToolCall {
                            call: TalosToolCall {
                                id: id.into(),
                                name: name.into(),
                                input,
                            },
                            provenance: Default::default(),
                            summary_fields: Vec::new(),
                        })
                        .await;
                    let _ = tx
                        .send(AgentEvent::TurnEnd {
                            stop_reason: StopReason::ToolUse,
                            usage: Usage::default(),
                        })
                        .await;
                }
                Next::Text => {
                    let _ = tx
                        .send(AgentEvent::TextDelta {
                            delta: "The available GUIYI evidence has been collected.".into(),
                        })
                        .await;
                    let _ = tx
                        .send(AgentEvent::TurnEnd {
                            stop_reason: StopReason::EndTurn,
                            usage: Usage::default(),
                        })
                        .await;
                }
            }
        });
        Ok(rx)
    }
}

#[async_trait]
impl LanguageModel for FixtureProvider {
    async fn stream(&self, messages: &[Message]) -> ProviderResult<Receiver<AgentEvent>> {
        self.emit(messages, &[]).await
    }

    async fn stream_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> ProviderResult<Receiver<AgentEvent>> {
        self.emit(messages, tools).await
    }
}

fn fixture() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!(
        "guiyi-eng030-fixture-{}-{nonce}",
        std::process::id()
    ));
    let storage = ProjectStorage::create(&root)?;
    let actor_path = ProjectPath::new("content/definitions/guard.json")?;
    let manifest = ProjectManifest {
        project_id: ProjectId::from_static("project.talos-slice"),
        name: "Talos tactical slice".into(),
        engine_api_version: EngineVersion::CURRENT.to_string(),
        content_schema_version: 1,
        enabled_extensions: vec!["tactical_rpg".into()],
        documents: vec![actor_path.clone()],
    };
    let actor = DocumentEnvelope {
        header: DocumentHeader {
            id: DocumentId::from_static("actor.guard.basic"),
            type_id: EngineTypeId::from_static(ACTOR_DEFINITION_TYPE),
            schema_version: 1,
            display_name: "Guard".into(),
        },
        references: Vec::new(),
        payload: json!({"health": 10, "movement": 4}),
    };
    let mut transaction = ProjectTransaction::generated(
        "eng030-fixture",
        AgentSessionId::from_static("session.fixture"),
        "fixture.create",
        json!({"kind": "immutable_fixture"}),
    )?;
    transaction.write_json(actor_path, &actor)?;
    transaction.write_manifest_json(ProjectPath::new("engine-project.json")?, &manifest)?;
    storage.commit(transaction)?;
    Ok(root)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture()?;
    let provider = Arc::new(FixtureProvider::default());
    let context = GuiyiExecutionContext::new(
        AgentSessionId::from_static("session.talos.eng030-ci"),
        PermissionSet::content_author(),
    );
    let packet = run_with_provider(&root, provider.clone(), context).await;
    let _ = std::fs::remove_dir_all(&root);
    let packet = packet?;
    let tools = provider
        .observed_tool_names
        .lock()
        .map_err(|_| "provider lock poisoned")?;
    assert!(tools
        .iter()
        .any(|name| name == "tactical.make_stage_playable"));
    assert!(tools.iter().any(|name| name == "demo.project.build"));
    let results = packet["results"]
        .as_array()
        .ok_or("missing structured Talos Tool results")?;
    let rejection = results
        .iter()
        .position(|entry| {
            entry["tool_use_id"] == "invalid-input"
                && entry["status"] == "rejected"
                && entry["output"]["error"]["code"] == "DEMO_INVALID_INPUT"
        })
        .ok_or("Talos did not observe the intentional typed Tool rejection")?;
    let subsequent_stage = results
        .iter()
        .position(|entry| entry["tool_use_id"] == "stage" && entry["status"] == "ok")
        .ok_or("Talos did not continue with semantic authoring after rejection")?;
    assert!(rejection < subsequent_stage);
    println!("{}", serde_json::to_string_pretty(&packet)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_provider_chooses_repair_from_actual_diagnostic_code() {
        assert!(FixtureProvider::requires_spawn_repair(&json!([
            {"code": "STAGE_NO_SPAWN_POINT", "severity": "warning"}
        ])));
        assert!(!FixtureProvider::requires_spawn_repair(&json!([])));
        assert!(!FixtureProvider::requires_spawn_repair(&json!([
            {"code": "STAGE_NAV_UNSUPPORTED_COORDINATE_SPACE", "severity": "error"}
        ])));
        assert!(!FixtureProvider::requires_spawn_repair(&Value::Null));
    }
}
