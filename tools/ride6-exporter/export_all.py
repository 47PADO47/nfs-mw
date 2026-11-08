"""Export every RIDE 6 bike (or a selection) with `bikeparts.py`, several at a time, resumable.

    python -I export_all.py [--out C:\r6work\bikes] [--only <regex on the vehicle id>] [--limit N] [--jobs 3] [--force] [--normals]

Each bike goes to `<out>/<vehicle id>/` (`parts.json`, `glb/`, `tex/`) with a `status.json`; a bike that has one is skipped
unless `--force`. When all are done `fallbacks.py` gives the surfaces no picture reaches the material of a similar model.
`report.py` turns the folders into the completeness report.
"""

import argparse
import json
import os
import re
import sys
import threading
import time
import traceback
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import r6site  # noqa: F401,E402

import bikeparts  # noqa: E402
import fallbacks  # noqa: E402
from bindings import Tables  # noqa: E402
from r6lib import Assets, Tool  # noqa: E402

local = threading.local()
log_lock = threading.Lock()


def worker_state():
    if not hasattr(local, "tool"):
        local.tool = Tool()
        local.assets = Assets(local.tool)
        local.tables = Tables(local.assets)
    return local


def run_one(vehicle, out_root, force, max_tex):
    folder = os.path.join(out_root, vehicle)
    status_path = os.path.join(folder, "status.json")
    if os.path.exists(status_path) and not force:
        return vehicle, "skipped"
    started = time.time()
    try:
        state = worker_state()
        status = bikeparts.build(vehicle, folder, state.tool, state.assets, state.tables, max_tex)
    except Exception as error:  # a crash of one bike is a result, not the end of the run
        status = {"ok": False, "reason": f"{type(error).__name__}: {error}", "trace": traceback.format_exc()}
        local.__dict__.pop("tool", None)
    status["seconds"] = round(time.time() - started, 1)
    os.makedirs(folder, exist_ok=True)
    with open(status_path, "w", encoding="utf-8") as f:
        json.dump(status, f, indent=1)
    with log_lock:
        print(vehicle, "ok" if status.get("ok") else "FAILED " + str(status.get("reason")), status["seconds"], flush=True)
    return vehicle, "ok" if status.get("ok") else "failed"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default=r"C:\r6work\bikes")
    parser.add_argument("--only", default="")
    parser.add_argument("--limit", type=int, default=0)
    parser.add_argument("--jobs", type=int, default=3)
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--max-tex", type=int, default=2048)
    parser.add_argument("--normals", action="store_true", help="also export the normal maps (for ride6-bike-import --normal-maps)")
    args = parser.parse_args()
    if args.normals:
        os.environ["R6_NORMALS"] = "1"
    tool = Tool()
    try:
        vehicles = sorted(Tables(Assets(tool)).vehicles())
    finally:
        tool.close()
    if args.only:
        vehicles = [v for v in vehicles if re.search(args.only, v)]
    if args.limit:
        vehicles = vehicles[: args.limit]
    os.makedirs(args.out, exist_ok=True)
    print(len(vehicles), "vehicles", flush=True)
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        results = list(pool.map(lambda v: run_one(v, args.out, args.force, args.max_tex), vehicles))
    # A surface no picture reaches takes the material of a similar model (fallbacks.py); the donors are the bikes of `--out`.
    rows = fallbacks.run(args.out, only=args.only or None)
    print(len(rows), "surfaces took the material of a similar model")
    counts = {}
    for _, state in results:
        counts[state] = counts.get(state, 0) + 1
    print(counts)


if __name__ == "__main__":
    main()
