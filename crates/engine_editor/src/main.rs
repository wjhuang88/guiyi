#![forbid(unsafe_code)]

use clap::Parser;
use guiyi_engine_agent_host::{AgentHost, AgentSession};
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_command::{
    register_builtin_document_commands, CommandExecutor, CommandRegistry, EngineState,
};
use guiyi_engine_content::{
    DocumentEnvelope, DocumentStore, ProjectFilesystem, ProjectManifest, ProjectPath,
};
use guiyi_engine_core::{AgentSessionId, PermissionSet};
use guiyi_engine_protocol::{decode_line, encode_line, ToolCall, ToolResult, ToolResultStatus};
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

fn run(args: Args) -> Result<(), String> {
    let (storage, mut manifest, store, mut paths) = load_project(&args.project)?;
    let mut commands = CommandRegistry::default();
    register_builtin_document_commands(&mut commands).map_err(|error| error.to_string())?;
    register_tactical_commands(&mut commands).map_err(|error| error.to_string())?;
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).map_err(|error| error.to_string())?;
    register_tactical_queries(&mut queries).map_err(|error| error.to_string())?;
    let catalog = ToolCatalog::from_registries(&commands, &queries);
    let mut host = AgentHost::new(
        EngineState { documents: store },
        CommandExecutor::new(commands),
        QueryExecutor::new(queries),
        catalog,
    );
    let session = AgentSession::new(
        AgentSessionId::from_static("session.workbench"),
        "External workbench session",
        if args.read_only {
            PermissionSet::read_only()
        } else {
            PermissionSet::content_author()
        },
    );

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| error.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let result = match decode_line::<ToolCall>(&line) {
            Ok(call) => host
                .execute_call(&session, call)
                .map_err(|error| error.to_string())?,
            Err(error) => ToolResult {
                call_id: "invalid".into(),
                status: ToolResultStatus::Failed,
                output: json!({"error": error.to_string()}),
                diagnostics: Vec::new(),
                transaction: None,
            },
        };
        if !args.read_only && result.status == ToolResultStatus::Ok && result.transaction.is_some()
        {
            persist_project(&storage, &mut manifest, &mut paths, &host.state.documents)?;
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

type DocumentPaths = BTreeMap<guiyi_engine_core::DocumentId, ProjectPath>;

fn load_project(
    root: &Path,
) -> Result<
    (
        ProjectFilesystem,
        ProjectManifest,
        DocumentStore,
        DocumentPaths,
    ),
    String,
> {
    let storage = ProjectFilesystem::open(root).map_err(|error| error.to_string())?;
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
    storage: &ProjectFilesystem,
    manifest: &mut ProjectManifest,
    paths: &mut DocumentPaths,
    store: &DocumentStore,
) -> Result<(), String> {
    let stale = paths
        .keys()
        .filter(|id| store.get(id).is_err())
        .cloned()
        .collect::<Vec<_>>();
    for id in stale {
        if let Some(relative) = paths.remove(&id) {
            if storage
                .exists(&relative)
                .map_err(|error| error.to_string())?
            {
                storage
                    .remove_file(&relative)
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    for (id, document) in store.iter() {
        let relative = if let Some(existing) = paths.get(id) {
            existing.clone()
        } else {
            let path = ProjectPath::new(format!("content/generated/{}.json", id.as_str()))
                .map_err(|error| error.to_string())?;
            manifest.documents.push(path.clone());
            paths.insert(id.clone(), path.clone());
            path
        };
        storage
            .save_json(&relative, document)
            .map_err(|error| error.to_string())?;
    }
    manifest
        .documents
        .retain(|path| paths.values().any(|known| known == path));
    storage
        .save_json(&logical("engine-project.json")?, manifest)
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_content::PROJECT_PATH_INVALID;
    use std::time::{SystemTime, UNIX_EPOCH};

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
