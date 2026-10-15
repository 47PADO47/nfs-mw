# Time-of-day lighting

Status: **sun motion and fog colour implemented, built and tested in the running game** (world tests 230 pass; sun and fog unit tests 17 pass; `nfsmw-data` 69 pass; offscreen screenshot renders). Day-night clock (E1): the clock, the sun phase for the hour, the lights-on and lights-off switch, and their config (`crates/nfsmw-data/src/day_night.rs`, `crates/nfsmw/src/scenes/world/day.rs`) are implemented with unit tests, not yet checked against the original at a known hour. The per-frame sun integrator is gone; the clock sets the sun. The double `sun.advance` noted in the earlier pass was already removed on the current branch. Outstanding: the visual match of the scattering fog and lighting against the original game, which needs a reference frame. Out of scope for this pass and still open: the sky dome, the colour grade's time link, scalar `+0xA0`, and the live blend weight `t` (held at 0.0685).
world scene (`crates/nfsmw/src/scenes/world/sun.rs`). The night colours are specified too: in the states
checked, the preset blend follows a location value, not the clock, and the clock's counter is never written.
The one open question is who would write that counter in a state that runs it (see below).

Source: the retail PC executable `speed.exe` (Ghidra, `SPEED_EXE_1_3`), the installed
`GLOBAL/ATTRIBUTES.BIN`, and live checks in CheatEngine on a running free-roam game. No decompiled source is
copied (see [licensing.md](../licensing.md#spec-first)).

## Data

The attribute class `timeofdaylighting` (class hash `0x399ED882`) has **8 collections**, each with
**13 fields**. Field names are not in the install; only the hashes are known. The game loads them by name
hash, in this order (the lighting object's constructor, `FUN_00770D70`):

| Index | Collection hash |
|---|---|
| 0 | `0x3DCFA673` |
| 1 | `0xB902CFAD` |
| 2 | `0x59C373C3` |
| 3 | `0xD835D0C8` |
| 4 | `0xA01A3942` |
| 5 | `0x0E1C797C` |
| 6 | `0x94ABDFF0` |
| 7 | `0x919B6689` |

Fields are floats or `Vector4`s. A missing collection makes the preset unusable and the caller keeps the
existing lighting. The index-to-time mapping is **[unconfirmed]**.

## Lighting object

The lighting object (`DAT_009B392C`, 400 bytes, built by `FUN_00770D70`) is the state the frame reads. Offsets
in the object:

| Offset | Meaning |
|---|---|
| `+0x00` | Rate `rate`, 1.0 by default **[verified live]**. |
| `+0x04` | Direction `dir`, ±1 **[verified live: 1]**. |
| `+0x08` | Sun phase `p` **[verified live]**. |
| `+0x0C` | Angle offset `a`, 3.28 by default **[verified live]**. |
| `+0x10` | Angle offset `b`, 0.5 by default **[verified live]**. |
| `+0x160` | Blend weight `t` between preset pairs, set by `FUN_00769580`. The override `DAT_00905CE8` is never written, so it is inactive. |
| `+0x164` | Dusk factor `1 − (clamp(sunZ, 0.25, 0.5) − 0.25) · 4`, with `sunZ` at `+0x188`. |
| `+0x170..0x17C` | Sun vector before normalising, written by `FUN_00769360`. |
| `+0x180..0x188` | Normalised sun direction (`+0x188` is its height). Used for the car and scene lights. |

## Sun direction **[verified]**

`FUN_00769360` runs once per frame. It advances the phase, clamps it, and builds the sun. The result is
checked against the live game below.

1. **Phase limit:** `lo = 0.35` if the game state `DAT_00925E90` is 3, otherwise `0.05`. The phase stays in
   `[lo, 1 − lo]`.
2. **Integrator** (used in free roam, see the clock below). Per frame, with `dt` the frame time in seconds
   (`DAT_009259BC`, read live as `1/75 s`):

   `p ← p + dt · dir · rate · 0.05 / 60`

   If the new `p` is below `lo`, set `p = lo` and flip `dir`. If it is above `1 − lo`, set `p = 1 − lo` and flip
   `dir`. The phase therefore runs from `lo` to `1 − lo` in `0.9 · 60 / 0.05 = 1080 s` (18 minutes), then back.
3. **Vector:** with `u = (p − 0.5) · π`:

   `v = (sin u · cos a + sin a · sin b · cos u,  sin b · cos a · cos u + sin u · sin a,  cos u · cos b)`

4. **Direction:** `sun = v / |v|`. The normalisation is required: `|v|` is about 0.95 at the live sample, not 1.

Checked against the live game (`p = 0.1403`, `a = 3.28`, `b = 0.5`): the formula gives
`(0.916, −0.081, 0.394)` against the game's `(0.913, −0.085, 0.400)`, within the sampling error. The formula
was also derived by a symbolic emulation of the disassembly, which gives the same expressions.

The sine in the game uses 16-bit angles, one turn per 65536, so `sin` is quantised to 1/65536 of a turn. That
error is negligible for lighting and is not modelled.

The sun's height at noon (`p = 0.5`) is `cos b` in `Z`. With `b = 0.5` rad that is about 61° of elevation. At
the phase limits the elevation stays above about 8°, so this sun never sets. Night is therefore not this sun;
it is the preset blend, which is **[unconfirmed]** (see below).

## Clock **[verified branch, unconfirmed use]**

`FUN_00735620` chooses the phase source. It returns 1 when the flag `0x009B0FB9` is set or
`FUN_0078EBA0()` is true (`DAT_009B41FC` or `DAT_009B4220` ≥ 1). When it returns 0, the integrator runs.

When it returns 1, `p` is set from `FUN_00735A30`, a one-hour triangle:

- `t = (ticks mod 3600) · 1000 + base − offset`, in milliseconds, wrapped at 3 600 000
- `p = t / 1 800 000`, folded so that `p ≥ 1` becomes `2 − p`

The triangle is 0 at the hour's start, 1 at 30 minutes, and 0 at the end of the hour. The ticks are the
whole-second counter at `0x009B0FE4` (`+0x44` in the object at `0x009B0FA0`). `base` is `FUN_0078B2C0()` and
`offset` is `DAT_009B0FC0`, neither read yet.

Live checks in free roam: the flag `0x009B0FB9` and the two race counters are all 0, so the integrator runs
and the triangle is not used. The counter does not move even in race or pursuit (the user checked). So in
this install the triangle branch is not active in the states checked. Its gate is **[unconfirmed]** beyond that.

The implemented function `time_of_day::clock_phase` gives the triangle and is tested.

**Writer of the counter (found, Ghidra `speed.exe`).** The counter `0x009B0FE4` is the `+0x44` field of the
clock object at `0x009B0FA0`. Absolute references to `0x009B0FE4` are two reads (`FUN_0073C980`,
`FUN_0079FB60`) and no absolute write. The write is through the object's base:

- `FUN_00735960` (set-clock) takes `this` in `ECX` (`0x009B0FA0`), stores its arguments at `+0x30`, `+0x34`,
  `+0x40` (a float) and `+0x44` (the whole-second counter), then recomputes `+0x20` and `+0x28` from the base
  time `FUN_0078B2C0` and `FUN_007C4B80` (the `0x009B42A4` timer). The write to `+0x44` is at `0x007359A5`.
- Its callers, with `this = 0x009B0FA0`: `FUN_007974F0` (at `0x00797559`, the state dispatcher, on its
  state-6 path), `FUN_007A0320` (at `0x007A04E0`), `FUN_007920B0` (at `0x00792130`) and one call in an
  unnamed function at `0x0079F3C6`.
- In `FUN_007974F0` the counter value is the time of the current object (`[obj+0x38]`, looked up by the name
  hash `0x7374696D`) plus 2, through the timer at `0x0045CE40`. So the writer runs when the game state
  dispatcher is in state 6; the state values come from `DAT_009B41FC` and `DAT_009B4220` (writers listed in
  Ghidra).
- The gate `0x009B0FB9` has six absolute reads (`SampleRainIntensity`, `FUN_0073C980`, `FUN_0073CA50` and
  three others) and no absolute write. It may be written through a base, not checked.

So the counter is written when the state dispatcher runs state 6. Whether state 6 is a free-roam state is not
confirmed: the live checks in free roam recorded no change, so either state 6 is not free roam or the value is
not used there. This is the open point for a live watchpoint on `0x009B0FE4` during a state-6 entry.

## Day clock, pause and time scale **[implemented, synthetic tests]**

The day-night clock (`DayClock` in `crates/nfsmw/src/scenes/world/day.rs`) is advanced by the simulation delta,
not the real frame time. `WorldScene::update` calls `update_day(sim_dt)` after `update_simulation`, and
`sim_dt` is the same delta physics uses:

- **Speedbreaker:** `SpeedbreakerClock::advance` (`drive/time_scale.rs`) gives the dilated delta: 1.0 when
  inactive, ramping to the configured target (0.25 by default) at the entry and exit rates while active. The
  day runs at that rate during the breaker.
- **GameSpeed (decision):** `update_simulation` multiplies the dilated delta by `tuning.game_speed`
  (`simulation.rs`). So GameSpeed scales the day clock as well as physics, the traffic and the rain: one clock.
  Kept on purpose. `AnimationSpeed` (`self.clock`, `mod.rs`) is a separate multiplier for scenery animation and
  does not change the day.
- **Pause:** `Pausable` (`frontend/scene.rs`) returns before `WorldScene::update` while paused, so the clock is not
  advanced. Its existing test fails if the inner update runs.
- **Loading and garage:** a missing local world sets `sim_dt` to 0 (`simulation.rs`), and an open garage passes
  `dt = 0` (`mod.rs`). The clock holds in both.

The tests are in `day.rs`: a zero delta does not advance the day, normal speed is unscaled (150 simulated
seconds is one game hour), an active speedbreaker gives 15.140625 simulated seconds over 60 real seconds, and
GameSpeed scales linearly. These check the clock in isolation; they do not run `WorldScene` or the pause wrapper.
Nothing here is checked against the live game.

The triangle `clock_phase` (the `FUN_00735A30` branch above) is not on this path. The day clock uses
`nfsmw_data::day_night`, not that triangle. The blend weight is still held at 0.0685 (`lighting.rs`).

## Other time terms **[unconfirmed]**

- `FUN_0073C980` and `FUN_0073C9F0` use the same counter with a 503-second wrap and a sine in degrees
  (`0.714` degrees per second). They feed `FUN_0073CA50`, which the world update uses through the map target
  `FUN_00753AC0`. That path's inputs are not confirmed to be live.
- `DAT_009B0A50` is a 0–360 angle advanced by `FUN_00758100` at 45°/s, an 8-second loop. It is not the day
  cycle. Its use is not known.
- The world's eased value `+0x290` is the blend weight `t` for the presets (`FUN_006DE300` passes it to
  `FUN_00769580`). It eases towards a map target from `FUN_00753AC0`. Whether it is the time of day, a
  tunnel or zone factor, is **[unconfirmed]**.

## Night colours and the preset blend **[verified live]**

- The preset pair is chosen by `FUN_006C0A20`: it returns `DAT_00901828 != 0`, which was `1` in free roam, so the
  pair is presets 1 and 3 (`+0xD4` and `+0xFC`), with preset 2 (`+0xE8`) as the third input.
- The blend weight `t` at `+0x160` equals the world's eased value at `+0x290` (`0.0685` at the checked
  location, steady over six seconds). `+0x290` eases towards a map target from `FUN_00753AC0`, which depends on
  the car's position. So the preset blend follows the location (tunnels, zones), not the time of day.
- The time term that would make the presets follow the day is `FUN_0073CA50`, which reads the 503-second
  counter. It returns 1 when the counter path is off, so in free roam the time term is neutral.
- The counter path needs the gate `0x009B0FB9` or the mode counters `0x009B41FC`/`0x009B4220` ≥ 1. The mode
  counters are written by the game-state functions (values 1–7). The gate `0x009B0FB9` has no writer. In
  free roam and in the race and pursuit states checked, the counter `0x009B0FE4` never changed.
- **Conclusion for this build:** the daily change the player sees is the sun's motion (above) and the
  location-driven preset blend. The night presets are not reached by a clock here.

**Open (corrected):** the counter `0x009B0FE4` has a static writer, `FUN_00735960`, called on the state-6 path of
`FUN_007974F0` (see "Writer of the counter" above). What is still open is which game state is state 6 and whether
the free-roam sun should follow the counter there. The earlier statement that no writer exists is withdrawn.
The sun and the location blend do not depend on it.

## Application

Field names were matched by hashing candidate names with the attribute hash (`vlt_hash`). Matched colour fields:
`AmbientColour` `0x52194D72`, `DiffuseColour` `0x7623FEB2`, `SpecularColour` `0x5855B1E7`, `FogHazeColour`
`0xC91C60C7`, `FogSkyColour` `0x1D76EA96`. The eight scalar fields have no match yet.

**Preset block layout (decoded).** Each preset holder's block is read by `FUN_00768EE0` at fixed offsets, and
verified against the live blocks (preset 0 at `+0xC0`, values match the dump):

| Block offset | Colour | Name hash |
|---|---|---|
| `+0x00` | Specular | `0x5855B1E7` |
| `+0x10` | Diffuse | `0x7623FEB2` |
| `+0x20` | Ambient | `0x52194D72` |
| `+0x30` | Fog haze | `0xC91C60C7` |
| (lookup) | Fog sky | `0x1D76EA96` |

The lighting object's colour block (`+0x20` onward) receives the mix of the two blends: Diffuse at `+0x20`
(read by `FUN_006C6300`), Ambient at `+0x30` (`FUN_006C6450`), Specular at `+0x40`.

**Implemented: lighting and fog colour.** The lighting is the mix of two blends by the dusk factor:

- The dusk factor is the game's formula from the sun's height `z`: `1 − (clamp(z, 0.25, 0.5) − 0.25) · 4`
  **[verified in `FUN_00769580`]**.
- The low-sun blend is presets 1 and 3 blended by `t` (`FUN_00768EE0` with `+0xD4` and `+0xFC`) **[verified]**.
- The high-sun blend is presets 0 and 2 blended by `t` (`+0xC0` and `+0xE8`) **[verified]**.
- The mix is `high + (low − high) · dusk` (`FUN_00769190`) **[verified]**.
- The fog colour is the fog-haze colour of that mix. The fog runs towards the frame's clear colour, so the fog and
  background take it.
- `t` is held at 0.0685, the value of the world's eased `+0x290` at the checked spot **[assumption]**. Porting that
  eased value needs the map sample (`FUN_00753AC0`), not yet done.

Implemented in `time_of_day::lighting`, `time_of_day::fog_colour` and `WorldScene::fog_colour`.

**Fog and sky path (decoded).** `FUN_006D6CF0` sets the fog and sky shader state. Its flag `DAT_00901934` is `1` in
the checked game, so it takes the else branch, which reads the lighting object directly **[verified live]**:

| Lighting offset | Use | Record key (the same field, by name hash) |
|---|---|---|
| `+0x70` vec4 (`w` at `+0x7C`) | Sky colour, scaled by the distance term | `0xA450F56C` |
| `+0x80` vec4 (`w` at `+0x8C`) | Fog haze colour, scaled by the fog term | `0x2CF0B378` |
| `+0x90` × 0.01 | Distance term (sky side) | `0x555FD699` |
| `+0x94` × 0.01 | Distance term (fog side) | `0xC8BAF5D6` |
| `+0x98` | Integer flag | `0x2F70D78A` |
| `+0x9C` | Scalar, shapes the fog curve | `0xDBA22A95` |
| `+0xA0` | Scalar | not matched yet |

So the time-of-day block drives the fog and the sky, and the fog colour is the haze colour at `+0x80`, which is the
mix of the two blends (see above). The sky colour at `+0x70` is set by the same lighting update; which blend field feeds it is not yet confirmed.
The renderer's `FrameParams` takes only a clear colour and a fog start and end, so the depth curve and the sky-dome
colour are not yet mapped into it.

**Fog model (decoded).** The fog effect's constant names (`Fog_Br_Plus_Bm`, `Fog_One_Over_BrBm`, `Fog_Const_1..3`,
`Fog_DistanceScale`) are in the embedded effect blobs (`IDI_WORLD_FX` and others, found by the `fx_2_0` magic).
They describe atmospheric scattering: Rayleigh (`Br`, from the sky colour `+0x70` times the distance `+0x90·0.01`)
and Mie (`Bm`, from the haze colour `+0x80` times the distance `+0x94·0.01`), with `g` at `+0x9C`. The game
scales the distances as `field·0.01·0.0099 + 0.0001` **[verified: the constants and the `FUN_006D6CF0` arithmetic]**.
With `E = exp(−(Br+Bm)·d)` and `In = (Br·3/(16π)·(1+c²) + Bm·HG(c))·(1−E)/(Br+Bm)`, the pixel is `scene·E + In`.
Implemented in `crates/nfsmw-data/src/fog.rs`, with tests. The Henyey-Greenstein term uses the `(1 − g)²`
form the game's constants suggest; the exact normalisation is **[unconfirmed]** against the shader.

**Renderer (wired, not visually verified).** `FrameParams` has an optional `Scattering` (Rayleigh, Mie, `g`, the
direction to the sun). The shared `Globals` grew to 176 bytes with those fields. The scene and glossy shaders use
`scatter_fog` when the Rayleigh `w` flag is set, and the linear fog otherwise. The effect and soft-particle shaders
still use the linear fog. The world frame sets the scattering each frame from the lighting. The three scalars
(`+0x90` = 3.0, `+0x94` = 2.27, `+0x9C` = −0.47) are held at their observed values. The headless shader tests pass
(one clear-colour test is off by one LSB, not on a fogged path). The game was not run for a visual check, and the
`nfsmw` crate does not build at the time of writing because of separate in-progress edits to the car lamp code.

**Surface lighting (wired).** The world surfaces take the lighting's ambient colour (`+0x30`) and diffuse colour
(`+0x20`), mixed by the dusk factor. `FrameParams.surface_light` carries them, and the shared `Globals` (now 208 bytes)
passes them to `scene.wgsl`. Without them the shader keeps the fixed 0.35 ambient and 0.65 diffuse. The specular
colour (`+0x40`) is not applied: the world surfaces have no specular term, and the game's specular belongs to the
car's glossy parts, which use their own per-car rig. The car rig is not changed. A screenshot renders; the match
with the original game is not verified.

**Findings for the three open items (checked in the embedded effects and `FUN_006D6CF0`).**

- **Sky dome (item 1).** The `SKYDOME` scenery uses effect 0, which has only the texture, the transform and the
  blend state: no fog and no sky constant. So the dome's colour is its texture times its vertex colour, not the
  lighting, in the original too. The lighting-driven sky colour is `FFSKYCOLOR` and `SUNCOLOR`, in the skybox
  effects (blobs 14, 17 and 18). Those are the sky model paths (`SkyRender`), which this renderer does not run.
  Driving them needs the skybox shader's semantics and the sky-model path, which is not implemented.
- **Colour grade (item 2).** The grade is the visual-treatment effect (blob 21): `ColourBloomTint`, `Desaturation`,
  `BlackBloomIntensity`, `ColourBloomIntensity`, `GAINMAP` and the visual-effect brightness, radial blur and vignette.
  Its values come from the `visuallook` attribute record. No read of the lighting object was found in the paths
  checked, so there is no verified time-of-day link yet.
- **Scalar `+0xA0` (item 3).** The record branch of `FUN_006D6CF0` does not read `+0xA0`; it uses 1.0 there.
  The names the effects use for brightness-like values did not match any hash. The attribute names are not stored
  in `speed.exe` (only the hashes), so matching `+0xA0` needs an external name list.

**Not implemented:**

- **Sky.** The sky is scenery models (`SKYDOME`, `SKYDOME_XENON`) with textures in `InGameA.bun`. Their colour is
  not set by the lighting object, so there is no verified sky-colour path yet. The horizon colour follows the fog.
  - **E1 sky-colour TODO, status:** the shared models of `InGameA.bun` now load (`GLOBAL_MODEL_FILES`), so the
    sky domes can resolve. The dome colour is still not tied to the time of day: `SKYDOME` uses effect 0, which
    takes no lighting (see above), and the lighting-driven sky colour is `FFSKYCOLOR` / `SUNCOLOR` in the skybox
    effects. The code that writes those from the lighting object is not found yet (Ghidra, `speed.exe`:
    `FUN_006C3D20` registers the names, `FUN_006D6000` sets up the 31 effect slots). Open.
- **Colour grade.** The grade is `visuallook` `default` (`ColourBloomTint`, curves). It is not driven by the time
  of day in any state checked, and the time-of-day blend does not reach it yet.
- **Scalar fields** (fog distance, sun intensity and similar). Their names are not matched, so fog distances stay
  as the world's fixed values.
- **Ambient, diffuse and specular colours.** Matched by name but not yet connected to the renderer's light model.

## Implementation plan

1. **Sun daily motion** (done, now clock-driven). `Sun` in `crates/nfsmw/src/scenes/world/sun.rs` holds the phase and
   the direction and feeds the scene light and the drive car's shading. Each frame `WorldScene::update_day`
   (`crates/nfsmw/src/scenes/world/day.rs`) advances the clock and sets the phase with
   `nfsmw_data::day_night::sun_phase`: sunrise `06:00` to sunset `18:00` runs the phase from the low limit to the
   high limit, and the night holds the limit reached. Default start is noon (`12:00`) and one game hour is 150
   real seconds (our choice, not an install value). Traffic cars still use the noon sun. Reflection captures follow the active world sun and lighting.
2. **Day-night clock and lights** (done, own config). `DayNightConfig` in `crates/nfsmw-data/src/day_night.rs`
   (serde, defaults above): `force_time` (`ForceTime`), `real_time` and `random_startup_time` (read, not applied),
   `sun_rise`, `sun_set`, `lights_on`, `lights_off`, `sun_azimuth` (the orbit's `+0x0C` angle `a`, our mapping).
   The player's headlights switch at `lights_on` and `lights_off` (`Drive::set_headlights`); a manual toggle holds
   until the next switch. World lights and the sky colour are not driven yet.
   Not yet wired: loading the config from the settings file (the defaults are used).
3. **Preset blend** (done as specified, in the location form). `time_of_day::blend` with `t` from the world's
   `+0x290` value. Not yet applied to the renderer's colours.
4. **Application** of the blended presets to fog, sky and grade (not yet started).

Implemented in `crates/nfsmw-data/src/time_of_day.rs`: `clock_phase`, `sun_phase_limit`,
`advance_sun_phase`, `sun_direction`, plus the preset loader and blend. Tests cover the noon sun, the live
sample, unit length, the bounces, and the 18-minute half cycle.

## Verification

- Unit tests on the formula, the integrator and the clock.
- The live sample above, from CheatEngine on the running game (`p`, the sun vector, `a`, `b`).
- Not yet verified: the sun's look against a screenshot of the game at a known phase, and the night.

## Session controls

F8 reads and writes the active world clock through the scene interface, including
normal Play's pause wrapper. Manual hour/rate changes rebase elapsed time and
clear a forced hour, keeping the requested current hour when the rate changes.
Freeze time shares Freeze sun; resumed time does not catch up. Elapsed simulated
seconds accumulate in double precision for stable high-frame-rate clock motion.
Manual changes refresh player light state and invalidate reflection captures.
Clear/Rain and rain amount forward through live settings and replacement scenes.
These controls expose the existing custom host curves; they do not complete the
unimplemented sky, world light or regional weather behavior described above.
