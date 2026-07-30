# Project storage boundary

## Purpose

All project-controlled paths and filesystem mutations pass through `ProjectPath` and `ProjectFilesystem`. Higher-level clients must not join manifest-controlled paths directly onto a project root or use unrestricted `std::fs` operations for project content.

This boundary confines authoring documents, manifests, autosaves, generated content, and build artifacts to one canonical project root. It is not a general operating-system sandbox and does not isolate external processes.

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

## ProjectFilesystem contract

`ProjectFilesystem` canonicalizes the declared project root when it is opened. Every operation accepts a `ProjectPath` and performs one of two checks:

1. Existing reads, deletes, and rename sources canonicalize the complete target and require it to remain under the canonical root.
2. New writes, directories, and rename destinations canonicalize the nearest existing ancestor and require that ancestor to remain under the canonical root. Existing final entries are also canonicalized.

This nearest-existing-ancestor rule protects paths whose final file does not exist yet while still detecting a symlinked parent directory. A path that resolves outside the root returns `PROJECT_PATH_ESCAPE` with the rejected logical path and resolved-root context.

The boundary provides common operations for:

- existence checks;
- directory creation;
- byte and JSON reads;
- byte and JSON writes;
- rename;
- delete.

## Client rules

- CLI initialization, doctor, validation, and compilation use `ProjectFilesystem`.
- CLI build output is interpreted as a project-relative logical path. Absolute and parent-traversing output arguments are rejected.
- The JSONL Workbench loads manifest paths and persists document creation, update, and deletion through the same boundary.
- Authoring autosave accepts a `ProjectFilesystem` and `ProjectPath`, not an unrestricted host path.
- Game-specific or UI clients must reuse this boundary rather than introducing a private path join or write path.

## Diagnostics and failure behavior

Path failures are recoverable structured errors. Their display representation includes the stable code, rejected logical path, and reason. CLI and Workbench entry points propagate these failures without panicking. A rejected path must not create, overwrite, rename, or delete an external file.

## Security limits

This boundary prevents logical traversal and symlink escape at operation validation time. It does not provide process isolation, kernel-enforced directory capabilities, protection against a hostile actor racing filesystem entries between validation and the final OS call, or remote storage semantics. Those require separate approved Stories if needed.
