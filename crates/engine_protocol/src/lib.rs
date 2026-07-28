#![forbid(unsafe_code)]

//! Stable JSONL protocol between agents, workbenches, and the engine host.

use guiyi_engine_core::ToolId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self { major: 1, minor: 0 };
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool: ToolId,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultStatus {
    Ok,
    Rejected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub status: ToolResultStatus,
    pub output: Value,
    #[serde(default)]
    pub diagnostics: Vec<Value>,
    pub transaction: Option<Value>,
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("invalid JSONL message: {0}")]
    Json(#[from] serde_json::Error),
    #[error("message contains a newline")]
    EmbeddedNewline,
}

pub fn encode_line<T: Serialize>(value: &T) -> Result<String, ProtocolError> {
    let line = serde_json::to_string(value)?;
    if line.contains('\n') {
        return Err(ProtocolError::EmbeddedNewline);
    }
    Ok(line)
}

pub fn decode_line<T: for<'de> Deserialize<'de>>(line: &str) -> Result<T, ProtocolError> {
    Ok(serde_json::from_str(line.trim_end())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_calls_round_trip_as_one_line() {
        let call = ToolCall {
            id: "call-1".into(),
            tool: ToolId::from_static("project.documents.list"),
            input: json!({}),
            dry_run: false,
        };
        let encoded = encode_line(&call).unwrap();
        assert!(!encoded.contains('\n'));
        assert_eq!(decode_line::<ToolCall>(&encoded).unwrap(), call);
    }
}
