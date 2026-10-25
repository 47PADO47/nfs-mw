# Custom HUD: development scenarios, screenshots and measurements

Deterministic HUD states for checking the Custom HUD by eye and in screenshots before every gameplay system is in
reach, and the measurements the stack owes. Built in PR 12 (it depends only on PRs 3a, 3b and 4, and may land right
after PR 5). Index: [README.md](README.md).

## 1. `--hud-scenario`

A hidden command-line value, `--hud-scenario <name>`, next to the existing hidden `--hud-demo`
(`crates/nfsmw/src/cli/hud_demo.rs`), which it extends rather than duplicating: both build a `HudState`; the scenario
also builds the `HudEvent`s and a notification queue state, and pins the elapsed time.

- **Never on the production path.** The value is hidden from `--help`, only parsed by the CLI, and only read by the
  devtools that build the fixture. Without it nothing changes.
- **Labelled.** A fixture frame draws a small `SCENARIO: <name>` label in the top-left safe-area corner, so a
  screenshot of fixture data cannot be mistaken for gameplay.
- **Deterministic.** Fixed values and a fixed clock: the same scenario gives the same frame.

| Name | What it sets up |
|---|---|
| `free-roam` | cruising at 128 km/h in 4th, nitrous half full, no race, no pursuit, map Centered, one discovery chip |
| `race-intro` | a 3-lap circuit, 5 opponents, the intro card and countdown step 3 |
| `racing` | place 2 of 6, lap 2 of 3, progress 0.55, 4.2 km left, rival gap 1.42 s ahead at checkpoint 7 |
| `race-results` | a finished race handing off to the results screens |
| `pursuit-active` | heat 3, 2:14 in, two sighted cops, the helicopter in sight, a roadblock reported |
| `search` | contact lost 3 s ago, last-known marks of two cops at different ages |
| `cooldown` | the cool-down meter at 0.4, hiding at 0.6 |
| `escape` | the escape card, 4:05, bounty earned |
| `critical-damage` | rear-left tyre blown, front-right punctured with 12 s left, heavy body damage |
| `notify-preempt` | three Medium notifications visible, then a Critical arrives and displaces the oldest |
| `map-centered` | free roam with a GPS route and two turns ahead, map Centered |
| `map-expanded` | the same with the map Expanded |

The fixtures are synthetic numbers chosen to exercise the layout; no game data is copied into them.

## 2. The screenshot matrix

`tools/hud_shots.ps1` and `tools/hud_shots.sh` render every scenario offscreen and muted:

```text
nfsmw play --ui-style custom --hud-scenario <name> --screenshot <out> --muted
```

- **Resolutions:** 1280×720 (16:9), 2560×1080 (21:9), 5120×1440 (32:9).
- **Plus** a high-contrast pass (`--exec "set custom_hud.high_contrast on"`) at 1280×720, and one pass per colour-vision
  palette at 1280×720.
- Output goes to a temporary folder the script prints; nothing is written into the repository.
- The scripts **never take focus**: every run is offscreen (`--screenshot`), muted, and waits for the process, not for
  a window.
- The `.ps1` and `.sh` file types are checked against the `.gitignore` whitelist in PR 12.

## 3. Runtime checks (by the supervisor, offscreen and muted)

- `nfsmw play --ui-style custom --screenshot … --muted`.
- `view-world --drive … --exec "pursuit 3"` for the pursuit layout on real state.
- `--exec "race start <race>"` for the race layout.
- `view-screen … --options --category hud` for the HUD options page.
- Original and Remastered look identical before and after the stack (`compose_tests` and a screenshot diff).
- Restarting a race does not duplicate panels; the map keys do not move the car (a drive script plus the mapped key).

## 4. Measurements owed

Every number below is **to be measured** with the existing bench (`--bench-seconds`, `--offscreen`, `--muted`,
`--no-vsync`) and recorded here by the PR named. No improvement or cost is claimed before it is measured.

| Case | Owner | p50 | p99 |
|---|---|---|---|
| Original HUD, idle | PR 12 | to be measured | to be measured |
| Custom HUD, idle | PR 12 | to be measured | to be measured |
| Original HUD, driving | PR 12 | to be measured | to be measured |
| Custom HUD, driving | PR 12 | to be measured | to be measured |
| Custom, map off against centered, driving | PR 7b | to be measured | to be measured |
| Custom, 6 world markers on screen | PR 9 | to be measured | to be measured |

The map's own numbers are also in [map.md](map.md) section 9.
