from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one replacement target, found {count}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "crates/engine_content/src/lib.rs",
    '''#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactEnvelope {
    pub id: ArtifactId,
    pub artifact_type: EngineTypeId,
    pub source_document: DocumentId,
    pub compiler_version: u32,
    pub source_hash: String,
    pub payload: Value,
}
''',
    '''#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactEnvelope {
    pub id: ArtifactId,
    pub artifact_type: EngineTypeId,
    pub source_document: DocumentId,
    pub compiler_version: u32,
    pub source_hash: String,
    #[serde(default)]
    pub artifact_hash: String,
    pub payload: Value,
}

impl ArtifactEnvelope {
    pub fn new(
        id: ArtifactId,
        artifact_type: EngineTypeId,
        source_document: DocumentId,
        compiler_version: u32,
        source_hash: String,
        payload: Value,
    ) -> Result<Self, ContentError> {
        let mut artifact = Self {
            id,
            artifact_type,
            source_document,
            compiler_version,
            source_hash,
            artifact_hash: String::new(),
            payload,
        };
        artifact.refresh_artifact_hash()?;
        Ok(artifact)
    }

    pub fn compute_artifact_hash(&self) -> Result<String, ContentError> {
        #[derive(Serialize)]
        struct IntegrityPayload<'a> {
            id: &'a ArtifactId,
            artifact_type: &'a EngineTypeId,
            source_document: &'a DocumentId,
            compiler_version: u32,
            source_hash: &'a str,
            payload: &'a Value,
        }

        let bytes = serde_json::to_vec(&IntegrityPayload {
            id: &self.id,
            artifact_type: &self.artifact_type,
            source_document: &self.source_document,
            compiler_version: self.compiler_version,
            source_hash: &self.source_hash,
            payload: &self.payload,
        })?;
        Ok(deterministic_hash(&bytes))
    }

    pub fn refresh_artifact_hash(&mut self) -> Result<(), ContentError> {
        self.artifact_hash = self.compute_artifact_hash()?;
        Ok(())
    }
}
''',
)

replace_once(
    "toolkits/tactical_rpg/content/src/lib.rs",
    '''        Ok(ArtifactEnvelope {
            id: ArtifactId::new(format!("artifact.{}", document.header.id.as_str()))
                .map_err(|error| ContentError::InvalidDocument(error.to_string()))?,
            artifact_type: self.artifact_type(),
            source_document: document.header.id.clone(),
            compiler_version: 1,
            source_hash,
            payload: serde_json::to_value(StageArtifactPayload {
                objects,
                metadata: json!({
                    "name": stage.name,
                    "coordinate_space": stage.coordinate_space,
                    "connections": stage.connections,
                }),
            })?,
        })
''',
    '''        ArtifactEnvelope::new(
            ArtifactId::new(format!("artifact.{}", document.header.id.as_str()))
                .map_err(|error| ContentError::InvalidDocument(error.to_string()))?,
            self.artifact_type(),
            document.header.id.clone(),
            1,
            source_hash,
            serde_json::to_value(StageArtifactPayload {
                objects,
                metadata: json!({
                    "name": stage.name,
                    "coordinate_space": stage.coordinate_space,
                    "connections": stage.connections,
                }),
            })?,
        )
''',
)

replace_once(
    "crates/engine_build/src/lib.rs",
    '''            Ok(ArtifactEnvelope {
                id: ArtifactId::new(format!("artifact.{}", document.header.id.as_str())).unwrap(),
                artifact_type: self.artifact_type(),
                source_document: document.header.id.clone(),
                compiler_version: 1,
                source_hash: document.content_hash()?,
                payload: document.payload.clone(),
            })
''',
    '''            ArtifactEnvelope::new(
                ArtifactId::new(format!("artifact.{}", document.header.id.as_str())).unwrap(),
                self.artifact_type(),
                document.header.id.clone(),
                1,
                document.content_hash()?,
                document.payload.clone(),
            )
''',
)
