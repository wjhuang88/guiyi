from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one replacement target, found {count}")
    target.write_text(text.replace(old, new, 1))


def replace_in_section(path: str, start_marker: str, end_marker: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    start = text.index(start_marker)
    end = text.index(end_marker, start)
    section = text[start:end]
    count = section.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one section target, found {count}")
    section = section.replace(old, new, 1)
    target.write_text(text[:start] + section + text[end:])


# Protocol-level parse codes are shared by all JSONL clients.
replace_once(
    "crates/engine_protocol/src/lib.rs",
    "use thiserror::Error;\n",
    "use thiserror::Error;\n\npub const PROTOCOL_INVALID_JSONL: &str = \"PROTOCOL_INVALID_JSONL\";\npub const PROTOCOL_INVALID_CALL: &str = \"PROTOCOL_INVALID_CALL\";\n",
)

protocol_path = Path("crates/engine_protocol/src/lib.rs")
protocol = protocol_path.read_text()
protocol_test = '''

    #[test]
    fn structured_error_result_round_trips_with_machine_code() {
        let result = ToolResult {
            call_id: "call-invalid".into(),
            status: ToolResultStatus::Rejected,
            output: json!({
                "error": {
                    "code": PROTOCOL_INVALID_CALL,
                    "message": "missing field `tool`"
                }
            }),
            diagnostics: Vec::new(),
            transaction: None,
        };
        let encoded = encode_line(&result).unwrap();
        let decoded = decode_line::<ToolResult>(&encoded).unwrap();
        assert_eq!(decoded, result);
        assert_eq!(decoded.output["error"]["code"], json!(PROTOCOL_INVALID_CALL));
    }
'''
protocol = protocol.rstrip()
if not protocol.endswith("}"):
    raise SystemExit("engine_protocol tests module has no closing brace")
protocol_path.write_text(protocol[:-1] + protocol_test + "}\n")

# AgentHost tool outcomes must never poison a live session.
replace_once(
    "crates/engine_agent_host/src/lib.rs",
    "use guiyi_engine_query::{QueryContext, QueryExecutor, QueryRequest};",
    "use guiyi_engine_query::{QueryContext, QueryError, QueryExecutor, QueryRequest};",
)
replace_once(
    "crates/engine_agent_host/src/lib.rs",
    "pub const AGENT_TOOL_FAILED: &str = \"AGENT_TOOL_FAILED\";\n",
    "pub const AGENT_TOOL_FAILED: &str = \"AGENT_TOOL_FAILED\";\n"
    "pub const COMMAND_INVALID_INPUT: &str = \"COMMAND_INVALID_INPUT\";\n"
    "pub const COMMAND_VALIDATION_FAILED: &str = \"COMMAND_VALIDATION_FAILED\";\n"
    "pub const QUERY_INVALID_INPUT: &str = \"QUERY_INVALID_INPUT\";\n",
)
replace_once(
    "crates/engine_agent_host/src/lib.rs",
    '''        let result = self.dispatch(session, &call);
        if result.status == ToolResultStatus::Failed {
            session.status = SessionStatus::Failed;
            session.final_summary = Some(error_message(&result));
        }
        record_action(session, call, result.clone());
        result
''',
    '''        let result = self.dispatch(session, &call);
        record_action(session, call, result.clone());
        result
''',
)
replace_once(
    "crates/engine_agent_host/src/lib.rs",
    '''            None => {
                return failed(
                    call,
                    AGENT_TOOL_NOT_FOUND,
                    format!("tool not found: {}", call.tool),
                    json!({"tool": call.tool}),
                )
            }
''',
    '''            None => {
                return rejected(
                    call,
                    AGENT_TOOL_NOT_FOUND,
                    format!("tool not found: {}", call.tool),
                    json!({"tool": call.tool}),
                )
            }
''',
)
replace_once(
    "crates/engine_agent_host/src/lib.rs",
    '                        "code": "COMMAND_VALIDATION_FAILED",',
    '                        "code": COMMAND_VALIDATION_FAILED,',
)
replace_in_section(
    "crates/engine_agent_host/src/lib.rs",
    "    fn execute_command(",
    "    fn execute_query(",
    '''            Err(CommandError::PermissionDenied(_)) => rejected(
''',
    '''            Err(CommandError::InvalidInput(message)) => rejected(
                call,
                COMMAND_INVALID_INPUT,
                message,
                json!({"tool": call.tool}),
            ),
            Err(CommandError::Serialization(error)) => rejected(
                call,
                COMMAND_INVALID_INPUT,
                error.to_string(),
                json!({"tool": call.tool}),
            ),
            Err(CommandError::CommandNotFound(_)) => rejected(
                call,
                AGENT_TOOL_NOT_FOUND,
                format!("tool not found: {}", call.tool),
                json!({"tool": call.tool}),
            ),
            Err(CommandError::PermissionDenied(_)) => rejected(
''',
)
replace_in_section(
    "crates/engine_agent_host/src/lib.rs",
    "    fn execute_query(",
    "    pub fn run(",
    '''            Err(error) => failed(
                call,
                AGENT_TOOL_FAILED,
                error.to_string(),
                json!({"tool": call.tool}),
            ),
''',
    '''            Err(QueryError::InvalidInput(message)) => rejected(
                call,
                QUERY_INVALID_INPUT,
                message,
                json!({"tool": call.tool}),
            ),
            Err(QueryError::Serialization(error)) => rejected(
                call,
                QUERY_INVALID_INPUT,
                error.to_string(),
                json!({"tool": call.tool}),
            ),
            Err(QueryError::PermissionDenied(_)) => rejected(
                call,
                AGENT_PERMISSION_DENIED,
                "query permission denied",
                json!({"tool": call.tool}),
            ),
            Err(QueryError::QueryNotFound(_)) => rejected(
                call,
                AGENT_TOOL_NOT_FOUND,
                format!("tool not found: {}", call.tool),
                json!({"tool": call.tool}),
            ),
            Err(error) => failed(
                call,
                AGENT_TOOL_FAILED,
                error.to_string(),
                json!({"tool": call.tool}),
            ),
''',
)
replace_once(
    "crates/engine_agent_host/src/lib.rs",
    '''fn error_message(result: &ToolResult) -> String {
    result
        .output
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("tool execution failed")
        .to_string()
}

''',
    "",
)
replace_once(
    "crates/engine_agent_host/src/lib.rs",
    '''    #[test]
    fn failed_calls_are_recorded_and_fail_the_session() {
        let mut host = host();
        let mut session = session(PermissionSet::read_only());
        let result = host.execute(
            &mut session,
            ToolCall {
                id: "call-missing".into(),
                tool: ToolId::from_static("missing.tool"),
                input: json!({}),
                dry_run: false,
            },
        );
        assert_eq!(result.status, ToolResultStatus::Failed);
        assert_eq!(error_code(&result), Some(AGENT_TOOL_NOT_FOUND));
        assert_eq!(session.actions.len(), 1);
        assert_eq!(session.status, SessionStatus::Failed);
    }
''',
    '''    #[test]
    fn rejected_calls_are_recorded_without_poisoning_the_session() {
        let mut host = host();
        let mut session = session(PermissionSet::read_only());
        let result = host.execute(
            &mut session,
            ToolCall {
                id: "call-missing".into(),
                tool: ToolId::from_static("missing.tool"),
                input: json!({}),
                dry_run: false,
            },
        );
        assert_eq!(result.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&result), Some(AGENT_TOOL_NOT_FOUND));
        assert_eq!(session.actions.len(), 1);
        assert_eq!(session.status, SessionStatus::Running);

        let next = host.execute(&mut session, list_call("call-next"));
        assert_eq!(next.status, ToolResultStatus::Ok);
        assert_eq!(session.actions.len(), 2);
        assert_eq!(session.status, SessionStatus::Running);
    }
''',
)

# Workbench parsing preserves IDs and emits one structured result per line.
replace_once(
    "crates/engine_editor/src/main.rs",
    "use guiyi_engine_protocol::{decode_line, encode_line, ToolCall, ToolResult, ToolResultStatus};",
    "use guiyi_engine_protocol::{\n"
    "    encode_line, ToolCall, ToolResult, ToolResultStatus, PROTOCOL_INVALID_CALL,\n"
    "    PROTOCOL_INVALID_JSONL,\n"
    "};",
)
replace_once(
    "crates/engine_editor/src/main.rs",
    "use serde_json::json;",
    "use serde_json::{json, Value};",
)
replace_once(
    "crates/engine_editor/src/main.rs",
    '''fn execute_line(host: &mut AgentHost, session: &mut AgentSession, line: &str) -> ToolResult {
    match decode_line::<ToolCall>(line) {
        Ok(call) => host.execute(session, call),
        Err(error) => ToolResult {
            call_id: "invalid".into(),
            status: ToolResultStatus::Failed,
            output: json!({
                "error": {
                    "code": "PROTOCOL_INVALID_JSONL",
                    "message": error.to_string()
                }
            }),
            diagnostics: Vec::new(),
            transaction: None,
        },
    }
}
''',
    '''fn execute_line(host: &mut AgentHost, session: &mut AgentSession, line: &str) -> ToolResult {
    let value = match serde_json::from_str::<Value>(line.trim_end()) {
        Ok(value) => value,
        Err(error) => {
            return protocol_rejection(
                "invalid".into(),
                PROTOCOL_INVALID_JSONL,
                error.to_string(),
            )
        }
    };
    let call_id = value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("invalid")
        .to_string();
    match serde_json::from_value::<ToolCall>(value) {
        Ok(call) => host.execute(session, call),
        Err(error) => protocol_rejection(call_id, PROTOCOL_INVALID_CALL, error.to_string()),
    }
}

fn protocol_rejection(call_id: String, code: &str, message: String) -> ToolResult {
    ToolResult {
        call_id,
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
''',
)
replace_once(
    "crates/engine_editor/src/main.rs",
    "    use guiyi_engine_agent_host::{SessionStatus, AGENT_BUDGET_EXCEEDED};",
    "    use guiyi_engine_agent_host::{\n"
    "        SessionStatus, AGENT_ACCESS_PLAN_INVALID, AGENT_BUDGET_EXCEEDED,\n"
    "        AGENT_PERMISSION_DENIED, AGENT_TOOL_NOT_FOUND,\n"
    "    };",
)
replace_once(
    "crates/engine_editor/src/main.rs",
    "    use serde_json::Value;\n",
    "",
)
editor_tests = '''
    #[test]
    fn parse_failure_preserves_a_parseable_call_id() {
        let mut host = test_host();
        let args = Args {
            project: PathBuf::from("."),
            read_only: true,
            max_actions: 4,
            working_set: Vec::new(),
        };
        let mut session = create_session(&args).unwrap();
        let result = execute_line(&mut host, &mut session, r#"{"id":"bad-1","input":{}}"#);
        assert_eq!(result.call_id, "bad-1");
        assert_eq!(result.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&result), Some(PROTOCOL_INVALID_CALL));
        assert!(session.actions.is_empty());
    }

    #[test]
    fn malformed_json_returns_a_structured_protocol_result() {
        let mut host = test_host();
        let args = Args {
            project: PathBuf::from("."),
            read_only: true,
            max_actions: 4,
            working_set: Vec::new(),
        };
        let mut session = create_session(&args).unwrap();
        let result = execute_line(&mut host, &mut session, r#"{"id":"bad-1""#);
        assert_eq!(result.call_id, "invalid");
        assert_eq!(result.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&result), Some(PROTOCOL_INVALID_JSONL));
        assert!(session.actions.is_empty());
    }

    #[test]
    fn unknown_tool_does_not_block_the_next_line() {
        let mut host = test_host();
        let args = Args {
            project: PathBuf::from("."),
            read_only: true,
            max_actions: 4,
            working_set: Vec::new(),
        };
        let mut session = create_session(&args).unwrap();
        let missing = execute_line(
            &mut host,
            &mut session,
            r#"{"id":"1","tool":"missing.tool","input":{},"dry_run":false}"#,
        );
        assert_eq!(missing.call_id, "1");
        assert_eq!(missing.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&missing), Some(AGENT_TOOL_NOT_FOUND));

        let next = execute_line(
            &mut host,
            &mut session,
            r#"{"id":"2","tool":"project.documents.list","input":{},"dry_run":false}"#,
        );
        assert_eq!(next.call_id, "2");
        assert_eq!(next.status, ToolResultStatus::Ok);
        assert_eq!(session.status, SessionStatus::Running);
        assert_eq!(session.actions.len(), 2);
    }

    #[test]
    fn invalid_input_and_permission_denial_do_not_poison_the_session() {
        let mut host = test_host();
        let args = Args {
            project: PathBuf::from("."),
            read_only: true,
            max_actions: 6,
            working_set: Vec::new(),
        };
        let mut session = create_session(&args).unwrap();

        let invalid = execute_line(
            &mut host,
            &mut session,
            r#"{"id":"invalid","tool":"project.document.get","input":{},"dry_run":false}"#,
        );
        assert_eq!(invalid.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&invalid), Some(AGENT_ACCESS_PLAN_INVALID));

        let denied = execute_line(
            &mut host,
            &mut session,
            r#"{"id":"denied","tool":"document.create","input":{},"dry_run":false}"#,
        );
        assert_eq!(denied.status, ToolResultStatus::Rejected);
        assert_eq!(error_code(&denied), Some(AGENT_PERMISSION_DENIED));

        let next = execute_line(
            &mut host,
            &mut session,
            r#"{"id":"next","tool":"project.documents.list","input":{},"dry_run":false}"#,
        );
        assert_eq!(next.status, ToolResultStatus::Ok);
        assert_eq!(session.status, SessionStatus::Running);
    }

'''
replace_once(
    "crates/engine_editor/src/main.rs",
    "    #[test]\n    fn workbench_session_parses_working_set() {",
    editor_tests + "    #[test]\n    fn workbench_session_parses_working_set() {",
)

integration = r'''#![forbid(unsafe_code)]

use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn project_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "guiyi-eng009-process-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("engine-project.json"),
        serde_json::to_vec_pretty(&json!({
            "project_id": "project.eng009",
            "name": "ENG-009 process fixture",
            "engine_api_version": "0.1.0",
            "content_schema_version": 1,
            "documents": []
        }))
        .unwrap(),
    )
    .unwrap();
    root
}

#[test]
fn unknown_tool_emits_one_json_line_and_following_call_succeeds() {
    let root = project_root();
    let mut child = Command::new(env!("CARGO_BIN_EXE_guiyi-engine-workbench"))
        .args([
            "--project",
            root.to_str().unwrap(),
            "--read-only",
            "--max-actions",
            "4",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    writeln!(
        stdin,
        "{}",
        r#"{"id":"1","tool":"missing.tool","input":{},"dry_run":false}"#
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        r#"{"id":"2","tool":"project.documents.list","input":{},"dry_run":false}"#
    )
    .unwrap();
    drop(stdin);

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2, "stdout: {stdout}");
    let first: Value = serde_json::from_str(lines[0]).unwrap();
    let second: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(first["call_id"], json!("1"));
    assert_eq!(first["status"], json!("rejected"));
    assert_eq!(first["output"]["error"]["code"], json!("AGENT_TOOL_NOT_FOUND"));
    assert_eq!(second["call_id"], json!("2"));
    assert_eq!(second["status"], json!("ok"));
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(root).unwrap();
}
'''
Path("crates/engine_editor/tests").mkdir(parents=True, exist_ok=True)
Path("crates/engine_editor/tests/jsonl_workbench.rs").write_text(integration)
