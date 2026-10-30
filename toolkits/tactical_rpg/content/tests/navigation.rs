use guiyi_engine_content::{CompileContext, DocumentCompiler, DocumentEnvelope, DocumentHeader};
use guiyi_engine_core::{DocumentId, EngineTypeId};
use serde_json::json;
use tactical_rpg_content::{
    shortest_path, CoordinateSpace, HexCoord, NavigationError, StageCompiler, StageDocument,
    StageNavigation, STAGE_DOCUMENT_TYPE,
};

#[test]
fn legacy_stage_payload_defaults_navigation_and_still_compiles() {
    let envelope = DocumentEnvelope {
        header: DocumentHeader {
            id: DocumentId::from_static("stage.legacy"),
            type_id: EngineTypeId::from_static(STAGE_DOCUMENT_TYPE),
            schema_version: 1,
            display_name: "Legacy".into(),
        },
        references: Vec::new(),
        payload: json!({
            "id": "stage.legacy",
            "name": "Legacy",
            "coordinate_space": {"kind": "hex_axial", "width": 4, "height": 4},
            "layers": ["gameplay"],
            "objects": [],
            "connections": []
        }),
    };

    let stage = StageDocument::from_envelope(&envelope).unwrap();
    assert!(stage.navigation.blocked.is_empty());
    StageCompiler
        .compile(
            &envelope,
            &CompileContext {
                project_root: ".".into(),
                profile: "test".into(),
            },
        )
        .unwrap();
}

#[test]
fn hex_shortest_path_is_deterministic_around_blocked_cells() {
    let mut stage = StageDocument::new_hex(DocumentId::from_static("stage.hex-nav"), "Hex", 4, 4);
    stage.navigation = StageNavigation {
        blocked: vec![HexCoord::new(1, 0)],
    };

    let path = shortest_path(&stage, HexCoord::new(0, 0), HexCoord::new(2, 0))
        .unwrap()
        .unwrap();
    assert_eq!(
        path,
        vec![
            HexCoord::new(0, 0),
            HexCoord::new(0, 1),
            HexCoord::new(1, 1),
            HexCoord::new(2, 0)
        ]
    );
}

#[test]
fn square_shortest_path_uses_stable_neighbor_order() {
    let stage = StageDocument {
        id: DocumentId::from_static("stage.square-nav"),
        name: "Square".into(),
        coordinate_space: CoordinateSpace::Square {
            width: 3,
            height: 3,
        },
        navigation: StageNavigation::default(),
        layers: vec!["gameplay".into()],
        objects: Vec::new(),
        connections: Vec::new(),
    };

    let path = shortest_path(&stage, HexCoord::new(0, 0), HexCoord::new(1, 1))
        .unwrap()
        .unwrap();
    assert_eq!(
        path,
        vec![
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(1, 1)
        ]
    );
}

#[test]
fn free_space_does_not_claim_grid_reachability() {
    let stage = StageDocument {
        id: DocumentId::from_static("stage.free-nav"),
        name: "Free".into(),
        coordinate_space: CoordinateSpace::Free2d,
        navigation: StageNavigation::default(),
        layers: vec!["gameplay".into()],
        objects: Vec::new(),
        connections: Vec::new(),
    };

    assert_eq!(
        shortest_path(&stage, HexCoord::new(0, 0), HexCoord::new(1, 1)),
        Err(NavigationError::UnsupportedCoordinateSpace)
    );
}
