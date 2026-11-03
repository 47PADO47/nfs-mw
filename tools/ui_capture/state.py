"""Copy small save/settings files around a session so defaults and progression can be identified later."""
from pathlib import Path
import shutil
import subprocess

from .common import digest, hidden_process, write_json

# Saves and settings are small; anything larger is hashed but not copied.
COPY_LIMIT = 4 * 1024 * 1024


def files_under(root):
    root = Path(root)
    if root.is_file():
        return [(root.name, root)]
    if not root.is_dir():
        return []
    return [(str(path.relative_to(root)), path) for path in sorted(root.rglob('*')) if path.is_file()]


def export_registry(key, target):
    """`reg export` keeps the native text format; a missing key is recorded, not treated as an error."""
    process = hidden_process(['reg', 'export', key, str(target), '/y'], stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE, text=True)
    _, error = process.communicate(timeout=30)
    if process.returncode:
        return dict(key=key, missing=True, error=error.strip())
    return dict(key=key, file=target.name, sha256=digest(target))


def snapshot(folder, label, paths=(), registry=()):
    """Write `state/<label>/` and its manifest. Returns the manifest."""
    target = Path(folder) / 'state' / label
    target.mkdir(parents=True, exist_ok=False)
    sources = []
    for index, root in enumerate(paths):
        entries = []
        for relative, path in files_under(root):
            size = path.stat().st_size
            entry = dict(path=relative, bytes=size, sha256=digest(path))
            if size <= COPY_LIMIT:
                copy = target / f'source{index}' / relative
                copy.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, copy)
                entry['copied'] = True
            entries.append(entry)
        sources.append(dict(index=index, root=str(root), exists=Path(root).exists(), files=entries))
    keys = [export_registry(key, target / f'registry{index}.reg') for index, key in enumerate(registry)]
    manifest = dict(label=label, sources=sources, registry=keys)
    write_json(target / 'manifest.json', manifest)
    return manifest


def changes(before, after):
    """Files added, removed or changed between two snapshots, per source root."""
    result = []
    for old, new in zip(before['sources'], after['sources']):
        a = {f['path']: f['sha256'] for f in old['files']}
        b = {f['path']: f['sha256'] for f in new['files']}
        result.append(dict(root=old['root'], added=sorted(b.keys() - a.keys()), removed=sorted(a.keys() - b.keys()),
                           changed=sorted(k for k in a.keys() & b.keys() if a[k] != b[k])))
    registry = [dict(key=o['key'], changed=o.get('sha256') != n.get('sha256'))
                for o, n in zip(before['registry'], after['registry'])]
    return dict(sources=result, registry=registry)
