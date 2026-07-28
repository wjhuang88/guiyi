#!/usr/bin/env python3
from pathlib import Path
import json
import sys
import tomllib

root = Path(__file__).resolve().parents[1]
workspace = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))
members = workspace['workspace']['members']
errors = []
for member in members:
    path = root / member
    if not (path / 'Cargo.toml').exists():
        errors.append(f'missing member manifest: {member}')
    source = path / 'src'
    if not source.exists() or not any(source.glob('*.rs')):
        errors.append(f'missing Rust source: {member}')
for manifest in root.rglob('Cargo.toml'):
    try:
        tomllib.loads(manifest.read_text(encoding='utf-8'))
    except Exception as error:
        errors.append(f'invalid TOML {manifest.relative_to(root)}: {error}')
for sample in root.rglob('*.json'):
    try:
        json.loads(sample.read_text(encoding='utf-8'))
    except Exception as error:
        errors.append(f'invalid JSON {sample.relative_to(root)}: {error}')
if errors:
    print('\n'.join(errors))
    sys.exit(1)
print(f'repository gate passed: {len(members)} workspace members')
