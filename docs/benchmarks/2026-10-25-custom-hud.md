# Benchmark: the Custom HUD 2026 against Original and Remastered, 2026-10-25

The Custom HUD rewrite ([ADR 0013](../decisions/0013-custom-hud.md)) adds a centred road map, layouts, markers and a
notification stack. This compares the frame time with each HUD look on the same scene.

## Method

- Release build of the Custom HUD stack's top (`hud/p11c-worldmap`), RTX 4070 SUPER, Vulkan, Windows 10.
- The scene: the `map-centered` HUD scenario (free roam, a GPS route, the Custom map Centered), so every look draws a
  full HUD over the same car and place. Each run is hidden, unfocused and `--muted`:

```sh
nfsmw view-world --drive --at 2150,1399 --no-traffic --ui-style <original|custom|remastered> \
  --hud-scenario map-centered --bench-seconds 20 --no-vsync --max-fps unlocked --offscreen --muted
```

## Results

| Run | Look | p50 ms | p95 ms | p99 ms | worst ms | CPU ms/frame |
|---|---|---|---|---|---|---|
| 1 | Original | 8.13 | 16.64 | 19.60 | 106.53 | 3.87 |
| 1 | Custom | 8.14 | 22.54 | 104.62 | 106.86 | 4.73 |
| 1 | Remastered | 8.43 | 11.21 | 13.54 | 29.11 | 5.07 |
| 2 | Custom | 8.14 | 10.53 | 11.25 | 29.84 | 4.72 |
| 2 | Original | 14.54 | 30.66 | 33.70 | 46.48 | 5.04 |
| 3 | Custom | 12.41 | 20.10 | 46.18 | 94.84 | 6.06 |
| 3 | Original | 10.58 | 16.47 | 17.24 | 37.21 | 1.34 |

## Reading

- **Inconclusive beyond "no large regression".** The PC was not idle (the same look swings from 8 to 14 ms p50
  between runs), so the tails are noise. In the quiet runs Custom and Original share a p50 of 8.1 ms, as the
  renderer, not the HUD, sets the frame.
- **CPU:** Custom's UI work is in the same range as Remastered's (about 4.7 against 5.1 ms of CPU per frame for the
  whole process in run 1). The map's roads are re-queried only past the move or zoom threshold
  ([custom-hud-map-impl.md](../specs/custom-hud-map-impl.md)), so a still car costs no rebuild.
- **To do:** rerun on an idle PC, and a driving run (`--drive-script`) so the map's re-query and the markers' rays are
  exercised while moving.
