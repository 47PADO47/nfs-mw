# Traffic sound events: near misses, crash horns, speeding pursuits

Three behaviours the original game has and the rewrite lacked, and the parameters that tune them:

1. **Near-miss sound**: a fast pass close to a traffic car plays a whoosh.
2. **Crash horn**: a traffic car in a hit plays its own horn sample.
3. **Speeding pursuit**: over the speeding line, with a patrol cop in sight, a chase starts.
4. **Event with parameters**: the hit and pass facts go into `TrafficEvent`, and `EventTuning` holds every threshold.

Evidence tags: **[verified]** checked in the repo's specs or the install's files, **[decomp]** read from `speed.exe` in
Ghidra, **[unconfirmed]** a guess that needs a test, **[design]** a choice made for this rewrite. Related:
[ai-pursuit.md](ai-pursuit.md) §4.2 (pursuit start), [ai-traffic-world.md](ai-traffic-world.md) §3 (horns),
[engine-sound-effects.md](engine-sound-effects.md) §5 and §8 (nitrous and collisions), [ai-traffic.md](ai-traffic.md).

## 1. What the original does

### 1.1 Speeding and the cop (pursuit start)

**[verified, ai-pursuit.md §4.2]** A cop on patrol checks the player every update:

1. The cop can see the car (`CanSeeTarget`, §4.1: 151 m all round at heat 1 to 7).
2. The car is ignored when all hold: not already pursued, no active 911 call, no traffic hit in the last second, heat
   3 or less, and speed below **65 mph (29.06 m/s)**.
3. Otherwise the cop takes the car as its target and the pursuit starts.

So at heat 1 to 3 the trigger is **speeding over 65 mph, a traffic hit in the last second, or a 911 call**. At heat 4
and up, being seen is enough. The line is a fixed 65 mph, not a per-road limit. Which infractions count as speeding
is not in the decompiled sources **[unconfirmed]**.

### 1.2 Near misses

**[unconfirmed]** The binary has no near-miss string (searched `miss`, `near`, `close call`, `whoosh`; no match). The
original's near-miss sound is probably a drive-by whoosh ([ai-traffic-world.md](ai-traffic-world.md) §3) or a scripted
event. Its trigger (gap, speed, angle) needs a measured run of the original. The rewrite uses the design rule in §3.1.

### 1.3 Crash horns

**[verified, ai-traffic-world.md §3.2]** A traffic car has a horn sample id, `pvehicle.HornType` (5, 6 or 11 in the data).
**[decomp]** The horn objects are `CARSFX_TrafficHorn` (`FUN_004f8500`) and `CARSFX_TruckHorn` (`FUN_004f9bd0`); their
vtables are at `0x00899870` and `0x00899b20`. Slot 2 of the car horn returns the sample id global at `0x008efc30`, so
the horn is a per-car sample.

**[verified, install files]** `SOUND/IG_GLOBAL/TRAFFIC_MB.abk` (listed under `AEMS_MiscBanks`,
[engine-sound.md](engine-sound.md) §2.3) holds 8 sounds. Probed with `ea-audio`: sound numbers 2 to 8 decode (1 does
not), at 44.1 kHz or 22.05 kHz, from 0.92 s to 2.45 s. `HornType` 5 is sound 5 (1.08 s) and 6 is sound 6 (0.92 s).
Type 11 is outside this bank, so trucks must use another bank **[unconfirmed]**. The horn plays `HornType` as the
sound number of this bank: the mapping is **[design]** until a listening check confirms it.

### 1.4 Nitrous

**[verified, engine-sound-effects.md §5]** The `CARSFX_Nitrous` object (constructor `FUN_004f8050`, vtable at
`0x00899698`) starts `FX_NITROUS` when the nitrous lights and plays `FX_PURGE` when the tank runs dry.

**[decomp, re-verified from the vtable]** The nitrous update is `FUN_004eef80` (vtable slot 9 at `0x008996BC`; the
slot is read from the table, not guessed). Its behaviour:

- Rising edge (`nos_active` on): the object's boost interpolator (`+0x44`) is set to rise from 0 to 1 over **400 ms**
  (`FUN_004b02b0(…, 0, 1.0, 400, …)`), and the instance starts (`FUN_004eedc0`). The instance's volume is mixer
  input 1 (the flag), and its initial pitch and filter words come from the mixer table.
- Falling edge: the instance is stopped (`FUN_004dcc60` sets its volume to 0 at once and sets its stop flag), and the
  boost falls from its current value to 0 over **1000 ms** (`FUN_004b02b0(…, current, 0, 1000, …)`).
- There is **no fall-out countdown** on this path. An earlier note placed a `+0x34` countdown here; that belongs to a
  different routine (`FUN_004ef7f0`, the generic effect updater), not to nitrous.
- Where the boost is read: not in `FUN_004eef80`, and not among the inputs the game publishes. `car-sound-mixer.md`
  §2 lists nitrous inputs `(5,1)` (loop) and `(5,2)` (purge) only, and states that an input never set reads 0. So the
  nitrous pitch (slot 3) and low-pass (slot 4) inputs are never fed in the original either.

**Rewrite.** `libs/blackbox-carsound/src/effects/nitrous.rs` plays the loop at the mixer's pitch (1.0) and publishes
inputs 1 and 2 only, as the spec's table does. An earlier version routed the boost to input 3 (**[inferred]**); the
published table does not contain that route, so it is removed. The invented `1 + 0.08 * boost` is also gone.

**Open:** the boost's route into the sound is not found. The 0.4 s rise and 1 s fall are verified as the
interpolator's timing, but nothing in the sources links them to the mix. The rewrite therefore does not apply them.
The rewrite stops the loop with a 120 ms fade where the original sets the volume to 0 at once; a small deliberate
difference.

## 2. Shared event and tuning

```text
TrafficEvent::NearMiss { speed, gap }           # speed: the player's speed, m/s; gap: metres between bounding spheres
TrafficEvent::CrashHorn { horn_id, impulse, by_player }
```

`EventTuning` (`crates/nfsmw/src/scenes/world/ai/traffic_events.rs`) holds the thresholds:

| Field | Default | Basis |
|---|---|---|
| `speeding` | 29.06 m/s (65 mph) | **[verified]** §1.1 |
| `cop_sight` | 151 m | **[verified]** §1.1 (distance only, see §4) |
| `hit_memory` | 1.0 s | **[verified]** §1.1 |
| `near_miss_speed` | 20 m/s | **[design]** a slower pass is not a near miss |
| `near_miss_gap` | 2.5 m | **[design]** gap between bounding spheres that counts as close |
| `cooldown` | 3.0 s | **[design]** a car does not repeat a near miss or horn sooner |

## 3. Rules

### 3.1 Near miss

```text
near_miss = not hit and ready and player_speed >= near_miss_speed and 0 < gap <= near_miss_gap
```

`gap` is the distance between the two bodies' centres minus their bounding-sphere radii (`collision_sweep_radius`).
A pass that touches is a hit, not a near miss. `ready` is the car's cooldown (§3.2). Volume is fixed at 0.6 **[design]**.

### 3.2 Crash horn

```text
crash_horn = involved and ready
```

Every traffic car in a hit honks with its own `HornType`, at a volume that rises with the impulse: full at 10 000 N s
and a third at the bottom **[design]**. Each honk starts the car's cooldown, so a pile-up does not produce a burst.

### 3.3 Speeding pursuit

```text
starts_pursuit(heat, speed, seen, recent_hit, has_911):
    if not seen:                  return false      # the cop can see the player
    if heat >= 4:                 return true       # being seen is enough
    return speed > 29.06 m/s  or  recent_hit  or  has_911
```

The trigger runs in the world simulation, after the physics step, only while no chase is on. The patrol heat is 1, the
first wave (`data.rs`). `recent_hit` is a 1 s timer set when the player hits a traffic car.

## 4. Implementation status

| Part | State |
|---|---|
| Rules (`traffic_events.rs`) | done, 7 unit tests |
| Near-miss sound | wired: `collide.rs` → `TrafficSound::NearMiss` → `Audio::play_traffic` |
| Crash horn | wired: `collide.rs` → `TrafficSound::Horn` with the car's `HornType`; sample from `TRAFFIC_MB.abk` |
| `HornType` read | `models.rs` `horn_of`, default 5 when the record has none |
| Speeding pursuit | wired: `ai/commands.rs` `speeding_pursuit`, called from `simulation.rs`; uses `begin_pursuit`, shared with the `pursuit` console command |
| Sound queue | `TrafficWorld::take_sounds`, scene `take_traffic_sounds`, drained in `audio/plugin.rs` |
| Sight | done: `TrafficWorld::update_sight` (`ai/collide.rs`) ray-casts from the cop's eyes to the player's, against the walls (`drive/walls.rs` `world_cast`), within `cop_sight`. Tested with a fake cast (3 tests). The cop cone is not used, since front and rear are both 151 m at heat 1 to 7 |
| Nitrous | the invented boost is removed. The loop follows the published inputs only (1 and 2), as `car-sound-mixer.md` §2 specifies. Input 4 is never fed in the original either. The boost's route into the mix is not found (§1.4) |

Known gaps, stated plainly:

- **No 911 calls** in the rewrite, so `has_911` is always false.
- **The horn and whoosh samples are not heard.** The bank and index are reasoned from the install, not from a listening
  check. The whoosh index 1 in `Stitch_Whoosh_MB.abk` is a guess.
- **Truck horns** (`HornType` 11) fall outside `TRAFFIC_MB.abk`, so they are silent.

## 5. Open questions

1. The original's near-miss trigger and sound. Needs a measured run of the original.
2. Where the nitrous boost interpolator (0.4 s rise, 1 s fall) is routed into the mix, if anywhere (§1.4).
3. Which bank holds the truck horn (`HornType` 11).
4. Whether the original's "speeding" is only the 65 mph line, or also a posted limit.
