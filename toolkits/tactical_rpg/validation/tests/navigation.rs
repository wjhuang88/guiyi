use guiyi_engine_core::{DocumentId, ObjectId};
use serde_json::json;
use tactical_rpg_content::{HexCoord, StageDocument, StageNavigation, StageObject, StageObjectKind};
use tactical_rpg_validation::validate_stage;

fn stage_with_spawn_and_marker(width: u32, height: u32) -> StageDocument {
    let mut stage = StageDocument::new_hex(
        DocumentId::from_static("stage.navigation-validation"),
        "Navigation validation",
        width,
        height,
    );
    stage.objects.push(StageObject {
        id: ObjectId::from_static("spawn.player"),
        position: HexCoord::new(0, 0),
        object: StageObjectKind::SpawnPoint {
            profile: "player".into(),
        },
        properties: json!({}),
    });
    stage.objects.push(StageObject {
        id: ObjectId::from_static("marker.goal"),
        position: HexCoord::new(width as i32 - 1, 0),
        object: StageObjectKind::Marker {
            marker_type: "goal".into(),
        },
        properties: json!({}),
    });
    stage
}

fn codes(stage: &StageDocument) -> Vec<String> {
    validate_stage(stage)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn validator_rejects_blocked_cells_outside_stage_bounds() {
    let mut stage = stage_with_spawn_and_marker(3, 1);
    stage.navigation = StageNavigation {
        blocked: vec![HexCoord::new(9, 0)],
    };

    assert!(codes(&stage)
        .iter()
        .any(|code| code == "STAGE_NAV_BLOCKED_CELL_OUT_OF_BOUNDS"));
}

#[test]
fn validator_reports_duplicate_blocked_cells_deterministically() {
    let mut stage = stage_with_spawn_and_marker(3, 1);
    stage.navigation = StageNavigation {
        blocked: vec![HexCoord::new(1, 0), HexCoord::new(1, 0)],
    };

    let bag = validate_stage(&stage);
    let duplicates = bag
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "STAGE_NAV_DUPLICATE_BLOCKED_CELL")
        .count();
    assert_eq!(duplicates, 1);
}

#[test]
fn validator_rejects_a_spawn_on_a_blocked_cell() {
    let mut stage = stage_with_spawn_and_marker(3, 1);
    stage.navigation = StageNavigation {
        blocked: vec![HexCoord::new(0, 0)],
    };

    let bag = validate_stage(&stage);
    let diagnostic = bag
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "STAGE_NAV_SPAWN_BLOCKED")
        .unwrap();
    assert_eq!(
        diagnostic
            .location
            .as_ref()
            .and_then(|location| location.object.as_ref())
            .unwrap()
            .as_str(),
        "spawn.player"
    );
}

#[test]
fn validator_marks_objects_unreachable_from_every_spawn() {
    let mut stage = stage_with_spawn_and_marker(3, 1);
    stage.navigation = StageNavigation {
        blocked: vec![HexCoord::new(1, 0)],
    };

    let bag = validate_stage(&stage);
    let diagnostic = bag
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "STAGE_OBJECT_UNREACHABLE")
        .unwrap();
    assert_eq!(
        diagnostic
            .location
            .as_ref()
            .and_then(|location| location.object.as_ref())
            .unwrap()
            .as_str(),
        "marker.goal"
    );
}
