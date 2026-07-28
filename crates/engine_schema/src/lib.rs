#![forbid(unsafe_code)]

//! Machine-readable schema registry used by agents, commands, queries, and inspectors.

use guiyi_engine_core::EngineTypeId;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub description: String,
    pub value_type: String,
    pub required: bool,
    pub default: Option<Value>,
    pub constraints: BTreeMap<String, Value>,
}

impl FieldSchema {
    pub fn required(name: impl Into<String>, value_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            value_type: value_type.into(),
            required: true,
            default: None,
            constraints: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectSchema {
    pub type_id: EngineTypeId,
    pub title: String,
    pub description: String,
    pub schema_version: u32,
    pub fields: Vec<FieldSchema>,
    pub metadata: BTreeMap<String, Value>,
}

impl ObjectSchema {
    pub fn to_json_schema(&self) -> Value {
        let properties = self
            .fields
            .iter()
            .map(|field| {
                let mut value = json!({
                    "type": field.value_type,
                    "description": field.description,
                });
                if let Some(default) = &field.default {
                    value["default"] = default.clone();
                }
                (field.name.clone(), value)
            })
            .collect::<serde_json::Map<String, Value>>();
        let required = self
            .fields
            .iter()
            .filter(|field| field.required)
            .map(|field| Value::String(field.name.clone()))
            .collect::<Vec<_>>();
        json!({
            "$id": self.type_id.as_str(),
            "title": self.title,
            "description": self.description,
            "type": "object",
            "properties": properties,
            "required": required,
            "x-schema-version": self.schema_version,
        })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SchemaRegistryError {
    #[error("schema already registered: {0}")]
    Duplicate(EngineTypeId),
    #[error("schema not found: {0}")]
    NotFound(EngineTypeId),
}

#[derive(Debug, Default)]
pub struct SchemaRegistry {
    schemas: BTreeMap<EngineTypeId, ObjectSchema>,
}

impl SchemaRegistry {
    pub fn register(&mut self, schema: ObjectSchema) -> Result<(), SchemaRegistryError> {
        if self.schemas.contains_key(&schema.type_id) {
            return Err(SchemaRegistryError::Duplicate(schema.type_id));
        }
        self.schemas.insert(schema.type_id.clone(), schema);
        Ok(())
    }

    pub fn get(&self, type_id: &EngineTypeId) -> Result<&ObjectSchema, SchemaRegistryError> {
        self.schemas
            .get(type_id)
            .ok_or_else(|| SchemaRegistryError::NotFound(type_id.clone()))
    }

    pub fn list(&self) -> impl Iterator<Item = &ObjectSchema> {
        self.schemas.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> ObjectSchema {
        ObjectSchema {
            type_id: EngineTypeId::from_static("example.actor"),
            title: "Actor".into(),
            description: "An actor".into(),
            schema_version: 1,
            fields: vec![FieldSchema::required("name", "string")],
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn registry_rejects_duplicate_types() {
        let mut registry = SchemaRegistry::default();
        registry.register(schema()).unwrap();
        assert!(matches!(
            registry.register(schema()),
            Err(SchemaRegistryError::Duplicate(_))
        ));
    }

    #[test]
    fn json_schema_is_machine_readable() {
        let value = schema().to_json_schema();
        assert_eq!(value["required"][0], "name");
    }
}
