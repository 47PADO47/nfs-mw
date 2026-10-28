"""Data and clocks shared by the private, opt-in capture tools."""
from datetime import datetime, timezone
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
import threading
import time


def digest(path):
    result = sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(block)
    return result.hexdigest()


def write_json(path, value):
    path = Path(path)
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value, indent=2), encoding='utf-8')
    os.replace(temporary, path)


class Timeline:
    def __init__(self, folder):
        self.origin_ns = time.perf_counter_ns()
        self.unix_ns = time.time_ns()
        self.lock = threading.Lock()
        self.sequence = 0
        self.file = (folder / 'events.jsonl').open('x', encoding='utf-8', buffering=1)

    def seconds(self):
        return (time.perf_counter_ns() - self.origin_ns) / 1e9

    def emit(self, kind, **fields):
        with self.lock:
            self.sequence += 1
            record = dict(seq=self.sequence, t=self.seconds(), kind=kind, **fields)
            self.file.write(json.dumps(record, separators=(',', ':')) + '\n')
        return record

    def header(self):
        return dict(origin_perf_ns=self.origin_ns, origin_unix_ns=self.unix_ns,
                    utc=datetime.fromtimestamp(self.unix_ns / 1e9, timezone.utc).isoformat())


def hidden_process(command, **kwargs):
    flags = subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
    return subprocess.Popen(command, creationflags=flags, **kwargs)


def external_output(path, checkout):
    path = Path(path).resolve()
    if path == checkout or checkout in path.parents:
        raise ValueError('Captures must be outside the source checkout')
    return path
