#!/usr/bin/env python3
"""Export every track of the install into its own pack and list what came out (docs/formats/ride6-track-pack.md).

    python -I tools/ride6-tracks/survey.py [--out C:/r6work/tracks] [--only REGEX]

One line per track: the time, the triangles, the materials, the pictures, the size of the pack (without the scratch glTF), and the
reason when the export failed. Resumable: a track with a `pack.json` is skipped unless `--force`.
"""

import argparse
import os
import re
import shutil
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import common  # noqa: E402  (first: it makes the user site-packages importable)
import export  # noqa: E402
import pack  # noqa: E402


def tracks(tool):
    """The names of the track folders that have a terrain level."""
    names = []
    for line in tool.request("list", r"maps/Tracks/[^/]+/Game/Terrain\.umap$"):
        parts = line.replace("\\", "/").split("/")
        names.append(parts[parts.index("Tracks") + 1])
    return sorted(set(names))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default=common.OUT_ROOT)
    parser.add_argument("--only", default=None)
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--max-texture", type=int, default=1024)
    args = parser.parse_args()
    tool = common.r6lib.Tool()
    names = tracks(tool)
    tool.close()
    for name in names:
        if args.only and not re.search(args.only, name, re.IGNORECASE):
            continue
        folder = os.path.join(args.out, name.lower() + "-pack")
        if os.path.exists(os.path.join(folder, "pack.json")) and not args.force:
            common.log(f"{name}: done")
            continue
        started = time.time()
        try:
            export.run(name, folder, args.max_texture)
            shutil.rmtree(os.path.join(folder, "_glb"), ignore_errors=True)
            common.log(f"{name}: OK {time.time() - started:.1f} s, {pack.pack_size(folder) / 1e6:.0f} MB")
        except Exception as error:  # a track that cannot be read is listed, the survey goes on
            shutil.rmtree(folder, ignore_errors=True)
            common.log(f"{name}: FAILED {type(error).__name__}: {error}")


if __name__ == "__main__":
    main()
