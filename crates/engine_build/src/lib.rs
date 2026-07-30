#![forbid(unsafe_code)]

//! Deterministic document compilation pipeline and build reports.

use guiyi_engine_content::{
    ArtifactEnvelope, CompileContext, CompilerRegistry, ContentError, DocumentEnvelope,
    DocumentStore,
};
use guiyi_engine_core::{DocumentId, EngineTypeId};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const BUILD_UNRESOLVED_REFERENCE: &str = "BUILD_UNRESOLVED_REFERENCE";
pub const BUILD_COMPILER_MISSING: &str = "BUILD_COMPILER_MISSING";
pub const BUILD_COMPILE_FAILED: &str = "BUILD_COMPILE_FAILED";
pub const BUILD_DOCUMENT_AUTHORING_ONLY: &str = "BUILD_DOCUMENT_AUTHORING_ONLY";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildProfile {
    pub name: String,
    pub strict: bool,
    #[serde(default)]
    pub authoring_only_types: BTreeSet<EngineTypeId>,
}

impl BuildProfile {
    pub fn with_authoring_only_type(mut self, type_id: EngineTypeId) -> Self {
        self.authoring_only_types.insert(type_id);
        self
    }

    pub fn is_authoring_only(&self, type_id: &EngineTypeId) -> bool {
        self.authoring_only_types.contains(type_id)
    }
}

impl Default for BuildProfile {
    fn default() -> Self {
        Self {
            name: "development".into(),
            strict: true,
            authoring_only_types: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSkipReason {
    AuthoringOnly,
    UnresolvedReference,
    MissingCompiler,
    CompilerFailed,
    BlockingDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedDocument {
    pub document_id: DocumentId,
    pub type_id: EngineTypeId,
    pub reason: BuildSkipReason,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BuildReport {
    pub artifacts: Vec<ArtifactEnvelope>,
    pub diagnostics: DiagnosticBag,
    #[serde(default)]
    pub skipped_documents: Vec<SkippedDocument>,
}

impl BuildReport {
    pub fn succeeded(&self) -> bool {
        !self.diagnostics.has_errors()
    }

    fn skip(
        &mut self,
        document: &DocumentEnvelope,
        reason: BuildSkipReason,
        message: impl Into<String>,
    ) {
        if self
            .skipped_documents
            .iter()
            .any(|item| item.document_id == document.header.id)
        {
            return;
        }
        self.skipped_documents.push(SkippedDocument {
            document_id: document.header.id.clone(),
            type_id: document.header.type_id.clone(),
            reason,
            message: message.into(),
        });
    }
}

pub struct BuildPipeline {
    compilers: CompilerRegistry,
    profile: BuildProfile,
}

impl BuildPipeline {
    pub fn new(compilers: CompilerRegistry) -> Self {
        Self::with_profile(compilers, BuildProfile::default())
    }

    pub fn with_profile(compilers: CompilerRegistry, profile: BuildProfile) -> Self {
        Self { compilers, profile }
    }

    pub fn profile(&self) -> &BuildProfile {
        &self.profile
    }

    pub fn build(&self, store: &DocumentStore, context: &CompileContext) -> BuildReport {
        let mut report = BuildReport::default();
        self.validate_references(store, &mut report);
        if report.diagnostics.has_errors() {
            self.mark_validation_blocked(store, &mut report);
            return report;
        }

        let mut compiled = Vec::new();
        for (_, document) in store.iter() {
            if self.profile.is_authoring_only(&document.header.type_id) {
                let message = format!(
                    "document type {} is declared authoring-only for profile {}",
                    document.header.type_id, self.profile.name
                );
                report.diagnostics.push(
                    Diagnostic::warning(BUILD_DOCUMENT_AUTHORING_ONLY, message.clone())
                        .at_document(document.header.id.clone()),
                );
                report.skip(document, BuildSkipReason::AuthoringOnly, message);
                continue;
            }

            match self.compilers.compile(document, context) {
                Ok(artifact) => compiled.push((document, artifact)),
                Err(ContentError::CompilerNotFound(type_id)) => {
                    let message = format!(
                        "no compiler is registered for buildable document type {type_id}"
                    );
                    if self.profile.strict {
                        report.diagnostics.push(
                            Diagnostic::error(BUILD_COMPILER_MISSING, message.clone())
                                .at_document(document.header.id.clone()),
                        );
                    } else {
                        report.diagnostics.push(
                            Diagnostic::warning(BUILD_COMPILER_MISSING, message.clone())
                                .at_document(document.header.id.clone()),
                        );
                    }
                    report.skip(document, BuildSkipReason::MissingCompiler, message);
                }
                Err(error) => {
                    let message = error.to_string();
                    report.diagnostics.push(
                        Diagnostic::error(BUILD_COMPILE_FAILED, message.clone())
                            .at_document(document.header.id.clone()),
                    );
                    report.skip(document, BuildSkipReason::CompilerFailed, message);
                }
            }
        }

        if report.diagnostics.has_errors() {
            for (document, _) in compiled {
                report.skip(
                    document,
                    BuildSkipReason::BlockingDiagnostics,
                    "artifact withheld because the strict build contains blocking diagnostics",
                );
            }
            return report;
        }

        report.artifacts = compiled
            .into_iter()
            .map(|(_, artifact)| artifact)
            .collect();
        report
    }

    fn validate_references(&self, store: &DocumentStore, report: &mut BuildReport) {
        for (_, document) in store.iter() {
            for reference in &document.references {
                if store.get(&reference.target_document).is_ok() {
                    continue;
                }
                let message = format!(
                    "{} reference points to missing document {}",
                    reference.kind, reference.target_document
                );
                report.diagnostics.push(
                    Diagnostic::error(BUILD_UNRESOLVED_REFERENCE, message.clone())
                        .at_document(document.header.id.clone()),
                );
                report.skip(document, BuildSkipReason::UnresolvedReference, message);
            }
        }
    }

    fn mark_validation_blocked(&self, store: &DocumentStore, report: &mut BuildReport) {
        for (_, document) in store.iter() {
            if self.profile.is_authoring_only(&document.header.type_id) {
                report.skip(
                    document,
                    BuildSkipReason::AuthoringOnly,
                    format!(
                        "document type {} is declared authoring-only for profile {}",
                        document.header.type_id, self.profile.name
                    ),
                );
            } else {
                report.skip(
                    document,
                    BuildSkipReason::BlockingDiagnostics,
                    "document was not compiled because project validation failed",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_content::{
        ArtifactEnvelope, CompileContext, ContentReference, DocumentCompiler, DocumentHeader,
    };
    use guiyi_engine_core::{ArtifactId, ObjectId};
    use serde_json::json;

    const BUILDABLE: &str = "test.buildable";
    const AUTHORING_ONLY: &str = "test.authoring_only";

    struct TestCompiler;

    impl DocumentCompiler for TestCompiler {
        fn id(&self) -> &'static str {
            "test.compiler"
        }

        fn source_type(&self) -> EngineTypeId {
            EngineTypeId::from_static(BUILDABLE)
        }

        fn artifact_type(&self) -> EngineTypeId {
            EngineTypeId::from_static("test.artifact")
        }

        fn compile(
            &self,
            document: &DocumentEnvelope,
            _context: &CompileContext,
        ) -> Result<ArtifactEnvelope, ContentError> {
            if document.payload["fail"] == json!(true) {
                return Err(ContentError::InvalidDocument("injected compiler failure".into()));
            }
            Ok(ArtifactEnvelope {
                id: ArtifactId::new(format!("artifact.{}", document.header.id.as_str()))
                    .unwrap(),
                artifact_type: self.artifact_type(),
                source_document: document.header.id.clone(),
                compiler_version: 1,
                source_hash: document.content_hash()?,
                payload: document.payload.clone(),
            })
        }
    }

    fn document(id: &str, type_id: &str) -> DocumentEnvelope {
        DocumentEnvelope {
            header: DocumentHeader {
                id: DocumentId::new(id).unwrap(),
                type_id: EngineTypeId::new(type_id).unwrap(),
                schema_version: 1,
                display_name: id.into(),
            },
            references: Vec::new(),
            payload: json!({}),
        }
    }

    fn context() -> CompileContext {
        CompileContext {
            project_root: ".".into(),
            profile: "test".into(),
        }
    }

    fn registry() -> CompilerRegistry {
        let mut registry = CompilerRegistry::default();
        registry.register(TestCompiler).unwrap();
        registry
    }

    #[test]
    fn empty_build_succeeds() {
        let pipeline = BuildPipeline::new(CompilerRegistry::default());
        let report = pipeline.build(&DocumentStore::default(), &context());
        assert!(report.succeeded());
        assert!(report.artifacts.is_empty());
    }

    #[test]
    fn strict_build_rejects_missing_compiler() {
        let mut store = DocumentStore::default();
        store.insert(document("doc.missing", "test.uncompiled")).unwrap();
        let report = BuildPipeline::new(CompilerRegistry::default()).build(&store, &context());
        assert!(!report.succeeded());
        assert!(report.artifacts.is_empty());
        assert_eq!(report.skipped_documents[0].reason, BuildSkipReason::MissingCompiler);
        assert_eq!(report.diagnostics.diagnostics[0].code, BUILD_COMPILER_MISSING);
    }

    #[test]
    fn unresolved_reference_fails_before_compilation() {
        let mut source = document("doc.source", BUILDABLE);
        source.references.push(ContentReference {
            source_object: Some(ObjectId::from_static("object.source")),
            target_document: DocumentId::from_static("doc.missing"),
            target_object: None,
            kind: "test_reference".into(),
        });
        let mut store = DocumentStore::default();
        store.insert(source).unwrap();
        let report = BuildPipeline::new(registry()).build(&store, &context());
        assert!(!report.succeeded());
        assert!(report.artifacts.is_empty());
        assert_eq!(
            report.diagnostics.diagnostics[0].code,
            BUILD_UNRESOLVED_REFERENCE
        );
    }

    #[test]
    fn authoring_only_document_is_reported_without_failing() {
        let mut store = DocumentStore::default();
        store.insert(document("doc.notes", AUTHORING_ONLY)).unwrap();
        let profile = BuildProfile::default()
            .with_authoring_only_type(EngineTypeId::from_static(AUTHORING_ONLY));
        let report = BuildPipeline::with_profile(CompilerRegistry::default(), profile)
            .build(&store, &context());
        assert!(report.succeeded());
        assert!(report.artifacts.is_empty());
        assert_eq!(report.skipped_documents[0].reason, BuildSkipReason::AuthoringOnly);
        assert_eq!(
            report.diagnostics.diagnostics[0].code,
            BUILD_DOCUMENT_AUTHORING_ONLY
        );
    }

    #[test]
    fn compiler_failure_withholds_all_artifacts() {
        let good = document("doc.good", BUILDABLE);
        let mut bad = document("doc.bad", BUILDABLE);
        bad.payload = json!({"fail": true});
        let mut store = DocumentStore::default();
        store.insert(good).unwrap();
        store.insert(bad).unwrap();
        let report = BuildPipeline::new(registry()).build(&store, &context());
        assert!(!report.succeeded());
        assert!(report.artifacts.is_empty());
        assert!(report
            .skipped_documents
            .iter()
            .any(|item| item.reason == BuildSkipReason::CompilerFailed));
        assert!(report
            .skipped_documents
            .iter()
            .any(|item| item.reason == BuildSkipReason::BlockingDiagnostics));
    }

    #[test]
    fn compatibility_mode_reports_missing_compiler_as_warning() {
        let mut store = DocumentStore::default();
        store.insert(document("doc.optional", "test.optional")).unwrap();
        let profile = BuildProfile {
            name: "compatibility".into(),
            strict: false,
            authoring_only_types: BTreeSet::new(),
        };
        let report = BuildPipeline::with_profile(CompilerRegistry::default(), profile)
            .build(&store, &context());
        assert!(report.succeeded());
        assert!(report.artifacts.is_empty());
        assert_eq!(report.skipped_documents[0].reason, BuildSkipReason::MissingCompiler);
    }
}
