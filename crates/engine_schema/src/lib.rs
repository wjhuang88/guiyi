#![forbid(unsafe_code)]

//! Versioned GUIYI schema dialect: the single authority for command-input
//! structural validation, default normalization, and deterministic rendering.
//!
//! See `docs/decisions/ADR-0015-SCHEMA-DRIVEN-COMMAND-VALIDATION.md`.

use guiyi_engine_core::EngineTypeId;
use guiyi_engine_validation::{Diagnostic, DiagnosticBag};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const SCHEMA_DIALECT_VERSION: u32 = 1;

pub mod codes {
    pub const COMMAND_INPUT_REQUIRED: &str = "COMMAND_INPUT_REQUIRED";
    pub const COMMAND_INPUT_TYPE_MISMATCH: &str = "COMMAND_INPUT_TYPE_MISMATCH";
    pub const COMMAND_INPUT_NULL_NOT_ALLOWED: &str = "COMMAND_INPUT_NULL_NOT_ALLOWED";
    pub const COMMAND_INPUT_ENUM_MISMATCH: &str = "COMMAND_INPUT_ENUM_MISMATCH";
    pub const COMMAND_INPUT_CONSTRAINT_FAILED: &str = "COMMAND_INPUT_CONSTRAINT_FAILED";
    pub const COMMAND_INPUT_ADDITIONAL_PROPERTY: &str = "COMMAND_INPUT_ADDITIONAL_PROPERTY";
    pub const SCHEMA_DEFINITION_INVALID: &str = "SCHEMA_DEFINITION_INVALID";
}

// ---------------------------------------------------------------------------
// ValueKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueKind {
    Any,
    String,
    Integer,
    Number,
    Boolean,
    Object,
    Array,
}

impl ValueKind {
    fn json_schema_type(self) -> Option<&'static str> {
        match self {
            ValueKind::Any => None,
            ValueKind::String => Some("string"),
            ValueKind::Integer => Some("integer"),
            ValueKind::Number => Some("number"),
            ValueKind::Boolean => Some("boolean"),
            ValueKind::Object => Some("object"),
            ValueKind::Array => Some("array"),
        }
    }
}

// ---------------------------------------------------------------------------
// AdditionalProperties
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdditionalProperties {
    #[default]
    Allowed,
    Forbidden,
}

// ---------------------------------------------------------------------------
// SchemaNode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaNode {
    #[serde(rename = "type")]
    pub kind: ValueKind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub nullable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default, rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub unique_items: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<SchemaNode>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldSchema>,
    #[serde(
        default = "AdditionalProperties::default",
        skip_serializing_if = "AdditionalProperties::is_allowed"
    )]
    pub additional_properties: AdditionalProperties,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl AdditionalProperties {
    fn is_allowed(&self) -> bool {
        *self == AdditionalProperties::Allowed
    }
}

impl Default for SchemaNode {
    fn default() -> Self {
        Self {
            kind: ValueKind::Any,
            description: String::new(),
            nullable: false,
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            min_items: None,
            max_items: None,
            unique_items: false,
            items: None,
            fields: Vec::new(),
            additional_properties: AdditionalProperties::Allowed,
        }
    }
}

// ---- Builder constructors ----

impl SchemaNode {
    pub fn new(kind: ValueKind) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    pub fn any() -> Self {
        Self::new(ValueKind::Any)
    }

    pub fn string() -> Self {
        Self::new(ValueKind::String)
    }

    pub fn integer() -> Self {
        Self::new(ValueKind::Integer)
    }

    pub fn number() -> Self {
        Self::new(ValueKind::Number)
    }

    pub fn boolean() -> Self {
        Self::new(ValueKind::Boolean)
    }

    pub fn object(fields: Vec<FieldSchema>) -> Self {
        Self {
            kind: ValueKind::Object,
            fields,
            ..Self::default()
        }
    }

    pub fn array(items: SchemaNode) -> Self {
        Self {
            kind: ValueKind::Array,
            items: Some(Box::new(items)),
            ..Self::default()
        }
    }
}

// ---- Fluent setters ----

impl SchemaNode {
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    pub fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }

    pub fn with_enum(mut self, values: Vec<Value>) -> Self {
        self.enum_values = Some(values);
        self
    }

    pub fn with_minimum(mut self, minimum: f64) -> Self {
        self.minimum = Some(minimum);
        self
    }

    pub fn with_maximum(mut self, maximum: f64) -> Self {
        self.maximum = Some(maximum);
        self
    }

    pub fn with_min_length(mut self, min_length: u32) -> Self {
        self.min_length = Some(min_length);
        self
    }

    pub fn with_max_length(mut self, max_length: u32) -> Self {
        self.max_length = Some(max_length);
        self
    }

    pub fn with_min_items(mut self, min_items: u32) -> Self {
        self.min_items = Some(min_items);
        self
    }

    pub fn with_max_items(mut self, max_items: u32) -> Self {
        self.max_items = Some(max_items);
        self
    }

    pub fn unique_items(mut self) -> Self {
        self.unique_items = true;
        self
    }

    pub fn forbid_additional_properties(mut self) -> Self {
        self.additional_properties = AdditionalProperties::Forbidden;
        self
    }
}

// ---------------------------------------------------------------------------
// FieldSchema
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub required: bool,
    #[serde(flatten)]
    pub schema: SchemaNode,
}

impl FieldSchema {
    pub fn required(name: impl Into<String>, schema: SchemaNode) -> Self {
        Self {
            name: name.into(),
            required: true,
            schema,
        }
    }

    pub fn optional(name: impl Into<String>, schema: SchemaNode) -> Self {
        Self {
            name: name.into(),
            required: false,
            schema,
        }
    }
}

// ---------------------------------------------------------------------------
// Schema definition validation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SchemaRegistryError {
    #[error("schema already registered: {0}")]
    Duplicate(EngineTypeId),
    #[error("schema not found: {0}")]
    NotFound(EngineTypeId),
    #[error("{0}")]
    DefinitionInvalid(Box<SchemaDefinitionError>, EngineTypeId),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code}: {message}")]
pub struct SchemaDefinitionError {
    pub code: String,
    pub type_id: Option<EngineTypeId>,
    pub field_path: String,
    pub keyword: String,
    pub message: String,
}

impl SchemaDefinitionError {
    fn new(
        keyword: impl Into<String>,
        message: impl Into<String>,
        field_path: impl Into<String>,
    ) -> Self {
        Self {
            code: codes::SCHEMA_DEFINITION_INVALID.into(),
            type_id: None,
            field_path: field_path.into(),
            keyword: keyword.into(),
            message: message.into(),
        }
    }
}

impl SchemaNode {
    /// Validates this schema definition for structural soundness.
    ///
    /// Returns `Err` if the schema violates any of:
    /// - constraints applied to incompatible value kinds
    /// - contradictory bounds (minimum > maximum, etc.)
    /// - invalid defaults (default does not conform to its own schema)
    /// - duplicate field names in objects
    /// - malformed nested schemas
    pub fn validate_definition(&self) -> Result<(), SchemaDefinitionError> {
        self.validate_definition_inner("")
    }

    fn validate_definition_inner(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        match self.kind {
            ValueKind::Any => {
                self.ensure_no_constraints(path)?;
            }
            ValueKind::String => {
                self.ensure_no_numeric_constraints(path)?;
                self.ensure_no_array_constraints(path)?;
                self.ensure_no_object_constraints(path)?;
                self.check_string_bounds(path)?;
            }
            ValueKind::Integer | ValueKind::Number => {
                self.ensure_no_string_constraints(path)?;
                self.ensure_no_array_constraints(path)?;
                self.ensure_no_object_constraints(path)?;
                self.check_numeric_definition_bounds(path)?;
            }
            ValueKind::Boolean => {
                self.ensure_no_numeric_constraints(path)?;
                self.ensure_no_string_constraints(path)?;
                self.ensure_no_array_constraints(path)?;
                self.ensure_no_object_constraints(path)?;
            }
            ValueKind::Object => {
                self.ensure_no_numeric_constraints(path)?;
                self.ensure_no_string_constraints(path)?;
                self.ensure_no_array_constraints(path)?;
                self.check_duplicate_fields(path)?;
                self.check_object_bounds(path)?;
                for field in &self.fields {
                    let field_path = append_pointer(path, &field.name);
                    field.schema.validate_definition_inner(&field_path)?;
                }
            }
            ValueKind::Array => {
                self.ensure_no_numeric_constraints(path)?;
                self.ensure_no_string_constraints(path)?;
                self.ensure_no_object_constraints(path)?;
                self.check_array_bounds(path)?;
                if let Some(items) = &self.items {
                    items.validate_definition_inner(&append_pointer(path, "items"))?;
                }
            }
        }

        self.check_enum_constraints(path)?;
        self.check_default_value(path)?;
        Ok(())
    }

    fn ensure_no_constraints(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        self.ensure_no_numeric_constraints(path)?;
        self.ensure_no_string_constraints(path)?;
        self.ensure_no_array_constraints(path)?;
        self.ensure_no_object_constraints(path)?;
        Ok(())
    }

    fn ensure_no_numeric_constraints(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        if self.minimum.is_some() {
            return Err(SchemaDefinitionError::new(
                "minimum",
                "numeric constraint `minimum` is not valid for this value kind",
                path,
            ));
        }
        if self.maximum.is_some() {
            return Err(SchemaDefinitionError::new(
                "maximum",
                "numeric constraint `maximum` is not valid for this value kind",
                path,
            ));
        }
        Ok(())
    }

    fn ensure_no_string_constraints(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        if self.min_length.is_some() {
            return Err(SchemaDefinitionError::new(
                "minLength",
                "string constraint `minLength` is not valid for this value kind",
                path,
            ));
        }
        if self.max_length.is_some() {
            return Err(SchemaDefinitionError::new(
                "maxLength",
                "string constraint `maxLength` is not valid for this value kind",
                path,
            ));
        }
        Ok(())
    }

    fn ensure_no_array_constraints(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        if self.min_items.is_some() {
            return Err(SchemaDefinitionError::new(
                "minItems",
                "array constraint `minItems` is not valid for this value kind",
                path,
            ));
        }
        if self.max_items.is_some() {
            return Err(SchemaDefinitionError::new(
                "maxItems",
                "array constraint `maxItems` is not valid for this value kind",
                path,
            ));
        }
        if self.unique_items {
            return Err(SchemaDefinitionError::new(
                "uniqueItems",
                "array constraint `uniqueItems` is not valid for this value kind",
                path,
            ));
        }
        if self.items.is_some() {
            return Err(SchemaDefinitionError::new(
                "items",
                "array constraint `items` is not valid for this value kind",
                path,
            ));
        }
        Ok(())
    }

    fn ensure_no_object_constraints(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        if !self.fields.is_empty() {
            return Err(SchemaDefinitionError::new(
                "fields",
                "object fields are not valid for this value kind",
                path,
            ));
        }
        if self.additional_properties == AdditionalProperties::Forbidden {
            return Err(SchemaDefinitionError::new(
                "additionalProperties",
                "object constraint `additionalProperties` is not valid for this value kind",
                path,
            ));
        }
        Ok(())
    }

    fn check_numeric_definition_bounds(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        if let (Some(min), Some(max)) = (self.minimum, self.maximum) {
            if min > max {
                return Err(SchemaDefinitionError::new(
                    "minimum/maximum",
                    format!("minimum ({min}) is greater than maximum ({max})"),
                    path,
                ));
            }
        }
        Ok(())
    }

    fn check_string_bounds(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        if let (Some(min), Some(max)) = (self.min_length, self.max_length) {
            if min > max {
                return Err(SchemaDefinitionError::new(
                    "minLength/maxLength",
                    format!("minLength ({min}) is greater than maxLength ({max})"),
                    path,
                ));
            }
        }
        Ok(())
    }

    fn check_array_bounds(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        if let (Some(min), Some(max)) = (self.min_items, self.max_items) {
            if min > max {
                return Err(SchemaDefinitionError::new(
                    "minItems/maxItems",
                    format!("minItems ({min}) is greater than maxItems ({max})"),
                    path,
                ));
            }
        }
        Ok(())
    }

    fn check_object_bounds(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        let mut seen = std::collections::BTreeSet::new();
        for field in &self.fields {
            if !seen.insert(field.name.clone()) {
                return Err(SchemaDefinitionError::new(
                    "fields",
                    format!("duplicate field name: `{}`", field.name),
                    path,
                ));
            }
        }
        Ok(())
    }

    fn check_duplicate_fields(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        self.check_object_bounds(path)
    }

    fn check_enum_constraints(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        if let Some(values) = &self.enum_values {
            if values.is_empty() {
                return Err(SchemaDefinitionError::new(
                    "enum",
                    "enum must contain at least one value",
                    path,
                ));
            }
        }
        Ok(())
    }

    fn check_default_value(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        if let Some(default) = &self.default {
            let mut bag = DiagnosticBag::default();
            self.validate_inner(default, path, &mut bag);
            if bag.has_errors() {
                let first = &bag.diagnostics[0];
                return Err(SchemaDefinitionError::new(
                    "default",
                    format!(
                        "default value does not conform to its own schema: {}",
                        first.message
                    ),
                    path,
                ));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Value validation
// ---------------------------------------------------------------------------

impl SchemaNode {
    /// Validates `value` against this schema and returns all diagnostics.
    pub fn validate_value(&self, value: &Value) -> DiagnosticBag {
        let mut bag = DiagnosticBag::default();
        self.validate_inner(value, "", &mut bag);
        bag
    }

    pub fn validate_and_normalize(&self, value: &Value) -> Result<Value, DiagnosticBag> {
        let mut bag = DiagnosticBag::default();
        self.validate_inner(value, "", &mut bag);
        if bag.has_errors() {
            return Err(bag);
        }
        Ok(self.normalize_value(value))
    }

    fn validate_inner(&self, value: &Value, path: &str, bag: &mut DiagnosticBag) {
        if value.is_null() {
            if !self.nullable && self.kind != ValueKind::Any {
                bag.push(
                    Diagnostic::error(
                        codes::COMMAND_INPUT_NULL_NOT_ALLOWED,
                        "null is not allowed for this field",
                    )
                    .at_field_path(path),
                );
            }
            return;
        }

        if let Some(values) = &self.enum_values {
            if !values.iter().any(|item| item == value) {
                bag.push(
                    Diagnostic::error(
                        codes::COMMAND_INPUT_ENUM_MISMATCH,
                        format!(
                            "value does not match any allowed enum option: [{}]",
                            values
                                .iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                    .at_field_path(path),
                );
                return;
            }
        }

        match self.kind {
            ValueKind::Any => {}
            ValueKind::String => self.validate_string(value, path, bag),
            ValueKind::Integer => self.validate_integer(value, path, bag),
            ValueKind::Number => self.validate_number(value, path, bag),
            ValueKind::Boolean => self.validate_boolean(value, path, bag),
            ValueKind::Object => self.validate_object(value, path, bag),
            ValueKind::Array => self.validate_array(value, path, bag),
        }
    }

    fn validate_string(&self, value: &Value, path: &str, bag: &mut DiagnosticBag) {
        match value {
            Value::String(s) => {
                let len = s.chars().count() as u32;
                if let Some(min) = self.min_length {
                    if len < min {
                        bag.push(
                            Diagnostic::error(
                                codes::COMMAND_INPUT_CONSTRAINT_FAILED,
                                format!("string length {len} is less than minLength {min}"),
                            )
                            .at_field_path(path),
                        );
                    }
                }
                if let Some(max) = self.max_length {
                    if len > max {
                        bag.push(
                            Diagnostic::error(
                                codes::COMMAND_INPUT_CONSTRAINT_FAILED,
                                format!("string length {len} is greater than maxLength {max}"),
                            )
                            .at_field_path(path),
                        );
                    }
                }
            }
            _ => bag.push(
                Diagnostic::error(
                    codes::COMMAND_INPUT_TYPE_MISMATCH,
                    format!("expected string, got {}", json_type_name(value)),
                )
                .at_field_path(path),
            ),
        }
    }

    fn validate_integer(&self, value: &Value, path: &str, bag: &mut DiagnosticBag) {
        match value {
            Value::Number(n) if n.is_i64() || n.is_u64() => {
                let f = n.as_f64().unwrap_or(f64::INFINITY);
                self.check_numeric_bounds(f, path, bag);
            }
            _ => bag.push(
                Diagnostic::error(
                    codes::COMMAND_INPUT_TYPE_MISMATCH,
                    format!("expected integer, got {}", json_type_name(value)),
                )
                .at_field_path(path),
            ),
        }
    }

    fn validate_number(&self, value: &Value, path: &str, bag: &mut DiagnosticBag) {
        match value {
            Value::Number(n) => {
                let f = n.as_f64().unwrap_or(f64::INFINITY);
                self.check_numeric_bounds(f, path, bag);
            }
            _ => bag.push(
                Diagnostic::error(
                    codes::COMMAND_INPUT_TYPE_MISMATCH,
                    format!("expected number, got {}", json_type_name(value)),
                )
                .at_field_path(path),
            ),
        }
    }

    fn check_numeric_bounds(&self, value: f64, path: &str, bag: &mut DiagnosticBag) {
        if let Some(min) = self.minimum {
            if value < min {
                bag.push(
                    Diagnostic::error(
                        codes::COMMAND_INPUT_CONSTRAINT_FAILED,
                        format!("value {value} is less than minimum {min}"),
                    )
                    .at_field_path(path),
                );
            }
        }
        if let Some(max) = self.maximum {
            if value > max {
                bag.push(
                    Diagnostic::error(
                        codes::COMMAND_INPUT_CONSTRAINT_FAILED,
                        format!("value {value} is greater than maximum {max}"),
                    )
                    .at_field_path(path),
                );
            }
        }
    }

    fn validate_boolean(&self, value: &Value, path: &str, bag: &mut DiagnosticBag) {
        if !value.is_boolean() {
            bag.push(
                Diagnostic::error(
                    codes::COMMAND_INPUT_TYPE_MISMATCH,
                    format!("expected boolean, got {}", json_type_name(value)),
                )
                .at_field_path(path),
            );
        }
    }

    fn validate_object(&self, value: &Value, path: &str, bag: &mut DiagnosticBag) {
        let map = match value {
            Value::Object(map) => map,
            _ => {
                bag.push(
                    Diagnostic::error(
                        codes::COMMAND_INPUT_TYPE_MISMATCH,
                        format!("expected object, got {}", json_type_name(value)),
                    )
                    .at_field_path(path),
                );
                return;
            }
        };

        for field in &self.fields {
            let field_path = append_pointer(path, &field.name);
            match map.get(&field.name) {
                Some(field_value) if !field_value.is_null() || field.schema.nullable => {
                    field.schema.validate_inner(field_value, &field_path, bag);
                }
                Some(_) => {
                    if field.required {
                        bag.push(
                            Diagnostic::error(
                                codes::COMMAND_INPUT_NULL_NOT_ALLOWED,
                                format!("field `{}` does not allow null", field.name),
                            )
                            .at_field_path(&field_path),
                        );
                    }
                }
                None => {
                    if field.required && field.schema.default.is_none() {
                        bag.push(
                            Diagnostic::error(
                                codes::COMMAND_INPUT_REQUIRED,
                                format!("required field `{}` is missing", field.name),
                            )
                            .at_field_path(&field_path),
                        );
                    }
                }
            }
        }

        if self.additional_properties == AdditionalProperties::Forbidden {
            let known: std::collections::BTreeSet<&str> =
                self.fields.iter().map(|f| f.name.as_str()).collect();
            for key in map.keys() {
                if !known.contains(key.as_str()) {
                    bag.push(
                        Diagnostic::error(
                            codes::COMMAND_INPUT_ADDITIONAL_PROPERTY,
                            format!("additional property `{key}` is not allowed"),
                        )
                        .at_field_path(append_pointer(path, key)),
                    );
                }
            }
        }
    }

    fn validate_array(&self, value: &Value, path: &str, bag: &mut DiagnosticBag) {
        let arr = match value {
            Value::Array(arr) => arr,
            _ => {
                bag.push(
                    Diagnostic::error(
                        codes::COMMAND_INPUT_TYPE_MISMATCH,
                        format!("expected array, got {}", json_type_name(value)),
                    )
                    .at_field_path(path),
                );
                return;
            }
        };

        let len = arr.len() as u32;
        if let Some(min) = self.min_items {
            if len < min {
                bag.push(
                    Diagnostic::error(
                        codes::COMMAND_INPUT_CONSTRAINT_FAILED,
                        format!("array length {len} is less than minItems {min}"),
                    )
                    .at_field_path(path),
                );
            }
        }
        if let Some(max) = self.max_items {
            if len > max {
                bag.push(
                    Diagnostic::error(
                        codes::COMMAND_INPUT_CONSTRAINT_FAILED,
                        format!("array length {len} is greater than maxItems {max}"),
                    )
                    .at_field_path(path),
                );
            }
        }

        if self.unique_items {
            for i in 0..arr.len() {
                for j in (i + 1)..arr.len() {
                    if arr[i] == arr[j] {
                        bag.push(
                            Diagnostic::error(
                                codes::COMMAND_INPUT_CONSTRAINT_FAILED,
                                format!("array items at indices {i} and {j} are duplicates"),
                            )
                            .at_field_path(format!("{path}/{i}")),
                        );
                        break;
                    }
                }
            }
        }

        if let Some(items_schema) = &self.items {
            for (i, item) in arr.iter().enumerate() {
                let item_path = format!("{path}/{i}");
                items_schema.validate_inner(item, &item_path, bag);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Default normalization
// ---------------------------------------------------------------------------

impl SchemaNode {
    fn normalize_value(&self, value: &Value) -> Value {
        if value.is_null() {
            return value.clone();
        }

        match self.kind {
            ValueKind::Object => {
                if let Value::Object(map) = value {
                    let mut result = map.clone();
                    for field in &self.fields {
                        if !result.contains_key(&field.name) {
                            if let Some(default) = &field.schema.default {
                                result.insert(field.name.clone(), default.clone());
                            }
                        }
                        if let Some(existing) = result.get(&field.name) {
                            let normalized = field.schema.normalize_value(existing);
                            if !normalized.is_null() || field.schema.nullable {
                                result.insert(field.name.clone(), normalized);
                            }
                        }
                    }
                    Value::Object(result)
                } else {
                    value.clone()
                }
            }
            ValueKind::Array => {
                if let Value::Array(arr) = value {
                    if let Some(items_schema) = &self.items {
                        Value::Array(
                            arr.iter()
                                .map(|item| items_schema.normalize_value(item))
                                .collect(),
                        )
                    } else {
                        value.clone()
                    }
                } else {
                    value.clone()
                }
            }
            _ => value.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// JSON Schema rendering
// ---------------------------------------------------------------------------

impl SchemaNode {
    pub fn to_json_schema(&self) -> Value {
        let mut map = Map::new();

        if let Some(type_str) = self.kind.json_schema_type() {
            map.insert("type".into(), Value::String(type_str.into()));
        }

        if !self.description.is_empty() {
            map.insert(
                "description".into(),
                Value::String(self.description.clone()),
            );
        }

        if self.nullable {
            let base_type = map
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| Value::String(s.to_string()))
                .unwrap_or_else(|| Value::String("any".into()));
            map.insert(
                "type".into(),
                Value::Array(vec![base_type, Value::String("null".into())]),
            );
        }

        if let Some(default) = &self.default {
            map.insert("default".into(), default.clone());
        }

        if let Some(values) = &self.enum_values {
            map.insert("enum".into(), Value::Array(values.clone()));
        }

        if let Some(min) = self.minimum {
            map.insert("minimum".into(), serde_json::json!(min));
        }
        if let Some(max) = self.maximum {
            map.insert("maximum".into(), serde_json::json!(max));
        }

        if let Some(min) = self.min_length {
            map.insert("minLength".into(), serde_json::json!(min));
        }
        if let Some(max) = self.max_length {
            map.insert("maxLength".into(), serde_json::json!(max));
        }

        if let Some(min) = self.min_items {
            map.insert("minItems".into(), serde_json::json!(min));
        }
        if let Some(max) = self.max_items {
            map.insert("maxItems".into(), serde_json::json!(max));
        }

        if self.unique_items {
            map.insert("uniqueItems".into(), Value::Bool(true));
        }

        if let Some(items) = &self.items {
            map.insert("items".into(), items.to_json_schema());
        }

        if self.kind == ValueKind::Object {
            let properties = self
                .fields
                .iter()
                .map(|field| (field.name.clone(), field.schema.to_json_schema()))
                .collect::<Map<String, Value>>();
            map.insert("properties".into(), Value::Object(properties));

            let required = self
                .fields
                .iter()
                .filter(|field| field.required)
                .map(|field| Value::String(field.name.clone()))
                .collect::<Vec<_>>();
            if !required.is_empty() {
                map.insert("required".into(), Value::Array(required));
            }

            map.insert(
                "additionalProperties".into(),
                Value::Bool(self.additional_properties == AdditionalProperties::Allowed),
            );
        }

        map.insert(
            "x-schema-version".into(),
            Value::Number(SCHEMA_DIALECT_VERSION.into()),
        );

        Value::Object(map)
    }
}

// ---------------------------------------------------------------------------
// JSON Pointer helpers (RFC 6901)
// ---------------------------------------------------------------------------

fn escape_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

fn append_pointer(base: &str, segment: &str) -> String {
    format!("{base}/{}", escape_pointer_segment(segment))
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ---------------------------------------------------------------------------
// SchemaRegistry
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct SchemaRegistry {
    schemas: BTreeMap<EngineTypeId, SchemaNode>,
}

impl SchemaRegistry {
    pub fn register(
        &mut self,
        type_id: EngineTypeId,
        schema: SchemaNode,
    ) -> Result<(), SchemaRegistryError> {
        schema
            .validate_definition()
            .map_err(|error| SchemaRegistryError::from_definition_error(error, &type_id))?;
        if self.schemas.contains_key(&type_id) {
            return Err(SchemaRegistryError::Duplicate(type_id));
        }
        self.schemas.insert(type_id, schema);
        Ok(())
    }

    pub fn get(&self, type_id: &EngineTypeId) -> Result<&SchemaNode, SchemaRegistryError> {
        self.schemas
            .get(type_id)
            .ok_or_else(|| SchemaRegistryError::NotFound(type_id.clone()))
    }

    pub fn list(&self) -> impl Iterator<Item = (&EngineTypeId, &SchemaNode)> {
        self.schemas.iter()
    }
}

impl SchemaRegistryError {
    fn from_definition_error(error: SchemaDefinitionError, type_id: &EngineTypeId) -> Self {
        SchemaRegistryError::DefinitionInvalid(Box::new(error), type_id.clone())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -- round-trip serialization --

    #[test]
    fn schema_node_round_trips_through_serde() {
        let schema = SchemaNode::object(vec![
            FieldSchema::required("name", SchemaNode::string().with_min_length(1)),
            FieldSchema::optional("age", SchemaNode::integer().with_minimum(0.0)),
        ]);
        let serialized = serde_json::to_string(&schema).unwrap();
        let deserialized: SchemaNode = serde_json::from_str(&serialized).unwrap();
        assert_eq!(schema, deserialized);
    }

    // -- value kind validation --

    #[test]
    fn validates_correct_string_value() {
        let schema = SchemaNode::string();
        assert!(schema
            .validate_value(&json!("hello"))
            .diagnostics
            .is_empty());
    }

    #[test]
    fn rejects_non_string_value() {
        let schema = SchemaNode::string();
        let bag = schema.validate_value(&json!(42));
        assert_eq!(bag.diagnostics[0].code, codes::COMMAND_INPUT_TYPE_MISMATCH);
        assert_eq!(
            bag.diagnostics[0].location.as_ref().unwrap().field_path,
            Some("".to_string())
        );
    }

    #[test]
    fn validates_integer_rejects_float() {
        let schema = SchemaNode::integer();
        let bag = schema.validate_value(&json!(3.5));
        assert!(bag.has_errors());
        assert_eq!(bag.diagnostics[0].code, codes::COMMAND_INPUT_TYPE_MISMATCH);
    }

    #[test]
    fn validates_number_accepts_integer_and_float() {
        let schema = SchemaNode::number();
        assert!(!schema.validate_value(&json!(42)).has_errors());
        assert!(!schema.validate_value(&json!(2.5)).has_errors());
    }

    #[test]
    fn validates_boolean() {
        let schema = SchemaNode::boolean();
        assert!(!schema.validate_value(&json!(true)).has_errors());
        assert!(schema.validate_value(&json!("true")).has_errors());
    }

    // -- nullable --

    #[test]
    fn null_rejected_on_non_nullable() {
        let schema = SchemaNode::string();
        let bag = schema.validate_value(&Value::Null);
        assert_eq!(
            bag.diagnostics[0].code,
            codes::COMMAND_INPUT_NULL_NOT_ALLOWED
        );
    }

    #[test]
    fn null_allowed_on_nullable() {
        let schema = SchemaNode::string().nullable();
        assert!(!schema.validate_value(&Value::Null).has_errors());
    }

    // -- enum --

    #[test]
    fn enum_rejects_non_member() {
        let schema = SchemaNode::string().with_enum(vec![json!("a"), json!("b")]);
        let bag = schema.validate_value(&json!("c"));
        assert_eq!(bag.diagnostics[0].code, codes::COMMAND_INPUT_ENUM_MISMATCH);
    }

    #[test]
    fn enum_accepts_member() {
        let schema = SchemaNode::string().with_enum(vec![json!("a"), json!("b")]);
        assert!(!schema.validate_value(&json!("a")).has_errors());
    }

    // -- numeric bounds --

    #[test]
    fn numeric_minimum_rejects_below() {
        let schema = SchemaNode::integer().with_minimum(1.0);
        let bag = schema.validate_value(&json!(0));
        assert_eq!(
            bag.diagnostics[0].code,
            codes::COMMAND_INPUT_CONSTRAINT_FAILED
        );
    }

    #[test]
    fn numeric_maximum_rejects_above() {
        let schema = SchemaNode::integer().with_maximum(10.0);
        let bag = schema.validate_value(&json!(11));
        assert_eq!(
            bag.diagnostics[0].code,
            codes::COMMAND_INPUT_CONSTRAINT_FAILED
        );
    }

    // -- string length --

    #[test]
    fn string_min_length_rejects_short() {
        let schema = SchemaNode::string().with_min_length(3);
        let bag = schema.validate_value(&json!("ab"));
        assert_eq!(
            bag.diagnostics[0].code,
            codes::COMMAND_INPUT_CONSTRAINT_FAILED
        );
    }

    #[test]
    fn string_max_length_rejects_long() {
        let schema = SchemaNode::string().with_max_length(3);
        let bag = schema.validate_value(&json!("abcd"));
        assert_eq!(
            bag.diagnostics[0].code,
            codes::COMMAND_INPUT_CONSTRAINT_FAILED
        );
    }

    // -- array constraints --

    #[test]
    fn array_min_items_rejects_short() {
        let schema = SchemaNode::array(SchemaNode::string()).with_min_items(2);
        let bag = schema.validate_value(&json!(["a"]));
        assert_eq!(
            bag.diagnostics[0].code,
            codes::COMMAND_INPUT_CONSTRAINT_FAILED
        );
    }

    #[test]
    fn array_max_items_rejects_long() {
        let schema = SchemaNode::array(SchemaNode::string()).with_max_items(2);
        let bag = schema.validate_value(&json!(["a", "b", "c"]));
        assert_eq!(
            bag.diagnostics[0].code,
            codes::COMMAND_INPUT_CONSTRAINT_FAILED
        );
    }

    #[test]
    fn array_unique_items_rejects_duplicates() {
        let schema = SchemaNode::array(SchemaNode::integer()).unique_items();
        let bag = schema.validate_value(&json!([1, 2, 1]));
        assert!(bag.has_errors());
        assert_eq!(
            bag.diagnostics[0].code,
            codes::COMMAND_INPUT_CONSTRAINT_FAILED
        );
    }

    #[test]
    fn array_item_validation_with_index_path() {
        let schema = SchemaNode::array(SchemaNode::integer());
        let bag = schema.validate_value(&json!([1, "two", 3]));
        assert_eq!(
            bag.diagnostics[0]
                .location
                .as_ref()
                .unwrap()
                .field_path
                .as_deref(),
            Some("/1")
        );
    }

    // -- object validation --

    #[test]
    fn object_required_field_missing() {
        let schema = SchemaNode::object(vec![FieldSchema::required("name", SchemaNode::string())]);
        let bag = schema.validate_value(&json!({}));
        assert_eq!(bag.diagnostics[0].code, codes::COMMAND_INPUT_REQUIRED);
        assert_eq!(
            bag.diagnostics[0]
                .location
                .as_ref()
                .unwrap()
                .field_path
                .as_deref(),
            Some("/name")
        );
    }

    #[test]
    fn object_wrong_root_type() {
        let schema = SchemaNode::object(vec![]);
        let bag = schema.validate_value(&json!([1, 2]));
        assert_eq!(bag.diagnostics[0].code, codes::COMMAND_INPUT_TYPE_MISMATCH);
    }

    #[test]
    fn object_additional_property_forbidden() {
        let schema = SchemaNode::object(vec![FieldSchema::required("name", SchemaNode::string())])
            .forbid_additional_properties();
        let bag = schema.validate_value(&json!({"name": "ok", "extra": 1}));
        assert_eq!(
            bag.diagnostics[0].code,
            codes::COMMAND_INPUT_ADDITIONAL_PROPERTY
        );
        assert_eq!(
            bag.diagnostics[0]
                .location
                .as_ref()
                .unwrap()
                .field_path
                .as_deref(),
            Some("/extra")
        );
    }

    #[test]
    fn object_additional_property_allowed_by_default() {
        let schema = SchemaNode::object(vec![FieldSchema::required("name", SchemaNode::string())]);
        assert!(!schema
            .validate_value(&json!({"name": "ok", "extra": 1}))
            .has_errors());
    }

    #[test]
    fn nested_object_field_path() {
        let schema = SchemaNode::object(vec![FieldSchema::required(
            "meta",
            SchemaNode::object(vec![FieldSchema::required(
                "version",
                SchemaNode::integer(),
            )]),
        )]);
        let bag = schema.validate_value(&json!({"meta": {"version": "x"}}));
        assert_eq!(
            bag.diagnostics[0]
                .location
                .as_ref()
                .unwrap()
                .field_path
                .as_deref(),
            Some("/meta/version")
        );
    }

    // -- default normalization --

    #[test]
    fn normalize_applies_defaults_for_optional_fields() {
        let schema = SchemaNode::object(vec![
            FieldSchema::required("id", SchemaNode::string()),
            FieldSchema::optional("version", SchemaNode::integer().with_default(json!(1))),
            FieldSchema::optional("payload", SchemaNode::any().with_default(json!({}))),
        ]);
        let normalized = schema
            .validate_and_normalize(&json!({"id": "doc.1"}))
            .unwrap();
        assert_eq!(normalized["version"], json!(1));
        assert_eq!(normalized["payload"], json!({}));
    }

    #[test]
    fn normalize_preserves_provided_values() {
        let schema = SchemaNode::object(vec![
            FieldSchema::required("id", SchemaNode::string()),
            FieldSchema::optional("version", SchemaNode::integer().with_default(json!(1))),
        ]);
        let normalized = schema
            .validate_and_normalize(&json!({"id": "doc.1", "version": 3}))
            .unwrap();
        assert_eq!(normalized["version"], json!(3));
    }

    #[test]
    fn normalize_array_defaults() {
        let schema = SchemaNode::object(vec![
            FieldSchema::required("id", SchemaNode::string()),
            FieldSchema::optional(
                "tags",
                SchemaNode::array(SchemaNode::string()).with_default(json!([])),
            ),
        ]);
        let normalized = schema
            .validate_and_normalize(&json!({"id": "doc.1"}))
            .unwrap();
        assert_eq!(normalized["tags"], json!([]));
    }

    // -- schema definition validation --

    #[test]
    fn definition_rejects_min_length_on_integer() {
        let schema = SchemaNode::integer().with_min_length(1);
        let error = schema.validate_definition().unwrap_err();
        assert_eq!(error.code, codes::SCHEMA_DEFINITION_INVALID);
        assert_eq!(error.keyword, "minLength");
    }

    #[test]
    fn definition_rejects_minimum_on_string() {
        let schema = SchemaNode::string().with_minimum(1.0);
        assert!(schema.validate_definition().is_err());
    }

    #[test]
    fn definition_rejects_items_on_object() {
        let schema = SchemaNode {
            kind: ValueKind::Object,
            items: Some(Box::new(SchemaNode::string())),
            ..SchemaNode::object(vec![])
        };
        assert!(schema.validate_definition().is_err());
    }

    #[test]
    fn definition_rejects_min_greater_than_max() {
        let schema = SchemaNode::integer().with_minimum(10.0).with_maximum(5.0);
        let error = schema.validate_definition().unwrap_err();
        assert!(error.message.contains("greater than maximum"));
    }

    #[test]
    fn definition_rejects_min_length_greater_than_max_length() {
        let schema = SchemaNode::string().with_min_length(10).with_max_length(5);
        assert!(schema.validate_definition().is_err());
    }

    #[test]
    fn definition_rejects_duplicate_field_names() {
        let schema = SchemaNode::object(vec![
            FieldSchema::required("name", SchemaNode::string()),
            FieldSchema::required("name", SchemaNode::string()),
        ]);
        let error = schema.validate_definition().unwrap_err();
        assert!(error.message.contains("duplicate field"));
    }

    #[test]
    fn definition_rejects_invalid_default() {
        let schema = SchemaNode::object(vec![FieldSchema::optional(
            "count",
            SchemaNode::integer()
                .with_minimum(1.0)
                .with_default(json!(0)),
        )]);
        let error = schema.validate_definition().unwrap_err();
        assert_eq!(error.keyword, "default");
    }

    #[test]
    fn definition_accepts_valid_schema() {
        let schema = SchemaNode::object(vec![
            FieldSchema::required("id", SchemaNode::string()),
            FieldSchema::optional(
                "version",
                SchemaNode::integer()
                    .with_minimum(1.0)
                    .with_default(json!(1)),
            ),
            FieldSchema::optional(
                "tags",
                SchemaNode::array(SchemaNode::string())
                    .with_min_items(0)
                    .with_default(json!([])),
            ),
        ]);
        assert!(schema.validate_definition().is_ok());
    }

    #[test]
    fn definition_validates_nested_schemas() {
        let schema = SchemaNode::object(vec![FieldSchema::required(
            "inner",
            SchemaNode::object(vec![FieldSchema::required(
                "count",
                SchemaNode::integer().with_min_length(1),
            )]),
        )]);
        let error = schema.validate_definition().unwrap_err();
        assert!(error.field_path.contains("inner"));
    }

    // -- JSON Schema rendering --

    #[test]
    fn renders_string_schema_with_version() {
        let schema = SchemaNode::string().with_min_length(1);
        let rendered = schema.to_json_schema();
        assert_eq!(rendered["type"], "string");
        assert_eq!(rendered["minLength"], 1);
        assert_eq!(rendered["x-schema-version"], SCHEMA_DIALECT_VERSION);
    }

    #[test]
    fn renders_object_with_properties_and_required() {
        let schema = SchemaNode::object(vec![
            FieldSchema::required("id", SchemaNode::string()),
            FieldSchema::optional("version", SchemaNode::integer().with_default(json!(1))),
        ]);
        let rendered = schema.to_json_schema();
        assert_eq!(rendered["type"], "object");
        assert_eq!(rendered["properties"]["id"]["type"], "string");
        assert_eq!(rendered["properties"]["version"]["default"], 1);
        assert_eq!(rendered["required"], json!(["id"]));
        assert_eq!(rendered["additionalProperties"], true);
        assert_eq!(rendered["x-schema-version"], SCHEMA_DIALECT_VERSION);
    }

    #[test]
    fn renders_any_type_as_empty_type() {
        let schema = SchemaNode::any();
        let rendered = schema.to_json_schema();
        assert!(rendered.get("type").is_none());
        assert_eq!(rendered["x-schema-version"], SCHEMA_DIALECT_VERSION);
    }

    #[test]
    fn renders_array_with_items() {
        let schema = SchemaNode::array(SchemaNode::string()).with_min_items(1);
        let rendered = schema.to_json_schema();
        assert_eq!(rendered["type"], "array");
        assert_eq!(rendered["items"]["type"], "string");
        assert_eq!(rendered["minItems"], 1);
    }

    #[test]
    fn renders_additional_properties_false() {
        let schema = SchemaNode::object(vec![FieldSchema::required("id", SchemaNode::string())])
            .forbid_additional_properties();
        let rendered = schema.to_json_schema();
        assert_eq!(rendered["additionalProperties"], false);
    }

    // -- JSON Pointer escaping --

    #[test]
    fn json_pointer_escapes_tilde() {
        assert_eq!(escape_pointer_segment("a~b"), "a~0b");
    }

    #[test]
    fn json_pointer_escapes_slash() {
        assert_eq!(escape_pointer_segment("a/b"), "a~1b");
    }

    #[test]
    fn json_pointer_escapes_both() {
        assert_eq!(escape_pointer_segment("~/"), "~0~1");
    }

    // -- SchemaRegistry --

    #[test]
    fn registry_rejects_duplicate_types() {
        let mut registry = SchemaRegistry::default();
        let type_id = EngineTypeId::from_static("example.actor");
        let schema = SchemaNode::object(vec![FieldSchema::required("name", SchemaNode::string())]);
        registry.register(type_id.clone(), schema).unwrap();
        let dup_schema =
            SchemaNode::object(vec![FieldSchema::required("name", SchemaNode::string())]);
        assert!(matches!(
            registry.register(type_id, dup_schema),
            Err(SchemaRegistryError::Duplicate(_))
        ));
    }

    #[test]
    fn registry_validates_definition_on_register() {
        let mut registry = SchemaRegistry::default();
        let bad_schema = SchemaNode::integer().with_min_length(1);
        assert!(registry
            .register(EngineTypeId::from_static("bad.type"), bad_schema)
            .is_err());
    }
}
