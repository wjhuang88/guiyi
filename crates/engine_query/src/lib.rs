#![forbid(unsafe_code)]

//! Structured project queries and a semantic reference graph.

use guiyi_engine_content::DocumentStore;
use guiyi_engine_core::{DocumentId, ObjectId, Permission, PermissionSet, ToolId};
use guiyi_engine_validation::DiagnosticBag;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryDescriptor {
    pub id: ToolId,
    pub title: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub required_permissions: PermissionSet,
    pub related_tools: Vec<ToolId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRequest {
    pub query: ToolId,
    pub input: Value,
}

#[derive(Debug, Clone)]
pub struct QueryContext {
    pub actor: String,
    pub permissions: PermissionSet,
}

pub trait QueryHandler: Send + Sync {
    fn descriptor(&self) -> QueryDescriptor;
    fn execute(&self, input: &Value, store: &DocumentStore) -> Result<Value, QueryError>;
}

#[derive(Default)]
pub struct QueryRegistry {
    handlers: BTreeMap<ToolId, Box<dyn QueryHandler>>,
}

impl QueryRegistry {
    pub fn register(&mut self, handler: impl QueryHandler + 'static) -> Result<(), QueryError> {
        let id = handler.descriptor().id;
        if self.handlers.contains_key(&id) {
            return Err(QueryError::DuplicateQuery(id));
        }
        self.handlers.insert(id, Box::new(handler));
        Ok(())
    }

    pub fn handler(&self, id: &ToolId) -> Result<&dyn QueryHandler, QueryError> {
        self.handlers
            .get(id)
            .map(Box::as_ref)
            .ok_or_else(|| QueryError::QueryNotFound(id.clone()))
    }

    pub fn descriptors(&self) -> Vec<QueryDescriptor> {
        self.handlers
            .values()
            .map(|item| item.descriptor())
            .collect()
    }
}

pub struct QueryExecutor {
    registry: QueryRegistry,
}

impl QueryExecutor {
    pub fn new(registry: QueryRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &QueryRegistry {
        &self.registry
    }

    pub fn execute(
        &self,
        request: QueryRequest,
        context: &QueryContext,
        store: &DocumentStore,
    ) -> Result<Value, QueryError> {
        let handler = self.registry.handler(&request.query)?;
        let descriptor = handler.descriptor();
        if !context
            .permissions
            .contains_all(&descriptor.required_permissions)
        {
            return Err(QueryError::PermissionDenied(request.query));
        }
        handler.execute(&request.input, store)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectGraph {
    pub outgoing: BTreeMap<DocumentId, BTreeSet<DocumentId>>,
    pub incoming: BTreeMap<DocumentId, BTreeSet<DocumentId>>,
}

impl ProjectGraph {
    pub fn build(store: &DocumentStore) -> Self {
        let mut graph = Self::default();
        for (id, document) in store.iter() {
            graph.outgoing.entry(id.clone()).or_default();
            graph.incoming.entry(id.clone()).or_default();
            for reference in &document.references {
                graph
                    .outgoing
                    .entry(id.clone())
                    .or_default()
                    .insert(reference.target_document.clone());
                graph
                    .incoming
                    .entry(reference.target_document.clone())
                    .or_default()
                    .insert(id.clone());
            }
        }
        graph
    }

    pub fn impact(&self, target: &DocumentId) -> BTreeSet<DocumentId> {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::from([target.clone()]);
        while let Some(current) = queue.pop_front() {
            if let Some(incoming) = self.incoming.get(&current) {
                for dependent in incoming {
                    if visited.insert(dependent.clone()) {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }
        visited
    }
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("duplicate query: {0}")]
    DuplicateQuery(ToolId),
    #[error("query not found: {0}")]
    QueryNotFound(ToolId),
    #[error("permission denied for query: {0}")]
    PermissionDenied(ToolId),
    #[error("invalid query input: {0}")]
    InvalidInput(String),
    #[error("query failed: {0}")]
    Failed(String),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

pub struct ListDocumentsQuery;

impl QueryHandler for ListDocumentsQuery {
    fn descriptor(&self) -> QueryDescriptor {
        QueryDescriptor {
            id: ToolId::from_static("project.documents.list"),
            title: "List documents".into(),
            description: "List all loaded authoring documents.".into(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "array"}),
            required_permissions: PermissionSet::new([Permission::Read]),
            related_tools: vec![ToolId::from_static("project.document.get")],
        }
    }

    fn execute(&self, _input: &Value, store: &DocumentStore) -> Result<Value, QueryError> {
        Ok(Value::Array(
            store
                .iter()
                .map(|(_, document)| {
                    json!({
                        "id": document.header.id,
                        "type_id": document.header.type_id,
                        "display_name": document.header.display_name,
                        "schema_version": document.header.schema_version
                    })
                })
                .collect(),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct DocumentInput {
    document_id: DocumentId,
}

pub struct GetDocumentQuery;

impl QueryHandler for GetDocumentQuery {
    fn descriptor(&self) -> QueryDescriptor {
        QueryDescriptor {
            id: ToolId::from_static("project.document.get"),
            title: "Get document".into(),
            description: "Return one complete authoring document.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["document_id"],
                "properties": {"document_id": {"type": "string"}}
            }),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::Read]),
            related_tools: vec![ToolId::from_static("project.references.find")],
        }
    }

    fn execute(&self, input: &Value, store: &DocumentStore) -> Result<Value, QueryError> {
        let input: DocumentInput = serde_json::from_value(input.clone())?;
        let document = store
            .get(&input.document_id)
            .map_err(|error| QueryError::Failed(error.to_string()))?;
        Ok(serde_json::to_value(document)?)
    }
}

#[derive(Debug, Deserialize)]
struct ReferenceInput {
    target_document: DocumentId,
    #[serde(default)]
    target_object: Option<ObjectId>,
}

pub struct FindReferencesQuery;

impl QueryHandler for FindReferencesQuery {
    fn descriptor(&self) -> QueryDescriptor {
        QueryDescriptor {
            id: ToolId::from_static("project.references.find"),
            title: "Find references".into(),
            description: "Find semantic references without scanning source text.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["target_document"],
                "properties": {
                    "target_document": {"type": "string"},
                    "target_object": {"type": ["string", "null"]}
                }
            }),
            output_schema: json!({"type": "array"}),
            required_permissions: PermissionSet::new([Permission::Read]),
            related_tools: vec![ToolId::from_static("project.impact.analyze")],
        }
    }

    fn execute(&self, input: &Value, store: &DocumentStore) -> Result<Value, QueryError> {
        let input: ReferenceInput = serde_json::from_value(input.clone())?;
        let matches = store
            .iter()
            .flat_map(|(source, document)| {
                document.references.iter().filter_map(move |reference| {
                    let object_match = input.target_object.is_none()
                        || input.target_object == reference.target_object;
                    if reference.target_document == input.target_document && object_match {
                        Some(json!({
                            "source_document": source,
                            "source_object": reference.source_object,
                            "kind": reference.kind,
                            "target_object": reference.target_object
                        }))
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<_>>();
        Ok(Value::Array(matches))
    }
}

pub struct ImpactQuery;

impl QueryHandler for ImpactQuery {
    fn descriptor(&self) -> QueryDescriptor {
        QueryDescriptor {
            id: ToolId::from_static("project.impact.analyze"),
            title: "Analyze impact".into(),
            description: "Return all transitively dependent documents.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["document_id"],
                "properties": {"document_id": {"type": "string"}}
            }),
            output_schema: json!({"type": "array"}),
            required_permissions: PermissionSet::new([Permission::Read]),
            related_tools: vec![ToolId::from_static("project.references.find")],
        }
    }

    fn execute(&self, input: &Value, store: &DocumentStore) -> Result<Value, QueryError> {
        let input: DocumentInput = serde_json::from_value(input.clone())?;
        let graph = ProjectGraph::build(store);
        Ok(serde_json::to_value(graph.impact(&input.document_id))?)
    }
}

pub fn register_builtin_queries(registry: &mut QueryRegistry) -> Result<(), QueryError> {
    registry.register(ListDocumentsQuery)?;
    registry.register(GetDocumentQuery)?;
    registry.register(FindReferencesQuery)?;
    registry.register(ImpactQuery)?;
    Ok(())
}

pub fn query_diagnostics(_store: &DocumentStore) -> DiagnosticBag {
    DiagnosticBag::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_content::{ContentReference, DocumentEnvelope, DocumentHeader};
    use guiyi_engine_core::EngineTypeId;

    #[test]
    fn impact_follows_reverse_references() {
        let mut store = DocumentStore::default();
        for (id, target) in [
            ("doc.a", None),
            ("doc.b", Some("doc.a")),
            ("doc.c", Some("doc.b")),
        ] {
            store
                .insert(DocumentEnvelope {
                    header: DocumentHeader {
                        id: DocumentId::new(id).unwrap(),
                        type_id: EngineTypeId::from_static("example"),
                        schema_version: 1,
                        display_name: id.into(),
                    },
                    references: target
                        .map(|value| {
                            vec![ContentReference {
                                source_object: None,
                                target_document: DocumentId::new(value).unwrap(),
                                target_object: None,
                                kind: "test".into(),
                            }]
                        })
                        .unwrap_or_default(),
                    payload: json!({}),
                })
                .unwrap();
        }
        let graph = ProjectGraph::build(&store);
        let impact = graph.impact(&DocumentId::from_static("doc.a"));
        assert!(impact.contains(&DocumentId::from_static("doc.b")));
        assert!(impact.contains(&DocumentId::from_static("doc.c")));
    }
}
