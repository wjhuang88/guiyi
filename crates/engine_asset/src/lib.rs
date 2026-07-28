#![forbid(unsafe_code)]

//! Asset identifiers, slots, manifests, dependency validation, and project-friendly indirection.

use guiyi_engine_core::{AssetId, AssetSlotId};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetEntry {
    pub id: AssetId,
    pub source: PathBuf,
    pub runtime_path: PathBuf,
    pub kind: String,
    #[serde(default)]
    pub dependencies: Vec<AssetId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetManifest {
    pub assets: BTreeMap<AssetId, AssetEntry>,
    pub slots: BTreeMap<AssetSlotId, AssetId>,
}

impl AssetManifest {
    pub fn validate(&self) -> DiagnosticBag {
        let mut diagnostics = DiagnosticBag::default();
        for entry in self.assets.values() {
            for dependency in &entry.dependencies {
                if !self.assets.contains_key(dependency) {
                    diagnostics.push(Diagnostic::error(
                        "ASSET_DEPENDENCY_NOT_FOUND",
                        format!("asset {} depends on missing asset {dependency}", entry.id),
                    ));
                }
            }
        }
        for (slot, asset) in &self.slots {
            if !self.assets.contains_key(asset) {
                diagnostics.push(Diagnostic::error(
                    "ASSET_SLOT_UNBOUND",
                    format!("slot {slot} references missing asset {asset}"),
                ));
            }
        }
        diagnostics
    }

    pub fn transitive_dependencies(&self, root: &AssetId) -> BTreeSet<AssetId> {
        let mut result = BTreeSet::new();
        let mut stack = vec![root.clone()];
        while let Some(current) = stack.pop() {
            if let Some(entry) = self.assets.get(&current) {
                for dependency in &entry.dependencies {
                    if result.insert(dependency.clone()) {
                        stack.push(dependency.clone());
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_slot_binding_is_an_error() {
        let mut manifest = AssetManifest::default();
        manifest.slots.insert(
            AssetSlotId::from_static("actor.hero.body"),
            AssetId::from_static("model.hero"),
        );
        assert!(manifest.validate().has_errors());
    }
}
