use guiyi_engine_content::{CompileContext, DocumentCompiler};
use guiyi_engine_core::{DocumentId, ObjectId};
use guiyi_engine_schema::{FieldSchema, SchemaNode};
use serde_json::json;
use tactical_rpg_content::{
    default_tactical_rule_registry, DialogueArtifactPayload, DialogueChoice, DialogueCompiler,
    DialogueDocument, DialogueNode, RuleInvocationError, RuleRegistryError, TacticalRuleDescriptor,
    TacticalRuleKind, DIALOGUE_ARTIFACT_TYPE, DIALOGUE_DOCUMENT_TYPE,
};

fn context() -> CompileContext {
    CompileContext {
        project_root: ".".into(),
        profile: "test".into(),
    }
}

fn dialogue() -> DialogueDocument {
    DialogueDocument {
        id: DocumentId::from_static("dialogue.demo"),
        name: "Gate Talk".into(),
        entry_node: ObjectId::from_static("node.entry"),
        nodes: vec![
            DialogueNode {
                id: ObjectId::from_static("node.entry"),
                speaker: "Guard".into(),
                text: "Halt.".into(),
                choices: vec![DialogueChoice {
                    id: ObjectId::from_static("choice.answer"),
                    text: "We mean no harm.".into(),
                    target: Some(ObjectId::from_static("node.exit")),
                    conditions: vec![json!({"type": "always"})],
                    effects: vec![json!({
                        "type": "emit_signal",
                        "signal": "dialogue.guard.cleared"
                    })],
                }],
            },
            DialogueNode {
                id: ObjectId::from_static("node.exit"),
                speaker: "Guard".into(),
                text: "Pass.".into(),
                choices: Vec::new(),
            },
        ],
    }
}

#[test]
fn built_in_rule_registry_is_deterministic_and_schema_validated() {
    let mut registry = default_tactical_rule_registry();
    assert_eq!(
        registry
            .conditions()
            .map(|descriptor| descriptor.id.as_str())
            .collect::<Vec<_>>(),
        vec!["always"]
    );
    assert_eq!(
        registry
            .effects()
            .map(|descriptor| descriptor.id.as_str())
            .collect::<Vec<_>>(),
        vec!["emit_signal"]
    );
    assert!(registry
        .validate_invocation(TacticalRuleKind::Condition, &json!({"type": "always"}))
        .is_ok());
    assert!(registry
        .validate_invocation(
            TacticalRuleKind::Effect,
            &json!({"type": "emit_signal", "signal": "demo.complete"}),
        )
        .is_ok());
    assert!(matches!(
        registry.validate_invocation(
            TacticalRuleKind::Effect,
            &json!({"type": "emit_signal", "signal": ""}),
        ),
        Err(RuleInvocationError::SchemaInvalid { .. })
    ));
    assert!(matches!(
        registry.validate_invocation(TacticalRuleKind::Condition, &json!({"type": "missing"})),
        Err(RuleInvocationError::Unknown { .. })
    ));

    let duplicate = registry
        .register_condition(TacticalRuleDescriptor::new(
            "always",
            "duplicate",
            SchemaNode::object(vec![FieldSchema::required(
                "type",
                SchemaNode::string(),
            )]),
        ))
        .unwrap_err();
    assert!(matches!(duplicate, RuleRegistryError::Duplicate { .. }));

    registry
        .register_effect(TacticalRuleDescriptor::new(
            "always",
            "same id in distinct effect namespace",
            SchemaNode::object(vec![FieldSchema::required(
                "type",
                SchemaNode::string(),
            )]),
        ))
        .unwrap();
}

#[test]
fn dialogue_round_trip_and_compiler_preserve_stable_graph_order_and_integrity() {
    let dialogue = dialogue();
    let envelope = dialogue.to_envelope().unwrap();
    assert_eq!(envelope.header.type_id.as_str(), DIALOGUE_DOCUMENT_TYPE);
    assert!(envelope.references.is_empty());
    assert_eq!(DialogueDocument::from_envelope(&envelope).unwrap(), dialogue);

    let artifact = DialogueCompiler.compile(&envelope, &context()).unwrap();
    assert_eq!(artifact.artifact_type.as_str(), DIALOGUE_ARTIFACT_TYPE);
    assert_eq!(artifact.source_hash, envelope.content_hash().unwrap());
    assert_eq!(
        artifact.artifact_hash,
        artifact.compute_artifact_hash().unwrap()
    );
    let payload: DialogueArtifactPayload = serde_json::from_value(artifact.payload).unwrap();
    assert_eq!(payload.entry_node.as_str(), "node.entry");
    assert_eq!(payload.nodes, dialogue.nodes);
}

#[test]
fn dialogue_compiler_rejects_missing_targets_and_invalid_rules() {
    let mut missing_target = dialogue();
    missing_target.nodes[0].choices[0].target = Some(ObjectId::from_static("node.missing"));
    assert!(DialogueCompiler
        .compile(&missing_target.to_envelope().unwrap(), &context())
        .is_err());

    let mut invalid_rule = dialogue();
    invalid_rule.nodes[0].choices[0].effects = vec![json!({
        "type": "emit_signal",
        "signal": ""
    })];
    assert!(DialogueCompiler
        .compile(&invalid_rule.to_envelope().unwrap(), &context())
        .is_err());
}
