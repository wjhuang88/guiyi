#![forbid(unsafe_code)]

//! High-level tactical RPG commands and queries for AI agents and human clients.

use guiyi_engine_command::{
    CommandDescriptor, CommandError, CommandHandler, CommandRegistry, EngineState,
};
use guiyi_engine_content::DocumentStore;
use guiyi_engine_core::{DocumentId, ObjectId, Permission, PermissionSet, ToolId};
use guiyi_engine_query::{QueryDescriptor, QueryError, QueryHandler, QueryRegistry};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag};
use serde::Deserialize;
use serde_json::{json, Value};
use tactical_rpg_content::{
    HexCoord, StageConnection, StageDocument, StageObject, StageObjectKind, STAGE_DOCUMENT_TYPE,
};
use tactical_rpg_validation::validate_stage;

#[derive(Debug, Deserialize)]
struct CreateStageInput {
    id: DocumentId,
    name: String,
    width: u32,
    height: u32,
}

pub struct CreateStageCommand;

impl CommandHandler for CreateStageCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("stage.create"),
            title: "Create tactical Stage".into(),
            description: "Create a hex-grid Stage authoring document.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["id", "name", "width", "height"],
                "properties": {
                    "id": {"type": "string"},
                    "name": {"type": "string", "minLength": 1},
                    "width": {"type": "integer", "minimum": 1},
                    "height": {"type": "integer", "minimum": 1}
                }
            }),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["creates_document".into()],
            related_tools: vec![ToolId::from_static("stage.place_actor")],
        }
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let mut bag = DiagnosticBag::default();
        match serde_json::from_value::<CreateStageInput>(input.clone()) {
            Ok(parsed) => {
                if parsed.width == 0 || parsed.height == 0 {
                    bag.push(Diagnostic::error(
                        "STAGE_DIMENSIONS_INVALID",
                        "Stage dimensions must be greater than zero",
                    ));
                }
                if state.documents.get(&parsed.id).is_ok() {
                    bag.push(
                        Diagnostic::error("DOCUMENT_ALREADY_EXISTS", "Stage id already exists")
                            .at_document(parsed.id),
                    );
                }
            }
            Err(error) => bag.push(Diagnostic::error("COMMAND_INPUT_INVALID", error.to_string())),
        }
        bag
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: CreateStageInput = serde_json::from_value(input.clone())?;
        let stage = StageDocument::new_hex(input.id.clone(), input.name, input.width, input.height);
        state.documents.insert(stage.to_envelope()?)?;
        Ok(json!({"stage_id": input.id}))
    }
}

#[derive(Debug, Deserialize)]
struct PlaceActorInput {
    stage_id: DocumentId,
    object_id: ObjectId,
    definition: DocumentId,
    q: i32,
    r: i32,
    #[serde(default)]
    properties: Value,
}

pub struct PlaceActorCommand;

impl CommandHandler for PlaceActorCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("stage.place_actor"),
            title: "Place actor".into(),
            description: "Place an actor definition instance into a tactical Stage.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["stage_id", "object_id", "definition", "q", "r"],
                "properties": {
                    "stage_id": {"type": "string"},
                    "object_id": {"type": "string"},
                    "definition": {"type": "string"},
                    "q": {"type": "integer"},
                    "r": {"type": "integer"},
                    "properties": {"type": "object"}
                }
            }),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into(), "creates_reference".into()],
            related_tools: vec![ToolId::from_static("stage.validate")],
        }
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: PlaceActorInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.stage_id)?.clone();
        let mut stage = StageDocument::from_envelope(&envelope)?;
        ensure_unique_object(&stage, &input.object_id)?;
        stage.objects.push(StageObject {
            id: input.object_id.clone(),
            position: HexCoord::new(input.q, input.r),
            object: StageObjectKind::Actor {
                definition: input.definition,
            },
            properties: input.properties,
        });
        let diagnostics = validate_stage(&stage);
        if diagnostics.has_errors() {
            return Err(CommandError::ValidationFailed(diagnostics));
        }
        state.documents.upsert(stage.to_envelope()?);
        Ok(json!({"stage_id": input.stage_id, "object_id": input.object_id}))
    }
}

#[derive(Debug, Deserialize)]
struct CreateSpawnInput {
    stage_id: DocumentId,
    object_id: ObjectId,
    profile: String,
    q: i32,
    r: i32,
}

pub struct CreateSpawnCommand;

impl CommandHandler for CreateSpawnCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("stage.create_spawn"),
            title: "Create spawn point".into(),
            description: "Create a named Stage spawn point.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["stage_id", "object_id", "profile", "q", "r"],
                "properties": {
                    "stage_id": {"type": "string"},
                    "object_id": {"type": "string"},
                    "profile": {"type": "string"},
                    "q": {"type": "integer"},
                    "r": {"type": "integer"}
                }
            }),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into()],
            related_tools: vec![ToolId::from_static("stage.validate")],
        }
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: CreateSpawnInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.stage_id)?.clone();
        let mut stage = StageDocument::from_envelope(&envelope)?;
        ensure_unique_object(&stage, &input.object_id)?;
        stage.objects.push(StageObject {
            id: input.object_id.clone(),
            position: HexCoord::new(input.q, input.r),
            object: StageObjectKind::SpawnPoint {
                profile: input.profile,
            },
            properties: json!({}),
        });
        let diagnostics = validate_stage(&stage);
        if diagnostics.has_errors() {
            return Err(CommandError::ValidationFailed(diagnostics));
        }
        state.documents.upsert(stage.to_envelope()?);
        Ok(json!({"stage_id": input.stage_id, "object_id": input.object_id}))
    }
}

#[derive(Debug, Deserialize)]
struct CreateTriggerInput {
    stage_id: DocumentId,
    object_id: ObjectId,
    activation: String,
    q: i32,
    r: i32,
    #[serde(default)]
    conditions: Vec<Value>,
    #[serde(default)]
    effects: Vec<Value>,
}

pub struct CreateTriggerCommand;

impl CommandHandler for CreateTriggerCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("stage.create_trigger"),
            title: "Create trigger".into(),
            description: "Create a trigger using registry-defined conditions and effects.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["stage_id", "object_id", "activation", "q", "r"],
                "properties": {
                    "stage_id": {"type": "string"},
                    "object_id": {"type": "string"},
                    "activation": {"type": "string"},
                    "q": {"type": "integer"},
                    "r": {"type": "integer"},
                    "conditions": {"type": "array"},
                    "effects": {"type": "array"}
                }
            }),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into()],
            related_tools: vec![ToolId::from_static("stage.validate")],
        }
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: CreateTriggerInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.stage_id)?.clone();
        let mut stage = StageDocument::from_envelope(&envelope)?;
        ensure_unique_object(&stage, &input.object_id)?;
        stage.objects.push(StageObject {
            id: input.object_id.clone(),
            position: HexCoord::new(input.q, input.r),
            object: StageObjectKind::Trigger {
                activation: input.activation,
                conditions: input.conditions,
                effects: input.effects,
            },
            properties: json!({}),
        });
        let diagnostics = validate_stage(&stage);
        if diagnostics.has_errors() {
            return Err(CommandError::ValidationFailed(diagnostics));
        }
        state.documents.upsert(stage.to_envelope()?);
        Ok(json!({"stage_id": input.stage_id, "object_id": input.object_id}))
    }
}

#[derive(Debug, Deserialize)]
struct ConnectStageInput {
    stage_id: DocumentId,
    connection_id: ObjectId,
    to_stage: DocumentId,
    entry_point: String,
}

pub struct ConnectStageCommand;

impl CommandHandler for ConnectStageCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("stage.connect"),
            title: "Connect Stage".into(),
            description: "Create a semantic connection from one Stage to another.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["stage_id", "connection_id", "to_stage", "entry_point"],
                "properties": {
                    "stage_id": {"type": "string"},
                    "connection_id": {"type": "string"},
                    "to_stage": {"type": "string"},
                    "entry_point": {"type": "string"}
                }
            }),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into(), "creates_reference".into()],
            related_tools: vec![ToolId::from_static("project.impact.analyze")],
        }
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: ConnectStageInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.stage_id)?.clone();
        let mut stage = StageDocument::from_envelope(&envelope)?;
        if stage
            .connections
            .iter()
            .any(|item| item.id == input.connection_id)
        {
            return Err(CommandError::InvalidInput(format!(
                "connection id already exists: {}",
                input.connection_id
            )));
        }
        stage.connections.push(StageConnection {
            id: input.connection_id,
            to_stage: input.to_stage,
            entry_point: input.entry_point,
        });
        state.documents.upsert(stage.to_envelope()?);
        Ok(json!({"stage_id": input.stage_id}))
    }
}

fn ensure_unique_object(stage: &StageDocument, id: &ObjectId) -> Result<(), CommandError> {
    if stage.objects.iter().any(|item| &item.id == id) {
        Err(CommandError::InvalidInput(format!(
            "object id already exists: {id}"
        )))
    } else {
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct StageInput {
    stage_id: DocumentId,
}

pub struct ValidateStageQuery;

impl QueryHandler for ValidateStageQuery {
    fn descriptor(&self) -> QueryDescriptor {
        QueryDescriptor {
            id: ToolId::from_static("stage.validate"),
            title: "Validate Stage".into(),
            description: "Run tactical Stage validation and return structured diagnostics.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["stage_id"],
                "properties": {"stage_id": {"type": "string"}}
            }),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::Read, Permission::RunValidation]),
            related_tools: vec![
                ToolId::from_static("stage.create_spawn"),
                ToolId::from_static("stage.place_actor"),
            ],
        }
    }

    fn execute(&self, input: &Value, store: &DocumentStore) -> Result<Value, QueryError> {
        let input: StageInput = serde_json::from_value(input.clone())?;
        let document = store
            .get(&input.stage_id)
            .map_err(|error| QueryError::Failed(error.to_string()))?;
        let stage = StageDocument::from_envelope(document)
            .map_err(|error| QueryError::Failed(error.to_string()))?;
        Ok(serde_json::to_value(validate_stage(&stage))?)
    }
}

pub struct StageSummaryQuery;

impl QueryHandler for StageSummaryQuery {
    fn descriptor(&self) -> QueryDescriptor {
        QueryDescriptor {
            id: ToolId::from_static("stage.summary"),
            title: "Stage summary".into(),
            description: "Return a compact semantic Stage summary for agent context.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["stage_id"],
                "properties": {"stage_id": {"type": "string"}}
            }),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::Read]),
            related_tools: vec![ToolId::from_static("stage.validate")],
        }
    }

    fn execute(&self, input: &Value, store: &DocumentStore) -> Result<Value, QueryError> {
        let input: StageInput = serde_json::from_value(input.clone())?;
        let document = store
            .get(&input.stage_id)
            .map_err(|error| QueryError::Failed(error.to_string()))?;
        let stage = StageDocument::from_envelope(document)
            .map_err(|error| QueryError::Failed(error.to_string()))?;
        let actor_count = stage
            .objects
            .iter()
            .filter(|item| matches!(&item.object, StageObjectKind::Actor { .. }))
            .count();
        let trigger_count = stage
            .objects
            .iter()
            .filter(|item| matches!(&item.object, StageObjectKind::Trigger { .. }))
            .count();
        Ok(json!({
            "id": stage.id,
            "name": stage.name,
            "coordinate_space": stage.coordinate_space,
            "objects": stage.objects.len(),
            "actors": actor_count,
            "triggers": trigger_count,
            "connections": stage.connections.len()
        }))
    }
}

pub fn register_tactical_commands(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    registry.register(CreateStageCommand)?;
    registry.register(PlaceActorCommand)?;
    registry.register(CreateSpawnCommand)?;
    registry.register(CreateTriggerCommand)?;
    registry.register(ConnectStageCommand)?;
    Ok(())
}

pub fn register_tactical_queries(registry: &mut QueryRegistry) -> Result<(), QueryError> {
    registry.register(ValidateStageQuery)?;
    registry.register(StageSummaryQuery)?;
    Ok(())
}

pub fn is_tactical_document(type_id: &str) -> bool {
    type_id == STAGE_DOCUMENT_TYPE
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_command::{CommandContext, CommandExecutor, CommandRequest};

    #[test]
    fn commands_create_a_valid_stage() {
        let mut registry = CommandRegistry::default();
        register_tactical_commands(&mut registry).unwrap();
        let mut executor = CommandExecutor::new(registry);
        let context = CommandContext {
            actor: "test-agent".into(),
            permissions: PermissionSet::content_author(),
        };
        let mut state = EngineState::default();
        for (tool, input) in [
            (
                "stage.create",
                json!({"id": "stage.demo", "name": "Demo", "width": 8, "height": 8}),
            ),
            (
                "stage.create_spawn",
                json!({
                    "stage_id": "stage.demo",
                    "object_id": "spawn.player",
                    "profile": "player",
                    "q": 1,
                    "r": 1
                }),
            ),
        ] {
            executor
                .execute(
                    CommandRequest {
                        command: ToolId::new(tool).unwrap(),
                        input,
                        dry_run: false,
                    },
                    &context,
                    &mut state,
                )
                .unwrap();
        }
        let document = state
            .documents
            .get(&DocumentId::from_static("stage.demo"))
            .unwrap();
        let stage = StageDocument::from_envelope(document).unwrap();
        assert!(!validate_stage(&stage).has_errors());
    }
}
