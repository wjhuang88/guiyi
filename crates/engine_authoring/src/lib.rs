#![forbid(unsafe_code)]

//! Authoring project state with command-driven undo, redo, and autosave.

use guiyi_engine_command::{
    CommandContext, CommandError, CommandExecutor, CommandRequest, EngineState, TransactionReport,
    TransactionStatus,
};
use guiyi_engine_content::{save_json, ContentError};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Default)]
pub struct AuthoringProject {
    pub state: EngineState,
    undo: Vec<EngineState>,
    redo: Vec<EngineState>,
}

impl AuthoringProject {
    pub fn apply(
        &mut self,
        executor: &mut CommandExecutor,
        request: CommandRequest,
        context: &CommandContext,
    ) -> Result<TransactionReport, AuthoringError> {
        let before = self.state.clone();
        let report = executor.execute(request, context, &mut self.state)?;
        if report.status == TransactionStatus::Applied {
            self.undo.push(before);
            self.redo.clear();
        }
        Ok(report)
    }

    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(self.state.clone());
            self.state = previous;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.state.clone());
            self.state = next;
            true
        } else {
            false
        }
    }

    pub fn autosave(&self, directory: &Path) -> Result<(), AuthoringError> {
        fs::create_dir_all(directory)?;
        for (id, document) in self.state.documents.iter() {
            save_json(&directory.join(format!("{}.json", id.as_str())), document)?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum AuthoringError {
    #[error(transparent)]
    Command(#[from] CommandError),
    #[error(transparent)]
    Content(#[from] ContentError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_command::{register_builtin_document_commands, CommandRegistry};
    use guiyi_engine_core::{PermissionSet, ToolId};
    use serde_json::json;

    #[test]
    fn undo_and_redo_restore_command_states() {
        let mut registry = CommandRegistry::default();
        register_builtin_document_commands(&mut registry).unwrap();
        let mut executor = CommandExecutor::new(registry);
        let mut project = AuthoringProject::default();
        project
            .apply(
                &mut executor,
                CommandRequest {
                    command: ToolId::from_static("document.create"),
                    input: json!({
                        "id": "doc.undo",
                        "type_id": "example",
                        "display_name": "Undo"
                    }),
                    dry_run: false,
                },
                &CommandContext {
                    actor: "test".into(),
                    permissions: PermissionSet::content_author(),
                },
            )
            .unwrap();
        assert_eq!(project.state.documents.len(), 1);
        assert!(project.undo());
        assert!(project.state.documents.is_empty());
        assert!(project.redo());
        assert_eq!(project.state.documents.len(), 1);
    }
}
