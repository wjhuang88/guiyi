#![forbid(unsafe_code)]

//! Deterministic document compilation pipeline and build reports.

use guiyi_engine_content::{
    ArtifactEnvelope, CompileContext, CompilerRegistry, ContentError, DocumentStore,
};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildProfile {
    pub name: String,
    pub strict: bool,
}

impl Default for BuildProfile {
    fn default() -> Self {
        Self {
            name: "development".into(),
            strict: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BuildReport {
    pub artifacts: Vec<ArtifactEnvelope>,
    pub diagnostics: DiagnosticBag,
}

impl BuildReport {
    pub fn succeeded(&self) -> bool {
        !self.diagnostics.has_errors()
    }
}

pub struct BuildPipeline {
    compilers: CompilerRegistry,
}

impl BuildPipeline {
    pub fn new(compilers: CompilerRegistry) -> Self {
        Self { compilers }
    }

    pub fn build(&self, store: &DocumentStore, context: &CompileContext) -> BuildReport {
        let mut report = BuildReport::default();
        for (_, document) in store.iter() {
            match self.compilers.compile(document, context) {
                Ok(artifact) => report.artifacts.push(artifact),
                Err(ContentError::CompilerNotFound(_)) => {}
                Err(error) => report.diagnostics.push(
                    Diagnostic::error("CONTENT_COMPILE_FAILED", error.to_string())
                        .at_document(document.header.id.clone()),
                ),
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_build_succeeds() {
        let pipeline = BuildPipeline::new(CompilerRegistry::default());
        let report = pipeline.build(
            &DocumentStore::default(),
            &CompileContext {
                project_root: ".".into(),
                profile: "test".into(),
            },
        );
        assert!(report.succeeded());
    }
}
