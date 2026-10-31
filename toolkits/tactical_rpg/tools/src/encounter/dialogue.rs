use guiyi_engine_command::{CommandDescriptor, CommandError, CommandHandler, EngineState};
use guiyi_engine_content::DocumentStore;
use guiyi_engine_core::{
    DocumentAccessPlan, DocumentId, ObjectId, Permission, PermissionSet, ToolId,
};
use guiyi_engine_query::{QueryDescriptor, QueryError, QueryHandler};
use guiyi_engine_schema::{FieldSchema, SchemaNode};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag, DiagnosticLocation, Severity};
use serde::Deserialize;
use serde_json::{json, Value};
use tactical_rpg_content::{
    default_tactical_rule_registry, DialogueChoice, DialogueDocument, DialogueNode,
};
use tactical_rpg_validation::validate_dialogue;

pub const DIALOGUE_NODE_NOT_FOUND: &str = "DIALOGUE_NODE_NOT_FOUND";

#[derive(Debug, Deserialize)]
struct CreateDialogueInput {
    dialogue_id: DocumentId,
    name: String,
    entry_node_id: ObjectId,
    speaker: String,
    text: String,
}

pub struct CreateDialogueCommand;

impl CommandHandler for CreateDialogueCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("dialogue.create"),
            title: "Create Dialogue".into(),
            description: "Create a Dialogue document with one stable entry node.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["creates_document".into()],
            related_tools: vec![ToolId::from_static("dialogue.add_node")],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("dialogue_id", SchemaNode::string()),
            FieldSchema::required("name", SchemaNode::string()),
            FieldSchema::required("entry_node_id", SchemaNode::string()),
            FieldSchema::required("speaker", SchemaNode::string()),
            FieldSchema::required("text", SchemaNode::string()),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: CreateDialogueInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.dialogue_id))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let Ok(input) = serde_json::from_value::<CreateDialogueInput>(input.clone()) else {
            return DiagnosticBag::default();
        };
        if state.documents.get(&input.dialogue_id).is_ok() {
            let mut bag = DiagnosticBag::default();
            bag.push(
                Diagnostic::error("DOCUMENT_ALREADY_EXISTS", "Dialogue id already exists")
                    .at_document(input.dialogue_id),
            );
            return bag;
        }
        validate_dialogue(&DialogueDocument::new(
            input.dialogue_id,
            input.name,
            input.entry_node_id,
            input.speaker,
            input.text,
        ))
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: CreateDialogueInput = serde_json::from_value(input.clone())?;
        let dialogue = DialogueDocument::new(
            input.dialogue_id.clone(),
            input.name,
            input.entry_node_id,
            input.speaker,
            input.text,
        );
        let diagnostics = validate_dialogue(&dialogue);
        if diagnostics.has_errors() {
            return Err(CommandError::ValidationFailed(diagnostics));
        }
        state.documents.insert(dialogue.to_envelope()?)?;
        Ok(json!({"dialogue_id": input.dialogue_id}))
    }
}

#[derive(Debug, Deserialize)]
struct AddDialogueNodeInput {
    dialogue_id: DocumentId,
    node_id: ObjectId,
    speaker: String,
    text: String,
}

pub struct AddDialogueNodeCommand;

impl CommandHandler for AddDialogueNodeCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("dialogue.add_node"),
            title: "Add Dialogue node".into(),
            description: "Add one stable node to a Dialogue graph.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into()],
            related_tools: vec![ToolId::from_static("dialogue.add_choice")],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("dialogue_id", SchemaNode::string()),
            FieldSchema::required("node_id", SchemaNode::string()),
            FieldSchema::required("speaker", SchemaNode::string()),
            FieldSchema::required("text", SchemaNode::string()),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: AddDialogueNodeInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.dialogue_id))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let Ok(input) = serde_json::from_value::<AddDialogueNodeInput>(input.clone()) else {
            return DiagnosticBag::default();
        };
        let Ok(envelope) = state.documents.get(&input.dialogue_id) else {
            return DiagnosticBag::default();
        };
        let Ok(mut dialogue) = DialogueDocument::from_envelope(envelope) else {
            return DiagnosticBag::default();
        };
        dialogue.nodes.push(DialogueNode {
            id: input.node_id,
            speaker: input.speaker,
            text: input.text,
            choices: Vec::new(),
        });
        validate_dialogue(&dialogue)
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: AddDialogueNodeInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.dialogue_id)?.clone();
        let mut dialogue = DialogueDocument::from_envelope(&envelope)?;
        dialogue.nodes.push(DialogueNode {
            id: input.node_id.clone(),
            speaker: input.speaker,
            text: input.text,
            choices: Vec::new(),
        });
        let diagnostics = validate_dialogue(&dialogue);
        if diagnostics.has_errors() {
            return Err(CommandError::ValidationFailed(diagnostics));
        }
        state.documents.upsert(dialogue.to_envelope()?);
        Ok(json!({
            "dialogue_id": input.dialogue_id,
            "node_id": input.node_id
        }))
    }
}

#[derive(Debug, Deserialize)]
struct AddDialogueChoiceInput {
    dialogue_id: DocumentId,
    from_node_id: ObjectId,
    choice_id: ObjectId,
    text: String,
    target_node_id: Option<ObjectId>,
    #[serde(default)]
    conditions: Vec<Value>,
    #[serde(default)]
    effects: Vec<Value>,
}

pub struct AddDialogueChoiceCommand;

impl CommandHandler for AddDialogueChoiceCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("dialogue.add_choice"),
            title: "Add Dialogue choice".into(),
            description: "Add a stable Dialogue edge with registry-defined conditions and effects."
                .into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into()],
            related_tools: vec![ToolId::from_static("tactical.rules.catalog")],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("dialogue_id", SchemaNode::string()),
            FieldSchema::required("from_node_id", SchemaNode::string()),
            FieldSchema::required("choice_id", SchemaNode::string()),
            FieldSchema::required("text", SchemaNode::string()),
            FieldSchema::required("target_node_id", SchemaNode::string().nullable()),
            FieldSchema::optional(
                "conditions",
                SchemaNode::array(SchemaNode::any()).with_default(json!([])),
            ),
            FieldSchema::optional(
                "effects",
                SchemaNode::array(SchemaNode::any()).with_default(json!([])),
            ),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: AddDialogueChoiceInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.dialogue_id))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let Ok(input) = serde_json::from_value::<AddDialogueChoiceInput>(input.clone()) else {
            return DiagnosticBag::default();
        };
        let Ok(envelope) = state.documents.get(&input.dialogue_id) else {
            return DiagnosticBag::default();
        };
        let Ok(mut dialogue) = DialogueDocument::from_envelope(envelope) else {
            return DiagnosticBag::default();
        };
        let Some(node) = dialogue
            .nodes
            .iter_mut()
            .find(|node| node.id == input.from_node_id)
        else {
            return node_not_found(&dialogue, &input.from_node_id);
        };
        node.choices.push(DialogueChoice {
            id: input.choice_id,
            text: input.text,
            target: input.target_node_id,
            conditions: input.conditions,
            effects: input.effects,
        });
        validate_dialogue(&dialogue)
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: AddDialogueChoiceInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.dialogue_id)?.clone();
        let mut dialogue = DialogueDocument::from_envelope(&envelope)?;
        let Some(node) = dialogue
            .nodes
            .iter_mut()
            .find(|node| node.id == input.from_node_id)
        else {
            return Err(CommandError::ValidationFailed(node_not_found(
                &dialogue,
                &input.from_node_id,
            )));
        };
        node.choices.push(DialogueChoice {
            id: input.choice_id.clone(),
            text: input.text,
            target: input.target_node_id,
            conditions: input.conditions,
            effects: input.effects,
        });
        let diagnostics = validate_dialogue(&dialogue);
        if diagnostics.has_errors() {
            return Err(CommandError::ValidationFailed(diagnostics));
        }
        state.documents.upsert(dialogue.to_envelope()?);
        Ok(json!({
            "dialogue_id": input.dialogue_id,
            "choice_id": input.choice_id
        }))
    }
}

pub struct TacticalRulesCatalogQuery;

impl QueryHandler for TacticalRulesCatalogQuery {
    fn descriptor(&self) -> QueryDescriptor {
        QueryDescriptor {
            id: ToolId::from_static("tactical.rules.catalog"),
            title: "Tactical rule catalog".into(),
            description: "List registered tactical condition/effect schemas for Agent authoring."
                .into(),
            input_schema: json!({"type": "object", "properties": {}}),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::Read]),
            related_tools: vec![
                ToolId::from_static("stage.create_trigger"),
                ToolId::from_static("dialogue.add_choice"),
            ],
        }
    }

    fn document_access(&self, _input: &Value) -> Result<DocumentAccessPlan, QueryError> {
        Ok(DocumentAccessPlan::none())
    }

    fn execute(&self, _input: &Value, _store: &DocumentStore) -> Result<Value, QueryError> {
        let registry = default_tactical_rule_registry();
        let conditions = registry
            .conditions()
            .map(|descriptor| {
                json!({
                    "id": descriptor.id,
                    "description": descriptor.description,
                    "schema": descriptor.schema.to_json_schema()
                })
            })
            .collect::<Vec<_>>();
        let effects = registry
            .effects()
            .map(|descriptor| {
                json!({
                    "id": descriptor.id,
                    "description": descriptor.description,
                    "schema": descriptor.schema.to_json_schema()
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({"conditions": conditions, "effects": effects}))
    }
}

fn node_not_found(dialogue: &DialogueDocument, node: &ObjectId) -> DiagnosticBag {
    let mut bag = DiagnosticBag::default();
    bag.push(Diagnostic {
        code: DIALOGUE_NODE_NOT_FOUND.into(),
        severity: Severity::Error,
        message: format!("Dialogue node {node} does not exist"),
        location: Some(DiagnosticLocation {
            document: Some(dialogue.id.clone()),
            object: Some(node.clone()),
            field_path: Some("/nodes".into()),
        }),
        suggested_tools: vec!["dialogue.add_node".into()],
        auto_fixable: false,
    });
    bag
}
