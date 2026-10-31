#![forbid(unsafe_code)]

//! Tactical Stage, Encounter, Dialogue, and rule validation with machine-readable error codes.

mod encounter;
pub use self::encounter::*;

use guiyi_engine_validation::{Diagnostic, DiagnosticBag, DiagnosticLocation, Severity};
use std::collections::{BTreeSet, VecDeque};
use tactical_rpg_content::{
    default_tactical_rule_registry, shortest_path, stage_position_in_bounds, CoordinateSpace,
    DialogueDocument, RuleInvocationError, StageDocument, StageObjectKind, TacticalRuleKind,
    TacticalRuleRegistry,
};

pub const TACTICAL_RULE_TYPE_INVALID: &str = "TACTICAL_RULE_TYPE_INVALID";
pub const TACTICAL_RULE_UNKNOWN: &str = "TACTICAL_RULE_UNKNOWN";
pub const TACTICAL_RULE_SCHEMA_INVALID: &str = "TACTICAL_RULE_SCHEMA_INVALID";
pub const DIALOGUE_NAME_EMPTY: &str = "DIALOGUE_NAME_EMPTY";
pub const DIALOGUE_DUPLICATE_NODE_ID: &str = "DIALOGUE_DUPLICATE_NODE_ID";
pub const DIALOGUE_ENTRY_NODE_NOT_FOUND: &str = "DIALOGUE_ENTRY_NODE_NOT_FOUND";
pub const DIALOGUE_NODE_TEXT_EMPTY: &str = "DIALOGUE_NODE_TEXT_EMPTY";
pub const DIALOGUE_DUPLICATE_CHOICE_ID: &str = "DIALOGUE_DUPLICATE_CHOICE_ID";
pub const DIALOGUE_CHOICE_TEXT_EMPTY: &str = "DIALOGUE_CHOICE_TEXT_EMPTY";
pub const DIALOGUE_CHOICE_TARGET_NOT_FOUND: &str = "DIALOGUE_CHOICE_TARGET_NOT_FOUND";
pub const DIALOGUE_NODE_UNREACHABLE: &str = "DIALOGUE_NODE_UNREACHABLE";

pub fn validate_stage(stage: &StageDocument) -> DiagnosticBag {
    let mut bag = DiagnosticBag::default();
    let mut ids = BTreeSet::new();
    let mut spawn_count = 0;
    let mut navigation_has_blocking_errors = false;
    let rules = default_tactical_rule_registry();

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
                severity: Severity::Warning,
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
                severity: Severity::Error,
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
    for (object_index, object) in stage.objects.iter().enumerate() {
        if !ids.insert(object.id.clone()) {
            bag.push(Diagnostic {
                code: "STAGE_DUPLICATE_OBJECT_ID".into(),
                severity: Severity::Error,
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
                    severity: Severity::Error,
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
        if let StageObjectKind::Trigger {
            conditions, effects, ..
        } = &object.object
        {
            validate_rule_list(
                &rules,
                TacticalRuleKind::Condition,
                conditions,
                &stage.id,
                Some(object.id.clone()),
                &format!("/objects/{object_index}/object/conditions"),
                &mut bag,
            );
            validate_rule_list(
                &rules,
                TacticalRuleKind::Effect,
                effects,
                &stage.id,
                Some(object.id.clone()),
                &format!("/objects/{object_index}/object/effects"),
                &mut bag,
            );
        }
        if !stage_position_in_bounds(&stage.coordinate_space, object.position) {
            bag.push(Diagnostic {
                code: "STAGE_OBJECT_OUT_OF_BOUNDS".into(),
                severity: Severity::Error,
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
                    severity: Severity::Error,
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

pub fn validate_dialogue(dialogue: &DialogueDocument) -> DiagnosticBag {
    let mut bag = DiagnosticBag::default();
    let rules = default_tactical_rule_registry();
    let mut node_ids = BTreeSet::new();
    let mut duplicate_node = false;

    if dialogue.name.trim().is_empty() {
        bag.push(
            Diagnostic::error(DIALOGUE_NAME_EMPTY, "Dialogue name cannot be empty")
                .at_document(dialogue.id.clone()),
        );
    }

    for (node_index, node) in dialogue.nodes.iter().enumerate() {
        if !node_ids.insert(node.id.clone()) {
            duplicate_node = true;
            bag.push(Diagnostic {
                code: DIALOGUE_DUPLICATE_NODE_ID.into(),
                severity: Severity::Error,
                message: format!("duplicate Dialogue node identifier: {}", node.id),
                location: Some(DiagnosticLocation {
                    document: Some(dialogue.id.clone()),
                    object: Some(node.id.clone()),
                    field_path: Some(format!("/nodes/{node_index}/id")),
                }),
                suggested_tools: vec!["dialogue.add_node".into()],
                auto_fixable: false,
            });
        }
        if node.text.trim().is_empty() {
            bag.push(Diagnostic {
                code: DIALOGUE_NODE_TEXT_EMPTY.into(),
                severity: Severity::Error,
                message: format!("Dialogue node {} has empty text", node.id),
                location: Some(DiagnosticLocation {
                    document: Some(dialogue.id.clone()),
                    object: Some(node.id.clone()),
                    field_path: Some(format!("/nodes/{node_index}/text")),
                }),
                suggested_tools: vec!["dialogue.add_node".into()],
                auto_fixable: false,
            });
        }
    }

    if !node_ids.contains(&dialogue.entry_node) {
        bag.push(Diagnostic {
            code: DIALOGUE_ENTRY_NODE_NOT_FOUND.into(),
            severity: Severity::Error,
            message: format!("Dialogue entry node {} does not exist", dialogue.entry_node),
            location: Some(DiagnosticLocation {
                document: Some(dialogue.id.clone()),
                object: Some(dialogue.entry_node.clone()),
                field_path: Some("/entry_node".into()),
            }),
            suggested_tools: vec!["dialogue.add_node".into()],
            auto_fixable: false,
        });
    }

    let mut choice_ids = BTreeSet::new();
    for (node_index, node) in dialogue.nodes.iter().enumerate() {
        for (choice_index, choice) in node.choices.iter().enumerate() {
            if !choice_ids.insert(choice.id.clone()) {
                bag.push(Diagnostic {
                    code: DIALOGUE_DUPLICATE_CHOICE_ID.into(),
                    severity: Severity::Error,
                    message: format!("duplicate Dialogue choice identifier: {}", choice.id),
                    location: Some(DiagnosticLocation {
                        document: Some(dialogue.id.clone()),
                        object: Some(choice.id.clone()),
                        field_path: Some(format!(
                            "/nodes/{node_index}/choices/{choice_index}/id"
                        )),
                    }),
                    suggested_tools: vec!["dialogue.add_choice".into()],
                    auto_fixable: false,
                });
            }
            if choice.text.trim().is_empty() {
                bag.push(Diagnostic {
                    code: DIALOGUE_CHOICE_TEXT_EMPTY.into(),
                    severity: Severity::Error,
                    message: format!("Dialogue choice {} has empty text", choice.id),
                    location: Some(DiagnosticLocation {
                        document: Some(dialogue.id.clone()),
                        object: Some(choice.id.clone()),
                        field_path: Some(format!(
                            "/nodes/{node_index}/choices/{choice_index}/text"
                        )),
                    }),
                    suggested_tools: vec!["dialogue.add_choice".into()],
                    auto_fixable: false,
                });
            }
            if let Some(target) = &choice.target {
                if !node_ids.contains(target) {
                    bag.push(Diagnostic {
                        code: DIALOGUE_CHOICE_TARGET_NOT_FOUND.into(),
                        severity: Severity::Error,
                        message: format!(
                            "Dialogue choice {} targets missing node {}",
                            choice.id, target
                        ),
                        location: Some(DiagnosticLocation {
                            document: Some(dialogue.id.clone()),
                            object: Some(choice.id.clone()),
                            field_path: Some(format!(
                                "/nodes/{node_index}/choices/{choice_index}/target"
                            )),
                        }),
                        suggested_tools: vec!["dialogue.add_node".into()],
                        auto_fixable: false,
                    });
                }
            }
            validate_rule_list(
                &rules,
                TacticalRuleKind::Condition,
                &choice.conditions,
                &dialogue.id,
                Some(choice.id.clone()),
                &format!("/nodes/{node_index}/choices/{choice_index}/conditions"),
                &mut bag,
            );
            validate_rule_list(
                &rules,
                TacticalRuleKind::Effect,
                &choice.effects,
                &dialogue.id,
                Some(choice.id.clone()),
                &format!("/nodes/{node_index}/choices/{choice_index}/effects"),
                &mut bag,
            );
        }
    }

    if !duplicate_node && node_ids.contains(&dialogue.entry_node) {
        let mut visited = BTreeSet::from([dialogue.entry_node.clone()]);
        let mut queue = VecDeque::from([dialogue.entry_node.clone()]);
        while let Some(current) = queue.pop_front() {
            if let Some(node) = dialogue.nodes.iter().find(|node| node.id == current) {
                for choice in &node.choices {
                    if let Some(target) = &choice.target {
                        if node_ids.contains(target) && visited.insert(target.clone()) {
                            queue.push_back(target.clone());
                        }
                    }
                }
            }
        }
        for (node_index, node) in dialogue.nodes.iter().enumerate() {
            if !visited.contains(&node.id) {
                bag.push(Diagnostic {
                    code: DIALOGUE_NODE_UNREACHABLE.into(),
                    severity: Severity::Warning,
                    message: format!(
                        "Dialogue node {} is unreachable from entry node {}",
                        node.id, dialogue.entry_node
                    ),
                    location: Some(DiagnosticLocation {
                        document: Some(dialogue.id.clone()),
                        object: Some(node.id.clone()),
                        field_path: Some(format!("/nodes/{node_index}")),
                    }),
                    suggested_tools: vec!["dialogue.add_choice".into()],
                    auto_fixable: false,
                });
            }
        }
    }

    bag
}

fn validate_rule_list(
    registry: &TacticalRuleRegistry,
    kind: TacticalRuleKind,
    values: &[serde_json::Value],
    document: &guiyi_engine_core::DocumentId,
    object: Option<guiyi_engine_core::ObjectId>,
    base_path: &str,
    bag: &mut DiagnosticBag,
) {
    for (index, value) in values.iter().enumerate() {
        if let Err(error) = registry.validate_invocation(kind, value) {
            let (code, message) = match error {
                RuleInvocationError::TypeInvalid { detail } => {
                    (TACTICAL_RULE_TYPE_INVALID, detail)
                }
                RuleInvocationError::Unknown {
                    kind: rule_kind,
                    rule_type,
                } => (
                    TACTICAL_RULE_UNKNOWN,
                    format!("unknown {} rule type `{rule_type}`", rule_kind.as_str()),
                ),
                RuleInvocationError::SchemaInvalid {
                    kind: rule_kind,
                    rule_type,
                    detail,
                } => (
                    TACTICAL_RULE_SCHEMA_INVALID,
                    format!(
                        "{} rule `{rule_type}` does not match its schema: {detail}",
                        rule_kind.as_str()
                    ),
                ),
            };
            bag.push(Diagnostic {
                code: code.into(),
                severity: Severity::Error,
                message,
                location: Some(DiagnosticLocation {
                    document: Some(document.clone()),
                    object: object.clone(),
                    field_path: Some(format!("{base_path}/{index}")),
                }),
                suggested_tools: vec!["tactical.rules.catalog".into()],
                auto_fixable: false,
            });
        }
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
