#![forbid(unsafe_code)]

//! Agent capability discovery across command and query registries.

use guiyi_engine_command::{CommandDescriptor, CommandRegistry};
use guiyi_engine_core::{PermissionSet, ToolId};
use guiyi_engine_query::{QueryDescriptor, QueryRegistry};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Command,
    Query,
}

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
    pub fn from_registries(commands: &CommandRegistry, queries: &QueryRegistry) -> Self {
        let mut catalog = Self::default();
        for descriptor in commands.descriptors() {
            catalog.insert_command(descriptor);
        }
        for descriptor in queries.descriptors() {
            catalog.insert_query(descriptor);
        }
        catalog
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

    fn insert_command(&mut self, value: CommandDescriptor) {
        self.tools.insert(
            value.id.clone(),
            ToolDescriptor {
                id: value.id,
                kind: ToolKind::Command,
                title: value.title,
                description: value.description,
                input_schema: value.input_schema,
                output_schema: value.output_schema,
                required_permissions: value.required_permissions,
                side_effects: value.side_effects,
                related_tools: value.related_tools,
            },
        );
    }

    fn insert_query(&mut self, value: QueryDescriptor) {
        self.tools.insert(
            value.id.clone(),
            ToolDescriptor {
                id: value.id,
                kind: ToolKind::Query,
                title: value.title,
                description: value.description,
                input_schema: value.input_schema,
                output_schema: value.output_schema,
                required_permissions: value.required_permissions,
                side_effects: Vec::new(),
                related_tools: value.related_tools,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_command::{register_builtin_document_commands, CommandRegistry};
    use guiyi_engine_query::{register_builtin_queries, QueryRegistry};

    #[test]
    fn catalog_combines_commands_and_queries() {
        let mut commands = CommandRegistry::default();
        register_builtin_document_commands(&mut commands).unwrap();
        let mut queries = QueryRegistry::default();
        register_builtin_queries(&mut queries).unwrap();
        let catalog = ToolCatalog::from_registries(&commands, &queries);
        assert!(catalog.get(&ToolId::from_static("document.create")).is_some());
        assert!(catalog
            .get(&ToolId::from_static("project.documents.list"))
            .is_some());
    }
}
