#![forbid(unsafe_code)]

use bevy_ecs::world::World;
use clap::Parser;
use guiyi_engine_content::{load_json, ArtifactEnvelope};
use guiyi_engine_runtime::StageRuntimeManager;
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "guiyi-engine-preview")]
#[command(about = "Headless Stage artifact preview runner")]
struct Args {
    #[arg(long)]
    artifact: PathBuf,
    #[arg(long)]
    json: bool,
}

fn run(args: Args) -> Result<(), String> {
    let artifact: ArtifactEnvelope =
        load_json(&args.artifact).map_err(|error| error.to_string())?;
    let mut world = World::new();
    let mut manager = StageRuntimeManager::default();
    let instance = manager
        .load(&mut world, &artifact)
        .map_err(|error| error.to_string())?;
    let output = json!({
        "status": "ready",
        "artifact": artifact.id,
        "stage_instance": instance,
        "entity_count": world.entities().len()
    });
    if args.json {
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("Preview ready: {output}");
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("preview failed: {error}");
            ExitCode::FAILURE
        }
    }
}
