#![forbid(unsafe_code)]

//! Stable identifiers, versions, permissions, and deterministic utility types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdError {
    #[error("identifier cannot be empty")]
    Empty,
    #[error("identifier contains invalid character: {0}")]
    InvalidCharacter(char),
}

macro_rules! string_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, IdError> {
                let value = value.into();
                validate_identifier(&value)?;
                Ok(Self(value))
            }

            pub fn from_static(value: &'static str) -> Self {
                Self::new(value).expect("static identifiers must be valid")
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

fn validate_identifier(value: &str) -> Result<(), IdError> {
    if value.trim().is_empty() {
        return Err(IdError::Empty);
    }
    for ch in value.chars() {
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')) {
            return Err(IdError::InvalidCharacter(ch));
        }
    }
    Ok(())
}

string_id!(ProjectId);
string_id!(DocumentId);
string_id!(ObjectId);
string_id!(EngineTypeId);
string_id!(ArtifactId);
string_id!(TransactionId);
string_id!(AgentSessionId);
string_id!(ToolId);
string_id!(AssetId);
string_id!(AssetSlotId);
string_id!(StageInstanceId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl EngineVersion {
    pub const CURRENT: Self = Self {
        major: 0,
        minor: 1,
        patch: 0,
    };

    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
}

impl Display for EngineVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Read,
    Plan,
    DryRun,
    EditContent,
    EditSchema,
    EditCode,
    RunValidation,
    RunBuild,
    RunPreview,
    RunExternalProcess,
    CommitChanges,
    Publish,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionSet {
    values: BTreeSet<Permission>,
}

impl PermissionSet {
    pub fn new(values: impl IntoIterator<Item = Permission>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }

    pub fn read_only() -> Self {
        Self::new([Permission::Read, Permission::Plan, Permission::DryRun])
    }

    pub fn content_author() -> Self {
        Self::new([
            Permission::Read,
            Permission::Plan,
            Permission::DryRun,
            Permission::EditContent,
            Permission::RunValidation,
            Permission::RunBuild,
            Permission::RunPreview,
        ])
    }

    pub fn contains(&self, permission: Permission) -> bool {
        self.values.contains(&permission)
    }

    pub fn contains_all(&self, required: &PermissionSet) -> bool {
        required.values.is_subset(&self.values)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Permission> {
        self.values.iter()
    }
}

pub fn deterministic_hash(bytes: &[u8]) -> String {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_reject_spaces() {
        assert!(DocumentId::new("bad id").is_err());
        assert_eq!(DocumentId::new("stage.demo").unwrap().as_str(), "stage.demo");
    }

    #[test]
    fn permissions_use_subset_semantics() {
        let author = PermissionSet::content_author();
        assert!(author.contains_all(&PermissionSet::read_only()));
        assert!(!author.contains(Permission::Publish));
    }

    #[test]
    fn deterministic_hash_is_stable() {
        assert_eq!(deterministic_hash(b"guiyi"), "65f6e3222a196356");
    }
}
