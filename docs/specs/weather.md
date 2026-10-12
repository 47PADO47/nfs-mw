# Weather (rain intensity, wind)

- **Sources read:** `speed.exe` (original PC build, image base 0x400000), decompiled in Ghidra and read as
  raw disassembly where Ghidra had no function. Read-only; no game data copied.
- **Status:** static analysis only. Behaviour below is from the decompiled code. Runtime confirmation
  (Cheat Engine or Ghidra debugger against a running race) is still pending and is listed under
  *How to check it*.
- **Data inputs:** script-callable `SetChanceOfRain`; game-state value `DAT_00925e90`; frame time
  `DAT_00925970`; per-object weather block at `+0x28c` onward.

## Behaviour

### Entry points

- **Script hook.** `SetChanceOfRain(p)` is registered by the script runtime (`FUN_0061e750`, name
  `0x8a4a98`, handler `0x6054a0`, renamed `SetChanceOfRain_clamp`). It clamps `p` to [0, 1] and writes
  global `g_RainOverride` (`0x9b0a48`, via `SetRainOverride` at `0x73c970`):

  ```
  p = clamp(p, 0, 1)
  g_RainOverride = (p >= 0.01) ? 0 : -1
  ```

  As decompiled, the function never stores `p` itself, so it can only switch between two values:
  `0` (no override) and `-1` (forced dry). Treat this as the main thing to verify at runtime.

- **Per-frame update.** `UpdateWeather` (`0x758100`, one caller `FUN_006de300`) runs once per frame.
  It only does work when `DAT_00925e90 == 6`. A second branch (`FUN_0064b680`) takes a different path
  and returns early; its meaning is not yet identified.

### Rain target

`SampleRainIntensity(x, z)` (`0x7539d0`) returns the rain target at a world position, using the
player's position from the vehicle block (`+0x40`, `+0x44`):

1. If `DAT_009b0fb9 == 0` and `FUN_0078eba0()` is false, and `g_RainOverride != 0`:
   - if override > 0, return it;
   - if override < 0, return 0.
2. Otherwise `r = FUN_0073c9f0()`. When the gate from step 1 is true, `r` is squared.
3. Look up the zone value at `(x, z)` in table `DAT_009b2a9c` via `FUN_0074b5c0`. If `r <= zone`,
   return the zone value; otherwise return 0.

**Base roll `FUN_0073c9f0`:** `r = max(0, 1 − |cos(a)|)`, where `a` is a 16-bit angle index.

- **Input.** `FUN_0073c980` produces a time-like value `t` in seconds (gate on), or `1.0` (gate off):
  `t = ((A + 1000·(X mod 503) − B) mod 500000) · 0.001`, where `X` is the counter at `0x9b0fe4`, `A` is
  the result of `FUN_0078b2c0`, and `B` is the value at `0x9b0fc0`. So `t` runs over 0 to 500 seconds
  and wraps. The input comes from the FPU stack, which `FUN_0073c980` leaves holding `t`. `FUN_007c4b80`
  is a generic float-to-integer conversion, used by about 430 functions, not a clock.
- **Angle.** `n = round(t)` (whole seconds). The index is `(n / 360) as u16`, so it is 0 for `n < 360`
  and 1 for `360 ≤ n ≤ 500`.
- **Cosine.** `FUN_0045dbf0` adds a quarter turn (`0x4000`) to the index, and `FUN_0045db30` is a sine of
  a 16-bit angle (its scale `9.58738e-5` is 2π/65536). So `c = cos(index · 2π/65536)`.
- **Result (contradicted by observation, see below).** `c ≈ 1` for both indices, so `r ≈ 0` for every input. The roll is effectively zero: any zone
  whose compare value is non-negative passes, and the zone map alone sets the rain. With the squared roll
  (gate on) it is smaller still. The Rust model implements this as `weather::base_roll`.

This is a static derivation from the decompiled code. The counter `X`, the values `A` and `B`, and the
gate's meaning are not yet confirmed in the running game.

**Zone table `DAT_009b2a9c`:** a 2-D sampled map. `FUN_0074b5c0(map, x, z)` reads a sample at the
position and returns `scale × sample + offset`, with the scale and offset held in the map object
(`+0x1c` and `+0x14`). Rain targets therefore vary by place. A second map at `DAT_009b0c48` uses the
same lookup and feeds the wind-like value in `FUN_00753ac0`, which falls back to the floor at
`0x9b0a3c` when its sample is zero.

Inside `UpdateWeather`:

- `+0x3694` (rain target) = `SampleRainIntensity(pos)`, multiplied by `0.125` when `FUN_006bf530() > 2`.
  The value `FUN_006bf530` is not yet identified; the factor is probably tied to player count or
  split-screen mode.
- `+0x3698` (second target) = `SampleRainIntensity(pos)`; if that is 0, it falls back to `FUN_00753ac0`.

### Easing

Each target is followed by a first-order lag, with one constant per value:

```
dt   = DAT_00925970            (0.001 when zero)
k    = 1 / dt
rain += (target - rain) / (k * 1.0)        // 0x904ac4 = 1.0
```

- Rain (`+0x28c`): `k` uses `0x904ac4 = 1.0`. Clamped to 1.0. Snapped to 0 when the target is 0
  and rain < 0.01.
- Second value (`+0x290`): uses `0x904ac8 = 10.0`. Clamped to 1.0. Snapped to the floor
  `0x9b0a3c` when its target is 0 and the value is under floor + 0.01.
- Ramp used by `+0x369c` / `+0x36a0`: moves by `(target - current) * 0.005` per frame. A value
  under 0.05 is set to 0. The "on" target is `0x904b38 = 1.0`.

### Wind-like drift

`g_WindHeading` (`0x9b0a50`) advances by `dt * 45.0` (`0x904aec = 45.0`) per frame and resets to 0
when it passes 360. This looks like a slowly turning direction. Whether it drives the visual wind is
not yet shown.

### Applied state

- `ApplyRainIntensity` (`0x73cc80`) stores the eased rain value at `+0x28c`. It also sets a timer
  at `+0x33e8`, a wrap value `0x15e - timer` at `+0x294`, and `1 - x` at `+0x33f0`. `0x15e` is 350.
  The meaning of the timer is not yet identified.
- `UpdateRainEmitter` (`0x757770`, about 2.4 KB) runs when `DAT_00925e90 == 6` and `DAT_009b085c == 0`.
  It builds an emitter frame from the player's vehicle: a position from `+0x3610` (near the vehicle),
  a camera-relative direction, and a wind offset. It then steps the drop simulation. A fade factor
  `1 - (1 - 30 * dt)^3`-style eases the emitter in over a few frames.
- Counters at `+0x33dc` to `+0x33e8` track how many drops are in each state. Their exact role is
  not yet confirmed.

### Drops

The drop pool is 200 entries of 0x20 bytes at `+0x3880`. Each entry has a state byte:

- **0 (falling):** `SpawnRainDrop` (`0x74a320`) placed it in a box around the emitter, then
  `UpdateRainDrops` (`0x74a5d0`) integrates it each frame:

  ```
  wind   = (wind_x, wind_z) * intensity_scale           // +0x3480, +0x3488, scaled by +0x33f8
  pos.x += wind.x + drift_x + speed_x * k
  pos.y += ...fall...   + speed_y
  pos.z += wind.z + drift_z + speed_z * k
  if pos.z < ground_low (+0x3868) or pos.z > ground_high (+0x3838):
      recycle (FUN_0074a490)
  ```

  The vertical speed uses `+0x3690` (eased rain) plus `+0x3424`. A horizontal velocity is
  added from the emitter's `+0x3420` term, which is the drop's own speed scale.
- **1 and 2:** the code sets these on drops that leave the box or hit the ground, and reuses the
  slot. The exact meaning of state 1 (likely impact or splash) and state 2 (likely free) is inferred,
  not confirmed.

The code only spawns drops while the rain value is above zero. The rate at which a dry track empties
the pool is not yet measured.

### Lightning and thunder

**Scope decision:** this spec covers rain and wind only. The original build has no lightning or
thunder that could be found; this was confirmed by the project owner. The evidence checked:

- **Executable text:** no `thund`, `lightn`, `storm` or `bolt` strings in `speed.exe` (case-insensitive
  scan of every initialised block).
- **Executable name hashes:** the engine's name hash (`bstring_hash`, see `libs/blackbox-hash`) of
  `THUNDER`, `LIGHTNING`, `THUNDERSTORM`, `STORM`, `RAIN`, `WEATHER`, `WIND` and `CLOUDS` does not appear
  in `speed.exe`. `FLASH` (`0x02C856ED`) appears three times in `.text`, which is probably the
  screen-flash effect.
- **Install data files:** a scan of 715 `.BIN`, `.BIG`, `.exe` and `.dll` files for the same hashes
  found one `LIGHTNING` hash in `GLOBAL\RIVALS.BIN` and two `WIND` hashes in `SOUND\SPEECH\copspeech.big`.
  Each 32-bit match is about a 25% chance across this much data, so these are not evidence of
  lightning. The `RIVALS.BIN` hit should be checked by hand before it counts.

So: no thunder or lightning logic has been found, and it is out of scope for this spec. The strings
`FX_Weather`, `FX_WIND_Weather`, `CARSFX_WindWeather` and `AnytimeEvents_WeatherReport` point to the
sound and event side, which was checked as far as the data tables go; their cross-references are data
tables, not code.

### Settings and effects

Rain options come from the registry (`FUN_006c1c00` reads, `FUN_006c1f10` writes). The strings
`g_RainEnable`, `DORainEnable`, `ENISRain`, `CARSFX_Rain` and `RAINDROP*` belong to the effects and
options side. They are not part of the weather state machine above.

## Constants

Read from the program image:

| Address | Value | Use |
|---|---|---|
| `0x904ac4` | 1.0 | rain ease rate |
| `0x904ac8` | 10.0 | second-value ease rate |
| `0x904aec` | 45.0 | heading drift (degrees per second) |
| `0x904b38` | 1.0 | ramp target when raining |
| `0x89096c` | 1.0 | clamp maximum (`SetChanceOfRain`) |
| `0x890dc4` | 0.01 | dry threshold (`SetChanceOfRain`) |
| `0x9b0a3c` | (floor) | second-value floor, read at run time |

Not yet read: `0x9b0a3c`, `0x890dc4`, `0x890968` (0.0 clamp minimum, assumed).

## Runtime check (Cheat Engine, one race, 2026-10-12)

Read from the running `speed.exe` process that was in a race (game state `0x925e90` = 6). The PE header at
the image base reads `MZ`, so the addresses match the static analysis.

| Global | Read | Meaning confirmed |
|---|---|---|
| `0x9b0a48` (`g_RainOverride`) | 0 | no forced-dry override in effect |
| `0x9b0fb9` (gate byte) | 0 | gate off, so the input is the value `FUN_0073c980` returns with the gate off |
| `0x9b0a50` (heading) | 23.25 and moving | the heading timer drifts as the spec describes |
| `0x9b2a9c` (zone map) | valid heap pointer | the zone map is loaded |
| `0x9b0c48` (wind map) | valid heap pointer | the second map is loaded |
| `0x9b0fe4` (counter `X`) | 0 | not yet confirmed as a running counter (one sample) |
| `0x9b0fc0` (`B`) | 0 | not yet confirmed as a running value (one sample) |
| `0x9b0a3c` (wind floor) | 0 | not what the spec assumed; value not yet explained |
| `0x925970` (frame time) | 0 | a single sample reads zero; not confirmed |

Follow-up reads (same race, several samples a few seconds apart):

- **Value `A`** is confirmed. `FUN_0078b2c0` returns `GetTickCount()` minus the start value at `0x9b40d8`
  (read as 316295265 ms). So `A` is milliseconds since the game started, and it advances in real time.
- **Counter `X` (`0x9b0fe4`) and `B` (`0x9b0fc0`)** read 0 at every sample. With both zero, the time-like
  input is `t = (A mod 500000) / 1000` seconds, so it runs over 0 to 500 s and wraps.
- **Frame time (`0x925970`)** reads 0.00417 s (about 240 fps) once the game is running. Earlier zeros came
  from the debugger holding the process paused.
- **Heading** advanced from 23.25 to 90.0 and on to about 337 within a few seconds, matching the drift.
- **Base roll** is not read directly. The derivation in the base-roll section still holds: with `t` in
  whole seconds, the index is 0 or 1, so the roll is effectively 0.

**Base roll, observed.** A breakpoint just after `SampleRainIntensity` stores the base roll (`FSTP` to a stack
slot, at `0x753A20`) recorded two values in two hits: about -0.85 and -0.44. Both are negative and they
change between hits. The derivation above says the roll lies in 0 to 1, so **the derivation is wrong
somewhere**: either the function's return differs from what the decompile suggests, or the slot holds
something else. The observation still shows the roll does not block rain, since a negative roll passes any
zone compare, but the magnitude and sign are unexplained and need the function's return traced.

The breakpoint was removed after each reading.

**Trace of `FUN_0073c9f0` (disassembly, which corrects the decompile):**

- The input is `t·K1 + s·K2` times a constant, where `t` is the time term from `FUN_0073c980` and `s` is the
  float at `[ESP+4]` on entry. That slot is the caller's last push, which is `EBX`, the caller's `x`
  coordinate. So the base roll depends on position as well as time.
- The index is `(input as integer) / 360`, computed with a multiply-by-reciprocal, then passed to the cosine.
- The clamp is not "negative becomes 0": a negative cosine gives a result of **1**, and a non-negative
  cosine gives `1 − cos`. So the result is in 0 to 1 by construction.

Constants (read from the program image): the time term is scaled by about 0.714, the position term by 0.02,
and the sum by 65536. So the input is `(0.714·t + 0.02·x) · 65536`. At entry the observed `x` was about -2234,
which makes the position term dominate the input and the angle vary with place.

The observed negative values (-0.85, -0.44) therefore contradict this trace as well. Either the slot read is
not the function's result, or the FPU store reads a different register than the one the trace assumes. This
is open; a fresh breakpoint that also reads `[ESP+4]` and the index would settle it.

## How to check it

Once the Cheat Engine bridge or Ghidra debugger is up:

1. Find `g_RainOverride` (`0x9b0a48`) and the rain value `+0x28c` of the weather block in a race.
   Confirm `SetRainOverride` only writes 0 or -1 when a script calls `SetChanceOfRain`.
2. Run dry, then rainy, and check that `+0x28c` ramps at the rate implied by `0x904ac4`.
3. Watch `g_WindHeading` (`0x9b0a50`): it should rise by 45 per second and wrap at 360.
4. Find the source of `FUN_0073c9f0` and the zone table at `0x9b2a9c` to replace the placeholder
   "chance" with the real roll.
