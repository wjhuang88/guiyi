# Project storage boundary

## Purpose

All project-controlled paths and filesystem mutations pass through `ProjectPath` and `ProjectStorage`. `ProjectFilesystem` remains the sandboxed low-level path boundary used internally by storage. Higher-level clients must not join manifest-controlled paths directly onto a project root or use unrestricted `std::fs` operations for project content.

This boundary confines authoring documents, manifests, autosaves, generated content, build artifacts, transaction journals, and audit records to one canonical project root. It is not a general operating-system sandbox and does not isolate external processes.

## ProjectPath contract

`ProjectPath` is a UTF-8 logical path serialized with forward slashes. Construction and deserialization reject:

- empty paths;
- absolute paths;
- `.` or `..` components;
- empty components such as `content//stage.json`;
- backslash separators and UNC forms;
- Windows drive prefixes such as `C:`;
- colon forms that could represent platform prefixes or alternate streams;
- non-normal platform path components.

A rejected logical path returns `PROJECT_PATH_INVALID` together with the original path and a reason. `ProjectManifest.documents` stores `ProjectPath` values, so an unsafe path is rejected while the manifest is deserialized and before any document read begins.

## Sandboxed filesystem contract

`ProjectFilesystem` canonicalizes the declared project root when it is opened. Every operation accepts a `ProjectPath` and performs one of two checks:

1. Existing reads, deletes, and rename sources canonicalize the complete target and require it to remain under the canonical root.
2. New writes, directories, and rename destinations canonicalize the nearest existing ancestor and require that ancestor to remain under the canonical root. Existing final entries are also canonicalized.

This nearest-existing-ancestor rule protects paths whose final file does not exist yet while still detecting a symlinked parent directory. A path that resolves outside the root returns `PROJECT_PATH_ESCAPE` with the rejected logical path and resolved-root context.

## ProjectStorage transaction contract

`ProjectStorage::open` runs recovery before exposing project data. Project mutations are represented by a `ProjectTransaction` containing a transaction ID, agent session ID, actor, structured report, and ordered write/delete operations.

A commit uses this protocol:

1. Validate the plan and reject duplicate logical targets.
2. Persist before and after snapshots plus a `prepared` write-ahead journal under `.agent-sessions/transactions/`.
3. Mark the journal `applying`.
4. Apply non-manifest operations.
5. Apply the manifest last, so it never references document content from an uncommitted state.
6. Mark the journal `committed`.
7. Persist an ordered audit record under `.agent-sessions/audit/`.

Each physical replacement uses a unique same-directory temporary file. File contents and the containing directory are synchronized around replacement. A failed normal commit invokes recovery before returning the structured storage error.

## Recovery behavior

Opening the project or explicitly calling `recover` scans persisted journals deterministically:

- `prepared` and `applying` transactions restore before snapshots, remove files that did not previously exist, record `rolled_back`, and preserve an audit record;
- `committed` transactions retain the new state and recreate a missing audit record;
- `rolled_back` transactions remain rolled back and keep their audit record.

Recovery is idempotent. Reopening and recovering the same project repeatedly produces the same readable state and does not duplicate audit sequence entries.

## Client rules

- CLI initialization commits the initial document, README, and manifest as one manifest-last project transaction.
- CLI doctor, validation, and compilation open `ProjectStorage`, so interrupted work is recovered before reads.
- CLI build artifacts are committed as one transaction only after the build report succeeds.
- The JSONL Workbench commits created, changed, and deleted documents together with the updated manifest and command transaction report.
- Authoring autosave commits all autosave documents through the same transaction and audit boundary.
- Game-specific or UI clients must reuse this boundary rather than introducing a private path join or write path.

## Structured diagnostics

Path errors use `PROJECT_PATH_INVALID` or `PROJECT_PATH_ESCAPE`. Storage errors include a stable code, logical operation, optional project path, and message. Current storage codes include:

- `PROJECT_STORAGE_FAILURE`;
- `PROJECT_STORAGE_PLAN_INVALID`;
- `PROJECT_STORAGE_RECOVERY_FAILED`;
- `PROJECT_STORAGE_INJECTED_FAILURE`.

CLI and Workbench entry points propagate host-level persistence failures without presenting a successful tool result. A rejected or failed transaction must not leave a manifest/document mismatch.

## Durability and platform limits

The implementation synchronizes temporary file contents before replacement and synchronizes the containing directory after replacement. On Unix, same-directory rename provides atomic replacement. On Windows, replacement uses a unique backup and restores it if the final rename fails.

Filesystem and storage-device durability semantics still vary by platform and hardware. This contract is single-process and single-writer; it does not provide distributed transactions, multi-user conflict resolution, protection against a hostile actor racing filesystem entries between validation and the final OS call, kernel-enforced directory capabilities, or remote storage semantics.
