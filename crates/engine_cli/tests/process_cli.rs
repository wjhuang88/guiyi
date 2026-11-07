#![forbid(unsafe_code)]

// ENG-019: exercise the compiled CLI process, not private run() helpers.
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn project_root(case: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "guiyi-eng019-cli-{case}-{}-{nonce}",
        std::process::id()
    ))
}

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_guiyi-engine-cli"))
        .args(args)
        .output()
        .unwrap()
}

fn require_json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "successful CLI emitted stderr");
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("CLI stdout was not JSON: {error}"))
}

#[test]
fn cli_subprocess_init_validate_compile_and_capabilities_are_noninteractive() {
    let root = project_root("success");
    let root_arg = root.to_str().unwrap();

    let capabilities = require_json(&cli(&["capabilities", "--json"]));
    assert!(
        capabilities
            .as_array()
            .is_some_and(|tools| !tools.is_empty())
            || capabilities
                .as_object()
                .is_some_and(|catalog| !catalog.is_empty()),
        "machine-readable capability catalog was empty"
    );

    let init = cli(&["init", "--path", root_arg, "--name", "ENG-019 process test"]);
    assert!(
        init.status.success(),
        "CLI init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(init.stderr.is_empty());
    assert!(root.join("engine-project.json").exists());

    let validated = require_json(&cli(&["validate", "--project", root_arg, "--json"]));
    assert_eq!(validated["ok"], true);
    assert!(validated["diagnostics"].is_array());

    let compiled = require_json(&cli(&[
        "compile",
        "--project",
        root_arg,
        "--out",
        "artifacts",
        "--json",
    ]));
    assert_eq!(compiled["ok"], true);
    assert_eq!(compiled["artifacts"], 1);
    assert!(
        root.join("artifacts/stage.demo.artifact.json").exists(),
        "CLI compile did not persist the generated Stage artifact"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_subprocess_failures_use_stderr_and_never_emit_partial_artifacts() {
    let missing = project_root("missing");
    let missing_arg = missing.to_str().unwrap();
    let output = cli(&["validate", "--project", missing_arg, "--json"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "fatal project error wrote stdout");
    assert!(
        !output.stderr.is_empty(),
        "fatal project error lost diagnostics"
    );

    let root = project_root("invalid-output");
    let root_arg = root.to_str().unwrap();
    assert!(
        cli(&["init", "--path", root_arg, "--name", "Output escape"])
            .status
            .success()
    );
    let invalid = cli(&[
        "compile",
        "--project",
        root_arg,
        "--out",
        "artifacts/../escape",
        "--json",
    ]);
    assert!(!invalid.status.success());
    assert!(
        invalid.stdout.is_empty(),
        "invalid output path wrote stdout"
    );
    assert!(!invalid.stderr.is_empty());
    assert!(
        fs::read_dir(root.join("artifacts"))
            .unwrap()
            .next()
            .is_none(),
        "refused compile produced partial artifact output"
    );
    assert!(!root.join("escape").exists());
    fs::remove_dir_all(&root).unwrap();
}
