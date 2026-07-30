#![forbid(unsafe_code)]

//! Agent capability discovery across command and query registries.

use guiyi_engine_command::{CommandDescriptor, CommandRegistry};
use guiyi_engine_core::{PermissionSet, ToolId};
use guiyi_engine_query::{QueryDescriptor, QueryRegistry};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub const TOOL_CATALOG_ID_COLLISION: &str = "TOOL_CATALOG_ID_COLLISION";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Command,
    Query,
}

impl fmt::Display for ToolKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Command => "command",
            Self::Query => "query",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCatalogError {
    pub code: String,
    pub tool_id: ToolId,
    pub existing_kind: ToolKind,
    pub incoming_kind: ToolKind,
}

impl fmt::Display for ToolCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: tool ID `{}` is registered as both {} and {}",
            self.code, self.tool_id, self.existing_kind, self.incoming_kind
        )
    }
}

impl Error for ToolCatalogError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: ToolId,
    pub kind: ToolKind,
    pub title: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub required_permissions: PermissionSet,
    pub side_effects: Vec<String>,
    pub related_tools: Vec<ToolId>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolCatalog {
    tools: BTreeMap<ToolId, ToolDescriptor>,
}

impl ToolCatalog {
    pub fn from_registries(
        commands: &CommandRegistry,
        queries: &QueryRegistry,
    ) -> Result<Self, ToolCatalogError> {
        Self::from_descriptors(commands.descriptors(), queries.descriptors())
    }

    fn from_descriptors(
        commands: impl IntoIterator<Item = CommandDescriptor>,
        queries: impl IntoIterator<Item = QueryDescriptor>,
    ) -> Result<Self, ToolCatalogError> {
        let mut catalog = Self::default();
        for descriptor in commands {
            catalog.insert_command(descriptor)?;
        }
        for descriptor in queries {
            catalog.insert_query(descriptor)?;
        }
        Ok(catalog)
    }

    pub fn get(&self, id: &ToolId) -> Option<&ToolDescriptor> {
        self.tools.get(id)
    }

    pub fn list(&self) -> impl Iterator<Item = &ToolDescriptor> {
        self.tools.values()
    }

    pub fn as_json(&self) -> Value {
        serde_json::to_value(self.tools.values().collect::<Vec<_>>())
            .expect("tool descriptors are serializable")
    }

    fn insert_command(&mut self, value: CommandDescriptor) -> Result<(), ToolCatalogError> {
        self.insert(ToolDescriptor {
            id: value.id,
            kind: ToolKind::Command,
            title: value.title,
            description: value.description,
            input_schema: value.input_schema,
            output_schema: value.output_schema,
            required_permissions: value.required_permissions,
            side_effects: value.side_effects,
            related_tools: value.related_tools,
        })
    }

    fn insert_query(&mut self, value: QueryDescriptor) -> Result<(), ToolCatalogError> {
        self.insert(ToolDescriptor {
            id: value.id,
            kind: ToolKind::Query,
            title: value.title,
            description: value.description,
            input_schema: value.input_schema,
            output_schema: value.output_schema,
            required_permissions: value.required_permissions,
            side_effects: Vec::new(),
            related_tools: value.related_tools,
        })
    }

    fn insert(&mut self, descriptor: ToolDescriptor) -> Result<(), ToolCatalogError> {
        if let Some(existing) = self.tools.get(&descriptor.id) {
            return Err(ToolCatalogError {
                code: TOOL_CATALOG_ID_COLLISION.into(),
                tool_id: descriptor.id,
                existing_kind: existing.kind,
                incoming_kind: descriptor.kind,
            });
        }
        self.tools.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_command::{register_builtin_document_commands, CommandRegistry};
    use guiyi_engine_query::{register_builtin_queries, QueryRegistry};
    use serde_json::json;

    fn command(id: &str) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::new(id).unwrap(),
            title: "Command".into(),
            description: "test command".into(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::default(),
            side_effects: Vec::new(),
            related_tools: Vec::new(),
        }
    }

    fn query(id: &str) -> QueryDescriptor {
        QueryDescriptor {
            id: ToolId::new(id).unwrap(),
            title: "Query".into(),
            description: "test query".into(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::default(),
            related_tools: Vec::new(),
        }
    }

    #[test]
    fn catalog_combines_commands_and_queries_deterministically() {
        let mut commands = CommandRegistry::default();
        register_builtin_document_commands(&mut commands).unwrap();
        let mut queries = QueryRegistry::default();
        register_builtin_queries(&mut queries).unwrap();
        let catalog = ToolCatalog::from_registries(&commands, &queries).unwrap();
        assert!(catalog
            .get(&ToolId::from_static("document.create"))
            .is_some());
        assert!(catalog
            .get(&ToolId::from_static("project.documents.list"))
            .is_some());
        let ids = catalog
            .list()
            .map(|descriptor| descriptor.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn cross_kind_collision_is_rejected_without_overwrite() {
        let error = ToolCatalog::from_descriptors([command("shared.tool")], [query("shared.tool")])
            .unwrap_err();
        assert_eq!(error.code, TOOL_CATALOG_ID_COLLISION);
        assert_eq!(error.tool_id.as_str(), "shared.tool");
        assert_eq!(error.existing_kind, ToolKind::Command);
        assert_eq!(error.incoming_kind, ToolKind::Query);
        assert!(error.to_string().contains("command"));
        assert!(error.to_string().contains("query"));
    }

    #[test]
    fn same_kind_descriptor_collision_is_also_rejected() {
        let error = ToolCatalog::from_descriptors(
            [command("duplicate.tool"), command("duplicate.tool")],
            [],
        )
        .unwrap_err();
        assert_eq!(error.code, TOOL_CATALOG_ID_COLLISION);
        assert_eq!(error.existing_kind, ToolKind::Command);
        assert_eq!(error.incoming_kind, ToolKind::Command);
    }
}
