from pathlib import Path
import subprocess
import sys

path = Path("scripts/eng009_apply.py")
text = path.read_text()
text = text.replace(
    '''    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one replacement target, found {count}")
    target.write_text(text.replace(old, new, 1))
''',
    '''    count = text.count(old)
    if count == 1:
        target.write_text(text.replace(old, new, 1))
        return
    if count == 0 and new in text:
        return
    raise SystemExit(f"{path}: expected one replacement target, found {count}")
''',
    1,
)
text = text.replace(
    '''    count = section.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one section target, found {count}")
    section = section.replace(old, new, 1)
    target.write_text(text[:start] + section + text[end:])
''',
    '''    count = section.count(old)
    if count == 1:
        section = section.replace(old, new, 1)
        target.write_text(text[:start] + section + text[end:])
        return
    if count == 0 and new in section:
        return
    raise SystemExit(f"{path}: expected one section target, found {count}")
''',
    1,
)
path.write_text(text)
subprocess.run([sys.executable, str(path)], check=True)
