#![forbid(unsafe_code)]

//! Authoring documents, compiled artifacts, sandboxed project storage, and compiler registration.

use guiyi_engine_core::{
    deterministic_hash, ArtifactId, DocumentId, EngineTypeId, ObjectId, ProjectId,
};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

pub const PROJECT_PATH_INVALID: &str = "PROJECT_PATH_INVALID";
pub const PROJECT_PATH_ESCAPE: &str = "PROJECT_PATH_ESCAPE";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{code}: rejected project path `{path}`: {reason}")]
pub struct ProjectPathError {
    pub code: String,
    pub path: String,
    pub reason: String,
}

impl ProjectPathError {
    fn invalid(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            code: PROJECT_PATH_INVALID.into(),
            path: path.into(),
            reason: reason.into(),
        }
    }

    fn escape(path: &ProjectPath, reason: impl Into<String>) -> Self {
        Self {
            code: PROJECT_PATH_ESCAPE.into(),
            path: path.as_str().into(),
            reason: reason.into(),
        }
    }
}

/// A UTF-8, forward-slash, project-relative logical path.
///
/// Values are validated before they can enter a manifest or storage operation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ProjectPath(String);

impl ProjectPath {
    pub fn new(value: impl Into<String>) -> Result<Self, ProjectPathError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ProjectPathError::invalid(value, "path is empty"));
        }
        if value.contains('\0') {
            return Err(ProjectPathError::invalid(value, "path contains a NUL byte"));
        }
        if value.contains('\\') {
            return Err(ProjectPathError::invalid(
                value,
                "backslash separators and Windows path forms are not allowed",
            ));
        }
        if value.starts_with('/') || Path::new(&value).is_absolute() {
            return Err(ProjectPathError::invalid(value, "absolute paths are not allowed"));
        }
        let bytes = value.as_bytes();
        if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return Err(ProjectPathError::invalid(
                value,
                "Windows drive prefixes are not allowed",
            ));
        }
        if value.contains(':') {
            return Err(ProjectPathError::invalid(
                value,
                "platform path prefixes and alternate streams are not allowed",
            ));
        }
        for segment in value.split('/') {
            if segment.is_empty() {
                return Err(ProjectPathError::invalid(
                    value,
                    "empty path segments are not allowed",
                ));
            }
            if segment == "." {
                return Err(ProjectPathError::invalid(
                    value,
                    "current-directory segments are not allowed",
                ));
            }
            if segment == ".." {
                return Err(ProjectPathError::invalid(
                    value,
                    "parent-directory traversal is not allowed",
                ));
            }
        }
        if Path::new(&value)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(ProjectPathError::invalid(
                value,
                "only normal project-relative components are allowed",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }

    pub fn join(&self, child: impl AsRef<str>) -> Result<Self, ProjectPathError> {
        Self::new(format!("{}/{}", self.as_str(), child.as_ref()))
    }
}

impl TryFrom<PathBuf> for ProjectPath {
    type Error = ProjectPathError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let display = value.to_string_lossy().into_owned();
        let value = value
            .to_str()
            .ok_or_else(|| ProjectPathError::invalid(display, "path is not valid UTF-8"))?;
        Self::new(value)
    }
}

impl TryFrom<&Path> for ProjectPath {
    type Error = ProjectPathError;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        Self::try_from(value.to_path_buf())
    }
}

impl<'de> Deserialize<'de> for ProjectPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Filesystem access confined to one canonical project root.
#[derive(Debug, Clone)]
pub struct ProjectFilesystem {
    root: PathBuf,
}

impl ProjectFilesystem {
    pub fn create(root: impl AsRef<Path>) -> Result<Self, ContentError> {
        fs::create_dir_all(root.as_ref())?;
        Self::open(root)
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self, ContentError> {
        let root = fs::canonicalize(root.as_ref())?;
        if !root.is_dir() {
            return Err(ContentError::InvalidProjectRoot(root));
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn candidate(&self, path: &ProjectPath) -> PathBuf {
        self.root.join(path.as_path())
    }

    fn ensure_within_root(
        &self,
        logical: &ProjectPath,
        resolved: &Path,
    ) -> Result<(), ContentError> {
        if resolved.starts_with(&self.root) {
            Ok(())
        } else {
            Err(ProjectPathError::escape(
                logical,
                format!(
                    "resolved path `{}` is outside project root `{}`",
                    resolved.display(),
                    self.root.display()
                ),
            )
            .into())
        }
    }

    fn resolve_existing(&self, logical: &ProjectPath) -> Result<PathBuf, ContentError> {
        let candidate = self.candidate(logical);
        let resolved = fs::canonicalize(&candidate)?;
        self.ensure_within_root(logical, &resolved)?;
        Ok(resolved)
    }

    fn resolve_for_write(&self, logical: &ProjectPath) -> Result<PathBuf, ContentError> {
        let candidate = self.candidate(logical);
        let mut ancestor = candidate.as_path();
        while fs::symlink_metadata(ancestor).is_err() {
            ancestor = ancestor.parent().ok_or_else(|| {
                ProjectPathError::escape(logical, "path has no existing ancestor")
            })?;
        }
        let resolved_ancestor = fs::canonicalize(ancestor)?;
        self.ensure_within_root(logical, &resolved_ancestor)?;
        if fs::symlink_metadata(&candidate).is_ok() {
            let resolved = fs::canonicalize(&candidate)?;
            self.ensure_within_root(logical, &resolved)?;
        }
        Ok(candidate)
    }

    pub fn exists(&self, logical: &ProjectPath) -> Result<bool, ContentError> {
        let candidate = self.resolve_for_write(logical)?;
        Ok(fs::symlink_metadata(candidate).is_ok())
    }

    pub fn create_dir_all(&self, logical: &ProjectPath) -> Result<(), ContentError> {
        let path = self.resolve_for_write(logical)?;
        fs::create_dir_all(path)?;
        Ok(())
    }

    pub fn read(&self, logical: &ProjectPath) -> Result<Vec<u8>, ContentError> {
        Ok(fs::read(self.resolve_existing(logical)?)?)
    }

    pub fn load_json<T: for<'de> Deserialize<'de>>(
        &self,
        logical: &ProjectPath,
    ) -> Result<T, ContentError> {
        Ok(serde_json::from_slice(&self.read(logical)?)?)
    }

    pub fn write(&self, logical: &ProjectPath, bytes: impl AsRef<[u8]>) -> Result<(), ContentError> {
        let path = self.resolve_for_write(logical)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
        Ok(())
    }

    pub fn save_json(
        &self,
        logical: &ProjectPath,
        value: &impl Serialize,
    ) -> Result<(), ContentError> {
        self.write(logical, serde_json::to_vec_pretty(value)?)
    }

    pub fn remove_file(&self, logical: &ProjectPath) -> Result<(), ContentError> {
        fs::remove_file(self.resolve_existing(logical)?)?;
        Ok(())
    }

    pub fn rename(&self, from: &ProjectPath, to: &ProjectPath) -> Result<(), ContentError> {
        let from = self.resolve_existing(from)?;
        let to = self.resolve_for_write(to)?;
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(from, to)?;
        Ok(())
    }
}

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
    pub documents: Vec<ProjectPath>,
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
    #[error("invalid project root: {0}")]
    InvalidProjectRoot(PathBuf),
    #[error(transparent)]
    ProjectPath(#[from] ProjectPathError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Generic non-project JSON helper. Project data must use [`ProjectFilesystem`].
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

/// Generic non-project JSON helper. Project data must use [`ProjectFilesystem`].
pub fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ContentError> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    fn temporary_directory(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("guiyi-{name}-{}-{nonce}", std::process::id()))
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

    #[test]
    fn project_path_rejects_absolute_traversal_and_platform_forms() {
        for path in [
            "",
            "../../outside.json",
            "/absolute/outside.json",
            "C:\\outside.json",
            "C:/outside.json",
            "content/../outside.json",
            "content//outside.json",
            "content/./outside.json",
            "\\\\server\\share\\outside.json",
        ] {
            let error = ProjectPath::new(path).unwrap_err();
            assert_eq!(error.code, PROJECT_PATH_INVALID, "{path}");
            assert_eq!(error.path, path, "{path}");
        }
    }

    #[test]
    fn manifest_deserialization_rejects_unsafe_document_path() {
        let error = serde_json::from_value::<ProjectManifest>(json!({
            "project_id": "project.test",
            "name": "Test",
            "engine_api_version": "0.1.0",
            "content_schema_version": 1,
            "documents": ["../../outside.json"]
        }))
        .unwrap_err();
        assert!(error.to_string().contains(PROJECT_PATH_INVALID));
        assert!(error.to_string().contains("../../outside.json"));
    }

    #[test]
    fn project_filesystem_reads_writes_renames_and_deletes_inside_root() {
        let root = temporary_directory("storage");
        let storage = ProjectFilesystem::create(&root).unwrap();
        let first = ProjectPath::new("content/first.json").unwrap();
        let second = ProjectPath::new("content/second.json").unwrap();
        storage.save_json(&first, &json!({"ok": true})).unwrap();
        assert_eq!(storage.load_json::<Value>(&first).unwrap(), json!({"ok": true}));
        storage.rename(&first, &second).unwrap();
        assert!(!storage.exists(&first).unwrap());
        assert!(storage.exists(&second).unwrap());
        storage.remove_file(&second).unwrap();
        assert!(!storage.exists(&second).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn project_filesystem_rejects_symlink_escape_without_touching_external_file() {
        use std::os::unix::fs::symlink;

        let root = temporary_directory("sandbox-root");
        let outside = temporary_directory("sandbox-outside");
        fs::create_dir_all(root.join("content")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("content/link-to-outside")).unwrap();
        let storage = ProjectFilesystem::open(&root).unwrap();
        let logical = ProjectPath::new("content/link-to-outside/file.json").unwrap();
        let error = storage.write(&logical, b"escaped").unwrap_err();
        match error {
            ContentError::ProjectPath(error) => {
                assert_eq!(error.code, PROJECT_PATH_ESCAPE);
                assert_eq!(error.path, logical.as_str());
            }
            other => panic!("unexpected error: {other}"),
        }
        assert!(!outside.join("file.json").exists());
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}
