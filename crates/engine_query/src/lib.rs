#![forbid(unsafe_code)]

//! Structured project queries and an incremental semantic reference graph.

use guiyi_engine_content::{DocumentEnvelope, DocumentStore};
use guiyi_engine_core::{
    DocumentAccessPlan, DocumentId, ObjectId, Permission, PermissionSet, ToolId,
};
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

    /// Declares directly required documents and whether the query scans the
    /// caller-visible project view. Restricted sessions receive a filtered
    /// document store for project-scanning queries.
    fn document_access(&self, _input: &Value) -> Result<DocumentAccessPlan, QueryError> {
        Ok(DocumentAccessPlan::project())
    }

    fn execute(&self, input: &Value, store: &DocumentStore) -> Result<Value, QueryError>;

    /// Executes with the maintained semantic Project Graph when available.
    ///
    /// Custom Queries that do not need graph relationships inherit the legacy
    /// `execute` behavior. Graph-aware Queries override this method so normal
    /// `QueryExecutor` callers reuse the incremental index.
    fn execute_indexed(
        &self,
        input: &Value,
        store: &DocumentStore,
        _graph: &ProjectGraphIndex,
    ) -> Result<Value, QueryError> {
        self.execute(input, store)
    }
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
    project_graph: Option<ProjectGraphIndex>,
}

impl QueryExecutor {
    pub fn new(registry: QueryRegistry) -> Self {
        Self {
            registry,
            project_graph: None,
        }
    }

    pub fn registry(&self) -> &QueryRegistry {
        &self.registry
    }

    pub fn document_access(
        &self,
        request: &QueryRequest,
    ) -> Result<DocumentAccessPlan, QueryError> {
        self.registry
            .handler(&request.query)?
            .document_access(&request.input)
    }

    /// Drop the derived graph so the next graph-backed query rebuilds it from
    /// authoritative Documents.
    pub fn invalidate_project_graph(&mut self) {
        self.project_graph = None;
    }

    /// Rebuild the derived graph explicitly from the complete current store.
    pub fn rebuild_project_graph(&mut self, store: &DocumentStore) {
        self.project_graph = Some(ProjectGraphIndex::build(store));
    }

    /// Refresh only changed source Documents in an already-initialized graph.
    ///
    /// If no graph-backed query has initialized the cache yet, this remains a
    /// no-op; the first later graph-backed query will build once from the then
    /// authoritative store.
    pub fn refresh_project_graph<'a, I>(&mut self, store: &DocumentStore, document_ids: I)
    where
        I: IntoIterator<Item = &'a DocumentId>,
    {
        if let Some(graph) = self.project_graph.as_mut() {
            graph.refresh_documents(store, document_ids);
        }
    }

    /// Return a reusable serializable graph snapshot, lazily initializing the
    /// index from authoritative Documents if necessary.
    pub fn project_graph_snapshot(&mut self, store: &DocumentStore) -> ProjectGraph {
        let graph = self
            .project_graph
            .get_or_insert_with(|| ProjectGraphIndex::build(store));
        graph.snapshot()
    }

    pub fn execute(
        &mut self,
        request: QueryRequest,
        context: &QueryContext,
        store: &DocumentStore,
    ) -> Result<Value, QueryError> {
        let descriptor = self.registry.handler(&request.query)?.descriptor();
        if !context
            .permissions
            .contains_all(&descriptor.required_permissions)
        {
            return Err(QueryError::PermissionDenied(request.query));
        }

        let graph = self
            .project_graph
            .get_or_insert_with(|| ProjectGraphIndex::build(store));
        let handler = self.registry.handler(&request.query)?;
        handler.execute_indexed(&request.input, store, graph)
    }
}

/// Serializable document-level Project Graph snapshot.
///
/// This remains the lightweight compatibility surface for callers that only
/// need document adjacency and transitive impact. `ProjectGraphIndex` owns the
/// incremental reference metadata used by normal query execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectGraph {
    pub outgoing: BTreeMap<DocumentId, BTreeSet<DocumentId>>,
    pub incoming: BTreeMap<DocumentId, BTreeSet<DocumentId>>,
}

impl ProjectGraph {
    /// Build a one-shot snapshot. Long-lived query execution should prefer
    /// `ProjectGraphIndex` / `QueryExecutor` so relationships can be refreshed
    /// incrementally.
    pub fn build(store: &DocumentStore) -> Self {
        ProjectGraphIndex::build(store).snapshot()
    }

    pub fn dependencies(&self, source: &DocumentId) -> BTreeSet<DocumentId> {
        self.outgoing.get(source).cloned().unwrap_or_default()
    }

    pub fn dependents(&self, target: &DocumentId) -> BTreeSet<DocumentId> {
        self.incoming.get(target).cloned().unwrap_or_default()
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

/// One semantic reference edge retained by the incremental Project Graph.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProjectReference {
    pub source_document: DocumentId,
    pub source_object: Option<ObjectId>,
    pub target_document: DocumentId,
    pub target_object: Option<ObjectId>,
    pub kind: String,
}

/// Incremental semantic index derived exclusively from `DocumentEnvelope.references`.
///
/// The index is cache state, not authoring truth. It is deliberately not
/// persisted; callers rebuild it from Documents on project open and refresh
/// changed source Documents after successful applied transactions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectGraphIndex {
    graph: ProjectGraph,
    documents: BTreeSet<DocumentId>,
    outgoing_references: BTreeMap<DocumentId, Vec<ProjectReference>>,
    incoming_references: BTreeMap<DocumentId, Vec<ProjectReference>>,
}

impl ProjectGraphIndex {
    pub fn build(store: &DocumentStore) -> Self {
        let mut index = Self::default();
        for (_, document) in store.iter() {
            index.upsert_document(document);
        }
        index
    }

    pub fn snapshot(&self) -> ProjectGraph {
        self.graph.clone()
    }

    pub fn dependencies(&self, source: &DocumentId) -> BTreeSet<DocumentId> {
        self.graph.dependencies(source)
    }

    pub fn dependents(&self, target: &DocumentId) -> BTreeSet<DocumentId> {
        self.graph.dependents(target)
    }

    pub fn references_from(&self, source: &DocumentId) -> Vec<ProjectReference> {
        self.outgoing_references
            .get(source)
            .cloned()
            .unwrap_or_default()
    }

    pub fn references_to(
        &self,
        target: &DocumentId,
        target_object: Option<&ObjectId>,
    ) -> Vec<ProjectReference> {
        self.incoming_references
            .get(target)
            .into_iter()
            .flatten()
            .filter(|reference| match target_object {
                Some(object) => reference.target_object.as_ref() == Some(object),
                None => true,
            })
            .cloned()
            .collect()
    }

    /// Return references whose source Document is present in the caller-visible
    /// store. Hidden Documents are never exposed by a restricted project view.
    pub fn references_to_visible(
        &self,
        target: &DocumentId,
        target_object: Option<&ObjectId>,
        visible_store: &DocumentStore,
    ) -> Vec<ProjectReference> {
        self.references_to(target, target_object)
            .into_iter()
            .filter(|reference| visible_store.get(&reference.source_document).is_ok())
            .collect()
    }

    pub fn impact(&self, target: &DocumentId) -> BTreeSet<DocumentId> {
        self.graph.impact(target)
    }

    /// Compute transitive impact inside one caller-visible project view.
    ///
    /// Hidden Documents are neither returned nor traversed as intermediate
    /// nodes, preventing a restricted working set from learning hidden paths.
    pub fn impact_visible(
        &self,
        target: &DocumentId,
        visible_store: &DocumentStore,
    ) -> BTreeSet<DocumentId> {
        if visible_store.get(target).is_err() {
            return BTreeSet::new();
        }
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::from([target.clone()]);
        while let Some(current) = queue.pop_front() {
            if let Some(incoming) = self.graph.incoming.get(&current) {
                for dependent in incoming {
                    if visible_store.get(dependent).is_err() {
                        continue;
                    }
                    if visited.insert(dependent.clone()) {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }
        visited
    }

    pub fn refresh_document(&mut self, store: &DocumentStore, document_id: &DocumentId) {
        match store.get(document_id) {
            Ok(document) => self.upsert_document(document),
            Err(_) => self.remove_document(document_id),
        }
    }

    pub fn refresh_documents<'a, I>(&mut self, store: &DocumentStore, document_ids: I)
    where
        I: IntoIterator<Item = &'a DocumentId>,
    {
        for document_id in document_ids {
            self.refresh_document(store, document_id);
        }
    }

    fn upsert_document(&mut self, document: &DocumentEnvelope) {
        let source = document.header.id.clone();
        self.remove_source_edges(&source);
        self.documents.insert(source.clone());
        self.graph.outgoing.entry(source.clone()).or_default();
        self.graph.incoming.entry(source.clone()).or_default();

        let mut outgoing = document
            .references
            .iter()
            .map(|reference| ProjectReference {
                source_document: source.clone(),
                source_object: reference.source_object.clone(),
                target_document: reference.target_document.clone(),
                target_object: reference.target_object.clone(),
                kind: reference.kind.clone(),
            })
            .collect::<Vec<_>>();
        outgoing.sort();

        for reference in &outgoing {
            self.graph
                .outgoing
                .entry(source.clone())
                .or_default()
                .insert(reference.target_document.clone());
            self.graph
                .incoming
                .entry(reference.target_document.clone())
                .or_default()
                .insert(source.clone());
            let incoming = self
                .incoming_references
                .entry(reference.target_document.clone())
                .or_default();
            incoming.push(reference.clone());
            incoming.sort();
        }
        self.outgoing_references.insert(source, outgoing);
    }

    fn remove_document(&mut self, document_id: &DocumentId) {
        self.remove_source_edges(document_id);
        self.documents.remove(document_id);
        self.graph.outgoing.remove(document_id);
        if self
            .graph
            .incoming
            .get(document_id)
            .is_some_and(BTreeSet::is_empty)
        {
            self.graph.incoming.remove(document_id);
        }
    }

    fn remove_source_edges(&mut self, source: &DocumentId) {
        let targets = self.graph.outgoing.remove(source).unwrap_or_default();
        for target in targets {
            let remove_incoming_node = if let Some(sources) = self.graph.incoming.get_mut(&target) {
                sources.remove(source);
                sources.is_empty() && !self.documents.contains(&target)
            } else {
                false
            };
            if remove_incoming_node {
                self.graph.incoming.remove(&target);
            }
        }

        if let Some(references) = self.outgoing_references.remove(source) {
            let targets = references
                .iter()
                .map(|reference| reference.target_document.clone())
                .collect::<BTreeSet<_>>();
            for target in targets {
                let remove_target = if let Some(incoming) = self.incoming_references.get_mut(&target)
                {
                    incoming.retain(|reference| &reference.source_document != source);
                    incoming.is_empty()
                } else {
                    false
                };
                if remove_target {
                    self.incoming_references.remove(&target);
                }
            }
        }
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

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, QueryError> {
        let input: DocumentInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.document_id))
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

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, QueryError> {
        let input: ReferenceInput = serde_json::from_value(input.clone())?;
        let mut plan = DocumentAccessPlan::document(input.target_document);
        plan.scans_project = true;
        Ok(plan)
    }

    fn execute(&self, input: &Value, store: &DocumentStore) -> Result<Value, QueryError> {
        let ReferenceInput {
            target_document,
            target_object,
        } = serde_json::from_value(input.clone())?;
        let matches = store
            .iter()
            .flat_map(|(source, document)| {
                let target_document = target_document.clone();
                let target_object = target_object.clone();
                document.references.iter().filter_map(move |reference| {
                    let object_match =
                        target_object.is_none() || target_object == reference.target_object;
                    if reference.target_document == target_document && object_match {
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

    fn execute_indexed(
        &self,
        input: &Value,
        store: &DocumentStore,
        graph: &ProjectGraphIndex,
    ) -> Result<Value, QueryError> {
        let ReferenceInput {
            target_document,
            target_object,
        } = serde_json::from_value(input.clone())?;
        let matches = graph
            .references_to_visible(&target_document, target_object.as_ref(), store)
            .into_iter()
            .map(|reference| {
                json!({
                    "source_document": reference.source_document,
                    "source_object": reference.source_object,
                    "kind": reference.kind,
                    "target_object": reference.target_object
                })
            })
            .collect();
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

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, QueryError> {
        let input: DocumentInput = serde_json::from_value(input.clone())?;
        let mut plan = DocumentAccessPlan::document(input.document_id);
        plan.scans_project = true;
        Ok(plan)
    }

    fn execute(&self, input: &Value, store: &DocumentStore) -> Result<Value, QueryError> {
        let input: DocumentInput = serde_json::from_value(input.clone())?;
        let graph = ProjectGraph::build(store);
        Ok(serde_json::to_value(graph.impact(&input.document_id))?)
    }

    fn execute_indexed(
        &self,
        input: &Value,
        store: &DocumentStore,
        graph: &ProjectGraphIndex,
    ) -> Result<Value, QueryError> {
        let input: DocumentInput = serde_json::from_value(input.clone())?;
        Ok(serde_json::to_value(
            graph.impact_visible(&input.document_id, store),
        )?)
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

    fn document(id: &'static str, target: Option<&'static str>) -> DocumentEnvelope {
        DocumentEnvelope {
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
        }
    }

    fn executor() -> QueryExecutor {
        let mut registry = QueryRegistry::default();
        register_builtin_queries(&mut registry).unwrap();
        QueryExecutor::new(registry)
    }

    #[test]
    fn targeted_queries_declare_document_access() {
        let executor = executor();
        let get = executor
            .document_access(&QueryRequest {
                query: ToolId::from_static("project.document.get"),
                input: json!({"document_id": "doc.a"}),
            })
            .unwrap();
        assert_eq!(
            get.required,
            BTreeSet::from([DocumentId::from_static("doc.a")])
        );
        assert!(!get.scans_project);
        let impact = executor
            .document_access(&QueryRequest {
                query: ToolId::from_static("project.impact.analyze"),
                input: json!({"document_id": "doc.a"}),
            })
            .unwrap();
        assert!(impact.scans_project);
    }

    #[test]
    fn impact_follows_reverse_references() {
        let mut store = DocumentStore::default();
        for (id, target) in [
            ("doc.a", None),
            ("doc.b", Some("doc.a")),
            ("doc.c", Some("doc.b")),
        ] {
            store.insert(document(id, target)).unwrap();
        }
        let graph = ProjectGraph::build(&store);
        let impact = graph.impact(&DocumentId::from_static("doc.a"));
        assert!(impact.contains(&DocumentId::from_static("doc.b")));
        assert!(impact.contains(&DocumentId::from_static("doc.c")));
    }

    #[test]
    fn incremental_index_refreshes_changed_sources_and_preserves_dangling_targets() {
        let mut store = DocumentStore::default();
        for (id, target) in [
            ("doc.a", None),
            ("doc.b", Some("doc.a")),
            ("doc.c", Some("doc.b")),
            ("doc.d", None),
        ] {
            store.insert(document(id, target)).unwrap();
        }
        let mut graph = ProjectGraphIndex::build(&store);
        assert_eq!(
            graph.impact(&DocumentId::from_static("doc.a")),
            BTreeSet::from([
                DocumentId::from_static("doc.b"),
                DocumentId::from_static("doc.c")
            ])
        );

        store
            .get_mut(&DocumentId::from_static("doc.b"))
            .unwrap()
            .references[0]
            .target_document = DocumentId::from_static("doc.d");
        graph.refresh_document(&store, &DocumentId::from_static("doc.b"));
        assert!(graph
            .impact(&DocumentId::from_static("doc.a"))
            .is_empty());
        assert_eq!(
            graph.impact(&DocumentId::from_static("doc.d")),
            BTreeSet::from([
                DocumentId::from_static("doc.b"),
                DocumentId::from_static("doc.c")
            ])
        );

        store.remove(&DocumentId::from_static("doc.b")).unwrap();
        graph.refresh_document(&store, &DocumentId::from_static("doc.b"));
        assert!(graph
            .impact(&DocumentId::from_static("doc.d"))
            .is_empty());
        assert_eq!(
            graph.dependents(&DocumentId::from_static("doc.b")),
            BTreeSet::from([DocumentId::from_static("doc.c")])
        );
    }

    #[test]
    fn query_executor_reuses_graph_until_explicit_incremental_refresh() {
        let mut store = DocumentStore::default();
        store.insert(document("doc.a", None)).unwrap();
        store.insert(document("doc.b", Some("doc.a"))).unwrap();
        store.insert(document("doc.c", None)).unwrap();
        let mut executor = executor();

        let initial = executor.project_graph_snapshot(&store);
        assert_eq!(
            initial.dependents(&DocumentId::from_static("doc.a")),
            BTreeSet::from([DocumentId::from_static("doc.b")])
        );

        store
            .get_mut(&DocumentId::from_static("doc.b"))
            .unwrap()
            .references[0]
            .target_document = DocumentId::from_static("doc.c");
        let reused = executor.project_graph_snapshot(&store);
        assert_eq!(reused, initial);

        let changed = DocumentId::from_static("doc.b");
        executor.refresh_project_graph(&store, [&changed]);
        let refreshed = executor.project_graph_snapshot(&store);
        assert!(refreshed
            .dependents(&DocumentId::from_static("doc.a"))
            .is_empty());
        assert_eq!(
            refreshed.dependents(&DocumentId::from_static("doc.c")),
            BTreeSet::from([DocumentId::from_static("doc.b")])
        );
    }

    #[test]
    fn indexed_queries_do_not_cross_hidden_working_set_nodes() {
        let mut store = DocumentStore::default();
        for (id, target) in [
            ("doc.a", None),
            ("doc.hidden", Some("doc.a")),
            ("doc.c", Some("doc.hidden")),
        ] {
            store.insert(document(id, target)).unwrap();
        }
        let mut executor = executor();
        let context = QueryContext {
            actor: "test".into(),
            permissions: PermissionSet::read_only(),
        };
        let full = executor
            .execute(
                QueryRequest {
                    query: ToolId::from_static("project.impact.analyze"),
                    input: json!({"document_id": "doc.a"}),
                },
                &context,
                &store,
            )
            .unwrap();
        assert_eq!(full, json!(["doc.c", "doc.hidden"]));

        let mut visible = DocumentStore::default();
        visible.upsert(store.get(&DocumentId::from_static("doc.a")).unwrap().clone());
        visible.upsert(store.get(&DocumentId::from_static("doc.c")).unwrap().clone());
        let restricted_impact = executor
            .execute(
                QueryRequest {
                    query: ToolId::from_static("project.impact.analyze"),
                    input: json!({"document_id": "doc.a"}),
                },
                &context,
                &visible,
            )
            .unwrap();
        assert_eq!(restricted_impact, json!([]));

        let restricted_references = executor
            .execute(
                QueryRequest {
                    query: ToolId::from_static("project.references.find"),
                    input: json!({"target_document": "doc.a"}),
                },
                &context,
                &visible,
            )
            .unwrap();
        assert_eq!(restricted_references, json!([]));
    }
}
