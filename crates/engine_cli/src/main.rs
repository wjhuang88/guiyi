#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use guiyi_engine_agent_tools::ToolCatalog;
use guiyi_engine_build::BuildPipeline;
use guiyi_engine_command::{register_builtin_document_commands, CommandRegistry};
use guiyi_engine_content::{
    load_json, save_json, CompileContext, CompilerRegistry, ContentReference, DocumentEnvelope,
    DocumentStore, ProjectManifest,
};
use guiyi_engine_core::{EngineVersion, ProjectId};
use guiyi_engine_query::{register_builtin_queries, QueryRegistry};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tactical_rpg_content::{StageCompiler, StageDocument, STAGE_DOCUMENT_TYPE};
use tactical_rpg_tools::{register_tactical_commands, register_tactical_queries};
use tactical_rpg_validation::validate_stage;

#[derive(Debug, Parser)]
#[command(name = "guiyi-engine-cli")]
#[command(about = "AI-native GUIYI Engine project toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Version,
    Init {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        name: String,
    },
    Doctor {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Capabilities {
        #[arg(long)]
        json: bool,
    },
    Validate {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Compile {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::Version => {
            println!("{}", EngineVersion::CURRENT);
            Ok(())
        }
        Commands::Init { path, name } => init_project(&path, &name),
        Commands::Doctor { project, json } => doctor(&project, json),
        Commands::Capabilities { json } => capabilities(json),
        Commands::Validate { project, json } => validate_project(&project, json),
        Commands::Compile { project, out, json } => compile_project(&project, &out, json),
    }
}

fn init_project(root: &Path, name: &str) -> Result<(), String> {
    if root.exists() && fs::read_dir(root).map_err(|error| error.to_string())?.next().is_some() {
        return Err(format!("target directory is not empty: {}", root.display()));
    }
    fs::create_dir_all(root.join("content/stages")).map_err(|error| error.to_string())?;
    fs::create_dir_all(root.join("content/definitions")).map_err(|error| error.to_string())?;
    fs::create_dir_all(root.join("artifacts")).map_err(|error| error.to_string())?;
    fs::create_dir_all(root.join(".agent-sessions")).map_err(|error| error.to_string())?;

    let project_id = ProjectId::new(format!("project.{}", slug(name)))
        .map_err(|error| error.to_string())?;
    let mut stage = StageDocument::new_hex(
        guiyi_engine_core::DocumentId::from_static("stage.demo"),
        "Demo Stage",
        8,
        8,
    );
    stage.objects.push(tactical_rpg_content::StageObject {
        id: guiyi_engine_core::ObjectId::from_static("spawn.player"),
        position: tactical_rpg_content::HexCoord::new(1, 1),
        object: tactical_rpg_content::StageObjectKind::SpawnPoint {
            profile: "player".into(),
        },
        properties: json!({}),
    });
    let document_path = PathBuf::from("content/stages/demo.stage.json");
    save_json(&root.join(&document_path), &stage.to_envelope().map_err(|e| e.to_string())?)
        .map_err(|error| error.to_string())?;
    let manifest = ProjectManifest {
        project_id,
        name: name.into(),
        engine_api_version: EngineVersion::CURRENT.to_string(),
        content_schema_version: 1,
        enabled_extensions: vec!["tactical_rpg".into()],
        documents: vec![document_path],
    };
    save_json(&root.join("engine-project.json"), &manifest).map_err(|error| error.to_string())?;
    fs::write(
        root.join("README.md"),
        format!(
            "# {name}\n\nCreated by GUIYI Engine. Run `guiyi-engine-cli doctor --project .`.\n"
        ),
    )
    .map_err(|error| error.to_string())?;
    println!("initialized {}", root.display());
    Ok(())
}

fn doctor(root: &Path, as_json: bool) -> Result<(), String> {
    let mut checks = Vec::new();
    checks.push(check(root.join("engine-project.json"), "project_manifest"));
    checks.push(check(root.join("content"), "content_directory"));
    checks.push(check(root.join("artifacts"), "artifact_directory"));
    let manifest_result = load_json::<ProjectManifest>(&root.join("engine-project.json"));
    checks.push(json!({
        "name": "manifest_parse",
        "ok": manifest_result.is_ok(),
        "detail": manifest_result.err().map(|error| error.to_string())
    }));
    let ok = checks
        .iter()
        .all(|item| item.get("ok").and_then(Value::as_bool).unwrap_or(false));
    let output = json!({"ok": ok, "checks": checks});
    print_value(&output, as_json);
    if ok {
        Ok(())
    } else {
        Err("project doctor found blocking problems".into())
    }
}

fn check(path: PathBuf, name: &str) -> Value {
    json!({
        "name": name,
        "ok": path.exists(),
        "path": path
    })
}

fn capabilities(as_json: bool) -> Result<(), String> {
    let (commands, queries) = create_registries()?;
    let catalog = ToolCatalog::from_registries(&commands, &queries);
    if as_json {
        println!("{}", serde_json::to_string_pretty(&catalog.as_json()).unwrap());
    } else {
        for tool in catalog.list() {
            println!("{} [{:?}] - {}", tool.id, tool.kind, tool.description);
        }
    }
    Ok(())
}

fn validate_project(root: &Path, as_json: bool) -> Result<(), String> {
    let (_, store) = load_project(root)?;
    let diagnostics = validate_store(&store);
    let output = json!({
        "ok": !diagnostics.has_errors(),
        "summary": diagnostics.summary(),
        "diagnostics": diagnostics.diagnostics,
    });
    print_value(&output, as_json);
    if diagnostics.has_errors() {
        Err("validation failed".into())
    } else {
        Ok(())
    }
}

fn compile_project(root: &Path, out: &Path, as_json: bool) -> Result<(), String> {
    let (_, store) = load_project(root)?;
    let diagnostics = validate_store(&store);
    if diagnostics.has_errors() {
        return Err("content has validation errors; compile refused".into());
    }
    let mut compilers = CompilerRegistry::default();
    compilers.register(StageCompiler).map_err(|error| error.to_string())?;
    let report = BuildPipeline::new(compilers).build(
        &store,
        &CompileContext {
            project_root: root.to_path_buf(),
            profile: "development".into(),
        },
    );
    fs::create_dir_all(out).map_err(|error| error.to_string())?;
    for artifact in &report.artifacts {
        let file = out.join(format!("{}.artifact.json", artifact.source_document.as_str()));
        save_json(&file, artifact).map_err(|error| error.to_string())?;
    }
    let output = json!({
        "ok": report.succeeded(),
        "artifacts": report.artifacts.len(),
        "diagnostics": report.diagnostics.diagnostics,
        "output_directory": out,
    });
    print_value(&output, as_json);
    if report.succeeded() {
        Ok(())
    } else {
        Err("compile failed".into())
    }
}

fn create_registries() -> Result<(CommandRegistry, QueryRegistry), String> {
    let mut commands = CommandRegistry::default();
    register_builtin_document_commands(&mut commands).map_err(|error| error.to_string())?;
    register_tactical_commands(&mut commands).map_err(|error| error.to_string())?;
    let mut queries = QueryRegistry::default();
    register_builtin_queries(&mut queries).map_err(|error| error.to_string())?;
    register_tactical_queries(&mut queries).map_err(|error| error.to_string())?;
    Ok((commands, queries))
}

fn load_project(root: &Path) -> Result<(ProjectManifest, DocumentStore), String> {
    let manifest: ProjectManifest =
        load_json(&root.join("engine-project.json")).map_err(|error| error.to_string())?;
    let mut store = DocumentStore::default();
    for relative in &manifest.documents {
        let document: DocumentEnvelope =
            load_json(&root.join(relative)).map_err(|error| error.to_string())?;
        store.insert(document).map_err(|error| error.to_string())?;
    }
    Ok((manifest, store))
}

fn validate_store(store: &DocumentStore) -> DiagnosticBag {
    let mut diagnostics = DiagnosticBag::default();
    for (_, document) in store.iter() {
        if document.header.type_id.as_str() == STAGE_DOCUMENT_TYPE {
            match StageDocument::from_envelope(document) {
                Ok(stage) => diagnostics.extend(validate_stage(&stage)),
                Err(error) => diagnostics.push(
                    Diagnostic::error("STAGE_PARSE_FAILED", error.to_string())
                        .at_document(document.header.id.clone()),
                ),
            }
        }
        validate_references(document, store, &mut diagnostics);
    }
    diagnostics
}

fn validate_references(
    document: &DocumentEnvelope,
    store: &DocumentStore,
    diagnostics: &mut DiagnosticBag,
) {
    for ContentReference {
        target_document,
        kind,
        ..
    } in &document.references
    {
        if store.get(target_document).is_err() {
            diagnostics.push(
                Diagnostic::error(
                    "REFERENCE_NOT_FOUND",
                    format!("{kind} reference points to missing document {target_document}"),
                )
                .at_document(document.header.id.clone()),
            );
        }
    }
}

fn print_value(value: &Value, as_json: bool) {
    if as_json {
        println!("{}", serde_json::to_string_pretty(value).unwrap());
    } else {
        println!("{value}");
    }
}

fn slug(name: &str) -> String {
    let value = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = value.trim_matches('-');
    if slug.is_empty() {
        "game".into()
    } else {
        slug.into()
    }
}
