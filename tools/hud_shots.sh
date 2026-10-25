#!/usr/bin/env sh
# Renders every HUD scenario offscreen and muted: the Custom HUD screenshot matrix (docs/specs/custom-hud-scenarios.md).
#
# Runs `nfsmw view-world --drive --hud-scenario <name> --screenshot ...` once per scenario and pass:
# - 1280x720 (16:9), 2560x1080 (21:9) and 5120x1440 (32:9);
# - a high-contrast pass at 1280x720 (`--exec "set custom_hud.high_contrast on"`);
# - one pass per colour-vision palette at 1280x720 (leave them out with --skip-palettes).
#
# Every run is a screenshot run: the window is hidden, unfocused and off the desktop, and the game is muted, so the
# matrix never takes the focus or makes a sound. Each run is waited for as a process, never as a window.
#
# Usage: tools/hud_shots.sh [--out DIR] [--exe PATH] [--game-dir PATH] [--skip-palettes] [SCENARIO...]
#   --out DIR        where the PNGs go; default a new nfsmw-hud-shots-<time> folder in the temporary directory. A folder
#                    inside the repository is refused.
#   --exe PATH       the nfsmw binary; default target/release/nfsmw, else target/debug/nfsmw (with .exe on Windows).
#   --game-dir PATH  the install, passed as --game-dir; default the usual discovery.
#   SCENARIO...      only these scenarios; default all of them.

set -eu

# The names of HudScenario::ALL (crates/nfsmw/src/devtools/hud_scenarios/mod.rs), in its order.
ALL_SCENARIOS="free-roam race-intro racing race-results pursuit-active search cooldown escape critical-damage notify-preempt map-compact map-centered map-expanded"
SIZES="1280x720 2560x1080 5120x1440"
PALETTES="deuteranopia tritanopia"
# Long enough after the scene is ready for the notices and the fades to settle (notify-preempt steps on frame 30).
DELAY=1.5

REPO_ROOT=$(cd "$(dirname "$0")/.." && pwd -P)

out=""
exe=""
game_dir=""
skip_palettes=0
scenarios=""
while [ $# -gt 0 ]; do
    case "$1" in
        --out) out=$2; shift 2 ;;
        --exe) exe=$2; shift 2 ;;
        --game-dir) game_dir=$2; shift 2 ;;
        --skip-palettes) skip_palettes=1; shift ;;
        -h|--help) sed -n '2,17p' "$0"; exit 0 ;;
        -*) echo "unknown option $1" >&2; exit 2 ;;
        *) scenarios="$scenarios $1"; shift ;;
    esac
done

find_exe() {
    if [ -n "$exe" ]; then
        echo "$exe"
        return
    fi
    for build in release debug; do
        for name in nfsmw nfsmw.exe; do
            if [ -x "$REPO_ROOT/target/$build/$name" ]; then
                echo "$REPO_ROOT/target/$build/$name"
                return
            fi
        done
    done
    echo "no nfsmw binary: build one with \`cargo build --release -p nfsmw\`, or pass --exe" >&2
    exit 1
}

binary=$(find_exe)

if [ -z "$out" ]; then
    out="${TMPDIR:-/tmp}/nfsmw-hud-shots-$(date +%Y%m%d-%H%M%S)"
fi
created=0
if [ ! -d "$out" ]; then
    mkdir -p "$out"
    created=1
fi
out=$(cd "$out" && pwd -P)
case "$out/" in
    "$REPO_ROOT"/*)
        # Take back a folder made only for this run (rmdir leaves it if anything is in it).
        if [ "$created" -eq 1 ]; then
            rmdir "$out" 2>/dev/null || true
        fi
        echo "refusing to write screenshots inside the repository: $out" >&2
        exit 1
        ;;
esac

if [ -z "$scenarios" ]; then
    scenarios=$ALL_SCENARIOS
fi
for name in $scenarios; do
    case " $ALL_SCENARIOS " in
        *" $name "*) ;;
        *) echo "unknown scenario $name (expected: $ALL_SCENARIOS)" >&2; exit 2 ;;
    esac
done

failed=""

# One run: scenario, file suffix, size, then an optional console command.
shot() {
    shot_name=$1
    shot_suffix=$2
    shot_size=$3
    shot_exec=${4:-}
    shot_file="$out/${shot_name}_${shot_suffix}.png"
    echo "$shot_name $shot_suffix"
    # The place the fixtures are built around (devtools/hud_scenarios/common.rs), with no traffic to move the background.
    set -- view-world --drive --at 2150,1399 --no-traffic --ui-style custom --hud-scenario "$shot_name" \
        --screenshot "$shot_file" --screenshot-size "$shot_size" --screenshot-delay "$DELAY" --muted
    if [ -n "$shot_exec" ]; then
        set -- "$@" --exec "$shot_exec"
    fi
    if [ -n "$game_dir" ]; then
        set -- --game-dir "$game_dir" "$@"
    fi
    # Wait for the process: a screenshot run opens no visible window.
    if ! "$binary" "$@"; then
        failed="$failed $shot_name/$shot_suffix"
    fi
}

for name in $scenarios; do
    for size in $SIZES; do
        shot "$name" "$size" "$size"
    done
    shot "$name" high-contrast_1280x720 1280x720 "set custom_hud.high_contrast on"
    if [ "$skip_palettes" -eq 1 ]; then
        continue
    fi
    for palette in $PALETTES; do
        shot "$name" "${palette}_1280x720" 1280x720 "set custom_hud.palette $palette"
    done
done

echo "screenshots in $out"
if [ -n "$failed" ]; then
    echo "failed:$failed" >&2
    exit 1
fi
