use super::{ContentError, ProjectFilesystem, ProjectPath};
use guiyi_engine_core::{AgentSessionId, TransactionId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub const PROJECT_STORAGE_FAILURE: &str = "PROJECT_STORAGE_FAILURE";
pub const PROJECT_STORAGE_PLAN_INVALID: &str = "PROJECT_STORAGE_PLAN_INVALID";
pub const PROJECT_STORAGE_RECOVERY_FAILED: &str = "PROJECT_STORAGE_RECOVERY_FAILED";
pub const PROJECT_STORAGE_INJECTED_FAILURE: &str = "PROJECT_STORAGE_INJECTED_FAILURE";

const STORAGE_ROOT: &str = ".agent-sessions";
const TRANSACTIONS_ROOT: &str = ".agent-sessions/transactions";
const AUDIT_ROOT: &str = ".agent-sessions/audit";
const JOURNAL_FILE: &str = "journal.json";
const FORMAT_VERSION: u32 = 1;

static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{code}: project storage operation `{operation}` failed{path_display}: {message}", path_display = path.as_ref().map(|value| format!(" for `{}`", value.as_str())).unwrap_or_default())]
pub struct ProjectStorageError {
    pub code: String,
    pub operation: String,
    pub path: Option<ProjectPath>,
    pub message: String,
}

impl ProjectStorageError {
    fn operation(
        code: &str,
        operation: impl Into<String>,
        path: Option<ProjectPath>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            operation: operation.into(),
            path,
            message: message.into(),
        }
    }

    fn injected(point: &StorageFailurePoint) -> Self {
        Self::operation(
            PROJECT_STORAGE_INJECTED_FAILURE,
            "commit",
            None,
            format!("injected failure at {point:?}"),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageTransactionState {
    Prepared,
    Applying,
    Committed,
    RolledBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageFailurePoint {
    AfterPrepare,
    AfterApplyingMarker,
    AfterOperation(usize),
    BeforeManifest,
    AfterManifest,
    AfterCommitMarker,
    AfterAudit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMutationKind {
    Write,
    Delete,
}

#[derive(Debug, Clone)]
struct ProjectMutation {
    kind: ProjectMutationKind,
    path: ProjectPath,
    bytes: Option<Vec<u8>>,
    manifest: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectTransaction {
    pub transaction_id: TransactionId,
    pub session_id: AgentSessionId,
    pub actor: String,
    pub report: Value,
    operations: Vec<ProjectMutation>,
}

impl ProjectTransaction {
    pub fn new(
        transaction_id: TransactionId,
        session_id: AgentSessionId,
        actor: impl Into<String>,
        report: Value,
    ) -> Self {
        Self {
            transaction_id,
            session_id,
            actor: actor.into(),
            report,
            operations: Vec::new(),
        }
    }

    pub fn generated(
        prefix: &str,
        session_id: AgentSessionId,
        actor: impl Into<String>,
        report: Value,
    ) -> Result<Self, ContentError> {
        let transaction_id = generated_transaction_id(prefix)?;
        Ok(Self::new(transaction_id, session_id, actor, report))
    }

    pub fn write(&mut self, path: ProjectPath, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.operations.push(ProjectMutation {
            kind: ProjectMutationKind::Write,
            path,
            bytes: Some(bytes.into()),
            manifest: false,
        });
        self
    }

    pub fn write_json(
        &mut self,
        path: ProjectPath,
        value: &impl Serialize,
    ) -> Result<&mut Self, ContentError> {
        self.write(path, serde_json::to_vec_pretty(value)?);
        Ok(self)
    }

    pub fn write_manifest_json(
        &mut self,
        path: ProjectPath,
        value: &impl Serialize,
    ) -> Result<&mut Self, ContentError> {
        self.operations.push(ProjectMutation {
            kind: ProjectMutationKind::Write,
            path,
            bytes: Some(serde_json::to_vec_pretty(value)?),
            manifest: true,
        });
        Ok(self)
    }

    pub fn delete(&mut self, path: ProjectPath) -> &mut Self {
        self.operations.push(ProjectMutation {
            kind: ProjectMutationKind::Delete,
            path,
            bytes: None,
            manifest: false,
        });
        self
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageOperationRecord {
    pub index: usize,
    pub kind: ProjectMutationKind,
    pub path: ProjectPath,
    pub existed_before: bool,
    pub before_snapshot: Option<ProjectPath>,
    pub after_snapshot: Option<ProjectPath>,
    pub manifest: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageJournal {
    pub format_version: u32,
    pub transaction_id: TransactionId,
    pub session_id: AgentSessionId,
    pub actor: String,
    pub audit_sequence: u64,
    pub state: StorageTransactionState,
    pub created_unix_ms: u128,
    pub operations: Vec<StorageOperationRecord>,
    pub report: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageAuditRecord {
    pub format_version: u32,
    pub sequence: u64,
    pub transaction_id: TransactionId,
    pub session_id: AgentSessionId,
    pub actor: String,
    pub state: StorageTransactionState,
    pub created_unix_ms: u128,
    pub operations: Vec<StorageOperationRecord>,
    pub report: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageCommit {
    pub transaction_id: TransactionId,
    pub audit_sequence: u64,
    pub state: StorageTransactionState,
}

/// Crash-safe, single-writer persistence for one sandboxed project root.
///
/// The storage protocol keeps before/after snapshots and a write-ahead journal
/// under `.agent-sessions/`. Any prepared or applying transaction is rolled
/// back during `open`; a committed transaction is retained and its audit record
/// is recreated if necessary.
#[derive(Debug, Clone)]
pub struct ProjectStorage {
    filesystem: ProjectFilesystem,
}

impl ProjectStorage {
    pub fn create(root: impl AsRef<Path>) -> Result<Self, ContentError> {
        let filesystem = ProjectFilesystem::create(root)?;
        Self::from_filesystem(filesystem)
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self, ContentError> {
        let filesystem = ProjectFilesystem::open(root)?;
        Self::from_filesystem(filesystem)
    }

    fn from_filesystem(filesystem: ProjectFilesystem) -> Result<Self, ContentError> {
        let storage = Self { filesystem };
        storage.ensure_metadata_directories()?;
        storage.recover()?;
        Ok(storage)
    }

    pub fn root(&self) -> &Path {
        self.filesystem.root()
    }

    pub fn filesystem(&self) -> &ProjectFilesystem {
        &self.filesystem
    }

    pub fn exists(&self, path: &ProjectPath) -> Result<bool, ContentError> {
        self.filesystem.exists(path)
    }

    pub fn create_dir_all(&self, path: &ProjectPath) -> Result<(), ContentError> {
        self.filesystem.create_dir_all(path)
    }

    pub fn read(&self, path: &ProjectPath) -> Result<Vec<u8>, ContentError> {
        self.filesystem.read(path)
    }

    pub fn load_json<T: for<'de> Deserialize<'de>>(
        &self,
        path: &ProjectPath,
    ) -> Result<T, ContentError> {
        self.filesystem.load_json(path)
    }

    /// Atomically replace one file using a unique same-directory temporary.
    pub fn write(&self, path: &ProjectPath, bytes: impl AsRef<[u8]>) -> Result<(), ContentError> {
        self.atomic_write(path, bytes.as_ref())
    }

    pub fn save_json(
        &self,
        path: &ProjectPath,
        value: &impl Serialize,
    ) -> Result<(), ContentError> {
        self.atomic_write(path, &serde_json::to_vec_pretty(value)?)
    }

    pub fn commit(&self, transaction: ProjectTransaction) -> Result<StorageCommit, ContentError> {
        match self.commit_inner(transaction, None) {
            Ok(commit) => Ok(commit),
            Err(error) => {
                let _ = self.recover();
                Err(error)
            }
        }
    }

    /// Test hook that leaves the journal exactly as a process crash would.
    pub fn commit_with_failure(
        &self,
        transaction: ProjectTransaction,
        failure: StorageFailurePoint,
    ) -> Result<StorageCommit, ContentError> {
        self.commit_inner(transaction, Some(failure))
    }

    pub fn recover(&self) -> Result<(), ContentError> {
        self.ensure_metadata_directories()?;
        let transactions = self.physical_path(&project_path(TRANSACTIONS_ROOT)?);
        let mut directories = fs::read_dir(&transactions)
            .map_err(|error| self.storage_error("scan_transactions", None, error))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        directories.sort();
        for directory in directories {
            let logical = self.logical_from_physical(&directory.join(JOURNAL_FILE))?;
            if !self.exists(&logical)? {
                continue;
            }
            let mut journal: StorageJournal = self.load_json(&logical)?;
            match journal.state {
                StorageTransactionState::Prepared | StorageTransactionState::Applying => {
                    self.rollback(&journal)?;
                    journal.state = StorageTransactionState::RolledBack;
                    self.write_journal(&journal)?;
                    self.ensure_audit(&journal)?;
                }
                StorageTransactionState::Committed | StorageTransactionState::RolledBack => {
                    self.ensure_audit(&journal)?;
                }
            }
        }
        Ok(())
    }

    pub fn audit_records(&self) -> Result<Vec<StorageAuditRecord>, ContentError> {
        let audit_root = self.physical_path(&project_path(AUDIT_ROOT)?);
        let mut paths = fs::read_dir(audit_root)
            .map_err(|error| self.storage_error("scan_audit", None, error))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().map(|kind| kind.is_file()).unwrap_or(false))
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        paths.sort();
        paths
            .into_iter()
            .map(|path| {
                let logical = self.logical_from_physical(&path)?;
                self.load_json(&logical)
            })
            .collect()
    }

    fn commit_inner(
        &self,
        transaction: ProjectTransaction,
        failure: Option<StorageFailurePoint>,
    ) -> Result<StorageCommit, ContentError> {
        self.validate_plan(&transaction)?;
        let mut journal = self.prepare(transaction)?;
        self.fail_if(&failure, StorageFailurePoint::AfterPrepare)?;

        journal.state = StorageTransactionState::Applying;
        self.write_journal(&journal)?;
        self.fail_if(&failure, StorageFailurePoint::AfterApplyingMarker)?;

        let mut operation_number = 0usize;
        for operation in journal.operations.iter().filter(|item| !item.manifest) {
            self.apply(operation)?;
            self.fail_if(&failure, StorageFailurePoint::AfterOperation(operation_number))?;
            operation_number += 1;
        }

        let manifest = journal.operations.iter().find(|item| item.manifest);
        if let Some(manifest) = manifest {
            self.fail_if(&failure, StorageFailurePoint::BeforeManifest)?;
            self.apply(manifest)?;
            self.fail_if(&failure, StorageFailurePoint::AfterManifest)?;
        }

        journal.state = StorageTransactionState::Committed;
        self.write_journal(&journal)?;
        self.fail_if(&failure, StorageFailurePoint::AfterCommitMarker)?;
        self.ensure_audit(&journal)?;
        self.fail_if(&failure, StorageFailurePoint::AfterAudit)?;

        Ok(StorageCommit {
            transaction_id: journal.transaction_id,
            audit_sequence: journal.audit_sequence,
            state: journal.state,
        })
    }

    fn prepare(&self, transaction: ProjectTransaction) -> Result<StorageJournal, ContentError> {
        let directory = transaction_directory(&transaction.transaction_id)?;
        if self.exists(&directory)? {
            return Err(ProjectStorageError::operation(
                PROJECT_STORAGE_PLAN_INVALID,
                "prepare",
                Some(directory),
                "transaction identifier already exists",
            )
            .into());
        }
        self.create_dir_all(&directory)?;
        self.create_dir_all(&directory.join("before")?)?;
        self.create_dir_all(&directory.join("after")?)?;

        let mut operations = Vec::with_capacity(transaction.operations.len());
        for (index, mutation) in transaction.operations.iter().enumerate() {
            let existed_before = self.exists(&mutation.path)?;
            let before_snapshot = if existed_before {
                let snapshot = directory.join(format!("before/{index:04}.bin"))?;
                self.atomic_write(&snapshot, &self.read(&mutation.path)?)?;
                Some(snapshot)
            } else {
                None
            };
            let after_snapshot = match &mutation.bytes {
                Some(bytes) => {
                    let snapshot = directory.join(format!("after/{index:04}.bin"))?;
                    self.atomic_write(&snapshot, bytes)?;
                    Some(snapshot)
                }
                None => None,
            };
            operations.push(StorageOperationRecord {
                index,
                kind: mutation.kind,
                path: mutation.path.clone(),
                existed_before,
                before_snapshot,
                after_snapshot,
                manifest: mutation.manifest,
            });
        }

        let journal = StorageJournal {
            format_version: FORMAT_VERSION,
            transaction_id: transaction.transaction_id,
            session_id: transaction.session_id,
            actor: transaction.actor,
            audit_sequence: self.next_audit_sequence()?,
            state: StorageTransactionState::Prepared,
            created_unix_ms: unix_ms(),
            operations,
            report: transaction.report,
        };
        self.write_journal(&journal)?;
        Ok(journal)
    }

    fn apply(&self, operation: &StorageOperationRecord) -> Result<(), ContentError> {
        match operation.kind {
            ProjectMutationKind::Write => {
                let snapshot = operation.after_snapshot.as_ref().ok_or_else(|| {
                    ProjectStorageError::operation(
                        PROJECT_STORAGE_PLAN_INVALID,
                        "apply_write",
                        Some(operation.path.clone()),
                        "write operation has no after snapshot",
                    )
                })?;
                self.atomic_write(&operation.path, &self.read(snapshot)?)
            }
            ProjectMutationKind::Delete => {
                if self.exists(&operation.path)? {
                    self.remove_and_sync(&operation.path)?;
                }
                Ok(())
            }
        }
    }

    fn rollback(&self, journal: &StorageJournal) -> Result<(), ContentError> {
        for operation in journal.operations.iter().rev() {
            if let Some(snapshot) = &operation.before_snapshot {
                self.atomic_write(&operation.path, &self.read(snapshot)?)?;
            } else if self.exists(&operation.path)? {
                self.remove_and_sync(&operation.path)?;
            }
        }
        Ok(())
    }

    fn validate_plan(&self, transaction: &ProjectTransaction) -> Result<(), ContentError> {
        if transaction.operations.is_empty() {
            return Err(ProjectStorageError::operation(
                PROJECT_STORAGE_PLAN_INVALID,
                "validate_plan",
                None,
                "transaction has no operations",
            )
            .into());
        }
        let mut paths = BTreeSet::new();
        let mut manifest_count = 0usize;
        for operation in &transaction.operations {
            if !paths.insert(operation.path.clone()) {
                return Err(ProjectStorageError::operation(
                    PROJECT_STORAGE_PLAN_INVALID,
                    "validate_plan",
                    Some(operation.path.clone()),
                    "transaction contains duplicate target paths",
                )
                .into());
            }
            if operation.manifest {
                manifest_count += 1;
                if operation.kind != ProjectMutationKind::Write {
                    return Err(ProjectStorageError::operation(
                        PROJECT_STORAGE_PLAN_INVALID,
                        "validate_plan",
                        Some(operation.path.clone()),
                        "manifest operation must be a write",
                    )
                    .into());
                }
            }
        }
        if manifest_count > 1 {
            return Err(ProjectStorageError::operation(
                PROJECT_STORAGE_PLAN_INVALID,
                "validate_plan",
                None,
                "transaction contains more than one manifest write",
            )
            .into());
        }
        Ok(())
    }

    fn write_journal(&self, journal: &StorageJournal) -> Result<(), ContentError> {
        let path = journal_path(&journal.transaction_id)?;
        self.save_json(&path, journal)
    }

    fn ensure_audit(&self, journal: &StorageJournal) -> Result<(), ContentError> {
        let path = audit_path(journal.audit_sequence)?;
        if self.exists(&path)? {
            let existing: StorageAuditRecord = self.load_json(&path)?;
            if existing.transaction_id != journal.transaction_id || existing.state != journal.state {
                return Err(ProjectStorageError::operation(
                    PROJECT_STORAGE_RECOVERY_FAILED,
                    "verify_audit",
                    Some(path),
                    "audit sequence is occupied by a different transaction or state",
                )
                .into());
            }
            return Ok(());
        }
        let audit = StorageAuditRecord {
            format_version: journal.format_version,
            sequence: journal.audit_sequence,
            transaction_id: journal.transaction_id.clone(),
            session_id: journal.session_id.clone(),
            actor: journal.actor.clone(),
            state: journal.state,
            created_unix_ms: journal.created_unix_ms,
            operations: journal.operations.clone(),
            report: journal.report.clone(),
        };
        self.save_json(&path, &audit)
    }

    fn next_audit_sequence(&self) -> Result<u64, ContentError> {
        let audit_root = self.physical_path(&project_path(AUDIT_ROOT)?);
        let audit_max = fs::read_dir(audit_root)
            .map_err(|error| self.storage_error("scan_audit_sequence", None, error))?
            .filter_map(Result::ok)
            .filter_map(|entry| entry.path().file_stem()?.to_str()?.parse::<u64>().ok())
            .max()
            .unwrap_or(0);

        let transactions_root = self.physical_path(&project_path(TRANSACTIONS_ROOT)?);
        let journal_max = fs::read_dir(transactions_root)
            .map_err(|error| self.storage_error("scan_transaction_sequence", None, error))?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path().join(JOURNAL_FILE);
                let bytes = fs::read(path).ok()?;
                serde_json::from_slice::<StorageJournal>(&bytes)
                    .ok()
                    .map(|journal| journal.audit_sequence)
            })
            .max()
            .unwrap_or(0);
        Ok(audit_max.max(journal_max) + 1)
    }

    fn atomic_write(&self, logical: &ProjectPath, bytes: &[u8]) -> Result<(), ContentError> {
        let target = self.filesystem.resolve_for_write(logical)?;
        let parent = target.parent().ok_or_else(|| {
            ProjectStorageError::operation(
                PROJECT_STORAGE_FAILURE,
                "atomic_write",
                Some(logical.clone()),
                "target has no parent directory",
            )
        })?;
        fs::create_dir_all(parent)
            .map_err(|error| self.storage_error("create_parent", Some(logical.clone()), error))?;
        let temporary = unique_sibling(&target, "tmp");
        let write_result = (|| -> Result<(), ContentError> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|error| {
                    self.storage_error("create_temporary", Some(logical.clone()), error)
                })?;
            file.write_all(bytes)
                .map_err(|error| self.storage_error("write_temporary", Some(logical.clone()), error))?;
            file.sync_all()
                .map_err(|error| self.storage_error("sync_temporary", Some(logical.clone()), error))?;
            replace_file(&temporary, &target).map_err(|error| {
                self.storage_error("replace_target", Some(logical.clone()), error)
            })?;
            sync_directory(parent).map_err(|error| {
                self.storage_error("sync_parent", Some(logical.clone()), error)
            })?;
            Ok(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        write_result
    }

    fn remove_and_sync(&self, logical: &ProjectPath) -> Result<(), ContentError> {
        let target = self.filesystem.resolve_existing(logical)?;
        let parent = target.parent().ok_or_else(|| {
            ProjectStorageError::operation(
                PROJECT_STORAGE_FAILURE,
                "remove",
                Some(logical.clone()),
                "target has no parent directory",
            )
        })?;
        fs::remove_file(&target)
            .map_err(|error| self.storage_error("remove", Some(logical.clone()), error))?;
        sync_directory(parent)
            .map_err(|error| self.storage_error("sync_parent", Some(logical.clone()), error))?;
        Ok(())
    }

    fn ensure_metadata_directories(&self) -> Result<(), ContentError> {
        for path in [STORAGE_ROOT, TRANSACTIONS_ROOT, AUDIT_ROOT] {
            self.filesystem.create_dir_all(&project_path(path)?)?;
        }
        Ok(())
    }

    fn fail_if(
        &self,
        configured: &Option<StorageFailurePoint>,
        current: StorageFailurePoint,
    ) -> Result<(), ContentError> {
        if configured.as_ref() == Some(&current) {
            Err(ProjectStorageError::injected(&current).into())
        } else {
            Ok(())
        }
    }

    fn physical_path(&self, path: &ProjectPath) -> PathBuf {
        self.filesystem.candidate(path)
    }

    fn logical_from_physical(&self, path: &Path) -> Result<ProjectPath, ContentError> {
        let relative = path.strip_prefix(self.root()).map_err(|error| {
            ProjectStorageError::operation(
                PROJECT_STORAGE_RECOVERY_FAILED,
                "logical_path",
                None,
                error.to_string(),
            )
        })?;
        Ok(ProjectPath::try_from(relative)?)
    }

    fn storage_error(
        &self,
        operation: &str,
        path: Option<ProjectPath>,
        error: impl std::fmt::Display,
    ) -> ContentError {
        ProjectStorageError::operation(
            PROJECT_STORAGE_FAILURE,
            operation,
            path,
            error.to_string(),
        )
        .into()
    }
}

fn generated_transaction_id(prefix: &str) -> Result<TransactionId, ContentError> {
    let sanitized = prefix
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let sequence = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(TransactionId::new(format!(
        "{}-{}-{}-{sequence}",
        sanitized,
        std::process::id(),
        unix_ms()
    ))?)
}

fn project_path(value: &str) -> Result<ProjectPath, ContentError> {
    Ok(ProjectPath::new(value)?)
}

fn transaction_directory(transaction_id: &TransactionId) -> Result<ProjectPath, ContentError> {
    project_path(&format!("{TRANSACTIONS_ROOT}/{}", transaction_id.as_str()))
}

fn journal_path(transaction_id: &TransactionId) -> Result<ProjectPath, ContentError> {
    transaction_directory(transaction_id)?.join(JOURNAL_FILE).map_err(Into::into)
}

fn audit_path(sequence: u64) -> Result<ProjectPath, ContentError> {
    project_path(&format!("{AUDIT_ROOT}/{sequence:020}.json"))
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn unique_sibling(target: &Path, suffix: &str) -> PathBuf {
    let sequence = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = target
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("project-file");
    target.with_file_name(format!(
        ".{file_name}.guiyi-{}-{}-{sequence}.{suffix}",
        std::process::id(),
        unix_ms()
    ))
}

#[cfg(unix)]
fn replace_file(temporary: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(temporary, target)
}

#[cfg(windows)]
fn replace_file(temporary: &Path, target: &Path) -> std::io::Result<()> {
    if !target.exists() {
        return fs::rename(temporary, target);
    }
    let backup = unique_sibling(target, "replace-backup");
    fs::rename(target, &backup)?;
    match fs::rename(temporary, target) {
        Ok(()) => {
            fs::remove_file(backup)?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(backup, target);
            Err(error)
        }
    }
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temporary_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "guiyi-storage-{name}-{}-{}",
            std::process::id(),
            unix_ms()
        ))
    }

    fn logical(value: &str) -> ProjectPath {
        ProjectPath::new(value).unwrap()
    }

    fn transaction(name: &str) -> ProjectTransaction {
        let mut transaction = ProjectTransaction::generated(
            name,
            AgentSessionId::from_static("session.storage-test"),
            "storage-test",
            json!({"kind": name}),
        )
        .unwrap();
        transaction
            .write_json(logical("content/a.json"), &json!({"version": 2}))
            .unwrap();
        transaction
            .write_json(logical("content/b.json"), &json!({"created": true}))
            .unwrap();
        transaction
            .write_manifest_json(
                logical("engine-project.json"),
                &json!({"documents": ["content/a.json", "content/b.json"]}),
            )
            .unwrap();
        transaction
    }

    fn seed(storage: &ProjectStorage) {
        storage
            .save_json(&logical("content/a.json"), &json!({"version": 1}))
            .unwrap();
        storage
            .save_json(
                &logical("engine-project.json"),
                &json!({"documents": ["content/a.json"]}),
            )
            .unwrap();
    }

    fn assert_old_state(storage: &ProjectStorage) {
        assert_eq!(
            storage
                .load_json::<Value>(&logical("content/a.json"))
                .unwrap(),
            json!({"version": 1})
        );
        assert!(!storage.exists(&logical("content/b.json")).unwrap());
        assert_eq!(
            storage
                .load_json::<Value>(&logical("engine-project.json"))
                .unwrap(),
            json!({"documents": ["content/a.json"]})
        );
    }

    fn assert_new_state(storage: &ProjectStorage) {
        assert_eq!(
            storage
                .load_json::<Value>(&logical("content/a.json"))
                .unwrap(),
            json!({"version": 2})
        );
        assert_eq!(
            storage
                .load_json::<Value>(&logical("content/b.json"))
                .unwrap(),
            json!({"created": true})
        );
        assert_eq!(
            storage
                .load_json::<Value>(&logical("engine-project.json"))
                .unwrap(),
            json!({"documents": ["content/a.json", "content/b.json"]})
        );
    }

    #[test]
    fn every_precommit_failure_recovers_the_previous_project_state() {
        let failures = [
            StorageFailurePoint::AfterPrepare,
            StorageFailurePoint::AfterApplyingMarker,
            StorageFailurePoint::AfterOperation(0),
            StorageFailurePoint::AfterOperation(1),
            StorageFailurePoint::BeforeManifest,
            StorageFailurePoint::AfterManifest,
        ];
        for (index, failure) in failures.into_iter().enumerate() {
            let root = temporary_directory(&format!("rollback-{index}"));
            let storage = ProjectStorage::create(&root).unwrap();
            seed(&storage);
            let result = storage.commit_with_failure(transaction("rollback"), failure);
            assert!(result.is_err());
            drop(storage);

            let recovered = ProjectStorage::open(&root).unwrap();
            assert_old_state(&recovered);
            recovered.recover().unwrap();
            assert_old_state(&recovered);
            let audit = recovered.audit_records().unwrap();
            assert_eq!(audit.len(), 1);
            assert_eq!(audit[0].state, StorageTransactionState::RolledBack);
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn failures_after_commit_marker_preserve_new_state_and_restore_audit() {
        for (index, failure) in [
            StorageFailurePoint::AfterCommitMarker,
            StorageFailurePoint::AfterAudit,
        ]
        .into_iter()
        .enumerate()
        {
            let root = temporary_directory(&format!("committed-{index}"));
            let storage = ProjectStorage::create(&root).unwrap();
            seed(&storage);
            let result = storage.commit_with_failure(transaction("committed"), failure);
            assert!(result.is_err());
            drop(storage);

            let recovered = ProjectStorage::open(&root).unwrap();
            assert_new_state(&recovered);
            recovered.recover().unwrap();
            assert_new_state(&recovered);
            let audit = recovered.audit_records().unwrap();
            assert_eq!(audit.len(), 1);
            assert_eq!(audit[0].state, StorageTransactionState::Committed);
            assert_eq!(audit[0].session_id.as_str(), "session.storage-test");
            assert_eq!(audit[0].report, json!({"kind": "committed"}));
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn successful_transaction_commits_manifest_last_and_survives_restart() {
        let root = temporary_directory("success");
        let storage = ProjectStorage::create(&root).unwrap();
        seed(&storage);
        let commit = storage.commit(transaction("success")).unwrap();
        assert_eq!(commit.state, StorageTransactionState::Committed);
        assert_new_state(&storage);
        drop(storage);

        let reopened = ProjectStorage::open(&root).unwrap();
        assert_new_state(&reopened);
        assert_eq!(reopened.audit_records().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_targets_return_a_structured_plan_error() {
        let root = temporary_directory("duplicate");
        let storage = ProjectStorage::create(&root).unwrap();
        let mut transaction = ProjectTransaction::generated(
            "duplicate",
            AgentSessionId::from_static("session.storage-test"),
            "storage-test",
            json!({}),
        )
        .unwrap();
        transaction.write(logical("content/a.json"), b"one".to_vec());
        transaction.write(logical("content/a.json"), b"two".to_vec());
        let error = storage.commit(transaction).unwrap_err();
        match error {
            ContentError::Storage(error) => {
                assert_eq!(error.code, PROJECT_STORAGE_PLAN_INVALID);
                assert_eq!(error.operation, "validate_plan");
                assert_eq!(error.path.unwrap().as_str(), "content/a.json");
            }
            other => panic!("unexpected error: {other}"),
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atomic_writes_do_not_leave_fixed_or_reused_temporary_files() {
        let root = temporary_directory("atomic");
        let storage = ProjectStorage::create(&root).unwrap();
        let path = logical("content/value.json");
        for value in 0..20 {
            storage.save_json(&path, &json!({"value": value})).unwrap();
        }
        let content_directory = root.join("content");
        let leftovers = fs::read_dir(content_directory)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".guiyi-"))
            .count();
        assert_eq!(leftovers, 0);
        fs::remove_dir_all(root).unwrap();
    }
}
