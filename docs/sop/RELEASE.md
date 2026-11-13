# Release SOP

Freeze → exact-head full validation → compatibility decision → release candidate → sample project validation → tag → assemble packages → fresh package inventory → changelog → upgrade guide.

Engine API version, content schema version, and save schema version are separate; breaking changes require an ADR and upgrade instructions. Do not treat a historical bootstrap snapshot as a release.

## Deterministic build and dependency policy (ENG-027)

- Check in `Cargo.lock`; use `cargo metadata --locked --format-version 1` and `--locked` on check, strict Clippy, test, doc, build and example runs. No new resolution that silently rewrites the lockfile during CI. First-resolution downloads still require network access unless dependencies are already cached.
- Select Rust `1.99.1` through `rust-toolchain.toml` and the exact CI install action toolchain; both must change together in a reviewed PR after validating required dependency MSRV.
- Third-party GitHub Actions **must be full immutable commit SHA pins**, with readable tag/version comments and verified upstream tag→commit provenance in the PR. Annotated tags must be peeled to the actual commit. Do not downgrade to floating major tags for a green badge.
- Dependabot (already configured) proposes reviewed Cargo updates weekly and GitHub Actions updates monthly. A pin update must rerun full exact-head CI and any applicable sample/project compatibility probes. Periodic review is required because pinned versions will not silently receive patches.
- The repository's Python-standard-library `scripts/check_reproducibility.py` gate protects pin syntax, version consistency, lockfile presence, license completeness markers and retirement of the stale manifest. It is **not** a vulnerability scanner, security proof or upstream signature verification service. Optional advisories/`cargo-deny` need another approved Story.

## Licenses and release inventory

Keep both full license texts `LICENSE-MIT` and `LICENSE-APACHE` with the `MIT OR Apache-2.0` workspace declaration. The Apache terms in this repo are the standard Apache License Version 2.0 text, sourced from the Apache Software Foundation distribution at [Apache Commons Lang LICENSE](https://github.com/apache/commons-lang/blob/master/LICENSE.txt) (identical standard legal terms); do not replace them with a shortened project notice.

The 2026-07-28 root `PACKAGE-MANIFEST.json` was a frozen bootstrap checksum snapshot and is deliberately **removed**, not a generated current file. For an actual release, freeze the exact tag, enumerate the packaged sources/artifacts from that tag using Git's tracked-file index and the release packaging recipe, compute file sizes/SHA-256 **from the built release bytes**, and attach that fresh inventory to the release artifacts. Never calculate or publish hashes from outdated bootstrap content, invent package files, or claim this Story has already shipped a release.

## Rollback and upgrade governance

Pinned toolchain/Action upgrades happen by PR with upstream provenance, repeatable CI and rollback to a **previous verified immutable ref** if needed. Do not revert to floating `stable`/major tags or remove `--locked` to make failures disappear. A packaging consumer requiring the retired static snapshot must request a separate compatibility/release Story rather than silently restoring a stale source of truth.
