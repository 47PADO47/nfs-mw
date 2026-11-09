"""The completeness report of an export run: which bikes lack which stock parts, and why.

    python -I report.py [--out C:\\r6work\\bikes] [--write <report.md>]

Reads `<out>/<vehicle>/{status.json,parts.json}` and the JSON cache of the package dumps (no game access, no tool). A part is
expected on every bike except the ones that only some bikes have (bar pad, hand guards, mirrors, sump guard).
"""

import argparse
import json
import os
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import r6site  # noqa: F401,E402

from bindings import Tables  # noqa: E402
from r6lib import Assets, object_name  # noqa: E402

ALWAYS = ("front_tyre", "back_tyre", "front_rim", "back_rim", "front_brake", "back_brake", "exmanifold", "muffler")
SOMETIMES = ("barpad", "handguards", "mirrors", "sump_guard", "enduro_preset", "homologation")


class NoTool:
    def request(self, *args):
        raise RuntimeError("report.py reads the cache only")


def load(out):
    bikes = {}
    for name in sorted(os.listdir(out)):
        folder = os.path.join(out, name)
        status_path = os.path.join(folder, "status.json")
        if not os.path.exists(status_path):
            continue
        with open(status_path, encoding="utf-8") as f:
            status = json.load(f)
        parts = None
        parts_path = os.path.join(folder, "parts.json")
        if status.get("ok") and os.path.exists(parts_path):
            with open(parts_path, encoding="utf-8") as f:
                parts = json.load(f)
        bikes[name] = (status, parts)
    return bikes


def stub_first(assets, tables, source):
    """Whether the data asset of a part row has a parent stub as its first export (the Hypermotard 698 case)."""
    table, _, key = source.partition(" ")
    if table not in tables.rows:
        return False
    asset = tables.asset(table, key)
    if not asset:
        return False
    exports = assets.exports(asset)
    if not exports or len(exports) < 2:
        return False
    return exports[0].get("Name", "").lower() != object_name(asset)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default=r"C:\r6work\bikes")
    parser.add_argument("--write", default="")
    args = parser.parse_args()
    assets = Assets(NoTool())
    tables = Tables.__new__(Tables)
    tables.assets = assets
    from bindings import TABLES, DIR

    tables.rows, tables.lower = {}, {}
    for name in TABLES:
        data = assets.exports(DIR + name)
        rows = (data[0].get("Rows") or {}) if data else {}
        tables.rows[name] = rows
        tables.lower[name] = {k.lower(): k for k in rows}
    bikes = load(args.out)
    refused = {v: s for v, (s, p) in bikes.items() if not s.get("ok")}
    done = {v: p for v, (s, p) in bikes.items() if s.get("ok")}
    lines = [f"# Export report\n\n{len(bikes)} vehicles: {len(done)} exported, {len(refused)} refused.\n"]
    reasons = Counter(s.get("reason", "?").split(" (")[0] for s in refused.values())
    lines.append("## Refused\n")
    for reason, n in reasons.most_common():
        lines.append(f"- {n}: {reason}")
    lines.append("")
    present = Counter()
    absent = defaultdict(list)
    why = defaultdict(Counter)
    stubs = defaultdict(list)
    liveries = Counter()
    for vehicle, parts in sorted(done.items()):
        ids = {p["id"] for p in parts["parts"]}
        liveries[len(parts.get("liveries", []))] += 1
        for pid in ALWAYS + SOMETIMES:
            if pid in ids:
                present[pid] += 1
        for pid in ALWAYS:
            if pid in ids:
                continue
            absent[pid].append(vehicle)
        for item in parts.get("missing", []):
            why[item["part"]][item["reason"]] += 1
        for p in parts["parts"]:
            if stub_first(assets, tables, p.get("source", "")):
                stubs[p["id"]].append(vehicle)
    lines.append("## Parts present\n")
    for pid in ALWAYS + SOMETIMES:
        lines.append(f"- {pid}: {present[pid]} of {len(done)}")
    lines.append("")
    lines.append("## Parts missing from bikes that should have them\n")
    for pid in ALWAYS:
        if not absent[pid]:
            continue
        lines.append(f"### {pid}: {len(absent[pid])}\n")
        for vehicle in absent[pid]:
            reasons = [m["reason"] + ": " + m["detail"] for m in done[vehicle].get("missing", []) if m["part"] in (pid, pid.split("_")[-1], "rims" if "rim" in pid else pid, "brakes" if "brake" in pid else pid)]
            lines.append(f"- {vehicle}: " + "; ".join(reasons))
        lines.append("")
    lines.append("## Reasons recorded (all bikes)\n")
    for part, counter in sorted(why.items()):
        for reason, n in counter.most_common():
            lines.append(f"- {part} / {reason}: {n}")
    lines.append("")
    lines.append("## Data assets whose first export is a parent stub\n")
    lines.append("A reader that takes the first export of such a package finds no mesh. The part is found here by the export named")
    lines.append("like the package.\n")
    for pid, vehicles in sorted(stubs.items()):
        lines.append(f"- {pid}: {len(vehicles)} bikes ({', '.join(vehicles[:6])}{', ...' if len(vehicles) > 6 else ''})")
    lines.append("")
    lines.append("## Liveries per bike\n")
    for n, count in sorted(liveries.items()):
        lines.append(f"- {n} liveries: {count} bikes")
    text = "\n".join(lines) + "\n"
    target = args.write or os.path.join(args.out, "export-report.md")
    with open(target, "w", encoding="utf-8") as f:
        f.write(text)
    print(text[:6000])


if __name__ == "__main__":
    main()
