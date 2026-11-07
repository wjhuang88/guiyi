#![forbid(unsafe_code)]

// ENG-019: assert the actual headless executable's JSON/exit/channel contract.
// ArtifactEnvelope::new uses the same integrity envelope as StageCompiler.
use guiyi_engine_content::{ArtifactEnvelope, RuntimeObjectDescriptor, StageArtifactPayload};
use guiyi_engine_core::{ArtifactId, DocumentId, EngineTypeId, ObjectId};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn root(case: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "guiyi-eng019-preview-{case}-{}-{nonce}",
        std::process::id()
    ))
}

fn artifact() -> ArtifactEnvelope {
    ArtifactEnvelope::new(
        ArtifactId::from_static("artifact.stage.eng019"),
        EngineTypeId::from_static("tactical.stage.artifact"),
        DocumentId::from_static("stage.eng019"),
        1,
        "source-hash-for-process-fixture".to_string(),
        serde_json::to_value(StageArtifactPayload {
            objects: vec![RuntimeObjectDescriptor {
                id: ObjectId::from_static("preview.entity"),
                type_id: EngineTypeId::from_static("example.runtime.object"),
                properties: json!({}),
            }],
            metadata: json!({"name": "ENG-019 preview"}),
        })
        .unwrap(),
    )
    .unwrap()
}

fn preview(path: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_guiyi-engine-preview"))
        .arg("--artifact")
        .arg(path)
        .arg("--json")
        .output()
        .unwrap()
}

fn error_json(output: &Output, code: &str) {
    assert!(!output.status.success(), "failure returned success exit");
    assert!(output.stdout.is_empty(), "failure wrote stdout");
    let value: Value = serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|error| panic!("failure stderr is not JSON: {error}"));
    assert_eq!(value["status"], "error");
    assert_eq!(value["error"]["code"], code);
    assert!(value["error"]["details"].is_object());
}

#[test]
fn preview_subprocess_accepts_valid_stage_artifact_without_a_window() {
    let root = root("ready");
    fs::create_dir_all(&root).unwrap();
    let path = root.join("stage.artifact.json");
    fs::write(&path, serde_json::to_vec_pretty(&artifact()).unwrap()).unwrap();

    let output = preview(&path);
    assert!(
        output.status.success(),
        "preview failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "successful preview wrote stderr");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "ready");
    assert_eq!(value["artifact"], "artifact.stage.eng019");
    // The CLI reports Bevy's allocated entity-table length, not precisely
    // the count of StageOwned objects. Assert that a real load occurred,
    // without strengthening this compatibility contract to an exact count.
    assert!(value["entity_count"]
        .as_u64()
        .is_some_and(|count| count >= 1));
    assert!(value["stage_instance"].is_string());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn preview_subprocess_refuses_missing_corrupt_and_wrong_type_artifacts() {
    let root = root("invalid");
    fs::create_dir_all(&root).unwrap();
    let path = root.join("stage.artifact.json");
    error_json(&preview(&path), "PREVIEW_ARTIFACT_READ_FAILED");

    let mut corrupted = artifact();
    corrupted.artifact_hash = "tampered-hash".to_string();
    fs::write(&path, serde_json::to_vec_pretty(&corrupted).unwrap()).unwrap();
    error_json(&preview(&path), "RUNTIME_ARTIFACT_INTEGRITY_FAILED");

    let mut wrong_type = artifact();
    wrong_type.artifact_type = EngineTypeId::from_static("different.artifact");
    wrong_type.refresh_artifact_hash().unwrap();
    fs::write(&path, serde_json::to_vec_pretty(&wrong_type).unwrap()).unwrap();
    error_json(&preview(&path), "RUNTIME_ARTIFACT_TYPE_MISMATCH");

    fs::remove_dir_all(root).unwrap();
}
