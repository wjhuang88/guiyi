#!/usr/bin/env python3
"""ENG-027: offline, fail-closed static pin/license/release-manifest hygiene.

This is not a dependency vulnerability scan or a cryptographic verification of
upstream action identity; pin provenance must still be reviewed in the PR.
"""
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXACT_COMMIT = re.compile(r"^[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+@[0-9a-f]{40}$")
USE = re.compile(r"(?m)^\s*-\s*uses:\s*([^\s#]+)")
CONCRETE_RUST = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
APACHE_SECTIONS = (
    "1. Definitions.",
    "2. Grant of Copyright License.",
    "3. Grant of Patent License.",
    "4. Redistribution.",
    "5. Submission of Contributions.",
    "6. Trademarks.",
    "7. Disclaimer of Warranty.",
    "8. Limitation of Liability.",
    "9. Accepting Warranty or Additional Liability.",
    "END OF TERMS AND CONDITIONS",
    "APPENDIX: How to apply the Apache License to your work.",
)


def action_errors(workflow: str, name: str) -> list[str]:
    """Check all third-party actions, including ones added in later PRs."""
    errors: list[str] = []
    uses = USE.findall(workflow)
    if not uses:
        return [f"{name}: no actions were found by the pin validator"]
    for ref in uses:
        if ref.startswith("./"):
            continue  # Repository-local actions are source-tracked by checkout.
        if not EXACT_COMMIT.fullmatch(ref):
            errors.append(
                f"{name}: action {ref!r} is not pinned to a full immutable commit SHA"
            )
    return errors


def toolchain_errors(channel: str, workflow: str) -> list[str]:
    if not CONCRETE_RUST.fullmatch(channel):
        return [f"rust-toolchain.toml: floating or invalid channel {channel!r}"]
    # Quoting is optional in YAML but must specify the same version.
    value = rf"(?m)^\s*toolchain:\s*['\"]?{re.escape(channel)}['\"]?\s*(?:#.*)?$"
    if not re.search(value, workflow):
        return [f"ci.yml: Rust installer must explicitly use toolchain {channel}"]
    return []


def cargo_errors(text: str, name: str, *, workflow: bool) -> list[str]:
    errors: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if workflow:
            if not stripped.startswith("- run: cargo "):
                continue
            command = stripped.removeprefix("- run: ").strip()
        else:
            if not stripped.startswith("cargo "):
                continue
            command = stripped
        if command.startswith("cargo fmt "):
            continue  # cargo fmt is a rustfmt wrapper, not a resolver.
        if "--locked" not in command.split():
            errors.append(f"{name}: unpinned Cargo resolution: {command}")
    if "cargo metadata --locked" not in text:
        errors.append(f"{name}: missing cargo metadata --locked smoke check")
    return errors


def license_errors(apache: str, mit: str) -> list[str]:
    errors: list[str] = []
    # An abbreviated LICENSE notice is NOT the full standard Apache text.
    if len(apache) < 10000 or any(section not in apache for section in APACHE_SECTIONS):
        errors.append("LICENSE-APACHE: standard Apache-2.0 terms are truncated")
    if "MIT License" not in mit or "Permission is hereby granted" not in mit or (
        'THE SOFTWARE IS PROVIDED "AS IS"' not in mit
    ):
        errors.append("LICENSE-MIT: standard MIT grant/warranty text is incomplete")
    return errors


def manifest_errors(has_stale_manifest: bool) -> list[str]:
    if has_stale_manifest:
        return [
            "PACKAGE-MANIFEST.json is a retired unverified bootstrap snapshot; "
            "remove it and create fresh inventories only for actual releases"
        ]
    return []


def self_test() -> list[str]:
    errors: list[str] = []
    full_sha = "a" * 40
    if action_errors(f"steps:\n  - uses: actions/checkout@{full_sha}\n", "good"):
        errors.append("self-test: full action SHA was rejected")
    if not action_errors("steps:\n  - uses: actions/checkout@v7\n", "bad"):
        errors.append("self-test: floating Action tag was accepted")
    if not toolchain_errors("stable", "    toolchain: stable\n"):
        errors.append("self-test: floating Rust stable was accepted")
    if toolchain_errors("1.99.1", "    toolchain: '1.99.1'\n"):
        errors.append("self-test: matching concrete Rust version was rejected")
    if not license_errors("Apache License - short notice", "MIT License - short notice"):
        errors.append("self-test: truncated licenses were accepted")
    if not manifest_errors(True):
        errors.append("self-test: retired stale manifest was accepted")
    if not cargo_errors("cargo test --workspace", "bad", workflow=False):
        errors.append("self-test: unlocked Cargo command was accepted")
    return errors


def verify_repository() -> list[str]:
    errors: list[str] = []
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    package = manifest["workspace"]["package"]
    if package.get("repository") != "https://github.com/wjhuang88/guiyi":
        errors.append("Cargo.toml: workspace repository URL does not match origin")
    if not (ROOT / "Cargo.lock").is_file():
        errors.append("Cargo.lock: committed dependency lockfile missing")

    toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))
    channel = toolchain["toolchain"]["channel"]
    ci = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    errors.extend(toolchain_errors(channel, ci))

    files = sorted((ROOT / ".github/workflows").glob("*.yml")) + sorted(
        (ROOT / ".github/workflows").glob("*.yaml")
    )
    if not files:
        errors.append("no GitHub Actions workflows found")
    for file in files:
        errors.extend(action_errors(file.read_text(encoding="utf-8"), str(file.relative_to(ROOT))))

    errors.extend(cargo_errors(ci, "ci.yml", workflow=True))
    first_run = (ROOT / "scripts/first_run.sh").read_text(encoding="utf-8")
    errors.extend(cargo_errors(first_run, "scripts/first_run.sh", workflow=False))
    for expected in [
        "python3 scripts/check_reproducibility.py --self-test",
        "python3 scripts/check_reproducibility.py",
    ]:
        if expected not in ci or expected not in first_run:
            errors.append(f"CI/first-run did not wire {expected}")

    apache = (ROOT / "LICENSE-APACHE").read_text(encoding="utf-8")
    mit = (ROOT / "LICENSE-MIT").read_text(encoding="utf-8")
    errors.extend(license_errors(apache, mit))
    errors.extend(manifest_errors((ROOT / "PACKAGE-MANIFEST.json").exists()))
    dependabot = (ROOT / ".github/dependabot.yml").read_text(encoding="utf-8")
    for ecosystem in ("cargo", "github-actions"):
        if f"package-ecosystem: {ecosystem}" not in dependabot:
            errors.append(f".github/dependabot.yml: missing {ecosystem} update source")
    return errors


def main() -> int:
    if len(sys.argv) > 2 or (len(sys.argv) == 2 and sys.argv[1] != "--self-test"):
        print("usage: python3 scripts/check_reproducibility.py [--self-test]")
        return 2
    errors = self_test() if len(sys.argv) == 2 else verify_repository()
    if errors:
        print("\n".join(errors))
        return 1
    print("ENG-027 reproducibility gate passed" + (" (negative probes)" if len(sys.argv) == 2 else ""))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
