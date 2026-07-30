#![forbid(unsafe_code)]

//! Tactical RPG authoring types and Stage compiler.

use guiyi_engine_content::{
    ArtifactEnvelope, CompileContext, ContentError, ContentReference, DocumentCompiler,
    DocumentEnvelope, DocumentHeader, RuntimeObjectDescriptor, StageArtifactPayload,
};
use guiyi_engine_core::{ArtifactId, DocumentId, EngineTypeId, ObjectId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub const STAGE_DOCUMENT_TYPE: &str = "tactical.stage.document";
pub const STAGE_ARTIFACT_TYPE: &str = "tactical.stage.artifact";
pub const ACTOR_OBJECT_TYPE: &str = "tactical.actor";
pub const TRIGGER_OBJECT_TYPE: &str = "tactical.trigger";
pub const SPAWN_OBJECT_TYPE: &str = "tactical.spawn_point";
pub const MARKER_OBJECT_TYPE: &str = "tactical.marker";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

impl HexCoord {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    pub fn distance(self, other: Self) -> u32 {
        let dq = self.q - other.q;
        let dr = self.r - other.r;
        let ds = (-self.q - self.r) - (-other.q - other.r);
        ((dq.abs() + dr.abs() + ds.abs()) / 2) as u32
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CoordinateSpace {
    HexAxial { width: u32, height: u32 },
    Square { width: u32, height: u32 },
    Free2d,
    Free3d,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StageObjectKind {
    Actor {
        definition: DocumentId,
    },
    Trigger {
        activation: String,
        #[serde(default)]
        conditions: Vec<Value>,
        #[serde(default)]
        effects: Vec<Value>,
    },
    SpawnPoint {
        profile: String,
    },
    Marker {
        marker_type: String,
    },
}

impl StageObjectKind {
    pub fn runtime_type(&self) -> EngineTypeId {
        match self {
            Self::Actor { .. } => EngineTypeId::from_static(ACTOR_OBJECT_TYPE),
            Self::Trigger { .. } => EngineTypeId::from_static(TRIGGER_OBJECT_TYPE),
            Self::SpawnPoint { .. } => EngineTypeId::from_static(SPAWN_OBJECT_TYPE),
            Self::Marker { .. } => EngineTypeId::from_static(MARKER_OBJECT_TYPE),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageObject {
    pub id: ObjectId,
    pub position: HexCoord,
    pub object: StageObjectKind,
    #[serde(default)]
    pub properties: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageConnection {
    pub id: ObjectId,
    pub to_stage: DocumentId,
    pub entry_point: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageDocument {
    pub id: DocumentId,
    pub name: String,
    pub coordinate_space: CoordinateSpace,
    #[serde(default)]
    pub layers: Vec<String>,
    #[serde(default)]
    pub objects: Vec<StageObject>,
    #[serde(default)]
    pub connections: Vec<StageConnection>,
}

impl StageDocument {
    pub fn new_hex(id: DocumentId, name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            id,
            name: name.into(),
            coordinate_space: CoordinateSpace::HexAxial { width, height },
            layers: vec!["gameplay".into()],
            objects: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn to_envelope(&self) -> Result<DocumentEnvelope, ContentError> {
        let mut references = Vec::new();
        for object in &self.objects {
            if let StageObjectKind::Actor { definition } = &object.object {
                references.push(ContentReference {
                    source_object: Some(object.id.clone()),
                    target_document: definition.clone(),
                    target_object: None,
                    kind: "actor_definition".into(),
                });
            }
        }
        for connection in &self.connections {
            references.push(ContentReference {
                source_object: Some(connection.id.clone()),
                target_document: connection.to_stage.clone(),
                target_object: None,
                kind: "stage_connection".into(),
            });
        }
        Ok(DocumentEnvelope {
            header: DocumentHeader {
                id: self.id.clone(),
                type_id: EngineTypeId::from_static(STAGE_DOCUMENT_TYPE),
                schema_version: 1,
                display_name: self.name.clone(),
            },
            references,
            payload: serde_json::to_value(self)?,
        })
    }

    pub fn from_envelope(value: &DocumentEnvelope) -> Result<Self, ContentError> {
        if value.header.type_id.as_str() != STAGE_DOCUMENT_TYPE {
            return Err(ContentError::InvalidDocument(format!(
                "expected {STAGE_DOCUMENT_TYPE}, found {}",
                value.header.type_id
            )));
        }
        Ok(serde_json::from_value(value.payload.clone())?)
    }
}

pub struct StageCompiler;

impl DocumentCompiler for StageCompiler {
    fn id(&self) -> &'static str {
        "tactical.stage.compiler"
    }

    fn source_type(&self) -> EngineTypeId {
        EngineTypeId::from_static(STAGE_DOCUMENT_TYPE)
    }

    fn artifact_type(&self) -> EngineTypeId {
        EngineTypeId::from_static(STAGE_ARTIFACT_TYPE)
    }

    fn compile(
        &self,
        document: &DocumentEnvelope,
        _context: &CompileContext,
    ) -> Result<ArtifactEnvelope, ContentError> {
        let stage = StageDocument::from_envelope(document)?;
        if stage.name.trim().is_empty() {
            return Err(ContentError::InvalidDocument(
                "stage name cannot be empty".into(),
            ));
        }
        if matches!(
            &stage.coordinate_space,
            CoordinateSpace::HexAxial { width: 0, .. }
                | CoordinateSpace::HexAxial { height: 0, .. }
                | CoordinateSpace::Square { width: 0, .. }
                | CoordinateSpace::Square { height: 0, .. }
        ) {
            return Err(ContentError::InvalidDocument(
                "grid Stage dimensions must be greater than zero".into(),
            ));
        }
        let mut object_ids = BTreeSet::new();
        for object in &stage.objects {
            if !object_ids.insert(object.id.clone()) {
                return Err(ContentError::InvalidDocument(format!(
                    "duplicate Stage object id: {}",
                    object.id
                )));
            }
            if !stage_position_in_bounds(&stage.coordinate_space, object.position) {
                return Err(ContentError::InvalidDocument(format!(
                    "Stage object {} is outside the coordinate-space bounds",
                    object.id
                )));
            }
        }
        let objects = stage
            .objects
            .iter()
            .map(|object| RuntimeObjectDescriptor {
                id: object.id.clone(),
                type_id: object.object.runtime_type(),
                properties: json!({
                    "position": object.position,
                    "object": object.object.clone(),
                    "properties": object.properties.clone(),
                }),
            })
            .collect::<Vec<_>>();
        let source_hash = document.content_hash()?;
        ArtifactEnvelope::new(
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
    }
}

fn stage_position_in_bounds(space: &CoordinateSpace, position: HexCoord) -> bool {
    match space {
        CoordinateSpace::HexAxial { width, height } | CoordinateSpace::Square { width, height } => {
            position.q >= 0
                && position.r >= 0
                && position.q < *width as i32
                && position.r < *height as i32
        }
        CoordinateSpace::Free2d | CoordinateSpace::Free3d => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_distance_is_symmetric() {
        let a = HexCoord::new(0, 0);
        let b = HexCoord::new(2, -1);
        assert_eq!(a.distance(b), 2);
        assert_eq!(a.distance(b), b.distance(a));
    }

    #[test]
    fn stage_compiler_produces_runtime_objects() {
        let mut stage = StageDocument::new_hex(DocumentId::from_static("stage.demo"), "Demo", 8, 8);
        stage.objects.push(StageObject {
            id: ObjectId::from_static("spawn.player"),
            position: HexCoord::new(1, 1),
            object: StageObjectKind::SpawnPoint {
                profile: "player".into(),
            },
            properties: json!({}),
        });
        let artifact = StageCompiler
            .compile(
                &stage.to_envelope().unwrap(),
                &CompileContext {
                    project_root: ".".into(),
                    profile: "test".into(),
                },
            )
            .unwrap();
        let payload: StageArtifactPayload = serde_json::from_value(artifact.payload).unwrap();
        assert_eq!(payload.objects.len(), 1);
    }
}
