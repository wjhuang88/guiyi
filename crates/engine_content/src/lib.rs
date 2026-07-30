#![forbid(unsafe_code)]

//! Authoring documents, compiled artifacts, storage, and compiler registration.

use guiyi_engine_core::{
    deterministic_hash, ArtifactId, DocumentId, EngineTypeId, ObjectId, ProjectId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentHeader {
    pub id: DocumentId,
    pub type_id: EngineTypeId,
    pub schema_version: u32,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentReference {
    pub source_object: Option<ObjectId>,
    pub target_document: DocumentId,
    pub target_object: Option<ObjectId>,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentEnvelope {
    pub header: DocumentHeader,
    #[serde(default)]
    pub references: Vec<ContentReference>,
    pub payload: Value,
}

impl DocumentEnvelope {
    pub fn content_hash(&self) -> Result<String, ContentError> {
        let bytes = serde_json::to_vec(self)?;
        Ok(deterministic_hash(&bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeObjectDescriptor {
    pub id: ObjectId,
    pub type_id: EngineTypeId,
    pub properties: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageArtifactPayload {
    pub objects: Vec<RuntimeObjectDescriptor>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactEnvelope {
    pub id: ArtifactId,
    pub artifact_type: EngineTypeId,
    pub source_document: DocumentId,
    pub compiler_version: u32,
    pub source_hash: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub project_id: ProjectId,
    pub name: String,
    pub engine_api_version: String,
    pub content_schema_version: u32,
    #[serde(default)]
    pub enabled_extensions: Vec<String>,
    #[serde(default)]
    pub documents: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DocumentStore {
    documents: BTreeMap<DocumentId, DocumentEnvelope>,
}

impl DocumentStore {
    pub fn insert(&mut self, document: DocumentEnvelope) -> Result<(), ContentError> {
        let id = document.header.id.clone();
        if self.documents.contains_key(&id) {
            return Err(ContentError::DuplicateDocument(id));
        }
        self.documents.insert(id, document);
        Ok(())
    }

    pub fn upsert(&mut self, document: DocumentEnvelope) {
        self.documents.insert(document.header.id.clone(), document);
    }

    pub fn get(&self, id: &DocumentId) -> Result<&DocumentEnvelope, ContentError> {
        self.documents
            .get(id)
            .ok_or_else(|| ContentError::DocumentNotFound(id.clone()))
    }

    pub fn get_mut(&mut self, id: &DocumentId) -> Result<&mut DocumentEnvelope, ContentError> {
        self.documents
            .get_mut(id)
            .ok_or_else(|| ContentError::DocumentNotFound(id.clone()))
    }

    pub fn remove(&mut self, id: &DocumentId) -> Result<DocumentEnvelope, ContentError> {
        self.documents
            .remove(id)
            .ok_or_else(|| ContentError::DocumentNotFound(id.clone()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&DocumentId, &DocumentEnvelope)> {
        self.documents.iter()
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct CompileContext {
    pub project_root: PathBuf,
    pub profile: String,
}

pub trait DocumentCompiler: Send + Sync {
    fn id(&self) -> &'static str;
    fn source_type(&self) -> EngineTypeId;
    fn artifact_type(&self) -> EngineTypeId;
    fn compile(
        &self,
        document: &DocumentEnvelope,
        context: &CompileContext,
    ) -> Result<ArtifactEnvelope, ContentError>;
}

#[derive(Default)]
pub struct CompilerRegistry {
    compilers: BTreeMap<EngineTypeId, Box<dyn DocumentCompiler>>,
}

impl CompilerRegistry {
    pub fn register(
        &mut self,
        compiler: impl DocumentCompiler + 'static,
    ) -> Result<(), ContentError> {
        let source = compiler.source_type();
        if self.compilers.contains_key(&source) {
            return Err(ContentError::DuplicateCompiler(source));
        }
        self.compilers.insert(source, Box::new(compiler));
        Ok(())
    }

    pub fn compile(
        &self,
        document: &DocumentEnvelope,
        context: &CompileContext,
    ) -> Result<ArtifactEnvelope, ContentError> {
        let compiler = self
            .compilers
            .get(&document.header.type_id)
            .ok_or_else(|| ContentError::CompilerNotFound(document.header.type_id.clone()))?;
        compiler.compile(document, context)
    }
}

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("duplicate document: {0}")]
    DuplicateDocument(DocumentId),
    #[error("document not found: {0}")]
    DocumentNotFound(DocumentId),
    #[error("compiler already registered for: {0}")]
    DuplicateCompiler(EngineTypeId),
    #[error("compiler not found for: {0}")]
    CompilerNotFound(EngineTypeId),
    #[error("invalid document: {0}")]
    InvalidDocument(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub fn save_json(path: &Path, value: &impl Serialize) -> Result<(), ContentError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

pub fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ContentError> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document() -> DocumentEnvelope {
        DocumentEnvelope {
            header: DocumentHeader {
                id: DocumentId::from_static("doc.demo"),
                type_id: EngineTypeId::from_static("example.document"),
                schema_version: 1,
                display_name: "Demo".into(),
            },
            references: Vec::new(),
            payload: json!({"value": 1}),
        }
    }

    #[test]
    fn store_rejects_duplicate_documents() {
        let mut store = DocumentStore::default();
        store.insert(document()).unwrap();
        assert!(matches!(
            store.insert(document()),
            Err(ContentError::DuplicateDocument(_))
        ));
    }

    #[test]
    fn document_hash_changes_with_payload() {
        let first = document();
        let mut second = document();
        second.payload["value"] = json!(2);
        assert_ne!(
            first.content_hash().unwrap(),
            second.content_hash().unwrap()
        );
    }
}
