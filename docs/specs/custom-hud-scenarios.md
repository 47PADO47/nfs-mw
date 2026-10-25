# Custom HUD: deterministic scenarios and the screenshot matrix

How to put the HUD in a known state and how to render the Custom HUD's screenshot matrix. The design is section 1 and 2
of the Custom HUD spec's `custom-hud/dev-scenarios.md` (PR 1); this page is the usage. Code: `crates/nfsmw/src/devtools/hud_scenarios/`.

## 1. `--hud-scenario <name>`

A hidden command-line value of every viewer, next to the hidden `--hud-demo` it extends (`cli/hud_demo.rs`). It names
a **development fixture**: a `HudState` with its events, built from synthetic numbers chosen to exercise the layout. No
game data is copied into it.

```text
nfsmw view-world --drive --at 2150,1399 --no-traffic --ui-style custom --hud-scenario cooldown
```

- **Off unless given.** Without the flag no `ActiveScenario` resource exists, the system that writes the fixture never
  runs, the label is not drawn and the settings are not touched. The production path carries only the injection points:
  the run option (`app::RunOptions::hud_scenario`), the system's registration in the HUD plugin, and the label's call in
  the UI pass.
- **Labelled.** Every frame drawn from a fixture shows `SCENARIO: <name>` in the top-left corner, so a screenshot of
  fixture data is never taken for gameplay.
- **Deterministic.** The values are fixed, the world markers project through the fixture's own camera (behind the car,
  looking along +x from 2150, 1399, with the ground at z = 0), and the events are stepped by the frame count since the
  HUD came up. The same scenario gives the same state on the same frame. The HUD's clock is not pinned: fades and
  notice lifetimes run in real time, which the matrix's 1.5 s capture delay leaves room for.
- **Over any scene.** The fixture replaces the scene's state each frame, after the HUD's sync and before the
  notifications read the events, so it shows over a world or any viewer alike. The player's unit and map
  orientation still apply.
- **Settings.** A run starts with the HUD on, km/h, the map north up and the `[custom_hud]` options at their defaults,
  with the scenario's map mode on top, whatever the config file says. `--exec` commands change them after that (the
  high-contrast and palette passes below use it). `--hud-scenario` and `--hud-demo` do not mix.

| Name | What it sets up |
|---|---|
| `free-roam` | 128 km/h in 4th, nitrous half full, no race or chase, an undiscovered race start and a safe house on the map |
| `race-intro` | a 3-lap circuit of six, on the grid in 6th, countdown at 3, gate 1 of 24 next |
| `racing` | 2nd of six on lap 2 of 3, 55 % done, 4.2 km left, gate 15 of 24 next, the leader 1.42 s ahead; checkpoint and place events |
| `race-results` | over the line in 2nd, every gate passed, the leader 0.86 s ahead at the finish |
| `pursuit-active` | heat 3, 2:14 in, two cops in sight, the helicopter with the player in sight, a roadblock reported |
| `search` | contact lost 3 s ago, the last-known place, the helicopter flying blind |
| `cooldown` | the cool-down meter at 0.4, a hiding spot latched to 0.6 |
| `escape` | got away at 4:05, the bounty banked |
| `critical-damage` | rear-left tyre blown, front-right punctured, heavy body damage, still driving |
| `notify-preempt` | a Low notice (bounty) and two Medium ones (a checkpoint and a lap) up; on frame 30 a Critical one (roadblock) displaces the Low |
| `map-centered` | free roam with a GPS route, left in 300 m then right, the map Centered |
| `map-expanded` | the same with the map Expanded |

What the state cannot say is left out rather than invented: the puncture's time to the blow-out is not in the HUD state,
and the police have one last-known place, not one per cop.

## 2. The screenshot matrix

`tools/hud_shots.ps1` (PowerShell) and `tools/hud_shots.sh` (POSIX shell) render every scenario:

```text
pwsh tools/hud_shots.ps1 [-OutDir DIR] [-Exe PATH] [-GameDir PATH] [-Scenario a,b] [-SkipPalettes]
sh tools/hud_shots.sh [--out DIR] [--exe PATH] [--game-dir PATH] [--skip-palettes] [SCENARIO...]
```

Each shot is one run of:

```text
nfsmw view-world --drive --at 2150,1399 --no-traffic --ui-style custom --hud-scenario <name> \
  --screenshot <out>/<name>_<pass>.png --screenshot-size <WxH> --screenshot-delay 1.5 --muted [--exec "<command>"]
```

- `view-world --drive` is the lightest scene with a car on the road: no boot flow or menus, and the HUD on by default.
  The car starts where the fixtures are built (`devtools/hud_scenarios/common.rs`), with no traffic to move the picture.
- **Passes:** 1280x720 (16:9), 2560x1080 (21:9) and 5120x1440 (32:9); a high-contrast pass at 1280x720
  (`--exec "set custom_hud.high_contrast on"`); one pass per colour-vision palette at 1280x720
  (`set custom_hud.palette deuteranopia`, `tritanopia`). The console's startup commands run on the first frames, before
  the screenshot, which waits for the scene to be ready and then for the delay.
- **Output:** a new `nfsmw-hud-shots-<time>` folder in the temporary directory unless one is given. A folder inside the
  repository is refused. The files are named `<scenario>_<pass>.png`, for example `cooldown_2560x1080.png` and
  `racing_high-contrast_1280x720.png`.
- **Never takes focus.** Every run is a screenshot run (`--screenshot`): the window is hidden, unfocused and placed off
  the desktop (`app/mod.rs`), and the game is muted. The scripts wait for each process, never for a window.
- **Binary:** `target/release/nfsmw(.exe)`, else the debug one, unless `-Exe`/`--exe` names another. The scripts do not
  build it.
- A failed run is reported at the end and the script exits 1; the other shots are still taken.

## 3. Adding a scenario

1. Add the variant to `HudScenario`, its name to `name` and `ALL`, and its builder to `fixture` (one file per group).
2. Give it a map mode in `map_mode` if it needs one, and steps in `events` if its events change after the first frame.
3. Add its name to the two scripts' lists and to the table above, and a test of its key fields in `tests.rs`.
