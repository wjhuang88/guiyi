use guiyi_engine_content::{
    ArtifactEnvelope, CompileContext, ContentError, ContentReference, DocumentCompiler,
    DocumentEnvelope, DocumentHeader,
};
use guiyi_engine_core::{ArtifactId, DocumentId, EngineTypeId, ObjectId};
use serde::{Deserialize, Serialize};

pub const ENCOUNTER_DOCUMENT_TYPE: &str = "tactical.encounter.document";
pub const ENCOUNTER_ARTIFACT_TYPE: &str = "tactical.encounter.artifact";
pub const ACTOR_DEFINITION_TYPE: &str = "tactical.actor.definition";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterParticipant {
    pub id: ObjectId,
    pub actor_definition: DocumentId,
    pub spawn_point: ObjectId,
    pub side: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterDocument {
    pub id: DocumentId,
    pub name: String,
    pub stage: DocumentId,
    #[serde(default)]
    pub participants: Vec<EncounterParticipant>,
}

impl EncounterDocument {
    pub fn new(id: DocumentId, name: impl Into<String>, stage: DocumentId) -> Self {
        Self {
            id,
            name: name.into(),
            stage,
            participants: Vec::new(),
        }
    }

    pub fn to_envelope(&self) -> Result<DocumentEnvelope, ContentError> {
        let mut references = vec![ContentReference {
            source_object: None,
            target_document: self.stage.clone(),
            target_object: None,
            kind: "encounter_stage".into(),
        }];
        for participant in &self.participants {
            references.push(ContentReference {
                source_object: Some(participant.id.clone()),
                target_document: participant.actor_definition.clone(),
                target_object: None,
                kind: "encounter_actor_definition".into(),
            });
            references.push(ContentReference {
                source_object: Some(participant.id.clone()),
                target_document: self.stage.clone(),
                target_object: Some(participant.spawn_point.clone()),
                kind: "encounter_spawn_point".into(),
            });
        }
        Ok(DocumentEnvelope {
            header: DocumentHeader {
                id: self.id.clone(),
                type_id: EngineTypeId::from_static(ENCOUNTER_DOCUMENT_TYPE),
                schema_version: 1,
                display_name: self.name.clone(),
            },
            references,
            payload: serde_json::to_value(self)?,
        })
    }

    pub fn from_envelope(value: &DocumentEnvelope) -> Result<Self, ContentError> {
        if value.header.type_id.as_str() != ENCOUNTER_DOCUMENT_TYPE {
            return Err(ContentError::InvalidDocument(format!(
                "expected {ENCOUNTER_DOCUMENT_TYPE}, found {}",
                value.header.type_id
            )));
        }
        Ok(serde_json::from_value(value.payload.clone())?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterArtifactPayload {
    pub name: String,
    pub stage: DocumentId,
    pub participants: Vec<EncounterParticipant>,
}

pub struct EncounterCompiler;

impl DocumentCompiler for EncounterCompiler {
    fn id(&self) -> &'static str {
        "tactical.encounter.compiler"
    }

    fn source_type(&self) -> EngineTypeId {
        EngineTypeId::from_static(ENCOUNTER_DOCUMENT_TYPE)
    }

    fn artifact_type(&self) -> EngineTypeId {
        EngineTypeId::from_static(ENCOUNTER_ARTIFACT_TYPE)
    }

    fn compile(
        &self,
        document: &DocumentEnvelope,
        _context: &CompileContext,
    ) -> Result<ArtifactEnvelope, ContentError> {
        let encounter = EncounterDocument::from_envelope(document)?;
        validate_for_compile(&encounter)?;
        ArtifactEnvelope::new(
            ArtifactId::new(format!("artifact.{}", document.header.id.as_str()))
                .map_err(|error| ContentError::InvalidDocument(error.to_string()))?,
            self.artifact_type(),
            document.header.id.clone(),
            1,
            document.content_hash()?,
            serde_json::to_value(EncounterArtifactPayload {
                name: encounter.name,
                stage: encounter.stage,
                participants: encounter.participants,
            })?,
        )
    }
}

fn validate_for_compile(encounter: &EncounterDocument) -> Result<(), ContentError> {
    if encounter.name.trim().is_empty() {
        return Err(ContentError::InvalidDocument(
            "encounter name cannot be empty".into(),
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for participant in &encounter.participants {
        if !ids.insert(participant.id.clone()) {
            return Err(ContentError::InvalidDocument(format!(
                "duplicate Encounter participant id: {}",
                participant.id
            )));
        }
        if participant.side.trim().is_empty() {
            return Err(ContentError::InvalidDocument(format!(
                "Encounter participant {} has an empty side",
                participant.id
            )));
        }
    }
    Ok(())
}
