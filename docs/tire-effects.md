# Tire effects

While driving, each loaded wheel uses the vehicle library's existing `smoke` and `skid`
outputs. A short projection onto resident collision faces anchors the effect to the road.
No new tire physics or slip thresholds are introduced. The visual appearance is procedural
new design, described in [the spec](specs/tire-effects.md); no original game textures are copied.

## Controls

Both effects default on. Settings resolve independently: CLI, environment, per-user TOML,
then defaults, just like vsync and HUD settings.

Video options in the main menu and pause menu include Tire Smoke and Skid Marks.
Changing either applies it to the driving scene, including while paused. Leaving
the settings screen saves the choice through the existing config writer.

| Effect | Config key | Environment | CLI overrides |
|---|---|---|---|
| Smoke | `tire_smoke = true` | `NFSMW_TIRE_SMOKE=on` or `off` | `--tire-smoke` / `--no-tire-smoke` |
| Marks | `skid_marks = true` | `NFSMW_SKID_MARKS=on` or `off` | `--skid-marks` / `--no-skid-marks` |
| Smoke quality | `smoke_quality = "standard"` | `NFSMW_SMOKE_QUALITY=standard` or `high` | `--smoke-quality standard` / `--smoke-quality high` |

In the F12 console, `get tire_smoke`, `set tire_smoke off`, `set skid_marks off`
and their `on` counterparts read or change the same settings. Changes last for the run.
`tire-effects smoke off` and `tire-effects marks off` are aliases for these settings.
`tire-effects` reports live counts, ages, totals and allocated GPU vertex capacities;
`tire-effects clear` removes current particles and marks while retaining reusable buffers.
`smoke_quality high` (or `set smoke_quality high`) enables denser, evolving procedural
puffs and depth-softened car/road/wall intersections immediately. `smoke_quality standard`
returns to the default. Changing quality clears smoke while retaining marks. The high
mode adds rendering work and does not simulate particle collisions or volumetric lighting.
Current main has no graphics settings menu; these existing configuration paths are used.

## Lifetime and limits

Smoke fades over 1.4–2.0 seconds, rises and expands, and is sorted for transparency.
At most 512 particles are live in Standard, or 1536 in High. Marks have a 45-second lifetime, fading in the final
5 seconds, and a limit of 2048 quads. Stationary wheelspin refreshes one grounded stamp
per wheel rather than stacking geometry each tick. The normal offset and reverse-Z depth
bias reduce fighting with the road; neither effect writes depth or modifies the HUD pass.

Lift-off, teleport/reset, large contact gaps and sharp normal changes break track continuity.
Marks disappear when their collision section unloads. Free camera parks the car and lets
existing effects age without emitting. Disabling an effect clears its history immediately;
reenabling it starts fresh. GPU buffer capacities stop growing at the history budgets and
are retained until the renderer is dropped.

Effect origins and mark widths use the displayed tread bounds, steering and suspension,
then project onto the road. Physics wheel load and slip still determine emission. This
corrects the former BMW physics-to-mesh offset (about 11-12 cm in the tested install).
Stationary stamps follow small wheel movements and steering. Completed track segments
stay on the ground. Fixed-step trails may still show a small difference from the
interpolated car pose at speed; exact original-game visual parity remains unverified.

## Reproducible checks

These commands read assets from your install and write local PNGs. `--drive-script` is a
hidden development option. Screenshot runs execute fixed simulation steps and freeze the
result during capture settling; they do not measure real-time particle expiry or performance.

```sh
nfsmw view-world --drive --drive-script "2" --screenshot idle.png
nfsmw view-world --drive --drive-script "3:throttle=1" --screenshot wheelspin.png
nfsmw view-world --drive --smoke-quality high --drive-script "3:throttle=1" --screenshot high-smoke.png
nfsmw view-world --drive --drive-script "3:throttle=1;1.5:throttle=0.6,steer=0.7,handbrake=1" --screenshot marks.png
nfsmw view-world --drive --drive-script "3:throttle=1" --no-tire-smoke --no-skid-marks --screenshot disabled.png
nfsmw view-world --drive --drive-script "2:throttle=1;5" --screenshot expired-smoke.png
nfsmw view-world --drive --drive-script "2:throttle=1;0.1:reset;2" --screenshot reset.png
```

The logs include per-second and final counts. Repeat a script to compare the fixed-step
vehicle trace and effects totals. Compare enabled/disabled captures at the same ending
pose. For a live check, launch with a longer throttle script, press F to park in free camera,
then use F12 and `tire-effects` after a few seconds: smoke reaches zero. After 45 seconds,
marks also expire. Toggle both settings and clear them while watching the HUD and console.
Use `--backend dx12` or `--backend vulkan` to check the render pipelines separately.

Synthetic tests cover contact/exclusion projection, quiet rolling, airborne wheels, bounded
stationary and moving emission, expiry, unloading, reset/teleport discontinuities, settings
precedence and deterministic geometry. Visual matching to the original game, surface/weather
appearance and original effect textures remain open work.
