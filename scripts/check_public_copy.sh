#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 - "$ROOT" <<'PY'
from pathlib import Path
import re, sys
root = Path(sys.argv[1])
terms = [
    'sta'+'blyai',
    'github.com/'+'sta'+'blyai',
    'github.com/'+'iii'+'-hq',
    'iii'+'-hq/'+'iii',
    'iii'+'-hq/'+'workers',
]
exact = [re.compile(r''+'or'+'ca'+r'', re.I)]
skip = {'.git','target'}
violations=[]
for p in root.rglob('*'):
    if not p.is_file() or any(part in skip for part in p.parts):
        continue
    if p.suffix.lower() not in {'.rs','.md','.toml','.yml','.yaml','.sh'}:
        continue
    text = p.read_text(errors='ignore').lower()
    for term in terms:
        if term in text:
            violations.append(f'{p.relative_to(root)}: {term}')
    for pattern in exact:
        if pattern.search(text):
            violations.append(f'{p.relative_to(root)}: {pattern.pattern}')
if violations:
    print('Restricted public references found:')
    print('\n'.join(violations))
    raise SystemExit(1)
print('Public copy check passed')
PY
