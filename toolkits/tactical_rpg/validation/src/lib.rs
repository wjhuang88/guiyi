#![forbid(unsafe_code)]

//! Tactical Stage validation with machine-readable error codes.

use guiyi_engine_validation::{Diagnostic, DiagnosticBag, DiagnosticLocation};
use std::collections::BTreeSet;
use tactical_rpg_content::{
    shortest_path, stage_position_in_bounds, CoordinateSpace, StageDocument, StageObjectKind,
};

pub fn validate_stage(stage: &StageDocument) -> DiagnosticBag {
    let mut bag = DiagnosticBag::default();
    let mut ids = BTreeSet::new();
    let mut spawn_count = 0;
    let mut navigation_has_blocking_errors = false;

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

    let grid_navigation = matches!(
        &stage.coordinate_space,
        CoordinateSpace::HexAxial { .. } | CoordinateSpace::Square { .. }
    );
    if !grid_navigation && !stage.navigation.blocked.is_empty() {
        navigation_has_blocking_errors = true;
        bag.push(
            Diagnostic::error(
                "STAGE_NAV_UNSUPPORTED_COORDINATE_SPACE",
                "blocked-cell navigation is only supported for HexAxial and Square Stages",
            )
            .at_document(stage.id.clone()),
        );
    }

    let mut blocked = BTreeSet::new();
    for (index, position) in stage.navigation.blocked.iter().enumerate() {
        let key = (position.q, position.r);
        if !blocked.insert(key) {
            bag.push(Diagnostic {
                code: "STAGE_NAV_DUPLICATE_BLOCKED_CELL".into(),
                severity: guiyi_engine_validation::Severity::Warning,
                message: format!(
                    "navigation blocked cell ({}, {}) is duplicated",
                    position.q, position.r
                ),
                location: Some(DiagnosticLocation {
                    document: Some(stage.id.clone()),
                    object: None,
                    field_path: Some(format!("/navigation/blocked/{index}")),
                }),
                suggested_tools: vec!["stage.set_navigation_blocked".into()],
                auto_fixable: false,
            });
        }
        if grid_navigation && !stage_position_in_bounds(&stage.coordinate_space, *position) {
            navigation_has_blocking_errors = true;
            bag.push(Diagnostic {
                code: "STAGE_NAV_BLOCKED_CELL_OUT_OF_BOUNDS".into(),
                severity: guiyi_engine_validation::Severity::Error,
                message: format!(
                    "navigation blocked cell ({}, {}) is outside Stage bounds",
                    position.q, position.r
                ),
                location: Some(DiagnosticLocation {
                    document: Some(stage.id.clone()),
                    object: None,
                    field_path: Some(format!("/navigation/blocked/{index}")),
                }),
                suggested_tools: vec!["stage.set_navigation_blocked".into()],
                auto_fixable: false,
            });
        }
    }

    let mut spawn_positions = Vec::new();
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
            spawn_positions.push(object.position);
            if blocked.contains(&(object.position.q, object.position.r)) {
                navigation_has_blocking_errors = true;
                bag.push(Diagnostic {
                    code: "STAGE_NAV_SPAWN_BLOCKED".into(),
                    severity: guiyi_engine_validation::Severity::Error,
                    message: format!("spawn point {} is on a blocked navigation cell", object.id),
                    location: Some(DiagnosticLocation {
                        document: Some(stage.id.clone()),
                        object: Some(object.id.clone()),
                        field_path: Some("position".into()),
                    }),
                    suggested_tools: vec!["stage.set_navigation_blocked".into()],
                    auto_fixable: false,
                });
            }
        }
        if !stage_position_in_bounds(&stage.coordinate_space, object.position) {
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

    if grid_navigation
        && !navigation_has_blocking_errors
        && !spawn_positions.is_empty()
        && !bag.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "STAGE_DIMENSIONS_INVALID"
                || diagnostic.code == "STAGE_OBJECT_OUT_OF_BOUNDS"
        })
    {
        for object in &stage.objects {
            if matches!(&object.object, StageObjectKind::SpawnPoint { .. }) {
                continue;
            }
            let reachable = spawn_positions.iter().any(|spawn| {
                shortest_path(stage, *spawn, object.position)
                    .ok()
                    .flatten()
                    .is_some()
            });
            if !reachable {
                bag.push(Diagnostic {
                    code: "STAGE_OBJECT_UNREACHABLE".into(),
                    severity: guiyi_engine_validation::Severity::Error,
                    message: format!(
                        "object {} is unreachable from every Stage spawn point",
                        object.id
                    ),
                    location: Some(DiagnosticLocation {
                        document: Some(stage.id.clone()),
                        object: Some(object.id.clone()),
                        field_path: Some("position".into()),
                    }),
                    suggested_tools: vec![
                        "stage.set_navigation_blocked".into(),
                        "stage.reachability".into(),
                        "stage.move_object".into(),
                    ],
                    auto_fixable: false,
                });
            }
        }
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
