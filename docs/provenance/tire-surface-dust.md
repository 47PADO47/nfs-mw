# Dry tire surface effects

- **Spec:** [specs/tire-surface-dust.md](../specs/tire-surface-dust.md), extending
  [specs/rain-pc.md](../specs/rain-pc.md) "Tire surface water".
- **Research date:** 2026-10-24.
- **Method:** read-only inspection of the original PC 1.3 `speed.exe` in Ghidra (no process was run), plus the
  repository's own attribute reader over the installed `GLOBAL/ATTRIBUTES.BIN`. The earlier tire-water section was
  "reference-derived and needs independent PC runtime comparison"; this pass supplies that comparison for the
  dispatcher. No decompiler output, disassembly or asset is copied into the repository; function addresses are
  navigation aids for this binary only. The Rust was written from the spec.

## Findings in `speed.exe`

All addresses are for the stock 1.3 executable (image base as loaded). Names are ours.

| Address | Role | What it shows |
|---|---|---|
| `0x00988e0c`, `0x00988e14`, `0x00988e18` | `SimSurface` class, "unknown" and "null" surface instances | Set by the initialiser at `0x006e9790`. The "null" surface (`0x90a4d09a`) is the default every wheel starts on. |
| `0x007453a0` | Per-wheel effect state constructor | Zeroes three group slots, then binds the null surface; the two surface flags at +0x84 (wet) and +0x85 (flat) start clear. |
| `0x00738d50` | Per-wheel surface bind | Reads the three record arrays of a `simsurface` (arrays at +0x00, +0x44 and +0x88 of its data, each a count then records of 20 bytes from +8). Index is 2 if flat and the array has 3+, else 1 if wet and 2+, else 0. It copies the record's group key and min/max into the wheel's slot and restarts the slot when the key changes. The array order is drive, slip, slide. |
| `0x00738f50` | Per-wheel per-frame rebind | Rebinds only when the surface, wet flag or flat flag changed. |
| `0x007457d0` | Per-wheel group update | Three inputs. All three zero disconnects the groups; otherwise, if the wheel is flagged in contact, rebinds and updates each group. |
| `0x00745290` | One group update | Ramp of the input over the record's min..max; zero or less disconnects and frees the emitter after a short delay; positive places the emitter at the tire, passes the car velocity and sets the emitter's intensity to the ramp. |
| `0x00401ae0` | Ramp | `(x-min)/(max-min)` clamped to 0..1; a range of 1e-6 or less gives 0. |
| `0x00746750` | Car tire effects, four wheels | Per wheel: contact bit and flat bit from the car state, wet bit from the car flags, then the drive/slip/slide inputs below. |
| `0x007564c0` | Car frame update | Calls the above once per frame. Sets the car's wet bit (flag word +0x3f8, bit 3) to the OR of the two player-view rain tests, otherwise clears it. |
| `0x00744c70` | Per-view rain test | True if the view's rain object has a non-zero amount, the car is under 60 units from the view's camera, and one of two flags on the view's rain object is clear; which flag is read depends on whether the car is nearer than 10 units to the camera and is the followed car. |
| `0x00744c00` and global `0x009885c8` | Effect gate | The tire-effect work runs when the global flag is set, for the followed car, or for a car whose camera distance field (at most 80) and a visibility counter pass. The visibility counter is not decoded. |

Inputs: slide-array group = per-axle factor times the tire's lateral figure; slip-array group = per-axle factor
times the tire's slip figure; drive-array group = the car's speed (magnitude of its velocity vector). The per-axle
factors come from two small float arrays in the vehicle's attribute record (counts at +0x90 and +0xc4) and are not
yet decoded. The tire figures are slots of the car's physics state; their exact identity is not established beyond
agreeing with the spec's slip and lateral speed.

**Not found:** any test of the surface's name or class. A surface behaves as dust, grass or leaves purely because
its arrays hold those groups. The hashes of the three field names do not appear as immediates in code, so the
fields are reached through a fixed layout, not by name.

## Installed data

Dumped with a throwaway test over the unmodified install (not committed): every `simsurface` with its three arrays,
and every referenced emitter group with texture, rate, life, size ramp, alpha ramp, velocity, gravity and intensity.
The table in the spec is that dump. Points that shaped the design:

- 47 surfaces; only eleven have a dry drive record (`water`, `cobble`, `gravel`, `grass`, the three `golf_*`, `sand`, `dirt`, `mud`, `roadfog`), and those are exactly the loose ground types. Dry
  asphalt-class slip and slide records are the original tire smoke, and `fxtd_sk_asphalt` additionally carries
  `emtd_dr_leaves`; `fxtd_sk_asphalt_no_leaves` is the same group without it.
- Several wet records have a zero speed range (sand and dirt drive); the ramp makes them silent.
- `roadfog` carries a fog emitter as its drive record, and `water` has a single wet-spray record.

## Validation

`cargo test -p nfsmw-data tire_tests -- --include-ignored` and `cargo test -p nfsmw --bin nfsmw vehicle_effects::water
-- --include-ignored` against an unmodified install (see the spec for what they assert). No visual comparison with
the original game was made, and the leaf behaviour on a particular park's ground was not checked in game.
