r"""The rider animations the standstill and the hands need, exported next to the rider's own export.

    python -I tools\ride6-exporter\rider_anims.py --out C:\r6work\rider-export

writes `skeleton.json`, `standstill.r6ra` (the sequences Unreal's own codecs store) and `acl/` (the ACL ones) with the tool's `skeleton`, `anims` and
`acl` modes. Add them to the rider already imported with

    ride6-rider-import C:\r6work\rider-export <mod>\files\RIDER --merge

The clips are the finger clips (`race_clutch_brake_gear*`, `race_Wrist_X`) and the stand, start and stop clips of every bike type
(docs/specs/ride6-rider-standstill.md, docs/specs/ride6-rider-hands.md). The scripts hold no game data.
"""

import argparse
import os
import re
import subprocess
import sys
import tempfile

TOOL = os.environ.get("R6_TOOL", r"C:\r6work\tools\bin\R6ToolParts.exe")
ROOT = "ride6/Content/assets/graphics/characters/riders/riderShared/Animations/"
# The sequences by folder: (folder, regular expression of the file names).
WANTED = [
    ("Race", r"race_(clutch_brake_gear.*|Wrist_X|stand_(both_feet|foot_[LR])|start_(both_feet|foot_[LR])|stop_(both_feet|foot_[LR]))"),
    ("naked", r"naked_(stand|start|stop)_(both_feet|foot_[LR])"),
    ("Scooter", r"rider_scooter_(stand|start|stop)_(BothFeet|foot_[LR])"),
    ("enduro", r"enduro_(restart_idle|start|stop)_(both_feet|foot_[LR])"),
]


def run(*args):
    done = subprocess.run([TOOL, *args], capture_output=True, text=True, encoding="utf-8")
    return done.stdout.splitlines()


def listing():
    """The package paths of the wanted sequences, by asking the tool for every file under the animations folder."""
    with tempfile.TemporaryDirectory() as folder:
        out = os.path.join(folder, "all.txt")
        run("list", re.escape("riderShared/Animations/") + ".*", out)
        with open(out, encoding="utf-8") as lines:
            paths = [line.strip() for line in lines if line.strip()]
    wanted = []
    for folder, pattern in WANTED:
        regex = re.compile(re.escape(ROOT + folder + "/") + pattern + r"\.uasset$")
        wanted += [p for p in paths if regex.fullmatch(p)]
    return sorted(wanted)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", required=True, help="the export folder to write")
    out = parser.parse_args().out
    os.makedirs(out, exist_ok=True)
    paths = listing()
    if not paths:
        sys.exit("no sequence found: is R6_PAKS / R6_USMAP set, and R6_TOOL pointing at a build with the anims modes?")
    with open(os.path.join(out, "list.txt"), "w", encoding="utf-8") as lines:
        lines.write("\n".join(paths) + "\n")
    skeleton = run("skeleton", paths[0], os.path.join(out, "skeleton.json"))
    anims = run("anims", os.path.join(out, "list.txt"), os.path.join(out, "standstill.r6ra"))
    acl = run("acl", os.path.join(out, "list.txt"), os.path.join(out, "acl"))
    for line in skeleton + anims + acl:
        if not line.startswith("OK") and not line.startswith("ACL") and not line.startswith("NOTACL"):
            print(line)
    print(f"{len(paths)} sequences: {sum(1 for l in anims if l.startswith('OK'))} sampled, "
          f"{sum(1 for l in acl if l.startswith('OK'))} ACL")


if __name__ == "__main__":
    main()
