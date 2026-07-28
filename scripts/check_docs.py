#!/usr/bin/env python3
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parents[1]
pattern = re.compile(r'\[[^\]]+\]\(([^)]+)\)')
errors = []
for document in list(root.rglob('*.md')):
    text = document.read_text(encoding='utf-8')
    for target in pattern.findall(text):
        if target.startswith(('http://', 'https://', 'mailto:', '#')):
            continue
        clean = target.split('#', 1)[0]
        if not clean:
            continue
        resolved = (document.parent / clean).resolve()
        if not resolved.exists():
            errors.append(f'{document.relative_to(root)} -> missing {target}')
if errors:
    print('\n'.join(errors))
    sys.exit(1)
print('documentation link gate passed')
