use guiyi_engine_core::{DocumentId, ObjectId};
use serde_json::json;
use tactical_rpg_content::{
    DialogueChoice, DialogueDocument, DialogueNode, HexCoord, StageDocument, StageObject,
    StageObjectKind,
};
use tactical_rpg_validation::{
    validate_dialogue, validate_stage, DIALOGUE_CHOICE_TARGET_NOT_FOUND,
    DIALOGUE_DUPLICATE_CHOICE_ID, DIALOGUE_DUPLICATE_NODE_ID, DIALOGUE_ENTRY_NODE_NOT_FOUND,
    DIALOGUE_NAME_EMPTY, DIALOGUE_NODE_TEXT_EMPTY, DIALOGUE_NODE_UNREACHABLE,
    TACTICAL_RULE_SCHEMA_INVALID, TACTICAL_RULE_TYPE_INVALID, TACTICAL_RULE_UNKNOWN,
};

fn stage_with_trigger(effect: serde_json::Value) -> StageDocument {
    let mut stage = StageDocument::new_hex(DocumentId::from_static("stage.rules"), "Rules", 4, 4);
    stage.objects.push(StageObject {
        id: ObjectId::from_static("spawn.player"),
        position: HexCoord::new(0, 0),
        object: StageObjectKind::SpawnPoint {
            profile: "player".into(),
        },
        properties: json!({}),
    });
    stage.objects.push(StageObject {
        id: ObjectId::from_static("trigger.demo"),
        position: HexCoord::new(1, 0),
        object: StageObjectKind::Trigger {
            activation: "enter".into(),
            conditions: vec![json!({"type": "always"})],
            effects: vec![effect],
        },
        properties: json!({}),
    });
    stage
}

#[test]
fn existing_emit_signal_trigger_remains_valid() {
    let stage = stage_with_trigger(json!({
        "type": "emit_signal",
        "signal": "demo.complete"
    }));
    let diagnostics = validate_stage(&stage);
    assert!(!diagnostics.has_errors());
    assert!(!diagnostics
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.starts_with("TACTICAL_RULE_")));
}

#[test]
fn stage_trigger_reports_type_unknown_and_schema_rule_failures() {
    for (effect, code) in [
        (json!({"type": 7}), TACTICAL_RULE_TYPE_INVALID),
        (json!({"type": "missing"}), TACTICAL_RULE_UNKNOWN),
        (
            json!({"type": "emit_signal", "signal": ""}),
            TACTICAL_RULE_SCHEMA_INVALID,
        ),
    ] {
        let diagnostics = validate_stage(&stage_with_trigger(effect));
        let diagnostic = diagnostics
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap();
        assert_eq!(
            diagnostic.location.as_ref().unwrap().object.as_ref().unwrap().as_str(),
            "trigger.demo"
        );
        assert!(diagnostic
            .location
            .as_ref()
            .unwrap()
            .field_path
            .as_deref()
            .unwrap()
            .contains("effects/0"));
    }
}

fn valid_dialogue() -> DialogueDocument {
    DialogueDocument {
        id: DocumentId::from_static("dialogue.validation"),
        name: "Validation".into(),
        entry_node: ObjectId::from_static("node.a"),
        nodes: vec![
            DialogueNode {
                id: ObjectId::from_static("node.a"),
                speaker: "Guide".into(),
                text: "Choose.".into(),
                choices: vec![DialogueChoice {
                    id: ObjectId::from_static("choice.go"),
                    text: "Go.".into(),
                    target: Some(ObjectId::from_static("node.b")),
                    conditions: vec![json!({"type": "always"})],
                    effects: vec![],
                }],
            },
            DialogueNode {
                id: ObjectId::from_static("node.b"),
                speaker: "Guide".into(),
                text: "Done.".into(),
                choices: vec![],
            },
        ],
    }
}

#[test]
fn dialogue_validation_reports_structural_graph_failures() {
    let mut dialogue = valid_dialogue();
    dialogue.name = " ".into();
    dialogue.entry_node = ObjectId::from_static("node.missing-entry");
    dialogue.nodes[0].text = "".into();
    dialogue.nodes[1].id = dialogue.nodes[0].id.clone();
    dialogue.nodes[0].choices.push(DialogueChoice {
        id: ObjectId::from_static("choice.go"),
        text: "".into(),
        target: Some(ObjectId::from_static("node.missing-target")),
        conditions: vec![],
        effects: vec![],
    });

    let diagnostics = validate_dialogue(&dialogue);
    for code in [
        DIALOGUE_NAME_EMPTY,
        DIALOGUE_ENTRY_NODE_NOT_FOUND,
        DIALOGUE_NODE_TEXT_EMPTY,
        DIALOGUE_DUPLICATE_NODE_ID,
        DIALOGUE_DUPLICATE_CHOICE_ID,
        DIALOGUE_CHOICE_TARGET_NOT_FOUND,
    ] {
        assert!(diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code));
    }
}

#[test]
fn unreachable_dialogue_node_is_non_blocking_warning() {
    let mut dialogue = valid_dialogue();
    dialogue.nodes.push(DialogueNode {
        id: ObjectId::from_static("node.unreachable"),
        speaker: "Narrator".into(),
        text: "Hidden.".into(),
        choices: vec![],
    });
    let diagnostics = validate_dialogue(&dialogue);
    assert!(!diagnostics.has_errors());
    assert!(diagnostics
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DIALOGUE_NODE_UNREACHABLE));
}
