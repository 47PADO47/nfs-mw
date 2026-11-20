"""Shared helpers of the RIDE 6 exporter scripts: the tool process, the JSON cache of package dumps, path names.

The scripts here are our own code and contain no game data. They drive `R6ToolParts` (tools/ride6-exporter/R6ToolParts, a
console tool around CUE4Parse) which reads the user's RIDE 6 install. Run python with `-I` when it reads extracted files.
"""

import json
import os
import subprocess
import threading

TOOL = os.environ.get("R6_TOOL", r"C:\r6work\tools\bin\R6ToolParts.exe")
CACHE = os.environ.get("R6_CACHE", r"C:\r6work\cache")
DONE = "\x01DONE"


class Tool:
    """One `R6ToolParts serve` process: requests are tab separated lines, each answered by lines then a marker line."""

    def __init__(self):
        self.lock = threading.Lock()
        self.proc = subprocess.Popen(
            [TOOL, "serve"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            encoding="utf-8",
            bufsize=1,
        )
        ready = self.proc.stdout.readline().strip()
        if ready != "READY":
            raise RuntimeError(f"R6ToolParts did not start: {ready!r}")

    def request(self, *args):
        """The answer lines of one request."""
        line = "\t".join(str(a) for a in args)
        with self.lock:
            self.proc.stdin.write(line + "\n")
            self.proc.stdin.flush()
            lines = []
            while True:
                text = self.proc.stdout.readline()
                if text == "":
                    raise RuntimeError("R6ToolParts exited")
                text = text.rstrip("\r\n")
                if text == DONE:
                    return lines
                lines.append(text)

    def close(self):
        if self.proc.poll() is not None:
            return
        try:
            self.proc.stdin.write("quit\n")
            self.proc.stdin.flush()
            self.proc.wait(timeout=20)
        except Exception:
            self.proc.kill()


def package_key(path):
    """The lower-case package path of an Unreal object path: `/Game/x/y.y.0` and `/Game/x/y` are `/game/x/y`."""
    path = path.replace("\\", "/")
    head, _, last = path.rpartition("/")
    return (head + "/" + last.split(".")[0]).lower()


def object_name(path):
    """The last component of an object path without suffixes: `/Game/x/Mat_A.Mat_A.0` is `mat_a`."""
    return path.replace("\\", "/").rsplit("/", 1)[-1].split(".")[0].lower()


class Assets:
    """The JSON dumps of packages, cached on disk (`<cache>/json`), fetched through the tool in batches."""

    def __init__(self, tool, cache=CACHE):
        self.tool = tool
        self.root = os.path.join(cache, "json")
        os.makedirs(self.root, exist_ok=True)
        self.index_path = os.path.join(self.root, "index.json")
        self.index = {}
        self.parsed = {}
        self.lock = threading.Lock()
        if os.path.exists(self.index_path):
            with open(self.index_path, encoding="utf-8") as f:
                self.index = json.load(f)

    def save_index(self):
        """Merge with what other workers saved meanwhile (a lost entry only means one package is dumped again)."""
        with self.lock:
            merged = {}
            try:
                with open(self.index_path, encoding="utf-8") as f:
                    merged = json.load(f)
            except (OSError, ValueError):
                merged = {}
            merged.update(self.index)
            temp = self.index_path + f".{os.getpid()}.{threading.get_ident()}.tmp"
            with open(temp, "w", encoding="utf-8") as f:
                json.dump(merged, f)
            try:
                os.replace(temp, self.index_path)
            except OSError:
                pass

    def ensure(self, paths):
        """Dump every package of `paths` that is not cached yet (one tool request)."""
        todo = {}
        for path in paths:
            key = package_key(path)
            if key in self.index or key in todo:
                continue
            todo[key] = path
        if not todo:
            return
        list_file = os.path.join(self.root, f"_list_{os.getpid()}_{threading.get_ident()}.txt")
        with open(list_file, "w", encoding="utf-8") as f:
            f.write("\n".join(todo.values()) + "\n")
        answer = self.tool.request("jsonmany", list_file, self.root)
        os.remove(list_file)
        by_path = {path: key for key, path in todo.items()}
        for line in answer:
            fields = line.split("\t")
            state = fields[0]
            if len(fields) < 2 or fields[1] not in by_path:
                continue
            key = by_path[fields[1]]
            if state == "OK":
                self.index[key] = fields[2]
            if state == "MISSING":
                self.index[key] = ""
            if state == "FAIL":
                print("FAIL", line)
        self.save_index()

    def exports(self, path):
        """The parsed export list of a package, or None when the install has no such package."""
        key = package_key(path)
        if key in self.parsed:
            return self.parsed[key]
        self.ensure([path])
        rel = self.index.get(key)
        if not rel:
            self.parsed[key] = None
            return None
        with open(os.path.join(self.root, rel), encoding="utf-8") as f:
            data = json.load(f)
        self.parsed[key] = data
        return data

    def named(self, path):
        """The export whose name is the package's own name (the object of a data asset), else None."""
        data = self.exports(path)
        if data is None:
            return None
        stem = object_name(path)
        for export in data:
            if export.get("Name", "").lower() == stem:
                return export
        return None

    def props(self, path):
        """The properties of the package's own export, `{}` when there is none."""
        export = self.named(path)
        if export is None:
            return {}
        return export.get("Properties") or {}


def ref_path(value):
    """The object path of an object reference property (`{"ObjectName", "ObjectPath"}`), or None."""
    if not isinstance(value, dict):
        return None
    return value.get("ObjectPath")


def ref_kind(value):
    """The class name an object reference property names: `SkeletalMesh'x'` is `SkeletalMesh`."""
    if not isinstance(value, dict):
        return ""
    return str(value.get("ObjectName", "")).split("'")[0]
