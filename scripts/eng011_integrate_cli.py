from pathlib import Path

path = Path("crates/engine_cli/src/main.rs")
text = path.read_text()


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one CLI replacement target, found {count}: {old[:80]!r}")
    text = text.replace(old, new, 1)


replace_once(
    '''    CompileContext, CompilerRegistry, ContentReference, DocumentEnvelope, DocumentStore,
    ProjectFilesystem, ProjectManifest, ProjectPath,
''',
    '''    CompileContext, CompilerRegistry, ContentReference, DocumentEnvelope, DocumentStore,
    ProjectManifest, ProjectPath, ProjectStorage, ProjectTransaction,
''',
)
replace_once(
    'use guiyi_engine_core::{EngineVersion, ProjectId};',
    'use guiyi_engine_core::{AgentSessionId, EngineVersion, ProjectId};',
)
replace_once(
    '    let storage = ProjectFilesystem::create(root).map_err(|error| error.to_string())?;',
    '    let storage = ProjectStorage::create(root).map_err(|error| error.to_string())?;',
)
replace_once(
    '''    storage
        .save_json(
            &document_path,
            &stage.to_envelope().map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    let manifest = ProjectManifest {
''',
    '''    let document = stage.to_envelope().map_err(|error| error.to_string())?;
    let manifest = ProjectManifest {
''',
)
replace_once(
    '''    storage
        .save_json(&logical("engine-project.json")?, &manifest)
        .map_err(|error| error.to_string())?;
    storage
        .write(
            &logical("README.md")?,
            format!(
                "# {name}\\n\\nCreated by GUIYI Engine. Run `guiyi-engine-cli doctor --project .`.\\n"
            ),
        )
        .map_err(|error| error.to_string())?;
''',
    '''    let mut transaction = ProjectTransaction::generated(
        "init",
        AgentSessionId::from_static("session.cli-init"),
        "engine_cli.init",
        json!({
            "kind": "project_init",
            "project_id": manifest.project_id,
            "documents": manifest.documents.len()
        }),
    )
    .map_err(|error| error.to_string())?;
    transaction
        .write_json(document_path, &document)
        .map_err(|error| error.to_string())?;
    transaction.write(
        logical("README.md")?,
        format!(
            "# {name}\\n\\nCreated by GUIYI Engine. Run `guiyi-engine-cli doctor --project .`.\\n"
        )
        .into_bytes(),
    );
    transaction
        .write_manifest_json(logical("engine-project.json")?, &manifest)
        .map_err(|error| error.to_string())?;
    storage
        .commit(transaction)
        .map_err(|error| error.to_string())?;
''',
)
replace_once(
    '    let storage = ProjectFilesystem::open(root).map_err(|error| error.to_string())?;',
    '    let storage = ProjectStorage::open(root).map_err(|error| error.to_string())?;',
)
replace_once(
    'fn check(storage: &ProjectFilesystem, path: &ProjectPath, name: &str) -> Value {',
    'fn check(storage: &ProjectStorage, path: &ProjectPath, name: &str) -> Value {',
)
replace_once(
    '''    storage
        .create_dir_all(&output_directory)
        .map_err(|error| error.to_string())?;
    for artifact in &report.artifacts {
        let file = output_directory
            .join(format!(
                "{}.artifact.json",
                artifact.source_document.as_str()
            ))
            .map_err(|error| error.to_string())?;
        storage
            .save_json(&file, artifact)
            .map_err(|error| error.to_string())?;
    }
    let output = json!({
''',
    '''    if report.succeeded() && !report.artifacts.is_empty() {
        storage
            .create_dir_all(&output_directory)
            .map_err(|error| error.to_string())?;
        let mut transaction = ProjectTransaction::generated(
            "build",
            AgentSessionId::from_static("session.cli-build"),
            "engine_cli.compile",
            json!({
                "kind": "build_output",
                "profile": "development",
                "artifacts": report.artifacts.len()
            }),
        )
        .map_err(|error| error.to_string())?;
        for artifact in &report.artifacts {
            let file = output_directory
                .join(format!(
                    "{}.artifact.json",
                    artifact.source_document.as_str()
                ))
                .map_err(|error| error.to_string())?;
            transaction
                .write_json(file, artifact)
                .map_err(|error| error.to_string())?;
        }
        storage
            .commit(transaction)
            .map_err(|error| error.to_string())?;
    }
    let output = json!({
''',
)
replace_once(
    ') -> Result<(ProjectFilesystem, ProjectManifest, DocumentStore), String> {\n    let storage = ProjectFilesystem::open(root).map_err(|error| error.to_string())?;',
    ') -> Result<(ProjectStorage, ProjectManifest, DocumentStore), String> {\n    let storage = ProjectStorage::open(root).map_err(|error| error.to_string())?;',
)

path.write_text(text)
