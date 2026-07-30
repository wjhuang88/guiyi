#![forbid(unsafe_code)]

//! Tactical Stage validation with machine-readable error codes.

use guiyi_engine_validation::{Diagnostic, DiagnosticBag, DiagnosticLocation};
use std::collections::BTreeSet;
use tactical_rpg_content::{CoordinateSpace, StageDocument, StageObjectKind};

pub fn validate_stage(stage: &StageDocument) -> DiagnosticBag {
    let mut bag = DiagnosticBag::default();
    let mut ids = BTreeSet::new();
    let mut spawn_count = 0;

    if matches!(
        &stage.coordinate_space,
        CoordinateSpace::HexAxial { width: 0, .. }
            | CoordinateSpace::HexAxial { height: 0, .. }
            | CoordinateSpace::Square { width: 0, .. }
            | CoordinateSpace::Square { height: 0, .. }
    ) {
        bag.push(
            Diagnostic::error(
                "STAGE_DIMENSIONS_INVALID",
                "grid Stage dimensions must be greater than zero",
            )
            .at_document(stage.id.clone()),
        );
    }

    for object in &stage.objects {
        if !ids.insert(object.id.clone()) {
            bag.push(Diagnostic {
                code: "STAGE_DUPLICATE_OBJECT_ID".into(),
                severity: guiyi_engine_validation::Severity::Error,
                message: format!("duplicate object identifier: {}", object.id),
                location: Some(DiagnosticLocation {
                    document: Some(stage.id.clone()),
                    object: Some(object.id.clone()),
                    field_path: Some("objects".into()),
                }),
                suggested_tools: vec!["stage.rename_object".into()],
                auto_fixable: false,
            });
        }
        if matches!(&object.object, StageObjectKind::SpawnPoint { .. }) {
            spawn_count += 1;
        }
        if !position_in_bounds(stage, object.position.q, object.position.r) {
            bag.push(Diagnostic {
                code: "STAGE_OBJECT_OUT_OF_BOUNDS".into(),
                severity: guiyi_engine_validation::Severity::Error,
                message: format!("object {} is outside the Stage bounds", object.id),
                location: Some(DiagnosticLocation {
                    document: Some(stage.id.clone()),
                    object: Some(object.id.clone()),
                    field_path: Some("position".into()),
                }),
                suggested_tools: vec!["stage.move_object".into()],
                auto_fixable: false,
            });
        }
    }

    if spawn_count == 0 {
        bag.push(
            Diagnostic::warning(
                "STAGE_NO_SPAWN_POINT",
                "stage has no spawn point and cannot be entered by a standard preview profile",
            )
            .at_document(stage.id.clone()),
        );
    }

    for connection in &stage.connections {
        if connection.to_stage == stage.id {
            bag.push(
                Diagnostic::warning(
                    "STAGE_SELF_CONNECTION",
                    format!("connection {} points to the same Stage", connection.id),
                )
                .at_document(stage.id.clone()),
            );
        }
    }

    bag
}

fn position_in_bounds(stage: &StageDocument, x: i32, y: i32) -> bool {
    match &stage.coordinate_space {
        CoordinateSpace::HexAxial { width, height } | CoordinateSpace::Square { width, height } => {
            x >= 0 && y >= 0 && x < *width as i32 && y < *height as i32
        }
        CoordinateSpace::Free2d | CoordinateSpace::Free3d => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_core::{DocumentId, ObjectId};
    use serde_json::json;
    use tactical_rpg_content::{HexCoord, StageObject};

    #[test]
    fn validator_reports_missing_spawn() {
        let stage = StageDocument::new_hex(DocumentId::from_static("stage.empty"), "Empty", 4, 4);
        assert_eq!(validate_stage(&stage).summary().warnings, 1);
    }

    #[test]
    fn validator_rejects_out_of_bounds_objects() {
        let mut stage =
            StageDocument::new_hex(DocumentId::from_static("stage.bounds"), "Bounds", 4, 4);
        stage.objects.push(StageObject {
            id: ObjectId::from_static("spawn.bad"),
            position: HexCoord::new(8, 8),
            object: StageObjectKind::SpawnPoint {
                profile: "player".into(),
            },
            properties: json!({}),
        });
        assert!(validate_stage(&stage).has_errors());
    }
}
