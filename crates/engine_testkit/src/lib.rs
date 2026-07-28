#![forbid(unsafe_code)]

//! Reusable builders for engine integration tests and examples.

use guiyi_engine_command::{register_builtin_document_commands, CommandExecutor, CommandRegistry};
use guiyi_engine_content::{DocumentEnvelope, DocumentHeader};
use guiyi_engine_core::{DocumentId, EngineTypeId, PermissionSet};
use guiyi_engine_query::{register_builtin_queries, QueryExecutor, QueryRegistry};
use serde_json::{json, Value};

pub fn sample_document(id: &str, type_id: &str, payload: Value) -> DocumentEnvelope {
    DocumentEnvelope {
        header: DocumentHeader {
            id: DocumentId::new(id).expect("test document id is valid"),
            type_id: EngineTypeId::new(type_id).expect("test type id is valid"),
            schema_version: 1,
            display_name: id.into(),
        },
        references: Vec::new(),
        payload,
    }
}

pub fn content_author_permissions() -> PermissionSet {
    PermissionSet::content_author()
}

pub fn builtin_executors() -> (CommandExecutor, QueryExecutor) {
    let mut commands = CommandRegistry::default();
    register_builtin_document_commands(&mut commands).expect("built-in commands register");
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).expect("built-in queries register");
    (CommandExecutor::new(commands), QueryExecutor::new(queries))
}

pub fn empty_payload() -> Value {
    json!({})
}
