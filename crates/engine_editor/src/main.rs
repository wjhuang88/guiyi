#![forbid(unsafe_code)]

use clap::Parser;
use guiyi_engine_agent_host::{AgentHost, AgentSession};
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_command::{
    register_builtin_document_commands, CommandExecutor, CommandRegistry, EngineState,
};
use guiyi_engine_content::{
    load_json, save_json, DocumentEnvelope, DocumentStore, ProjectManifest,
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

fn run(args: Args) -> Result<(), String> {
    let (mut manifest, store, mut paths) = load_project(&args.project)?;
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
            persist_project(
                &args.project,
                &mut manifest,
                &mut paths,
                &host.state.documents,
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

fn load_project(
    root: &Path,
) -> Result<
    (
        ProjectManifest,
        DocumentStore,
        BTreeMap<guiyi_engine_core::DocumentId, PathBuf>,
    ),
    String,
> {
    let manifest: ProjectManifest =
        load_json(&root.join("engine-project.json")).map_err(|error| error.to_string())?;
    let mut store = DocumentStore::default();
    let mut paths = BTreeMap::new();
    for relative in &manifest.documents {
        let document: DocumentEnvelope =
            load_json(&root.join(relative)).map_err(|error| error.to_string())?;
        paths.insert(document.header.id.clone(), relative.clone());
        store.insert(document).map_err(|error| error.to_string())?;
    }
    Ok((manifest, store, paths))
}

fn persist_project(
    root: &Path,
    manifest: &mut ProjectManifest,
    paths: &mut BTreeMap<guiyi_engine_core::DocumentId, PathBuf>,
    store: &DocumentStore,
) -> Result<(), String> {
    let stale = paths
        .keys()
        .filter(|id| store.get(id).is_err())
        .cloned()
        .collect::<Vec<_>>();
    for id in stale {
        if let Some(relative) = paths.remove(&id) {
            let file = root.join(relative);
            if file.exists() {
                std::fs::remove_file(file).map_err(|error| error.to_string())?;
            }
        }
    }
    for (id, document) in store.iter() {
        let relative = paths.entry(id.clone()).or_insert_with(|| {
            let path = PathBuf::from(format!("content/generated/{}.json", id.as_str()));
            manifest.documents.push(path.clone());
            path
        });
        save_json(&root.join(relative.as_path()), document).map_err(|error| error.to_string())?;
    }
    manifest
        .documents
        .retain(|path| paths.values().any(|known| known == path));
    save_json(&root.join("engine-project.json"), manifest).map_err(|error| error.to_string())?;
    Ok(())
}
