#!/usr/bin/env python3
from pathlib import Path
import sys
import tomllib

root = Path(__file__).resolve().parents[1]
packages = {}
for manifest in root.rglob('Cargo.toml'):
    if manifest == root / 'Cargo.toml':
        continue
    data = tomllib.loads(manifest.read_text(encoding='utf-8'))
    package = data.get('package', {}).get('name')
    if package:
        dependencies = set(data.get('dependencies', {}))
        packages[package] = (manifest, dependencies)

errors = []
core = packages.get('guiyi-engine-core')
if core:
    for dependency in core[1]:
        if dependency.startswith('tactical-rpg') or dependency.startswith('guiyi-game'):
            errors.append(f'engine core has forbidden dependency: {dependency}')

for name, (manifest, dependencies) in packages.items():
    if name.startswith('guiyi-engine-') and name not in {
        'guiyi-engine-cli', 'guiyi-engine-editor'
    }:
        for dependency in dependencies:
            if dependency.startswith('tactical-rpg'):
                errors.append(f'{name} must not depend on toolkit crate {dependency}')
    if name == 'guiyi-engine-runtime':
        for forbidden in {'guiyi-engine-authoring', 'guiyi-engine-editor', 'guiyi-engine-agent-host'}:
            if forbidden in dependencies:
                errors.append(f'runtime has forbidden dependency: {forbidden}')
    if name == 'guiyi-engine-content':
        for forbidden in {'guiyi-engine-runtime', 'guiyi-engine-authoring', 'guiyi-engine-editor'}:
            if forbidden in dependencies:
                errors.append(f'content has forbidden dependency: {forbidden}')

if errors:
    print('\n'.join(errors))
    sys.exit(1)
print(f'architecture gate passed for {len(packages)} packages')
