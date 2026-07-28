#![forbid(unsafe_code)]

use guiyi_engine_content::ProjectManifest;
use guiyi_engine_core::{EngineVersion, ProjectId};

fn main() {
    let project = ProjectManifest {
        project_id: ProjectId::from_static("example.minimal"),
        name: "Minimal Project".into(),
        engine_api_version: EngineVersion::CURRENT.to_string(),
        content_schema_version: 1,
        enabled_extensions: Vec::new(),
        documents: Vec::new(),
    };
    println!("{}", serde_json::to_string_pretty(&project).unwrap());
}
