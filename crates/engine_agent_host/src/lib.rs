#![forbid(unsafe_code)]

//! Agent sessions, budgets, loop adapters, permission checks, and tool routing.

use guiyi_engine_agent_tools::{ToolCatalog, ToolKind};
use guiyi_engine_command::{
    CommandContext, CommandError, CommandExecutor, CommandRequest, EngineState, TransactionStatus,
};
use guiyi_engine_core::{AgentSessionId, DocumentId, PermissionSet};
use guiyi_engine_protocol::{ToolCall, ToolResult, ToolResultStatus};
use guiyi_engine_query::{QueryContext, QueryExecutor, QueryRequest};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Ready,
    Running,
    Completed,
    Stopped,
    BudgetExceeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBudget {
    pub max_actions: u32,
}

impl Default for AgentBudget {
    fn default() -> Self {
        Self { max_actions: 32 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentActionRecord {
    pub sequence: u32,
    pub call: ToolCall,
    pub result: ToolResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: AgentSessionId,
    pub objective: String,
    pub permissions: PermissionSet,
    pub working_set: Vec<DocumentId>,
    pub budget: AgentBudget,
    pub status: SessionStatus,
    pub actions: Vec<AgentActionRecord>,
    pub final_summary: Option<String>,
}

impl AgentSession {
    pub fn new(
        id: AgentSessionId,
        objective: impl Into<String>,
        permissions: PermissionSet,
    ) -> Self {
        Self {
            id,
            objective: objective.into(),
            permissions,
            working_set: Vec::new(),
            budget: AgentBudget::default(),
            status: SessionStatus::Ready,
            actions: Vec::new(),
            final_summary: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentDirective {
    Tool(ToolCall),
    Complete { summary: String },
    Stop { reason: String },
}

pub trait AgentLoopDriver {
    fn next_action(
        &mut self,
        session: &AgentSession,
        catalog: &ToolCatalog,
        last_result: Option<&ToolResult>,
    ) -> Result<AgentDirective, AgentHostError>;
}

pub struct AgentHost {
    pub state: EngineState,
    command_executor: CommandExecutor,
    query_executor: QueryExecutor,
    catalog: ToolCatalog,
}

impl AgentHost {
    pub fn new(
        state: EngineState,
        command_executor: CommandExecutor,
        query_executor: QueryExecutor,
        catalog: ToolCatalog,
    ) -> Self {
        Self {
            state,
            command_executor,
            query_executor,
            catalog,
        }
    }

    pub fn catalog(&self) -> &ToolCatalog {
        &self.catalog
    }

    pub fn execute_call(
        &mut self,
        session: &AgentSession,
        call: ToolCall,
    ) -> Result<ToolResult, AgentHostError> {
        let descriptor = self
            .catalog
            .get(&call.tool)
            .ok_or_else(|| AgentHostError::ToolNotFound(call.tool.to_string()))?;
        if !session
            .permissions
            .contains_all(&descriptor.required_permissions)
        {
            return Ok(ToolResult {
                call_id: call.id,
                status: ToolResultStatus::Rejected,
                output: json!({"error": "permission_denied"}),
                diagnostics: Vec::new(),
                transaction: None,
            });
        }

        let tool_kind = descriptor.kind;
        match tool_kind {
            ToolKind::Command => {
                let result = self.command_executor.execute(
                    CommandRequest {
                        command: call.tool,
                        input: call.input,
                        dry_run: call.dry_run,
                    },
                    &CommandContext {
                        actor: session.id.to_string(),
                        permissions: session.permissions.clone(),
                    },
                    &mut self.state,
                );
                match result {
                    Ok(report) => Ok(ToolResult {
                        call_id: call.id,
                        status: ToolResultStatus::Ok,
                        output: report.output.clone(),
                        diagnostics: report
                            .diagnostics
                            .diagnostics
                            .iter()
                            .map(|item| serde_json::to_value(item).unwrap_or(Value::Null))
                            .collect(),
                        transaction: Some(serde_json::to_value(&report)?),
                    }),
                    Err(CommandError::ValidationFailed(bag)) => Ok(ToolResult {
                        call_id: call.id,
                        status: ToolResultStatus::Rejected,
                        output: json!({"error": "validation_failed"}),
                        diagnostics: bag
                            .diagnostics
                            .iter()
                            .map(|item| serde_json::to_value(item).unwrap_or(Value::Null))
                            .collect(),
                        transaction: None,
                    }),
                    Err(error) => Ok(ToolResult {
                        call_id: call.id,
                        status: ToolResultStatus::Failed,
                        output: json!({"error": error.to_string()}),
                        diagnostics: Vec::new(),
                        transaction: None,
                    }),
                }
            }
            ToolKind::Query => {
                let output = self.query_executor.execute(
                    QueryRequest {
                        query: call.tool,
                        input: call.input,
                    },
                    &QueryContext {
                        actor: session.id.to_string(),
                        permissions: session.permissions.clone(),
                    },
                    &self.state.documents,
                );
                match output {
                    Ok(output) => Ok(ToolResult {
                        call_id: call.id,
                        status: ToolResultStatus::Ok,
                        output,
                        diagnostics: Vec::new(),
                        transaction: None,
                    }),
                    Err(error) => Ok(ToolResult {
                        call_id: call.id,
                        status: ToolResultStatus::Failed,
                        output: json!({"error": error.to_string()}),
                        diagnostics: Vec::new(),
                        transaction: None,
                    }),
                }
            }
        }
    }

    pub fn run(
        &mut self,
        driver: &mut dyn AgentLoopDriver,
        session: &mut AgentSession,
    ) -> Result<(), AgentHostError> {
        session.status = SessionStatus::Running;
        let mut last_result: Option<ToolResult> = None;
        loop {
            if session.actions.len() as u32 >= session.budget.max_actions {
                session.status = SessionStatus::BudgetExceeded;
                return Err(AgentHostError::BudgetExceeded);
            }
            match driver.next_action(session, &self.catalog, last_result.as_ref())? {
                AgentDirective::Tool(call) => {
                    let result = self.execute_call(session, call.clone())?;
                    session.actions.push(AgentActionRecord {
                        sequence: session.actions.len() as u32 + 1,
                        call,
                        result: result.clone(),
                    });
                    last_result = Some(result);
                }
                AgentDirective::Complete { summary } => {
                    session.status = SessionStatus::Completed;
                    session.final_summary = Some(summary);
                    return Ok(());
                }
                AgentDirective::Stop { reason } => {
                    session.status = SessionStatus::Stopped;
                    session.final_summary = Some(reason);
                    return Ok(());
                }
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum AgentHostError {
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("agent action budget exceeded")]
    BudgetExceeded,
    #[error("agent driver failed: {0}")]
    Driver(String),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

pub struct ScriptedAgentDriver {
    actions: VecDeque<AgentDirective>,
}

impl ScriptedAgentDriver {
    pub fn new(actions: impl IntoIterator<Item = AgentDirective>) -> Self {
        Self {
            actions: actions.into_iter().collect(),
        }
    }
}

impl AgentLoopDriver for ScriptedAgentDriver {
    fn next_action(
        &mut self,
        _session: &AgentSession,
        _catalog: &ToolCatalog,
        _last_result: Option<&ToolResult>,
    ) -> Result<AgentDirective, AgentHostError> {
        self.actions
            .pop_front()
            .ok_or_else(|| AgentHostError::Driver("script exhausted without completion".into()))
    }
}

pub fn transaction_was_applied(result: &ToolResult) -> bool {
    result
        .transaction
        .as_ref()
        .and_then(|value| value.get("status"))
        == Some(&json!(TransactionStatus::Applied))
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_agent_tools::ToolCatalog;
    use guiyi_engine_command::{register_builtin_document_commands, CommandRegistry};
    use guiyi_engine_core::{PermissionSet, ToolId};
    use guiyi_engine_query::{register_builtin_queries, QueryRegistry};

    #[test]
    fn scripted_agent_creates_and_queries_a_document() {
        let mut command_registry = CommandRegistry::default();
        register_builtin_document_commands(&mut command_registry).unwrap();
        let mut query_registry = QueryRegistry::default();
        register_builtin_queries(&mut query_registry).unwrap();
        let catalog = ToolCatalog::from_registries(&command_registry, &query_registry);
        let mut host = AgentHost::new(
            EngineState::default(),
            CommandExecutor::new(command_registry),
            QueryExecutor::new(query_registry),
            catalog,
        );
        let mut session = AgentSession::new(
            AgentSessionId::from_static("session.test"),
            "Create a document",
            PermissionSet::content_author(),
        );
        let mut driver = ScriptedAgentDriver::new([
            AgentDirective::Tool(ToolCall {
                id: "call-1".into(),
                tool: ToolId::from_static("document.create"),
                input: json!({
                    "id": "doc.agent",
                    "type_id": "example.document",
                    "display_name": "Agent document"
                }),
                dry_run: false,
            }),
            AgentDirective::Tool(ToolCall {
                id: "call-2".into(),
                tool: ToolId::from_static("project.documents.list"),
                input: json!({}),
                dry_run: false,
            }),
            AgentDirective::Complete {
                summary: "Created and checked the document".into(),
            },
        ]);
        host.run(&mut driver, &mut session).unwrap();
        assert_eq!(session.status, SessionStatus::Completed);
        assert_eq!(host.state.documents.len(), 1);
    }
}
