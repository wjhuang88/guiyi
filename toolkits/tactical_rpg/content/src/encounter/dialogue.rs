use super::{default_tactical_rule_registry, TacticalRuleKind};
use guiyi_engine_content::{
    ArtifactEnvelope, CompileContext, ContentError, DocumentCompiler, DocumentEnvelope,
    DocumentHeader,
};
use guiyi_engine_core::{ArtifactId, DocumentId, EngineTypeId, ObjectId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const DIALOGUE_DOCUMENT_TYPE: &str = "tactical.dialogue.document";
pub const DIALOGUE_ARTIFACT_TYPE: &str = "tactical.dialogue.artifact";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueChoice {
    pub id: ObjectId,
    pub text: String,
    pub target: Option<ObjectId>,
    #[serde(default)]
    pub conditions: Vec<Value>,
    #[serde(default)]
    pub effects: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueNode {
    pub id: ObjectId,
    pub speaker: String,
    pub text: String,
    #[serde(default)]
    pub choices: Vec<DialogueChoice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueDocument {
    pub id: DocumentId,
    pub name: String,
    pub entry_node: ObjectId,
    #[serde(default)]
    pub nodes: Vec<DialogueNode>,
}

impl DialogueDocument {
    pub fn new(
        id: DocumentId,
        name: impl Into<String>,
        entry_node: ObjectId,
        speaker: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            entry_node: entry_node.clone(),
            nodes: vec![DialogueNode {
                id: entry_node,
                speaker: speaker.into(),
                text: text.into(),
                choices: Vec::new(),
            }],
        }
    }

    pub fn to_envelope(&self) -> Result<DocumentEnvelope, ContentError> {
        Ok(DocumentEnvelope {
            header: DocumentHeader {
                id: self.id.clone(),
                type_id: EngineTypeId::from_static(DIALOGUE_DOCUMENT_TYPE),
                schema_version: 1,
                display_name: self.name.clone(),
            },
            references: Vec::new(),
            payload: serde_json::to_value(self)?,
        })
    }

    pub fn from_envelope(value: &DocumentEnvelope) -> Result<Self, ContentError> {
        if value.header.type_id.as_str() != DIALOGUE_DOCUMENT_TYPE {
            return Err(ContentError::InvalidDocument(format!(
                "expected {DIALOGUE_DOCUMENT_TYPE}, found {}",
                value.header.type_id
            )));
        }
        Ok(serde_json::from_value(value.payload.clone())?)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueArtifactPayload {
    pub name: String,
    pub entry_node: ObjectId,
    pub nodes: Vec<DialogueNode>,
}

pub struct DialogueCompiler;

impl DocumentCompiler for DialogueCompiler {
    fn id(&self) -> &'static str {
        "tactical.dialogue.compiler"
    }

    fn source_type(&self) -> EngineTypeId {
        EngineTypeId::from_static(DIALOGUE_DOCUMENT_TYPE)
    }

    fn artifact_type(&self) -> EngineTypeId {
        EngineTypeId::from_static(DIALOGUE_ARTIFACT_TYPE)
    }

    fn compile(
        &self,
        document: &DocumentEnvelope,
        _context: &CompileContext,
    ) -> Result<ArtifactEnvelope, ContentError> {
        let dialogue = DialogueDocument::from_envelope(document)?;
        validate_for_compile(&dialogue)?;
        ArtifactEnvelope::new(
            ArtifactId::new(format!("artifact.{}", document.header.id.as_str()))
                .map_err(|error| ContentError::InvalidDocument(error.to_string()))?,
            self.artifact_type(),
            document.header.id.clone(),
            1,
            document.content_hash()?,
            serde_json::to_value(DialogueArtifactPayload {
                name: dialogue.name,
                entry_node: dialogue.entry_node,
                nodes: dialogue.nodes,
            })?,
        )
    }
}

fn validate_for_compile(dialogue: &DialogueDocument) -> Result<(), ContentError> {
    if dialogue.name.trim().is_empty() {
        return Err(ContentError::InvalidDocument(
            "dialogue name cannot be empty".into(),
        ));
    }
    let mut node_ids = BTreeSet::new();
    for node in &dialogue.nodes {
        if !node_ids.insert(node.id.clone()) {
            return Err(ContentError::InvalidDocument(format!(
                "duplicate Dialogue node id: {}",
                node.id
            )));
        }
        if node.text.trim().is_empty() {
            return Err(ContentError::InvalidDocument(format!(
                "Dialogue node {} has empty text",
                node.id
            )));
        }
    }
    if !node_ids.contains(&dialogue.entry_node) {
        return Err(ContentError::InvalidDocument(format!(
            "Dialogue entry node {} does not exist",
            dialogue.entry_node
        )));
    }

    let registry = default_tactical_rule_registry();
    let mut choice_ids = BTreeSet::new();
    for node in &dialogue.nodes {
        for choice in &node.choices {
            if !choice_ids.insert(choice.id.clone()) {
                return Err(ContentError::InvalidDocument(format!(
                    "duplicate Dialogue choice id: {}",
                    choice.id
                )));
            }
            if choice.text.trim().is_empty() {
                return Err(ContentError::InvalidDocument(format!(
                    "Dialogue choice {} has empty text",
                    choice.id
                )));
            }
            if let Some(target) = &choice.target {
                if !node_ids.contains(target) {
                    return Err(ContentError::InvalidDocument(format!(
                        "Dialogue choice {} targets missing node {}",
                        choice.id, target
                    )));
                }
            }
            for condition in &choice.conditions {
                registry
                    .validate_invocation(TacticalRuleKind::Condition, condition)
                    .map_err(|error| ContentError::InvalidDocument(error.to_string()))?;
            }
            for effect in &choice.effects {
                registry
                    .validate_invocation(TacticalRuleKind::Effect, effect)
                    .map_err(|error| ContentError::InvalidDocument(error.to_string()))?;
            }
        }
    }
    Ok(())
}
