"""Shared helpers of the RIDE 6 track exporter: paths, the tool process of `tools/ride6-exporter`, small maths.

Our own code; it contains no game data. The exporter's scripts (`r6lib.py`, `materials.py`) are found through the
environment variable `R6_EXPORTER_DIR` or, by default, the folder `tools/ride6-exporter` next to this one.
"""

import math
import os
import site
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
EXPORTER = os.environ.get("R6_EXPORTER_DIR") or os.path.join(os.path.dirname(HERE), "ride6-exporter")
OUT_ROOT = os.environ.get("R6_TRACKS_OUT", r"C:\r6work\tracks")

for extra in (site.getusersitepackages(), EXPORTER, HERE):
    if extra not in sys.path:
        sys.path.append(extra)

try:
    import r6lib  # (the exporter's tool process and package cache)
    import materials  # (material instances resolved through their parents)
except ImportError:  # only the exporting needs them; the viewers of a finished pack do not
    r6lib = materials = None


def open_assets():
    """The tool process and the JSON cache of the install's packages."""
    tool = r6lib.Tool()
    return tool, r6lib.Assets(tool)


def ue_to_render(x, y, z):
    """A point of Unreal (centimetres, x forward, y right, z up) as the game's render space (metres, x forward, y left, z up)."""
    return (x / 100.0, -y / 100.0, z / 100.0)


def yaw_matrix(yaw_degrees):
    """The 2 by 2 turn of an Unreal yaw (degrees; positive turns x towards y)."""
    a = math.radians(yaw_degrees)
    return math.cos(a), -math.sin(a), math.sin(a), math.cos(a)


def ue_world(location, yaw_degrees, point):
    """A point of an actor with an Unreal location and yaw (the pitch and roll of a track's splines are 0)."""
    c, ns, s, c2 = yaw_matrix(yaw_degrees)
    x, y, z = point
    return (location["X"] + c * x + ns * y, location["Y"] + s * x + c2 * y, location["Z"] + z)


def log(*args):
    print(*args, flush=True)
