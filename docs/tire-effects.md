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
| Smoke colour mode | `smoke_color_mode = "off"` | `NFSMW_SMOKE_COLOR_MODE=off`, `solid`, `rainbow`, `speed` or `slip` | (none) |
| Smoke colour | `smoke_color = "red"` or `"#ff8800"` | `NFSMW_SMOKE_COLOR=orange` or `#ff8800` | (none) |
| Smoke variant (a mod's) | `[mod_values.nfsmw] option_tire_smoke_variant = "mw2012"` | (none) | (none) |

In the F12 console, `get tire_smoke`, `set tire_smoke off`, `set skid_marks off`
and their `on` counterparts read or change the same settings. Changes last for the run.
`tire-effects smoke off` and `tire-effects marks off` are aliases for these settings.
`tire-effects` reports live counts, ages, totals and allocated GPU vertex capacities;
`tire-effects clear` removes current particles and marks while retaining reusable buffers.
`smoke_quality high` (or `set smoke_quality high`) enables denser, evolving procedural
puffs and depth-softened car/road/wall intersections immediately. `smoke_quality standard`
returns to the default. Changing quality clears smoke while retaining marks. The high
mode adds rendering work and does not simulate particle collisions or volumetric lighting.
Smoke Quality in the main and pause menus' Video options selects Standard or High
and saves the selection when leaving the settings screen.

## Colourful smoke

The smoke is white by default. **Smoke Color Mode** in the Video options (main and pause menus) and `smoke_color_mode` colour it,
and **Smoke Color** and `smoke_color` say with what. Both apply to the game's own smoke and to a mod's tire smoke variant (below)
and are saved with the other settings when the settings screen is left; they do not change the physics, the amount of smoke
or the number of particles.

| Mode | The smoke is |
|---|---|
| `off` (default) | as it was: the game's own grey white (or the variant's own colours) |
| `solid` | the chosen colour |
| `rainbow` | a hue that turns with time (0.35 turns a second) and along the ground (0.12 turns a metre), so a trail shows the colours one after another; the chosen colour is not used |
| `speed` | from its own colour to the chosen one as the car speeds up, reaching it at 60 m/s |
| `slip` | from its own colour to the chosen one as the wheel's smoke (its slip) goes from 0 to 1 |

The colour is one of the palette (`red`, `orange`, `yellow`, `green`, `cyan`, `blue`, `purple`, `pink`; the menu steps through
it) or any `#rrggbb` written in the config file (`smoke_color = "#ff8800"`; the menu then shows the hex value and steps on from the
nearest palette colour). In the console, `get smoke_color_mode`, `set smoke_color_mode rainbow`, `set smoke_color #00ff88` read
and change the same settings for the run (`smoke_color_mode rainbow` is a shorthand). The settings reach the scene in `WorldTuning::smoke_tint`, like the other
world settings. The colour is chosen per sprite while the
quads are built (`scenes/world/effects/tint.rs`, pure functions with tests): the game's smoke keeps one number per puff (the
speed or slip it was born with) and a variant's sprites use the car's speed or slip now, so nothing is allocated per particle.
The alpha is never touched, so the smoke fades as it did.

## Tire smoke variants

The gameplay settings have a **Tire smoke variant** option whose only value in the game is **Original**: the smoke described
above. A mod adds values to it ([effect-variants.md](modding/effect-variants.md)); `nfsmw-2012-content` adds **NFS Most Wanted
2012**, the smoke of the 2012 game's tarmac, converted from the player's own install ([chameleon-effects.md](formats/chameleon-effects.md#wheel-effects-tyre-smoke)).
With a variant chosen, the game's procedural puffs are not drawn: each layer of the variant's profile is an emitter at every
wheel that smokes, on the road under it, with the mod's own pictures, and the wheel's smoke (the physics' slip intensity, 0 to 1)
multiplies its spawn rate, so the **Tire Smoke** switch, wetness, the free camera and the reset still act as above. The particles
come from `blackbox-particles` (up to 128 live ones for each layer at each wheel, no allocation per particle, one textured batch
per layer), so the **Smoke Quality** setting does not change a variant. If the variant's files cannot be read the game's own smoke
is drawn.

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
