#![forbid(unsafe_code)]

//! Authoring project state with command-driven undo, redo, and crash-safe autosave.

use guiyi_engine_command::{
    CommandContext, CommandError, CommandExecutor, CommandRequest, EngineState, TransactionReport,
    TransactionStatus,
};
use guiyi_engine_content::{ContentError, ProjectPath, ProjectStorage, ProjectTransaction};
use guiyi_engine_core::AgentSessionId;
use serde_json::json;
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

    pub fn autosave(
        &self,
        storage: &ProjectStorage,
        directory: &ProjectPath,
    ) -> Result<(), AuthoringError> {
        let mut transaction = ProjectTransaction::generated(
            "autosave",
            AgentSessionId::from_static("session.authoring-autosave"),
            "authoring.autosave",
            json!({
                "kind": "autosave",
                "documents": self.state.documents.len()
            }),
        )?;
        for (id, document) in self.state.documents.iter() {
            transaction.write_json(directory.join(format!("{}.json", id.as_str()))?, document)?;
        }
        if !transaction.is_empty() {
            storage.commit(transaction)?;
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
    ProjectPath(#[from] guiyi_engine_content::ProjectPathError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_command::{register_builtin_document_commands, CommandRegistry};
    use guiyi_engine_core::{PermissionSet, ToolId};
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn autosave_uses_crash_safe_project_transaction() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "guiyi-authoring-autosave-{}-{nonce}",
            std::process::id()
        ));
        let storage = ProjectStorage::create(&root).unwrap();
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
                        "id": "doc.autosave",
                        "type_id": "example",
                        "display_name": "Autosave"
                    }),
                    dry_run: false,
                },
                &CommandContext {
                    actor: "test".into(),
                    permissions: PermissionSet::content_author(),
                },
            )
            .unwrap();
        let directory = ProjectPath::new(".agent-sessions/autosave").unwrap();
        project.autosave(&storage, &directory).unwrap();
        assert!(storage
            .exists(&ProjectPath::new(".agent-sessions/autosave/doc.autosave.json").unwrap())
            .unwrap());
        let audit = storage.audit_records().unwrap();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].session_id.as_str(), "session.authoring-autosave");
        std::fs::remove_dir_all(root).unwrap();
    }
}
