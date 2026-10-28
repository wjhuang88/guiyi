use guiyi_engine_command::{
    CommandDescriptor, CommandError, CommandHandler, EngineState,
};
use guiyi_engine_core::{DocumentAccessPlan, DocumentId, ObjectId, Permission, PermissionSet, ToolId};
use guiyi_engine_schema::{FieldSchema, SchemaNode};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag, DiagnosticLocation, Severity};
use serde::Deserialize;
use serde_json::{json, Value};
use tactical_rpg_content::{HexCoord, StageDocument};
use tactical_rpg_validation::validate_stage;

pub(crate) const STAGE_OBJECT_NOT_FOUND: &str = "STAGE_OBJECT_NOT_FOUND";

#[derive(Debug, Deserialize)]
struct MoveStageObjectInput {
    stage_id: DocumentId,
    object_id: ObjectId,
    q: i32,
    r: i32,
}

pub(crate) struct MoveStageObjectCommand;

impl CommandHandler for MoveStageObjectCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("stage.move_object"),
            title: "Move Stage object".into(),
            description: "Move one stable Stage object to a new coordinate.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into()],
            related_tools: vec![
                ToolId::from_static("stage.validate"),
                ToolId::from_static("stage.remove_object"),
            ],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("stage_id", SchemaNode::string()),
            FieldSchema::required("object_id", SchemaNode::string()),
            FieldSchema::required("q", SchemaNode::integer()),
            FieldSchema::required("r", SchemaNode::integer()),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: MoveStageObjectInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.stage_id))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let mut bag = DiagnosticBag::default();
        let Ok(input) = serde_json::from_value::<MoveStageObjectInput>(input.clone()) else {
            return bag;
        };
        let Some(mut stage) = stage_for_validation(state, &input.stage_id) else {
            return bag;
        };
        let Some(object) = stage
            .objects
            .iter_mut()
            .find(|object| object.id == input.object_id)
        else {
            bag.push(stage_object_not_found(&input.stage_id, &input.object_id));
            return bag;
        };
        object.position = HexCoord::new(input.q, input.r);
        bag.extend(validate_stage(&stage));
        bag
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: MoveStageObjectInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.stage_id)?.clone();
        let mut stage = StageDocument::from_envelope(&envelope)?;
        let object = stage
            .objects
            .iter_mut()
            .find(|object| object.id == input.object_id)
            .ok_or_else(|| {
                CommandError::InvalidInput(format!(
                    "Stage object not found: {}",
                    input.object_id
                ))
            })?;
        let previous = object.position;
        let next = HexCoord::new(input.q, input.r);
        object.position = next;
        state.documents.upsert(stage.to_envelope()?);
        Ok(json!({
            "stage_id": input.stage_id,
            "object_id": input.object_id,
            "from": previous,
            "to": next
        }))
    }
}

#[derive(Debug, Deserialize)]
struct RemoveStageObjectInput {
    stage_id: DocumentId,
    object_id: ObjectId,
}

pub(crate) struct RemoveStageObjectCommand;

impl CommandHandler for RemoveStageObjectCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("stage.remove_object"),
            title: "Remove Stage object".into(),
            description: "Remove one stable object from a Stage authoring document.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into()],
            related_tools: vec![
                ToolId::from_static("stage.validate"),
                ToolId::from_static("project.references.find"),
            ],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("stage_id", SchemaNode::string()),
            FieldSchema::required("object_id", SchemaNode::string()),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: RemoveStageObjectInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.stage_id))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let mut bag = DiagnosticBag::default();
        let Ok(input) = serde_json::from_value::<RemoveStageObjectInput>(input.clone()) else {
            return bag;
        };
        let Some(mut stage) = stage_for_validation(state, &input.stage_id) else {
            return bag;
        };
        let Some(index) = stage
            .objects
            .iter()
            .position(|object| object.id == input.object_id)
        else {
            bag.push(stage_object_not_found(&input.stage_id, &input.object_id));
            return bag;
        };
        stage.objects.remove(index);
        bag.extend(validate_stage(&stage));
        bag
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: RemoveStageObjectInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.stage_id)?.clone();
        let mut stage = StageDocument::from_envelope(&envelope)?;
        let index = stage
            .objects
            .iter()
            .position(|object| object.id == input.object_id)
            .ok_or_else(|| {
                CommandError::InvalidInput(format!(
                    "Stage object not found: {}",
                    input.object_id
                ))
            })?;
        stage.objects.remove(index);
        let remaining_objects = stage.objects.len();
        state.documents.upsert(stage.to_envelope()?);
        Ok(json!({
            "stage_id": input.stage_id,
            "object_id": input.object_id,
            "remaining_objects": remaining_objects
        }))
    }
}

fn stage_for_validation(state: &EngineState, stage_id: &DocumentId) -> Option<StageDocument> {
    state
        .documents
        .get(stage_id)
        .ok()
        .and_then(|envelope| StageDocument::from_envelope(envelope).ok())
}

fn stage_object_not_found(stage_id: &DocumentId, object_id: &ObjectId) -> Diagnostic {
    Diagnostic {
        code: STAGE_OBJECT_NOT_FOUND.into(),
        severity: Severity::Error,
        message: format!("Stage object not found: {object_id}"),
        location: Some(DiagnosticLocation {
            document: Some(stage_id.clone()),
            object: Some(object_id.clone()),
            field_path: Some("/objects".into()),
        }),
        suggested_tools: vec!["project.document.get".into()],
        auto_fixable: false,
    }
}
