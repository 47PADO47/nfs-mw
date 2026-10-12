# Log trailer: why the logs do not fall on a side hit

Status: diagnosed, not fixed. The rewrite releases the logs only from a rear hit. The original releases them from
side hits too, and the rule that picks the impact level for side zones is not read yet. Tags follow
[frontend-menus.md](frontend-menus.md): **[verified]** checked against the install data, **[decomp]** read from
decompiled code, **[ours]** the rewrite's choice.

Vehicle: `TRAILERLOG` (its parent `trailers`), in `CARS/TRAILERLOG/`. Its logs are the four `ATTACHMENT1..7`
parts the sequence `TrailerLog` releases.

## 1. The pipeline [ours, matches the spec]

1. A car hits the trailer (`drive/sim/pair.rs`, `drive/sim/mod.rs`). Each side gets a `DamageHit` with the zone
   the contact point falls in (`events/damage.rs`).
2. `VehicleEvents::advance` (`scenes/world/events.rs`) sends the stimuli `COLLISION`, then `COLLISION_VEHICLE` /
   `COLLISION_COP` / `COLLISION_PLAYER` when they apply, then `IMPACT_1` … `IMPACT_level` to the zone's system.
   `level = floor(force / strength * visual_scale[zone])`, capped at six, with `strength = FORCE * 1000`
   (`docs/specs/world-sequencers.md`, "Damage triggers").
3. The `TrailerLog` engine runs `TriggerLoad` on `IMPACT_1` and `DropLoad` releases the nine fragments; seven of
   them are the logs (`cargo.rs`, `props`). The sequence is verified by the real-install test
   `nfsmw-data/tests/real_install/sequences.rs`, which sends `IMPACT_1` to `DZ_LEFT` and gets all nine fragments.

## 2. What the install says [verified]

`nfsmw-data/tests/real_install/trailer_logs.rs` reads the `damagespecs` record of `TRAILERLOG` and of its parent
`trailers`. The first float of each `DZ_*` record (the `VisualScale`) is:

| zone | FRONT | REAR | LEFT | RIGHT | LFRONT | RFRONT | LREAR | RREAR | TOP | BOTTOM |
|---|---|---|---|---|---|---|---|---|---|---|
| VisualScale | 0 | 1 | **0** | **0** | 0 | 0 | 1 | 1 | 0 | 0 |

`TRAILERLOG`'s `FORCE` is 20 (strength 20000 N after the ×1000). Its event spec is present.

## 3. The contradiction [verified against the spec]

With step 1's formula, a side hit (`DZ_LEFT`, `DZ_RIGHT`, the front corners) has `visual_scale = 0`, so the level is
0 and no `IMPACT_1` is sent. `TriggerLoad` never fires, so the logs never fall from a side hit, whatever the speed.
The rear zones (`scale 1`) do release them.

The original's own sequence test sends `IMPACT_1` to `DZ_LEFT` and expects the release. So the original's
level for side zones cannot be `floor(impulse / strength * 0)`. The
[world-interactions note](../world-interactions.md) already records the gap: "Original side-zone scaling does not
release cargo".

## 4. What is not known yet

- The rule that gives side zones an impact level. The damage-level callback is reached through the simulation's
  `+0xF8` function pointer (`FUN_0079d100` calls `*(sim + 0xF8)(param_1, *(sim + 0xFC))`); its target is not read.
- Whether the original sends `IMPACT_1` on any hit above a threshold (a guess), or scales by a different float
  of the record (`HitPointScale` is the second float, `0` for every zone of `TRAILERLOG`).
- The "over the speed limit" condition from the TODO. The rewrite's trigger is `closing_speed >= 1 m/s` and a
  non-zero level; the speed threshold of the original is not identified.

## 5. Next steps

1. Decompile the target of `*(sim + 0xF8)` (start from `FUN_0079d100` and the vehicle damage code near
   `FUN_0079df40`). Find where the zone's level is computed and which float it reads.
2. Confirm with a trace or the decompile: for a side hit, does the original send `IMPACT_1` at impact speeds
   below any threshold?
3. Only then change `events.rs` (the level rule) and add a test that a side hit above the threshold releases the
   logs through the real `TrailerLog` sequence.

## 6. Ghidra trail

The dead ends and the decompiled steps, sections 6 to 41, are in [trailer-logs-trail.md](trailer-logs-trail.md).
