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

**[decomp, `FUN_004C3980`]** The original's near-miss check runs for the player's car. It finds the nearest other car
(`FUN_004B0790`) and, when all of these hold, it plays the sound event `BlewByCop` and posts event 8 to the world's
event dispatcher (vtable slot 3 of `DAT_00911F80`):

- the centres are within **12 m** (`FUN_0045F340` distance `< 12`);
- the relative speed (the length of the difference of the two velocities, `+0x50..+0x58`) is at least **15 m/s**;
- the player's state word `+0x228` is **2** for the sound. Event 8 is posted without that test. The meaning of state
  2 is **[unconfirmed]**.

`BlewByCop` is referenced by hash (`FUN_005CC240("BlewByCop")`). Its sample is not in the install's sound files
(a search of the install finds the name only in `speed.exe`), so the sample is not identified. The rewrite plays the
whoosh until that is found.

The rewrite's rule (§3.1) uses this: centres under 12 m and relative speed of 15 m/s or more, with the cooldown.

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

**Leads (Ghidra, `speed.exe`), recorded so the trace can resume:**

- The nitrous vtable is `0x00899698`, referenced by constructors `FUN_004f6000` and `FUN_004f6080`. Slot 9
  (`0x008996BC`) is `FUN_004eef80`, the update.
- `FUN_004eedc0` starts the instance (`FX_NITROUS`) from the nitrous mixer table: volume from its first word, one
  pitch word from the low half. It stores the instance at `+0x28` of the object.
- The boost interpolator is at `+0x44` of the object: `FUN_004b02b0` sets it (start, target, duration in ms, mode).
  Its evaluator is not yet found.
- `FUN_004ef7f0` writes instance fields (`+4` volume, `+8` pitch, `+0x24` low-pass) at `+0x38`. It belongs to another
  class (a vtable entry at `0x0089972c`). Its class is not identified, so it is not yet linked to nitrous.
- Nitrous vtable `0x00899698`, slot by slot (Ghidra): 0 `0x004F80E0` (destructor), 1 `FUN_004DCB30` (type id), 2
  `FUN_004DCB40` (class id), 3 `FUN_004DCB50` (argument count), 4 `FUN_004DCB80` (connect a sub-object: type 2 to `+0x40`,
  type 7 to `+0x3C`), 5 `0x004DCBC0` (true when the type is 2 or 3), 6 `FUN_004E8620` (selects the bank and posts the
  record), 7 `0x004DCC10` (sets a flag on the connected object), 8 `FUN_004DCD00` (releases the sub-objects), 9
  `FUN_004EEF80` (the update: boost ramp and start/stop), 10 `0x004EF050` (stops the instance: volume and pitch
  words set to 0 at once), 11 to 15 accessors (`0x004B32F0` returns, `0x004B3300` returns, `FUN_004DCD90` and
  `FUN_004DCDA0` return globals, `FUN_004DCDB0` returns 6 or -1). None of them evaluates the `+0x44` interpolator.
- Slot 6 is the bank choice, now confirmed. Its hash `0x8aca07f9` is a field of the `audiosystem` class
  (`D3C18F03`, mostwanted collection), with the value `["Nitrous_00_MB.abk", "Nitrous_01_MB.abk"]`. The index is 1
  for state 2 and 3 and 0 otherwise, which matches the spec's "Nitrous_01 for levels 2 and 3".
- Mixer writes by the nitrous object (Ghidra): the loop flag goes to `*(obj+0xc)+4` (mixer input 1) at start, and 0 on
  stop (`FUN_004EEDC0`, `FUN_004DCC60`). The purge start `FUN_004E6A00` writes `*(obj+0xc)+8` (input 2) at 0x7FFF, and
  clears the car's purge request at `+0xB0`. Nothing in the nitrous object writes `*(obj+0xc)+0xC` (input 3) or
  `+0x10` (input 4). So the boost interpolator at `+0x44` never reaches the mixer through the nitrous object. This is
  the evidence for dropping the input-3 routing.
- Field-reader scan (Ghidra script, temporary, removed afterwards). Every instruction in `0x4DC000` to `0x500000` (the
  car-sound code range) with an access to `[reg+0x44]`, excluding stack and frame bases: 98 hits. Results:
  - The nitrous update's `LEA ECX,[ESI+0x44]` (`0x4EEFDB`, `0x4EF023`) passes the interpolator as `this` to its setter.
  - `FUN_004EF7F0` reads `+0x44` as a table index (already identified), not the boost.
  - `FUN_004E7000`, `FUN_004E5A80`, `FUN_004FEE70`, `FUN_004FCAC0`, `0x4DDC40` read car positions through a pointer at
    `+0x44`. `FUN_004ED9B0` is the engine's table-to-instance updater (RPM hash `0xD377B339`).
  - No float load (`FLD`/`FSUB`/`FADD`) reads the object's own `+0x44` (base `ESI`/`EAX` as the object) in this range.
  So the embedded interpolator's value is not read as a float in the car-sound code. Integer reads of `+0x44` (`MOV`)
  are not all checked, and a copy through an integer register can hide a float read. The scan's range is the limit.
- Whole-binary float scan (temporary script, removed): every `FLD`/`FSUB`/`FADD`/`FMUL`/`FDIV`/`FCOM` on `[reg+0x44]`
  (stack bases excluded): 175 hits. The car-sound candidates were checked and none reads the nitrous interpolator:
  `FUN_004CA090` (a vector constructor), `FUN_004D5D40` (the per-car telemetry loop of the car sound: positions and the
  surface flags `CDActionDrive`, `CDActionTrackCar`, `CDActionIce`), and `FUN_004B1040` (an envelope with its own timer
  at `+0x44`, a different struct layout from the nitrous interpolator). The rest of the 175 are outside the car sound.
  Integer-register reads (`MOV`) of `+0x44` in the car-sound range, decompiled: `FUN_004EE820`, `FUN_004EEA00`
  (car-object pointer reads), `FUN_004E4FD0` (sound-object setup), `FUN_004E5440` (destructor), `FUN_004E1EB0` (a
  per-car channel map), `FUN_004F10E0` (table-to-instance writer: mixer table words, not an interpolator), `FUN_004FE5E0`
  (car positions), `FUN_004F4B70` and `FUN_004DD690` (release a sub-object), `FUN_004E0C60` (sound-object setup). None
  reads the nitrous interpolator. Unnamed `MOV` reads (`0x4DDC...`, `0x4EE4xx` to `0x4EE7xx`, `0x4EF38F`, `0x4EFAA7`,
  `0x4F0BC7`) are pointer chains or setters. Checked one by one since: `0x4F0D64` (a car pointer from `0x4B89D0`, then
  the car's position copied at `+0x40` to `+0x48`) and `0x4DDC61` (the same car-position pattern). No unnamed read in the
  car-sound range loads an embedded interpolator's value.
- State word `+0x228` (`FUN_004C9CB0`, the constructor of the car's sound-sim object). It is a constructor argument,
  not a runtime state. Its caller `FUN_004CAEF0` passes a role from a data record. The constructor sets a flag at `+0x22D`
  for roles 3 and 5 to 6, so those are the cop-related roles. Role 2 (the near-miss gate) is a data class whose name is
  not in the decompiled sources **[unconfirmed]**.
- Integer loads of `+0x44` in `0x4C0000` to `0x500000` (Ghidra script, temporary, removed): 44 hits. The named ones were
  decompiled: `FUN_004CF720` (instance copy), `FUN_004C6250` and `FUN_004C7F30` (stream handling), `FUN_004C48C0` (stream
  buffers), `FUN_004CA4B0` (loads a car-sound file: it checks the chunk tags `SCEl`, `SCCl`, `SCDl` and `SCHl`). The
  `SCHl` tag is the car horn chunk. None reads the nitrous interpolator. The unnamed ones are pointer chains (§1.4).
- The boost interpolator's evaluator is `0x004B0320`. It adds `dt` to the elapsed time at `[this+0]`, compares with the
  duration at `[this+4]`, and writes the current value to `[this+0x14]`, with a flag at `[this+0x18]`. For the nitrous
  object (interpolator at `obj+0x44`), the live value is therefore at **`obj+0x58`**. The nitrous update reads `obj+0x58`
  as the start of the fall (`param_1[0x16]`), which fits.
- Callers of the evaluator (Ghidra xrefs): `FUN_004BC0C0`, `FUN_004BCDC0`, `FUN_004BCD50`, `FUN_004BEB40`, `FUN_004E66B0`,
  `FUN_004EFDB0`, an unnamed block at `0x004DC900` (interpolator at `+0x34`, writes mixer input `*(obj+0xC)+8 = 0x7FFF`),
  `FUN_004C4DB0` (the car-sound state machine, interpolators at `+0x88`, `+0x6C`, `+0xA4`), `FUN_004BBAE0`, `FUN_004BBB60`,
  `FUN_004BA340`, `FUN_004BCCD0`, `FUN_004BC84B`, `FUN_004BC857`, `FUN_004E66B0`. None evaluates the nitrous object's
  interpolator at `obj+0x44`.
- `LEA reg,[reg+0x44]` across the binary (Ghidra script, temporary, removed): 99 hits. In the car range the only ones
  that pass `+0x44` as `this` are the nitrous setter (`FUN_004EEF80`, `0x4EEFDB` and `0x4EF023`) and the object's
  constructors (`0x4F6045`, `0x4F60E9`). `FUN_004BD650` passes `+0x44` to a curve evaluator (`FUN_00653790`, a piecewise
  curve of `{pointer, count}`), which is not the interpolator layout.
- So the nitrous boost is evaluated, if at all, through a computed address or an indirect call (a vtable or a list of
  objects). Those cannot be enumerated with the tools here. The route is narrowed to that step; it is **not located**.
- Float reads of `[reg+0x58]` across the whole binary (Ghidra script, temporary, removed): 165 hits. In the car-sound range
  they are car velocity reads (the car keeps its velocity at `+0x50` to `+0x58`), for example `0x4EFC3E` (a speed from the
  velocity vector) and `0x4DC957`. None is shown to be the nitrous object reading its boost at `obj+0x58`. The rest of the
  hits are in physics, rendering and UI code and were not decompiled one by one.
- The `+0x58` float hits, checked in batches: speed-from-velocity readers (`FUN_004AB890`, `FUN_004AB8C0`, `FUN_004B14E0`,
  `FUN_004D5BD0`, `FUN_004DAD70`, `FUN_004DBB30`, `FUN_004EFC3E`); car-pass event code (`FUN_004BE4A0` posts event types 4
  and 5, `FUN_004BF2E0` sets approaching and receding flags, both candidates for the `BlewByCop` pass event); the tractor and
  truck effects (`FUN_004F11E0`); road-noise instances (`FUN_004ED4C0`); tire and skid update (`FUN_00428720`); pose,
  matrix and position code (`FUN_0046A040`, `FUN_0046A480`, `FUN_0046B1F0`, `FUN_0046B410`, `FUN_0046D3F0`, `FUN_00417790`,
  `FUN_00423580`, `FUN_0047F490`); drive and sound update (`FUN_0047D3C0`); and the car-sound state update `FUN_00472740`,
  whose ramps (rise 4 per second, fall 1 per second at `this+0x50`; rise and fall 0.5 per second at `this+0x6C`) do not match
  the nitrous timing. Not yet checked: about 60 of the 165 hits, mostly physics, rendering and UI code.
- Evaluator call sites, all read (Ghidra): every direct caller of `0x004B0320` has been decompiled. Each evaluates an
  interpolator at a different offset of its own object: `FUN_004BC0C0` (`+0x110`, `+0x12C`), `FUN_004BCDC0` (`+0x4C`),
  `FUN_004BCCD0` (`+0x30`), `FUN_004BCD50` (`+0x68`), `FUN_004BEB40` (`+0x78`), `FUN_004BA340` (a wrapper over whatever it
  is given), `FUN_004BBB60` (`+0x14C`, `+0x88`), `FUN_004BBAE0` (`+0x6C`), the unnamed block at `0x004BC820` (`+0xD8`,
  `+0xF4`), `FUN_004EFDB0` (`+0x48`), `FUN_004E66B0` (`+0x3C`), `FUN_004C4DB0` (`+0x88`, `+0x6C`, `+0xA4`), and `0x004DC900`
  (`+0x34`). None evaluates an interpolator at `+0x44`. The nitrous object's own interpolator (`obj+0x44`) therefore has no
  direct evaluator call in the binary. Either it is evaluated through a computed or indirect call, or the boost is never
  evaluated in the original. This is a negative result for the direct-call route, not proof of absence.
- Indirect route (Ghidra script, temporary, removed): `CALL dword ptr [reg+0x24]` (vtable slot 9, the nitrous update) in
  `0x4C0000` to `0x500000` has 7 hits. The one that iterates objects is `FUN_004D91D0`, a list walker. It takes two
  intrusive lists at `this+0x20` and `this+0x24` (each node's first dword is its vtable, the second its next pointer) and
  calls slot 9 (`+0x24`) and slot `+0x30` on every node. It is called from `FUN_004E52A0`, the per-frame tick of a car-sound
  owner (its car is at `+0x34`). The nitrous object's next pointer is at its own `+0x04` (set to 0 in its constructor), which
  fits being a node in such a list, but the membership is not yet proven.
- For the nitrous object, slot `+0x30` is `0x004B3300`, a bare `RET`. So along this path the nitrous update (slot 9) only
  starts or stops the boost, and nothing evaluates its interpolator. The boost value at `obj+0x58` then stays at the start
  value that `FUN_004B02B0` writes (0), if the object is on this list. This is a strong lead that the original never
  applies the boost ramp, not a finished proof. The remaining check is where the nitrous object is inserted into the list.
- The list owner (membership check, in progress). Its vtable is `0x00898614`. Its constructor is `FUN_004D8EC0` (writes the
  vtable at `0x004D8EC4`), its destructor is `FUN_004E53B0`, which calls `FUN_004D9250` (walks `+0x20` and `+0x24`, calls
  each node's slot 0 with delete). Vtable slots: 0 `0x004EB860`, 1 `0x004F4AC0`, 2 `0x004AFB00`, 3 `0x004D9300`, 4
  `0x004D9190` (walks `+0x24`, calls node slot `+0x18`), 5 `0x004D91D0` (the update walker of section 1.4), 6 `0x004D91B0`
  (walks `+0x24`, calls node slot `+0x28`), 7 `0x004F4AF0`, 8 `0x004D9250` (cleanup), 9 `0x004D8EA0`, 10 `0x004D8EB0`,
  11 `0x00574E50`, 12 `0x004D95A0`, 13 `0x004D95D0`, 14 `0x004D95E0`, 15 `0x004D9600`.
- `FUN_004D8EF0` only resets node data; it does not insert. The insertion of a node into `+0x20` or `+0x24` was not found.
  A whole-binary scan finds 74 stores of a register into `[reg+0x20]` or `[reg+0x24]` in `0x4D0000` to `0x4F0000`; most
  belong to other classes. The insertion is the next check.
- Hash check: `vlt_hash("nos")` is `B1669F64` (the class `nos`), so the probe hashing is correct.
- The sound-instance queue is `DAT_00912270` (posted by `FUN_004e52c0`, consumed by `FUN_004ce960`), with instances in
  a table at `0x00912858` (stride `0x68`). The per-instance processor is `FUN_004d0a60` and `FUN_004cfac0`, and the
  sound starter is `FUN_004cffa0`.

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
| `near_miss_speed` | 15 m/s | **[decomp]** relative speed of a near miss, `FUN_004C3980` |
| `near_miss_distance` | 12 m | **[decomp]** centre distance of a near miss, `FUN_004C3980` |
| `cooldown` | 3.0 s | **[design]** a car does not repeat a near miss or horn sooner |

## 3. Rules

### 3.1 Near miss

```text
near_miss = not hit and ready and relative_speed >= 15 m/s and distance < 12 m
```

`distance` is the distance between the two cars' centres, and `relative_speed` is the length of the difference of their
velocities. A pass that touches is a hit, not a near miss. `ready` is the car's cooldown (§3.2). Volume is fixed at 0.6 **[design]**.

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
| Near-miss sound | wired: `collide.rs` → `TrafficSound::NearMiss` → `Audio::play_traffic`. Rule from `FUN_004C3980` (12 m centres, 15 m/s relative). The sample is the whoosh guess; the original's is `BlewByCop` (unidentified) |
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
