use guiyi_engine_command::{CommandDescriptor, CommandError, CommandHandler, EngineState};
use guiyi_engine_core::{DocumentAccessPlan, DocumentId, ObjectId, Permission, PermissionSet, ToolId};
use guiyi_engine_schema::{FieldSchema, SchemaNode};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag, DiagnosticLocation, Severity};
use serde::Deserialize;
use serde_json::{json, Value};
use tactical_rpg_content::{EncounterDocument, EncounterParticipant};
use tactical_rpg_validation::{validate_encounter, validate_encounter_project};

pub const ENCOUNTER_PARTICIPANT_NOT_FOUND: &str = "ENCOUNTER_PARTICIPANT_NOT_FOUND";
pub const ENCOUNTER_STAGE_MISMATCH: &str = "ENCOUNTER_STAGE_MISMATCH";

#[derive(Debug, Deserialize)]
struct CreateEncounterInput {
    encounter_id: DocumentId,
    name: String,
    stage_id: DocumentId,
}

pub struct CreateEncounterCommand;

impl CommandHandler for CreateEncounterCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("encounter.create"),
            title: "Create Encounter".into(),
            description: "Create a tactical Encounter bound to one authoritative Stage.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["creates_document".into(), "creates_reference".into()],
            related_tools: vec![ToolId::from_static("encounter.add_participant")],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("encounter_id", SchemaNode::string()),
            FieldSchema::required("name", SchemaNode::string()),
            FieldSchema::required("stage_id", SchemaNode::string()),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: CreateEncounterInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::documents([
            input.encounter_id,
            input.stage_id,
        ]))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let Ok(input) = serde_json::from_value::<CreateEncounterInput>(input.clone()) else {
            return DiagnosticBag::default();
        };
        if state.documents.get(&input.encounter_id).is_ok() {
            let mut bag = DiagnosticBag::default();
            bag.push(
                Diagnostic::error("DOCUMENT_ALREADY_EXISTS", "Encounter id already exists")
                    .at_document(input.encounter_id),
            );
            return bag;
        }
        let encounter = EncounterDocument::new(input.encounter_id, input.name, input.stage_id);
        validate_encounter_project(&encounter, &state.documents)
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: CreateEncounterInput = serde_json::from_value(input.clone())?;
        let encounter = EncounterDocument::new(
            input.encounter_id.clone(),
            input.name,
            input.stage_id,
        );
        state.documents.insert(encounter.to_envelope()?)?;
        Ok(json!({"encounter_id": input.encounter_id}))
    }
}

#[derive(Debug, Deserialize)]
struct AddEncounterParticipantInput {
    encounter_id: DocumentId,
    stage_id: DocumentId,
    participant_id: ObjectId,
    actor_definition: DocumentId,
    spawn_point: ObjectId,
    side: String,
}

pub struct AddEncounterParticipantCommand;

impl CommandHandler for AddEncounterParticipantCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("encounter.add_participant"),
            title: "Add Encounter participant".into(),
            description: "Add a stable participant bound to an actor definition and Stage spawn point.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into(), "creates_reference".into()],
            related_tools: vec![
                ToolId::from_static("project.references.find"),
                ToolId::from_static("encounter.remove_participant"),
            ],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("encounter_id", SchemaNode::string()),
            FieldSchema::required("stage_id", SchemaNode::string()),
            FieldSchema::required("participant_id", SchemaNode::string()),
            FieldSchema::required("actor_definition", SchemaNode::string()),
            FieldSchema::required("spawn_point", SchemaNode::string()),
            FieldSchema::required("side", SchemaNode::string()),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: AddEncounterParticipantInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::documents([
            input.encounter_id,
            input.stage_id,
            input.actor_definition,
        ]))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let Ok(input) = serde_json::from_value::<AddEncounterParticipantInput>(input.clone()) else {
            return DiagnosticBag::default();
        };
        let Ok(envelope) = state.documents.get(&input.encounter_id) else {
            return DiagnosticBag::default();
        };
        let Ok(mut encounter) = EncounterDocument::from_envelope(envelope) else {
            return DiagnosticBag::default();
        };
        if encounter.stage != input.stage_id {
            return stage_mismatch(&encounter, &input.stage_id);
        }
        encounter.participants.push(EncounterParticipant {
            id: input.participant_id,
            actor_definition: input.actor_definition,
            spawn_point: input.spawn_point,
            side: input.side,
        });
        validate_encounter_project(&encounter, &state.documents)
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: AddEncounterParticipantInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.encounter_id)?.clone();
        let mut encounter = EncounterDocument::from_envelope(&envelope)?;
        if encounter.stage != input.stage_id {
            return Err(CommandError::ValidationFailed(stage_mismatch(
                &encounter,
                &input.stage_id,
            )));
        }
        encounter.participants.push(EncounterParticipant {
            id: input.participant_id.clone(),
            actor_definition: input.actor_definition,
            spawn_point: input.spawn_point,
            side: input.side,
        });
        let diagnostics = validate_encounter_project(&encounter, &state.documents);
        if diagnostics.has_errors() {
            return Err(CommandError::ValidationFailed(diagnostics));
        }
        state.documents.upsert(encounter.to_envelope()?);
        Ok(json!({
            "encounter_id": input.encounter_id,
            "participant_id": input.participant_id
        }))
    }
}

#[derive(Debug, Deserialize)]
struct RemoveEncounterParticipantInput {
    encounter_id: DocumentId,
    participant_id: ObjectId,
}

pub struct RemoveEncounterParticipantCommand;

impl CommandHandler for RemoveEncounterParticipantCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("encounter.remove_participant"),
            title: "Remove Encounter participant".into(),
            description: "Remove one Encounter participant by stable participant ID.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into(), "removes_reference".into()],
            related_tools: vec![ToolId::from_static("encounter.add_participant")],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("encounter_id", SchemaNode::string()),
            FieldSchema::required("participant_id", SchemaNode::string()),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: RemoveEncounterParticipantInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.encounter_id))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let Ok(input) = serde_json::from_value::<RemoveEncounterParticipantInput>(input.clone())
        else {
            return DiagnosticBag::default();
        };
        let Ok(envelope) = state.documents.get(&input.encounter_id) else {
            return DiagnosticBag::default();
        };
        let Ok(mut encounter) = EncounterDocument::from_envelope(envelope) else {
            return DiagnosticBag::default();
        };
        let before = encounter.participants.len();
        encounter
            .participants
            .retain(|participant| participant.id != input.participant_id);
        if encounter.participants.len() == before {
            return participant_not_found(&encounter, &input.participant_id);
        }
        validate_encounter(&encounter)
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: RemoveEncounterParticipantInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.encounter_id)?.clone();
        let mut encounter = EncounterDocument::from_envelope(&envelope)?;
        let before = encounter.participants.len();
        encounter
            .participants
            .retain(|participant| participant.id != input.participant_id);
        if encounter.participants.len() == before {
            return Err(CommandError::ValidationFailed(participant_not_found(
                &encounter,
                &input.participant_id,
            )));
        }
        state.documents.upsert(encounter.to_envelope()?);
        Ok(json!({
            "encounter_id": input.encounter_id,
            "participant_id": input.participant_id
        }))
    }
}

fn stage_mismatch(encounter: &EncounterDocument, supplied: &DocumentId) -> DiagnosticBag {
    let mut bag = DiagnosticBag::default();
    bag.push(
        Diagnostic::error(
            ENCOUNTER_STAGE_MISMATCH,
            format!(
                "supplied Stage {} does not match Encounter Stage {}",
                supplied, encounter.stage
            ),
        )
        .at_document(encounter.id.clone()),
    );
    bag
}

fn participant_not_found(encounter: &EncounterDocument, participant: &ObjectId) -> DiagnosticBag {
    let mut bag = DiagnosticBag::default();
    bag.push(Diagnostic {
        code: ENCOUNTER_PARTICIPANT_NOT_FOUND.into(),
        severity: Severity::Error,
        message: format!("Encounter participant {} does not exist", participant),
        location: Some(DiagnosticLocation {
            document: Some(encounter.id.clone()),
            object: Some(participant.clone()),
            field_path: Some("/participants".into()),
        }),
        suggested_tools: vec!["encounter.add_participant".into()],
        auto_fixable: false,
    });
    bag
}
