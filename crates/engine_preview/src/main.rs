#![forbid(unsafe_code)]

use bevy_ecs::world::World;
use clap::Parser;
use guiyi_engine_content::{load_json, ArtifactEnvelope};
use guiyi_engine_runtime::{RuntimeError, StageRuntimeManager};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::ExitCode;
use thiserror::Error;

const PREVIEW_ARTIFACT_READ_FAILED: &str = "PREVIEW_ARTIFACT_READ_FAILED";

#[derive(Debug, Parser)]
#[command(name = "guiyi-engine-preview")]
#[command(about = "Headless Stage artifact preview runner")]
struct Args {
    #[arg(long)]
    artifact: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Error)]
enum PreviewError {
    #[error("artifact read failed: {0}")]
    ArtifactRead(String),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

impl PreviewError {
    fn code(&self) -> &'static str {
        match self {
            Self::ArtifactRead(_) => PREVIEW_ARTIFACT_READ_FAILED,
            Self::Runtime(error) => error.code(),
        }
    }

    fn details(&self) -> Value {
        match self {
            Self::ArtifactRead(message) => json!({"reason": message}),
            Self::Runtime(error) => error.details(),
        }
    }
}

fn run(args: &Args) -> Result<Value, PreviewError> {
    let artifact: ArtifactEnvelope =
        load_json(&args.artifact).map_err(|error| PreviewError::ArtifactRead(error.to_string()))?;
    let mut world = World::new();
    let mut manager = StageRuntimeManager::default();
    let instance = manager.load(&mut world, &artifact)?;
    Ok(json!({
        "status": "ready",
        "artifact": artifact.id,
        "stage_instance": instance,
        "entity_count": world.entities().len()
    }))
}

fn error_output(error: &PreviewError) -> Value {
    json!({
        "status": "error",
        "error": {
            "code": error.code(),
            "message": error.to_string(),
            "details": error.details()
        }
    })
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(output) => {
            if args.json {
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                println!("Preview ready: {output}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            if args.json {
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&error_output(&error)).unwrap()
                );
            } else {
                eprintln!("preview failed [{}]: {error}", error.code());
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_core::EngineTypeId;

    #[test]
    fn runtime_failure_is_rendered_as_structured_json() {
        let error = PreviewError::Runtime(RuntimeError::ArtifactTypeMismatch {
            expected: EngineTypeId::from_static("tactical.stage.artifact"),
            actual: EngineTypeId::from_static("different.artifact"),
        });
        let output = error_output(&error);
        assert_eq!(output["status"], json!("error"));
        assert_eq!(
            output["error"]["code"],
            json!("RUNTIME_ARTIFACT_TYPE_MISMATCH")
        );
        assert_eq!(
            output["error"]["details"]["actual_type"],
            json!("different.artifact")
        );
    }
}
