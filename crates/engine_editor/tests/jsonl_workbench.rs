#![forbid(unsafe_code)]

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_project(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "guiyi-jsonl-{name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("engine-project.json"),
        r#"{
            "project_id": "project.jsonl-test",
            "name": "JSONL Test",
            "engine_api_version": "0.1.0",
            "content_schema_version": 1,
            "documents": []
        }"#,
    )
    .unwrap();
    root
}

fn run_workbench(project: &Path, extra_args: &[&str], input: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_guiyi-engine-workbench"));
    command
        .arg("--project")
        .arg(project)
        .args(extra_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn json_lines(output: &Output) -> Vec<Value> {
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn error_code(value: &Value) -> Option<&str> {
    value
        .get("output")
        .and_then(|output| output.get("error"))
        .and_then(|error| error.get("code"))
        .and_then(Value::as_str)
}

#[test]
fn unknown_tool_returns_one_line_and_following_call_succeeds() {
    let project = temporary_project("unknown-tool");
    let output = run_workbench(
        &project,
        &[],
        concat!(
            "{\"id\":\"1\",\"tool\":\"missing.tool\",\"input\":{},\"dry_run\":false}\n",
            "{\"id\":\"2\",\"tool\":\"project.documents.list\",\"input\":{},\"dry_run\":false}\n"
        ),
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let lines = json_lines(&output);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["call_id"], "1");
    assert_eq!(lines[0]["status"], "rejected");
    assert_eq!(error_code(&lines[0]), Some("AGENT_TOOL_NOT_FOUND"));
    assert_eq!(lines[1]["call_id"], "2");
    assert_eq!(lines[1]["status"], "ok");
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn invalid_input_permission_and_validation_errors_do_not_stop_processing() {
    let project = temporary_project("tool-errors");
    let output = run_workbench(
        &project,
        &[],
        concat!(
            "{\"id\":\"invalid-input\",\"tool\":\"project.document.get\",\"input\":{},\"dry_run\":false}\n",
            "{\"id\":\"validation\",\"tool\":\"stage.create\",\"input\":{\"id\":\"stage.bad\",\"name\":\"Bad\",\"width\":0,\"height\":0},\"dry_run\":false}\n",
            "{\"id\":\"after\",\"tool\":\"project.documents.list\",\"input\":{},\"dry_run\":false}\n"
        ),
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let lines = json_lines(&output);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["call_id"], "invalid-input");
    assert_eq!(lines[0]["status"], "rejected");
    assert_eq!(error_code(&lines[0]), Some("AGENT_ACCESS_PLAN_INVALID"));
    assert_eq!(lines[1]["call_id"], "validation");
    assert_eq!(lines[1]["status"], "rejected");
    assert!(lines[1]["diagnostics"].as_array().unwrap().len() >= 1);
    assert_eq!(lines[2]["call_id"], "after");
    assert_eq!(lines[2]["status"], "ok");
    fs::remove_dir_all(project).unwrap();

    let project = temporary_project("permission");
    let output = run_workbench(
        &project,
        &["--read-only"],
        concat!(
            "{\"id\":\"permission\",\"tool\":\"document.create\",\"input\":{\"id\":\"doc.denied\",\"type_id\":\"example.document\",\"display_name\":\"Denied\"},\"dry_run\":false}\n",
            "{\"id\":\"after\",\"tool\":\"project.documents.list\",\"input\":{},\"dry_run\":false}\n"
        ),
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let lines = json_lines(&output);
    assert_eq!(lines.len(), 2);
    assert_eq!(error_code(&lines[0]), Some("AGENT_PERMISSION_DENIED"));
    assert_eq!(lines[1]["status"], "ok");
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn syntactically_valid_invalid_call_preserves_parseable_call_id() {
    let project = temporary_project("invalid-call");
    let output = run_workbench(
        &project,
        &[],
        concat!(
            "{\"id\":\"kept-id\",\"tool\":7,\"input\":{}}\n",
            "{\"id\":\"after\",\"tool\":\"project.documents.list\",\"input\":{}}\n"
        ),
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let lines = json_lines(&output);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["call_id"], "kept-id");
    assert_eq!(lines[0]["status"], "rejected");
    assert_eq!(error_code(&lines[0]), Some("PROTOCOL_INVALID_CALL"));
    assert_eq!(lines[1]["status"], "ok");
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn host_level_project_open_failure_exits_nonzero_without_stdout() {
    let missing = std::env::temp_dir().join(format!(
        "guiyi-jsonl-missing-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let output = run_workbench(&missing, &[], "");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("workbench failed"));
}
