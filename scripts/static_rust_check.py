#!/usr/bin/env python3
"""Lightweight offline checks; this never replaces cargo check."""
from pathlib import Path
import re
import sys
import tomllib

root = Path(__file__).resolve().parents[1]
errors: list[str] = []

# Check basic delimiter integrity while ignoring comments and quoted literals.
for path in root.rglob("*.rs"):
    source = path.read_text(encoding="utf-8")
    stack: list[tuple[str, int]] = []
    state = "code"
    block_depth = 0
    index = 0
    while index < len(source):
        char = source[index]
        next_char = source[index + 1] if index + 1 < len(source) else ""
        if state == "code":
            if char == "/" and next_char == "/":
                state = "line"
                index += 2
                continue
            if char == "/" and next_char == "*":
                state = "block"
                block_depth = 1
                index += 2
                continue
            if char == '"':
                state = "string"
                index += 1
                continue
            if char in "({[":
                stack.append((char, index))
            elif char in ")}]":
                expected = {")": "(", "}": "{", "]": "["}[char]
                if not stack or stack[-1][0] != expected:
                    errors.append(f"{path.relative_to(root)}: mismatched {char} at {index}")
                    break
                stack.pop()
            index += 1
        elif state == "line":
            if char == "\n":
                state = "code"
            index += 1
        elif state == "block":
            if char == "/" and next_char == "*":
                block_depth += 1
                index += 2
            elif char == "*" and next_char == "/":
                block_depth -= 1
                index += 2
                if block_depth == 0:
                    state = "code"
            else:
                index += 1
        elif state == "string":
            if char == "\\":
                index += 2
            elif char == '"':
                state = "code"
                index += 1
            else:
                index += 1
    if stack:
        errors.append(f"{path.relative_to(root)}: unclosed delimiters")

# Check that external crate names observed in source are declared directly.
standard_roots = {"std", "core", "alloc", "self", "super", "crate"}
for manifest in root.rglob("Cargo.toml"):
    if manifest == root / "Cargo.toml":
        continue
    data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    package = data.get("package", {}).get("name", str(manifest.parent))
    dependencies = set(data.get("dependencies", {})) | set(data.get("dev-dependencies", {}))
    dependency_roots = {name.replace("-", "_") for name in dependencies}
    used: set[str] = set()
    for path in (manifest.parent / "src").glob("*.rs"):
        source = path.read_text(encoding="utf-8")
        used.update(re.findall(r"^(?:pub\s+)?use\s+([A-Za-z_][\w]*)::", source, re.MULTILINE))
        for name in re.findall(r"\b([A-Za-z_][\w]*)::", source):
            if name.startswith(("guiyi_engine", "tactical_rpg", "serde", "bevy", "clap", "thiserror")):
                used.add(name)
    missing = sorted(used - dependency_roots - standard_roots)
    if missing:
        errors.append(f"{package}: undeclared dependency roots: {', '.join(missing)}")

if errors:
    print("\n".join(errors))
    sys.exit(1)
print(f"offline Rust checks passed for {len(list(root.rglob('*.rs')))} files")
