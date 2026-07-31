#![forbid(unsafe_code)]

//! Typed command registry, dry-run, transaction diff, audit, and atomic apply.

use guiyi_engine_content::{ContentError, DocumentEnvelope, DocumentHeader, DocumentStore};
use guiyi_engine_core::{
    DocumentAccessPlan, DocumentId, EngineTypeId, Permission, PermissionSet, ToolId, TransactionId,
};
use guiyi_engine_schema::{SchemaDefinitionError, SchemaNode};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EngineState {
    pub documents: DocumentStore,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub id: ToolId,
    pub title: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub required_permissions: PermissionSet,
    pub side_effects: Vec<String>,
    pub related_tools: Vec<ToolId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandRequest {
    pub command: ToolId,
    pub input: Value,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct CommandContext {
    pub actor: String,
    pub permissions: PermissionSet,
}

pub trait CommandHandler: Send + Sync {
    fn descriptor(&self) -> CommandDescriptor;

    fn input_schema(&self) -> SchemaNode;

    fn document_access(&self, _input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        Ok(DocumentAccessPlan::project())
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let _ = (input, state);
        DiagnosticBag::default()
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError>;
}

#[derive(Default)]
pub struct CommandRegistry {
    handlers: BTreeMap<ToolId, Box<dyn CommandHandler>>,
    schemas: BTreeMap<ToolId, SchemaNode>,
}

impl CommandRegistry {
    pub fn register(&mut self, handler: impl CommandHandler + 'static) -> Result<(), CommandError> {
        let id = handler.descriptor().id;
        if self.handlers.contains_key(&id) {
            return Err(CommandError::DuplicateCommand(id));
        }
        let schema = handler.input_schema();
        schema
            .validate_definition()
            .map_err(CommandError::SchemaDefinitionInvalid)?;
        self.schemas.insert(id.clone(), schema);
        self.handlers.insert(id, Box::new(handler));
        Ok(())
    }

    pub fn handler(&self, id: &ToolId) -> Result<&dyn CommandHandler, CommandError> {
        self.handlers
            .get(id)
            .map(Box::as_ref)
            .ok_or_else(|| CommandError::CommandNotFound(id.clone()))
    }

    pub fn schema(&self, id: &ToolId) -> Option<&SchemaNode> {
        self.schemas.get(id)
    }

    pub fn descriptors(&self) -> Vec<CommandDescriptor> {
        self.handlers
            .values()
            .map(|handler| {
                let mut descriptor = handler.descriptor();
                if let Some(schema) = self.schemas.get(&descriptor.id) {
                    descriptor.input_schema = schema.to_json_schema();
                }
                descriptor
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Previewed,
    Applied,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentDiff {
    pub document_id: DocumentId,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
    pub change: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionReport {
    pub transaction_id: TransactionId,
    pub command: ToolId,
    pub actor: String,
    pub status: TransactionStatus,
    pub output: Value,
    pub document_diffs: Vec<DocumentDiff>,
    pub diagnostics: DiagnosticBag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub sequence: u64,
    pub report: TransactionReport,
}

pub struct CommandExecutor {
    registry: CommandRegistry,
    next_transaction: u64,
    audit: Vec<AuditRecord>,
}

impl CommandExecutor {
    pub fn new(registry: CommandRegistry) -> Self {
        Self {
            registry,
            next_transaction: 1,
            audit: Vec::new(),
        }
    }

    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    pub fn audit_log(&self) -> &[AuditRecord] {
        &self.audit
    }

    pub fn prepare_input(&self, request: &CommandRequest) -> Result<Value, CommandError> {
        match self.registry.schema(&request.command) {
            Some(schema) => schema
                .validate_and_normalize(&request.input)
                .map_err(CommandError::ValidationFailed),
            None => Ok(request.input.clone()),
        }
    }

    pub fn document_access(
        &self,
        request: &CommandRequest,
    ) -> Result<DocumentAccessPlan, CommandError> {
        let normalized = self.prepare_input(request)?;
        self.registry
            .handler(&request.command)?
            .document_access(&normalized)
    }

    pub fn execute(
        &mut self,
        request: CommandRequest,
        context: &CommandContext,
        state: &mut EngineState,
    ) -> Result<TransactionReport, CommandError> {
        self.execute_scoped(request, context, state, None)
    }

    pub fn execute_scoped(
        &mut self,
        request: CommandRequest,
        context: &CommandContext,
        state: &mut EngineState,
        allowed_documents: Option<&BTreeSet<DocumentId>>,
    ) -> Result<TransactionReport, CommandError> {
        let handler = self.registry.handler(&request.command)?;
        let descriptor = handler.descriptor();
        if !context
            .permissions
            .contains_all(&descriptor.required_permissions)
        {
            return Err(CommandError::PermissionDenied(request.command));
        }
        if request.dry_run && !context.permissions.contains(Permission::DryRun) {
            return Err(CommandError::PermissionDenied(request.command));
        }

        let normalized_input = self.prepare_input(&request)?;

        let diagnostics = handler.validate(&normalized_input, state);
        if diagnostics.has_errors() {
            return Err(CommandError::ValidationFailed(diagnostics));
        }

        let before = state.clone();
        let mut working = state.clone();
        let output = handler.apply(&normalized_input, &mut working)?;
        let diffs = diff_stores(&before.documents, &working.documents)?;
        if let Some(allowed) = allowed_documents {
            let denied = diffs
                .iter()
                .filter(|diff| !allowed.contains(&diff.document_id))
                .map(|diff| diff.document_id.clone())
                .collect::<Vec<_>>();
            if !denied.is_empty() {
                return Err(CommandError::WorkingSetDenied(denied));
            }
        }
        let status = if request.dry_run {
            TransactionStatus::Previewed
        } else {
            *state = working;
            TransactionStatus::Applied
        };
        let transaction_id = TransactionId::new(format!("tx-{:08}", self.next_transaction))
            .expect("generated transaction identifiers are valid");
        let report = TransactionReport {
            transaction_id,
            command: request.command,
            actor: context.actor.clone(),
            status,
            output,
            document_diffs: diffs,
            diagnostics,
        };
        self.audit.push(AuditRecord {
            sequence: self.next_transaction,
            report: report.clone(),
        });
        self.next_transaction += 1;
        Ok(report)
    }
}

fn diff_stores(
    before: &DocumentStore,
    after: &DocumentStore,
) -> Result<Vec<DocumentDiff>, CommandError> {
    let ids = before
        .iter()
        .map(|(id, _)| id.clone())
        .chain(after.iter().map(|(id, _)| id.clone()))
        .collect::<BTreeSet<_>>();
    let mut diffs = Vec::new();
    for id in ids {
        let old = before.get(&id).ok();
        let new = after.get(&id).ok();
        let old_hash = old.map(DocumentEnvelope::content_hash).transpose()?;
        let new_hash = new.map(DocumentEnvelope::content_hash).transpose()?;
        if old_hash == new_hash {
            continue;
        }
        let change = match (old, new) {
            (None, Some(_)) => "created",
            (Some(_), None) => "deleted",
            (Some(_), Some(_)) => "modified",
            (None, None) => continue,
        };
        diffs.push(DocumentDiff {
            document_id: id,
            before_hash: old_hash,
            after_hash: new_hash,
            change: change.into(),
        });
    }
    Ok(diffs)
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("duplicate command: {0}")]
    DuplicateCommand(ToolId),
    #[error("command not found: {0}")]
    CommandNotFound(ToolId),
    #[error("permission denied for command: {0}")]
    PermissionDenied(ToolId),
    #[error("command modified documents outside the working set: {0:?}")]
    WorkingSetDenied(Vec<DocumentId>),
    #[error("invalid command input: {0}")]
    InvalidInput(String),
    #[error("command validation failed")]
    ValidationFailed(DiagnosticBag),
    #[error("schema definition invalid: {0}")]
    SchemaDefinitionInvalid(SchemaDefinitionError),
    #[error(transparent)]
    Content(#[from] ContentError),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Built-in document commands
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CreateDocumentInput {
    id: DocumentId,
    type_id: EngineTypeId,
    display_name: String,
    schema_version: u32,
    payload: Value,
}

fn document_create_schema() -> SchemaNode {
    use guiyi_engine_schema::codes;
    let _ = codes::COMMAND_INPUT_REQUIRED;
    SchemaNode::object(vec![
        guiyi_engine_schema::FieldSchema::required("id", SchemaNode::string()),
        guiyi_engine_schema::FieldSchema::required("type_id", SchemaNode::string()),
        guiyi_engine_schema::FieldSchema::required("display_name", SchemaNode::string()),
        guiyi_engine_schema::FieldSchema::optional(
            "schema_version",
            SchemaNode::integer()
                .with_minimum(1.0)
                .with_default(json!(1)),
        ),
        guiyi_engine_schema::FieldSchema::optional(
            "payload",
            SchemaNode::any().with_default(Value::Null),
        ),
    ])
}

pub struct CreateDocumentCommand;

impl CommandHandler for CreateDocumentCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("document.create"),
            title: "Create document".into(),
            description: "Create a typed authoring document in the current transaction.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["creates_document".into()],
            related_tools: vec![ToolId::from_static("project.document.get")],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        document_create_schema()
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: CreateDocumentInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.id))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let mut bag = DiagnosticBag::default();
        if let Ok(parsed) = serde_json::from_value::<CreateDocumentInput>(input.clone()) {
            if state.documents.get(&parsed.id).is_ok() {
                bag.push(
                    Diagnostic::error(
                        "DOCUMENT_ALREADY_EXISTS",
                        "document identifier already exists",
                    )
                    .at_document(parsed.id),
                );
            }
        }
        bag
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: CreateDocumentInput = serde_json::from_value(input.clone())?;
        let id = input.id.clone();
        state.documents.insert(DocumentEnvelope {
            header: DocumentHeader {
                id: input.id,
                type_id: input.type_id,
                schema_version: input.schema_version,
                display_name: input.display_name,
            },
            references: Vec::new(),
            payload: input.payload,
        })?;
        Ok(json!({"document_id": id}))
    }
}

#[derive(Debug, Deserialize)]
struct DeleteDocumentInput {
    document_id: DocumentId,
}

pub struct DeleteDocumentCommand;

impl CommandHandler for DeleteDocumentCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("document.delete"),
            title: "Delete document".into(),
            description: "Delete one authoring document atomically.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["deletes_document".into()],
            related_tools: vec![ToolId::from_static("project.references.find")],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![guiyi_engine_schema::FieldSchema::required(
            "document_id",
            SchemaNode::string(),
        )])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: DeleteDocumentInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.document_id))
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: DeleteDocumentInput = serde_json::from_value(input.clone())?;
        state.documents.remove(&input.document_id)?;
        Ok(json!({"document_id": input.document_id}))
    }
}

#[derive(Debug, Deserialize)]
struct SetFieldInput {
    document_id: DocumentId,
    path: Vec<String>,
    value: Value,
}

pub struct SetDocumentFieldCommand;

impl CommandHandler for SetDocumentFieldCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("document.set_field"),
            title: "Set document field".into(),
            description: "Set a JSON object field using a typed path inside a transaction.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into()],
            related_tools: vec![ToolId::from_static("project.document.get")],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            guiyi_engine_schema::FieldSchema::required("document_id", SchemaNode::string()),
            guiyi_engine_schema::FieldSchema::required(
                "path",
                SchemaNode::array(SchemaNode::string()).with_min_items(1),
            ),
            guiyi_engine_schema::FieldSchema::required("value", SchemaNode::any()),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: SetFieldInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.document_id))
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: SetFieldInput = serde_json::from_value(input.clone())?;
        let document = state.documents.get_mut(&input.document_id)?;
        set_json_path(&mut document.payload, &input.path, input.value)?;
        Ok(json!({"document_id": input.document_id, "path": input.path}))
    }
}

fn set_json_path(target: &mut Value, path: &[String], value: Value) -> Result<(), CommandError> {
    let (last, parents) = path
        .split_last()
        .ok_or_else(|| CommandError::InvalidInput("path cannot be empty".into()))?;
    let mut current = target;
    for segment in parents {
        let object = current
            .as_object_mut()
            .ok_or_else(|| CommandError::InvalidInput(format!("{segment} is not an object")))?;
        current = object.entry(segment.clone()).or_insert_with(|| json!({}));
    }
    let object = current
        .as_object_mut()
        .ok_or_else(|| CommandError::InvalidInput(format!("{last} parent is not an object")))?;
    object.insert(last.clone(), value);
    Ok(())
}

pub fn register_builtin_document_commands(
    registry: &mut CommandRegistry,
) -> Result<(), CommandError> {
    registry.register(CreateDocumentCommand)?;
    registry.register(DeleteDocumentCommand)?;
    registry.register(SetDocumentFieldCommand)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_schema::codes;

    fn executor() -> CommandExecutor {
        let mut registry = CommandRegistry::default();
        register_builtin_document_commands(&mut registry).unwrap();
        CommandExecutor::new(registry)
    }

    fn context() -> CommandContext {
        CommandContext {
            actor: "test-agent".into(),
            permissions: PermissionSet::content_author(),
        }
    }

    fn create_document(executor: &mut CommandExecutor, state: &mut EngineState, id: &str) {
        executor
            .execute(
                CommandRequest {
                    command: ToolId::from_static("document.create"),
                    input: json!({
                        "id": id,
                        "type_id": "example.document",
                        "display_name": id
                    }),
                    dry_run: false,
                },
                &context(),
                state,
            )
            .unwrap();
    }

    #[test]
    fn dry_run_does_not_mutate_state() {
        let mut executor = executor();
        let mut state = EngineState::default();
        let report = executor
            .execute(
                CommandRequest {
                    command: ToolId::from_static("document.create"),
                    input: json!({
                        "id": "doc.preview",
                        "type_id": "example.document",
                        "display_name": "Preview"
                    }),
                    dry_run: true,
                },
                &context(),
                &mut state,
            )
            .unwrap();
        assert_eq!(report.status, TransactionStatus::Previewed);
        assert!(state.documents.is_empty());
    }

    #[test]
    fn failed_commands_are_atomic() {
        let mut executor = executor();
        let mut state = EngineState::default();
        create_document(&mut executor, &mut state, "doc.one");
        let before = state.clone();
        let result = executor.execute(
            CommandRequest {
                command: ToolId::from_static("document.set_field"),
                input: json!({"document_id": "doc.one", "path": [], "value": 3}),
                dry_run: false,
            },
            &context(),
            &mut state,
        );
        assert!(result.is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn scoped_execution_rejects_actual_cross_document_effects_atomically() {
        struct CrossDocumentCommand;

        impl CommandHandler for CrossDocumentCommand {
            fn descriptor(&self) -> CommandDescriptor {
                CommandDescriptor {
                    id: ToolId::from_static("test.cross_document"),
                    title: "Cross document".into(),
                    description: "Test command".into(),
                    input_schema: Value::Null,
                    output_schema: json!({"type": "object"}),
                    required_permissions: PermissionSet::new([Permission::EditContent]),
                    side_effects: vec!["modifies_document".into()],
                    related_tools: Vec::new(),
                }
            }

            fn input_schema(&self) -> SchemaNode {
                SchemaNode::object(vec![])
            }

            fn document_access(&self, _input: &Value) -> Result<DocumentAccessPlan, CommandError> {
                Ok(DocumentAccessPlan::document(DocumentId::from_static(
                    "doc.a",
                )))
            }

            fn apply(
                &self,
                _input: &Value,
                state: &mut EngineState,
            ) -> Result<Value, CommandError> {
                state
                    .documents
                    .get_mut(&DocumentId::from_static("doc.b"))?
                    .payload = json!({"changed": true});
                Ok(json!({}))
            }
        }

        let mut registry = CommandRegistry::default();
        register_builtin_document_commands(&mut registry).unwrap();
        registry.register(CrossDocumentCommand).unwrap();
        let mut executor = CommandExecutor::new(registry);
        let mut state = EngineState::default();
        create_document(&mut executor, &mut state, "doc.a");
        create_document(&mut executor, &mut state, "doc.b");
        let before = state.clone();
        let allowed = BTreeSet::from([DocumentId::from_static("doc.a")]);
        let result = executor.execute_scoped(
            CommandRequest {
                command: ToolId::from_static("test.cross_document"),
                input: json!({}),
                dry_run: false,
            },
            &context(),
            &mut state,
            Some(&allowed),
        );
        assert!(matches!(result, Err(CommandError::WorkingSetDenied(_))));
        assert_eq!(state, before);
    }

    #[test]
    fn missing_required_field_produces_stable_diagnostic() {
        let mut executor = executor();
        let mut state = EngineState::default();
        let result = executor.execute(
            CommandRequest {
                command: ToolId::from_static("document.create"),
                input: json!({"id": "doc.test"}),
                dry_run: false,
            },
            &context(),
            &mut state,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::ValidationFailed(bag) => {
                let diag = &bag.diagnostics[0];
                assert_eq!(diag.code, codes::COMMAND_INPUT_REQUIRED);
                assert_eq!(
                    diag.location.as_ref().unwrap().field_path.as_deref(),
                    Some("/type_id")
                );
            }
            other => panic!("expected ValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn wrong_type_produces_stable_diagnostic() {
        let mut executor = executor();
        let mut state = EngineState::default();
        let result = executor.execute(
            CommandRequest {
                command: ToolId::from_static("document.create"),
                input: json!({
                    "id": 42,
                    "type_id": "example.document",
                    "display_name": "Test"
                }),
                dry_run: false,
            },
            &context(),
            &mut state,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::ValidationFailed(bag) => {
                let diag = &bag.diagnostics[0];
                assert_eq!(diag.code, codes::COMMAND_INPUT_TYPE_MISMATCH);
                assert_eq!(
                    diag.location.as_ref().unwrap().field_path.as_deref(),
                    Some("/id")
                );
            }
            other => panic!("expected ValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn default_normalization_applies_schema_version() {
        let mut executor = executor();
        let mut state = EngineState::default();
        executor
            .execute(
                CommandRequest {
                    command: ToolId::from_static("document.create"),
                    input: json!({
                        "id": "doc.norm",
                        "type_id": "example.document",
                        "display_name": "Normalized"
                    }),
                    dry_run: false,
                },
                &context(),
                &mut state,
            )
            .unwrap();
        let doc = state
            .documents
            .get(&DocumentId::from_static("doc.norm"))
            .unwrap();
        assert_eq!(doc.header.schema_version, 1);
    }

    #[test]
    fn structural_failure_does_not_consume_transaction() {
        let mut executor = executor();
        let mut state = EngineState::default();
        let tx_before = executor.next_transaction;
        let _ = executor.execute(
            CommandRequest {
                command: ToolId::from_static("document.create"),
                input: json!({"missing": true}),
                dry_run: false,
            },
            &context(),
            &mut state,
        );
        assert_eq!(executor.next_transaction, tx_before);
        assert!(executor.audit_log().is_empty());
    }

    #[test]
    fn empty_path_rejected_structurally() {
        let mut executor = executor();
        let mut state = EngineState::default();
        create_document(&mut executor, &mut state, "doc.path");
        let result = executor.execute(
            CommandRequest {
                command: ToolId::from_static("document.set_field"),
                input: json!({"document_id": "doc.path", "path": [], "value": 1}),
                dry_run: false,
            },
            &context(),
            &mut state,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::ValidationFailed(bag) => {
                assert!(bag.diagnostics.iter().any(|d| {
                    d.code == codes::COMMAND_INPUT_CONSTRAINT_FAILED
                        && d.location.as_ref().unwrap().field_path.as_deref() == Some("/path")
                }));
            }
            other => panic!("expected ValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn descriptor_input_schema_generated_from_authority() {
        let executor = executor();
        let descriptors = executor.registry().descriptors();
        let create = descriptors
            .iter()
            .find(|d| d.id == ToolId::from_static("document.create"))
            .unwrap();
        assert_eq!(create.input_schema["x-schema-version"], 1);
        assert_eq!(create.input_schema["type"], "object");
        assert!(create.input_schema["properties"]["id"]["type"] == "string");
    }

    #[test]
    fn document_create_null_schema_version_rejected_structurally() {
        let mut executor = executor();
        let mut state = EngineState::default();
        let tx_before = executor.next_transaction;
        let result = executor.execute(
            CommandRequest {
                command: ToolId::from_static("document.create"),
                input: json!({
                    "id": "doc.null",
                    "type_id": "example.document",
                    "display_name": "Null Test",
                    "schema_version": null
                }),
                dry_run: false,
            },
            &context(),
            &mut state,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::ValidationFailed(bag) => {
                let diag = &bag.diagnostics[0];
                assert_eq!(diag.code, "COMMAND_INPUT_NULL_NOT_ALLOWED");
                assert_eq!(
                    diag.location.as_ref().unwrap().field_path.as_deref(),
                    Some("/schema_version")
                );
            }
            other => panic!("expected ValidationFailed, got {other:?}"),
        }
        assert_eq!(executor.next_transaction, tx_before);
        assert!(executor.audit_log().is_empty());
        assert!(state.documents.is_empty());
    }

    #[test]
    fn document_create_defaults_from_schema_not_serde() {
        let mut executor = executor();
        let mut state = EngineState::default();
        executor
            .execute(
                CommandRequest {
                    command: ToolId::from_static("document.create"),
                    input: json!({
                        "id": "doc.defaults",
                        "type_id": "example.document",
                        "display_name": "Defaults Test"
                    }),
                    dry_run: false,
                },
                &context(),
                &mut state,
            )
            .unwrap();
        let doc = state
            .documents
            .get(&DocumentId::from_static("doc.defaults"))
            .unwrap();
        assert_eq!(doc.header.schema_version, 1);
        assert!(doc.payload.is_null());
    }

    #[test]
    fn normalized_input_seen_identically_by_all_stages() {
        use std::sync::{Arc, Mutex};

        #[derive(Default)]
        struct ObservedInputs {
            access: Option<Value>,
            validate: Option<Value>,
            apply: Option<Value>,
        }

        struct ObservingCommand {
            observed: Arc<Mutex<ObservedInputs>>,
        }

        impl CommandHandler for ObservingCommand {
            fn descriptor(&self) -> CommandDescriptor {
                CommandDescriptor {
                    id: ToolId::from_static("test.observe"),
                    title: "Observe".into(),
                    description: "Test".into(),
                    input_schema: Value::Null,
                    output_schema: json!({"type": "object"}),
                    required_permissions: PermissionSet::new([Permission::EditContent]),
                    side_effects: vec!["modifies_document".into()],
                    related_tools: Vec::new(),
                }
            }

            fn input_schema(&self) -> SchemaNode {
                SchemaNode::object(vec![
                    guiyi_engine_schema::FieldSchema::required("name", SchemaNode::string()),
                    guiyi_engine_schema::FieldSchema::optional(
                        "count",
                        SchemaNode::integer().with_default(json!(5)),
                    ),
                ])
            }

            fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
                self.observed.lock().unwrap().access = Some(input.clone());
                Ok(DocumentAccessPlan::project())
            }

            fn validate(&self, input: &Value, _state: &EngineState) -> DiagnosticBag {
                self.observed.lock().unwrap().validate = Some(input.clone());
                DiagnosticBag::default()
            }

            fn apply(
                &self,
                input: &Value,
                _state: &mut EngineState,
            ) -> Result<Value, CommandError> {
                self.observed.lock().unwrap().apply = Some(input.clone());
                Ok(json!({}))
            }
        }

        let observed = Arc::new(Mutex::new(ObservedInputs::default()));
        let handler = ObservingCommand {
            observed: observed.clone(),
        };
        let mut registry = CommandRegistry::default();
        registry.register(handler).unwrap();
        let mut executor = CommandExecutor::new(registry);
        let mut state = EngineState::default();

        let request = CommandRequest {
            command: ToolId::from_static("test.observe"),
            input: json!({"name": "test"}),
            dry_run: false,
        };

        executor.document_access(&request).unwrap();
        executor.execute(request, &context(), &mut state).unwrap();

        let obs = observed.lock().unwrap();
        let expected = json!({"name": "test", "count": 5});
        assert_eq!(obs.access.as_ref().unwrap(), &expected, "access mismatch");
        assert_eq!(
            obs.validate.as_ref().unwrap(),
            &expected,
            "validate mismatch"
        );
        assert_eq!(obs.apply.as_ref().unwrap(), &expected, "apply mismatch");
    }
}
