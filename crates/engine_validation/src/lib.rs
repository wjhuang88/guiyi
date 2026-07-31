#![forbid(unsafe_code)]

//! Structured diagnostics and gate summaries.

use guiyi_engine_core::{DocumentId, ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticLocation {
    pub document: Option<DocumentId>,
    pub object: Option<ObjectId>,
    pub field_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub location: Option<DiagnosticLocation>,
    pub suggested_tools: Vec<String>,
    pub auto_fixable: bool,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            location: None,
            suggested_tools: Vec::new(),
            auto_fixable: false,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            message: message.into(),
            location: None,
            suggested_tools: Vec::new(),
            auto_fixable: false,
        }
    }

    pub fn at_document(mut self, document: DocumentId) -> Self {
        self.location = Some(DiagnosticLocation {
            document: Some(document),
            object: None,
            field_path: None,
        });
        self
    }

    /// Attaches a JSON Pointer (RFC 6901) field path to this diagnostic.
    ///
    /// The root value is the empty string `""`. Nested paths use `/segment`
    /// notation such as `/stage_id` or `/conditions/2/type`.
    pub fn at_field_path(mut self, field_path: impl Into<String>) -> Self {
        self.location = Some(DiagnosticLocation {
            document: None,
            object: None,
            field_path: Some(field_path.into()),
        });
        self
    }

    /// Attaches both a document reference and a JSON Pointer field path.
    pub fn at_document_field(
        mut self,
        document: DocumentId,
        field_path: impl Into<String>,
    ) -> Self {
        self.location = Some(DiagnosticLocation {
            document: Some(document),
            object: None,
            field_path: Some(field_path.into()),
        });
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticBag {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn extend(&mut self, other: Self) {
        self.diagnostics.extend(other.diagnostics);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|item| item.severity == Severity::Error)
    }

    pub fn summary(&self) -> DiagnosticSummary {
        let mut summary = DiagnosticSummary::default();
        for item in &self.diagnostics {
            match item.severity {
                Severity::Info => summary.info += 1,
                Severity::Warning => summary.warnings += 1,
                Severity::Error => summary.errors += 1,
            }
        }
        summary
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    pub info: usize,
    pub warnings: usize,
    pub errors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_severities() {
        let mut bag = DiagnosticBag::default();
        bag.push(Diagnostic::warning("W", "warning"));
        bag.push(Diagnostic::error("E", "error"));
        assert_eq!(bag.summary().warnings, 1);
        assert!(bag.has_errors());
    }

    #[test]
    fn at_field_path_sets_json_pointer_location() {
        let diagnostic = Diagnostic::error("CODE", "message").at_field_path("/conditions/2/type");
        let location = diagnostic.location.unwrap();
        assert_eq!(location.field_path.as_deref(), Some("/conditions/2/type"));
        assert!(location.document.is_none());
    }

    #[test]
    fn at_document_field_sets_both_references() {
        let diagnostic = Diagnostic::error("CODE", "message")
            .at_document_field(DocumentId::from_static("doc.1"), "/path/0");
        let location = diagnostic.location.unwrap();
        assert_eq!(location.document, Some(DocumentId::from_static("doc.1")));
        assert_eq!(location.field_path.as_deref(), Some("/path/0"));
    }

    #[test]
    fn diagnostic_with_field_path_round_trips() {
        let diagnostic = Diagnostic::error("COMMAND_INPUT_TYPE_MISMATCH", "expected integer")
            .at_field_path("/width");
        let json = serde_json::to_value(&diagnostic).unwrap();
        assert_eq!(json["location"]["field_path"], "/width");
        assert_eq!(json["code"], "COMMAND_INPUT_TYPE_MISMATCH");
    }
}
