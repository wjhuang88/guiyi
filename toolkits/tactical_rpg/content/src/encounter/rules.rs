use guiyi_engine_schema::{FieldSchema, SchemaNode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const CONDITION_ALWAYS: &str = "always";
pub const EFFECT_EMIT_SIGNAL: &str = "emit_signal";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TacticalRuleKind {
    Condition,
    Effect,
}

impl TacticalRuleKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Condition => "condition",
            Self::Effect => "effect",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TacticalRuleDescriptor {
    pub id: String,
    pub description: String,
    pub schema: SchemaNode,
}

impl TacticalRuleDescriptor {
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        schema: SchemaNode,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            schema,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleRegistryError {
    Duplicate {
        kind: TacticalRuleKind,
        id: String,
    },
    InvalidSchema {
        kind: TacticalRuleKind,
        id: String,
        detail: String,
    },
}

impl std::fmt::Display for RuleRegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duplicate { kind, id } => {
                write!(formatter, "duplicate {} rule: {id}", kind.as_str())
            }
            Self::InvalidSchema { kind, id, detail } => write!(
                formatter,
                "invalid {} rule schema for {id}: {detail}",
                kind.as_str()
            ),
        }
    }
}

impl std::error::Error for RuleRegistryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleInvocationError {
    TypeInvalid { detail: String },
    Unknown {
        kind: TacticalRuleKind,
        rule_type: String,
    },
    SchemaInvalid {
        kind: TacticalRuleKind,
        rule_type: String,
        detail: String,
    },
}

impl std::fmt::Display for RuleInvocationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeInvalid { detail } => write!(formatter, "invalid tactical rule type: {detail}"),
            Self::Unknown { kind, rule_type } => {
                write!(formatter, "unknown {} rule type: {rule_type}", kind.as_str())
            }
            Self::SchemaInvalid {
                kind,
                rule_type,
                detail,
            } => write!(
                formatter,
                "invalid {} rule `{rule_type}`: {detail}",
                kind.as_str()
            ),
        }
    }
}

impl std::error::Error for RuleInvocationError {}

#[derive(Debug, Clone, Default)]
pub struct TacticalRuleRegistry {
    conditions: BTreeMap<String, TacticalRuleDescriptor>,
    effects: BTreeMap<String, TacticalRuleDescriptor>,
}

impl TacticalRuleRegistry {
    pub fn register_condition(
        &mut self,
        descriptor: TacticalRuleDescriptor,
    ) -> Result<(), RuleRegistryError> {
        self.register(TacticalRuleKind::Condition, descriptor)
    }

    pub fn register_effect(
        &mut self,
        descriptor: TacticalRuleDescriptor,
    ) -> Result<(), RuleRegistryError> {
        self.register(TacticalRuleKind::Effect, descriptor)
    }

    fn register(
        &mut self,
        kind: TacticalRuleKind,
        descriptor: TacticalRuleDescriptor,
    ) -> Result<(), RuleRegistryError> {
        if let Err(error) = descriptor.schema.validate_definition() {
            return Err(RuleRegistryError::InvalidSchema {
                kind,
                id: descriptor.id,
                detail: error.to_string(),
            });
        }
        let registry = match kind {
            TacticalRuleKind::Condition => &mut self.conditions,
            TacticalRuleKind::Effect => &mut self.effects,
        };
        if registry.contains_key(&descriptor.id) {
            return Err(RuleRegistryError::Duplicate {
                kind,
                id: descriptor.id,
            });
        }
        registry.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    pub fn conditions(&self) -> impl Iterator<Item = &TacticalRuleDescriptor> {
        self.conditions.values()
    }

    pub fn effects(&self) -> impl Iterator<Item = &TacticalRuleDescriptor> {
        self.effects.values()
    }

    pub fn descriptor(
        &self,
        kind: TacticalRuleKind,
        id: &str,
    ) -> Option<&TacticalRuleDescriptor> {
        match kind {
            TacticalRuleKind::Condition => self.conditions.get(id),
            TacticalRuleKind::Effect => self.effects.get(id),
        }
    }

    pub fn validate_invocation(
        &self,
        kind: TacticalRuleKind,
        value: &Value,
    ) -> Result<(), RuleInvocationError> {
        let object = value.as_object().ok_or_else(|| RuleInvocationError::TypeInvalid {
            detail: "rule invocation must be an object".into(),
        })?;
        let rule_type = object
            .get("type")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| RuleInvocationError::TypeInvalid {
                detail: "rule invocation requires a non-empty string `type`".into(),
            })?;
        let descriptor = self
            .descriptor(kind, rule_type)
            .ok_or_else(|| RuleInvocationError::Unknown {
                kind,
                rule_type: rule_type.into(),
            })?;
        let diagnostics = descriptor.schema.validate_value(value);
        if diagnostics.has_errors() {
            let detail = diagnostics
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(RuleInvocationError::SchemaInvalid {
                kind,
                rule_type: rule_type.into(),
                detail,
            });
        }
        Ok(())
    }
}

pub fn default_tactical_rule_registry() -> TacticalRuleRegistry {
    let mut registry = TacticalRuleRegistry::default();
    registry
        .register_condition(TacticalRuleDescriptor::new(
            CONDITION_ALWAYS,
            "Always evaluates true when a runtime rule executor supports it.",
            SchemaNode::object(vec![FieldSchema::required(
                "type",
                SchemaNode::string().with_enum(vec![json!(CONDITION_ALWAYS)]),
            )])
            .forbid_additional_properties(),
        ))
        .expect("built-in `always` condition schema must be valid");
    registry
        .register_effect(TacticalRuleDescriptor::new(
            EFFECT_EMIT_SIGNAL,
            "Emit a named tactical signal when a runtime rule executor supports it.",
            SchemaNode::object(vec![
                FieldSchema::required(
                    "type",
                    SchemaNode::string().with_enum(vec![json!(EFFECT_EMIT_SIGNAL)]),
                ),
                FieldSchema::required("signal", SchemaNode::string().with_min_length(1)),
            ])
            .forbid_additional_properties(),
        ))
        .expect("built-in `emit_signal` effect schema must be valid");
    registry
}
