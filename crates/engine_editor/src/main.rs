#![forbid(unsafe_code)]

use clap::Parser;
use guiyi_engine_agent_host::{AgentHost, AgentSession};
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_command::{
    register_builtin_document_commands, CommandExecutor, CommandRegistry, EngineState,
};
use guiyi_engine_content::{
    DocumentEnvelope, DocumentStore, ProjectManifest, ProjectPath, ProjectStorage,
    ProjectTransaction,
};
use guiyi_engine_core::{AgentSessionId, DocumentId, PermissionSet, TransactionId};
use guiyi_engine_protocol::{encode_line, ToolCall, ToolResult, ToolResultStatus};
use guiyi_engine_query::{register_builtin_queries, QueryExecutor, QueryRegistry};
use serde_json::json;
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tactical_rpg_tools::{register_tactical_commands, register_tactical_queries};

#[derive(Debug, Parser)]
#[command(name = "guiyi-engine-workbench")]
#[command(about = "JSONL command/query workbench for agents and automation")]
struct Args {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    read_only: bool,
    /// Maximum number of valid tool calls that may enter execution.
    #[arg(long, default_value_t = 32)]
    max_actions: u32,
    /// Restrict the session to these document IDs. Repeating the option adds
    /// documents. Omitting it means unrestricted project access.
    #[arg(long = "working-set")]
    working_set: Vec<String>,
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("workbench failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn logical(value: &str) -> Result<ProjectPath, String> {
    ProjectPath::new(value).map_err(|error| error.to_string())
}

fn create_session(args: &Args) -> Result<AgentSession, String> {
    let mut session = AgentSession::new(
        AgentSessionId::from_static("session.workbench"),
        "External workbench session",
        if args.read_only {
            PermissionSet::read_only()
        } else {
            PermissionSet::content_author()
        },
    );
    session.budget.max_actions = args.max_actions;
    session.working_set = args
        .working_set
        .iter()
        .map(|value| DocumentId::new(value.clone()).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(session)
}

fn run(args: Args) -> Result<(), String> {
    let (storage, mut manifest, store, mut paths) = load_project(&args.project)?;
    let mut commands = CommandRegistry::default();
    register_builtin_document_commands(&mut commands).map_err(|error| error.to_string())?;
    register_tactical_commands(&mut commands).map_err(|error| error.to_string())?;
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).map_err(|error| error.to_string())?;
    register_tactical_queries(&mut queries).map_err(|error| error.to_string())?;
    let catalog =
        ToolCatalog::from_registries(&commands, &queries).map_err(|error| error.to_string())?;
    let mut host = AgentHost::new(
        EngineState { documents: store },
        CommandExecutor::new(commands),
        QueryExecutor::new(queries),
        catalog,
    );
    let mut session = create_session(&args)?;

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| error.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let result = execute_line(&mut host, &mut session, &line);
        if !args.read_only && result.status == ToolResultStatus::Ok && result.transaction.is_some()
        {
            persist_project(
                &storage,
                &mut manifest,
                &mut paths,
                &host.state.documents,
                &session,
                &result,
            )?;
        }
        writeln!(
            stdout,
            "{}",
            encode_line(&result).map_err(|error| error.to_string())?
        )
        .map_err(|error| error.to_string())?;
        stdout.flush().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn execute_line(host: &mut AgentHost, session: &mut AgentSession, line: &str) -> ToolResult {
    let value = match serde_json::from_str::<serde_json::Value>(line) {
        Ok(value) => value,
        Err(error) => {
            return protocol_error("invalid", "PROTOCOL_INVALID_JSONL", error.to_string())
        }
    };
    let call_id = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("invalid")
        .to_string();
    match serde_json::from_value::<ToolCall>(value) {
        Ok(call) => host.execute(session, call),
        Err(error) => protocol_error(&call_id, "PROTOCOL_INVALID_CALL", error.to_string()),
    }
}

fn protocol_error(call_id: &str, code: &str, message: String) -> ToolResult {
    ToolResult {
        call_id: call_id.into(),
        status: ToolResultStatus::Rejected,
        output: json!({
            "error": {
                "code": code,
                "message": message
            }
        }),
        diagnostics: Vec::new(),
        transaction: None,
    }
}

type DocumentPaths = BTreeMap<DocumentId, ProjectPath>;

fn load_project(
    root: &Path,
) -> Result<
    (
        ProjectStorage,
        ProjectManifest,
        DocumentStore,
        DocumentPaths,
    ),
    String,
> {
    let storage = ProjectStorage::open(root).map_err(|error| error.to_string())?;
    let manifest: ProjectManifest = storage
        .load_json(&logical("engine-project.json")?)
        .map_err(|error| error.to_string())?;
    let mut store = DocumentStore::default();
    let mut paths = BTreeMap::new();
    for relative in &manifest.documents {
        let document: DocumentEnvelope = storage
            .load_json(relative)
            .map_err(|error| error.to_string())?;
        paths.insert(document.header.id.clone(), relative.clone());
        store.insert(document).map_err(|error| error.to_string())?;
    }
    Ok((storage, manifest, store, paths))
}

fn persist_project(
    storage: &ProjectStorage,
    manifest: &mut ProjectManifest,
    paths: &mut DocumentPaths,
    store: &DocumentStore,
    session: &AgentSession,
    result: &ToolResult,
) -> Result<(), String> {
    let report = result
        .transaction
        .clone()
        .ok_or_else(|| "successful mutation has no transaction report".to_string())?;
    let transaction_id = report
        .get("transaction_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "transaction report has no transaction_id".to_string())?;
    let transaction_id = TransactionId::new(transaction_id).map_err(|error| error.to_string())?;
    let mut transaction = ProjectTransaction::new(
        transaction_id,
        session.id.clone(),
        session.id.to_string(),
        report,
    );
    let mut next_manifest = manifest.clone();
    let mut next_paths = paths.clone();

    let stale = next_paths
        .keys()
        .filter(|id| store.get(id).is_err())
        .cloned()
        .collect::<Vec<_>>();
    for id in stale {
        if let Some(relative) = next_paths.remove(&id) {
            transaction.delete(relative);
        }
    }
    for (id, document) in store.iter() {
        let relative = if let Some(existing) = next_paths.get(id) {
            existing.clone()
        } else {
            let path = ProjectPath::new(format!("content/generated/{}.json", id.as_str()))
                .map_err(|error| error.to_string())?;
            next_manifest.documents.push(path.clone());
            next_paths.insert(id.clone(), path.clone());
            path
        };
        transaction
            .write_json(relative, document)
            .map_err(|error| error.to_string())?;
    }
    next_manifest
        .documents
        .retain(|path| next_paths.values().any(|known| known == path));
    transaction
        .write_manifest_json(logical("engine-project.json")?, &next_manifest)
        .map_err(|error| error.to_string())?;
    storage
        .commit(transaction)
        .map_err(|error| error.to_string())?;
    *manifest = next_manifest;
    *paths = next_paths;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_agent_host::{SessionStatus, AGENT_BUDGET_EXCEEDED};
    use guiyi_engine_content::PROJECT_PATH_INVALID;
    use serde_json::Value;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_host() -> AgentHost {
        let mut commands = CommandRegistry::default();
        register_builtin_document_commands(&mut commands).unwrap();
        let mut queries = QueryRegistry::default();
        register_builtin_queries(&mut queries).unwrap();
        let catalog = ToolCatalog::from_registries(&commands, &queries)
            .expect("built-in tool IDs are unique");
        AgentHost::new(
            EngineState::default(),
            CommandExecutor::new(commands),
            QueryExecutor::new(queries),
            catalog,
        )
    }

    fn error_code(result: &ToolResult) -> Option<&str> {
        result
            .output
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str)
    }

    #[test]
    fn workbench_path_enforces_budget_and_records_every_result() {
        let mut host = test_host();
        let args = Args {
            project: PathBuf::from("."),
            read_only: true,
            max_actions: 1,
            working_set: Vec::new(),
        };
        let mut session = create_session(&args).unwrap();
        let call = r#"{"id":"call-1","tool":"project.documents.list","input":{}}"#;
        let first = execute_line(&mut host, &mut session, call);
        assert_eq!(first.status, ToolResultStatus::Ok);
        let second = execute_line(&mut host, &mut session, call);
        assert_eq!(second.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&second), Some(AGENT_BUDGET_EXCEEDED));
        assert_eq!(session.actions_used, 1);
        assert_eq!(session.actions.len(), 2);
        assert_eq!(session.status, SessionStatus::BudgetExceeded);
    }

    #[test]
    fn workbench_session_parses_working_set() {
        let args = Args {
            project: PathBuf::from("."),
            read_only: true,
            max_actions: 4,
            working_set: vec!["stage.a".into(), "stage.b".into()],
        };
        let session = create_session(&args).unwrap();
        assert_eq!(session.working_set.len(), 2);
        assert_eq!(session.budget.max_actions, 4);
    }

    #[test]
    fn workbench_rejects_manifest_path_escape_without_reading_external_file() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "guiyi-workbench-sandbox-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("engine-project.json"),
            r#"{
                "project_id": "project.test",
                "name": "Test",
                "engine_api_version": "0.1.0",
                "content_schema_version": 1,
                "documents": ["../../outside.json"]
            }"#,
        )
        .unwrap();
        let error = load_project(&root).unwrap_err();
        assert!(error.contains(PROJECT_PATH_INVALID));
        assert!(error.contains("../../outside.json"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
