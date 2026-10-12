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
- Membership of the nitrous object in the owner's list (resolved to a condition, Ghidra):
  - The owner's slot 1 is `FUN_004F4AC0`. It stores its argument (a mask) at `+0x28`, clears `+0x1C` and `+0x38`, and calls
    `FUN_004F2BB0`, then `FUN_004F2C60`.
  - `FUN_004F2BB0` loops over the 32 bits of the mask at `+0x28`. For each set bit it creates an object with
    `FUN_004E5770(owner+0xC, owner+0x10, bit)` and appends it to the owner's `+0x24` list (count at `+0x2C`).
  - `FUN_004E5770` looks up a record by class ID in a table and calls the record's factory with the record's flag
    bits. For the nitrous object the factory is `FUN_004F8050` (its argument is the flag). That the record's factory is
    `FUN_004F8050` is **[inferred]** from the call shape, not read from the table.
  - So the nitrous object is on the list whenever its record's bit is set in the mask at `+0x28`. The mask comes from car
    data and was not read. `FUN_004EB8E0` also appends to `+0x20` on demand, by ID.
  - Consequence: if the nitrous object is on the list, its update (slot 9) runs each frame and its slot `+0x30` is a `RET`.
    The boost interpolator is then never evaluated, so the boost ramp is never applied (see the list-walker entry above).
- The record-to-factory link is read, not inferred. The record table head `DAT_009148BC` is zero in the file: records are
  registered at run time by `FUN_004F2760` (calls `FUN_004EA670(&DAT_009148BC, descriptor)` per record). The nitrous
  descriptor is at `0x008EFB6C`: class ID `(word0 >> 4) & 0xFFF` = **5**, type `(word0 >> 16) & 0xFF` = 2, name pointer
  `0x008981D8` (`CARSFX_Nitrous`), factory `0x004F8050` (`FUN_004F8050`).
- Admission to the owner's list (`FUN_004E5770`, `FUN_004F2BB0`): the nitrous object (ID 5) is created for mask bit 5
  (`0x20`) at the owner's `+0x28`, and it passes the type test when the owner's `+0x0C` is 3, 4, 5 or 12, or equals 2.
  So the membership condition is: mask `+0x28` has bit `0x20` set, and owner `+0x0C` is 3, 4, 5, 12 or 2. Both values come
  from car-sound data and were not read yet.
- The concrete owner class is vtable `0x00896A30` (its constructor sets it after `FUN_004D8EC0`). Its slot 1 is the setup
  `FUN_004F4AC0` (mask argument), slot 5 the list walker `FUN_004D91D0`, slot 8 the cleanup `FUN_004D9250`. Its destructor is
  `FUN_004AFB10`.
- Its factory is `FUN_004B9D10(flag)`: it allocates 0x48 bytes (allocation flag `0x1000`, or `0x1040` when `flag` is 1), runs
  the base constructor, and sets the vtable. `FUN_004AFAE0` is a second constructor. Neither has a direct caller in the
  binary; both are reached through a table, so the caller that passes the mask (and the owner's type `+0x0C`) is not yet
  found. That is the remaining membership step.
- **Membership of the nitrous object is resolved (Ghidra).** The caller that sets the owner's mask is `FUN_004F2E90`:
  - For each local player it creates the owner with `FUN_004E5970(manager, 0)`. That looks up the owner's record (type 5,
    id 0, factory `FUN_004B9D10`) in the second registration list (`DAT_009145B0`, registered by `FUN_004F2B00`) and
    appends the object to the manager's list.
  - It then calls the owner's setup (slot 1) with the mask `0x00053BFC`, or `0x000D3BFC` when a flag is set. Both include bit
    `0x20` (the nitrous ID 5). The mask is hard-coded in this function, not read from car data.
  - Bit 8 is also set, and `FUN_004EB8E0` creates object 8 on demand. The mask matches the mixer spec's player object list
    (`car-sound-mixer.md` §1, mask `0x53BFA`, which includes object 5). The one-bit difference is in bit 1 only.
  - The nitrous admission test (`FUN_004E5770`) is satisfied: the manager's type is 5, and the nitrous record's type is 2,
    which passes the rule for owner types 3, 4, 5 and 12.
  - **Therefore the nitrous object is in the owner's list for the player's car.** Its update runs each frame (slot 9), its
    slot `+0x30` is a `RET`, and its boost interpolator is never evaluated. So the boost ramp is never applied on this path.
    This is strong, since the mask and admission are both read. The one open point is whether another path evaluates the
    interpolator, which the direct-call and list-walker checks above do not show.
- State-2 gate (partly read). `FUN_004C3980` reads `+0x228` of the object at `+0x34` of its argument (the sim object, not
  the 0x28-byte record) and compares it with the integer 2. Its only callers are two thiscall thunks, `0x004C3E00` and
  `0x004C3E30`, which are slot 7 of two vtables at `0x00896FF0` and `0x00897040` (same thunk body, both call
  `FUN_004C3980` with `this`, then `FUN_004E52A0` and clear `+0x124`). Neither vtable base has a direct xref, and there is
  no MSVC RTTI locator before them, so the owning class and the constructor that installs the vtable are not found yet.
  The `+0x228` value of that sim object is still the unresolved part. The earlier notes below used the 0x28-byte record,
  which is not established as the same object.
- Creation path of the 0x28 `CarSoundConn` object (read). Its name is the string at `0x00896840`; a static initializer
  at `0x00885770` hashes it (`vlt_hash` = `0x1DD80B70`) and pushes a record onto the list whose head is `0x00988EC0`
  (record: hash at `+0`, factory `FUN_004CC790` at `+4`, next at `+8`). The creator is `FUN_006F2540`, which walks that
  list for a hash and calls the record's factory. Its callers are `FUN_006F4400`, `FUN_006F4620` and `FUN_006F44E0`.
  `FUN_006F44E0` passes the constant `0x998C21C0`, which is not `CarSoundConn` (checked against the four connection names
  in the same list). The `CarSoundConn` hash is read in two static-cache getters: `0x004AE550` (guard `0x00913BFC`, reached
  only through the vtable slot at `0x00896828`) and `0x00693BE0` (guard `0x00938680`, reached only through the vtable
  slot at `0x008AB320`). The class descriptor at `0x00896800` is passed to the registration call `0x00465240` from
  `0x004ABFEE` and `0x004CC3D9`. The object that calls the getter, and so the owner of the `+0x228` state word, is not
  yet identified.
- Registration callers of `FUN_00465240` (read): `FUN_004CC3A0` (the sound-system initializer: a 0x2000-byte pool, then
  the sound managers of sizes 0xA8, 0x1C, 0x24, 0x28, 0x20 and so on, stored in `DAT_00911FA8`..`DAT_00911F90`),
  `FUN_005B0D70`, `FUN_005F7390`, `FUN_0056B820`, `FUN_0070F500`, `FUN_0075C550`, `FUN_0078AE80`, and an undefined
  site at `0x004ABFF6`. `FUN_00465240` stores the descriptor in a per-id array and calls `FUN_00464D80`, so `0x00896800`
  is a **pool-type descriptor** of the generic allocator, and its slot 10 is the `CarSoundConn` name lookup. The owner
  of `+0x228` is therefore a pool client, reached through the allocator's id table (`DAT_0090DFF0`, stride 100). Its
  client code is not yet read.
- The other registration callers, read: `FUN_005B0D70` registers descriptor `0x008A2C94`, `FUN_005F7390` registers
  `0x008A3AD0`, `FUN_0056B820` registers the language-texture pool with `0x00899D40`, `FUN_0070F500` registers
  `0x008B2578`, `FUN_0075C550` registers the streaming pool with `0x008B5220`, and `FUN_0078AE80` registers the "Online
  Buffer" pool with `0x008B6888`. None of these is `0x00896800`. So the only registrations of the sound descriptor are
  `FUN_004CC3A0` and the site at `0x004ABFEE`. That site is in a function at `0x004ABFC0` (`MOV EAX,[ESP+4]`, a size
  test, then a 0x2000 pool allocation and the registration with `0x00896800`). `0x004ABFC0` has no direct caller and no
  stored pointer anywhere in initialized memory, so it is not reached by any static path, like `FUN_004CC790`.
- `FUN_004CC3A0` is called only from `FUN_00665FC0`, the game's boot sequence, so the sound pool and its managers are
  created once at startup.
- The pool-id global `DAT_008EE558` is read by a family of owner factories whose records sit in a 0x10-byte table at
  `0x008EE628` (type, name, link, factory at `+0xC`). Each allocates from the pool. Sizes and vtables read:
  `FUN_004B9D10` (nitrous owner), `FUN_004B9E60` 0x140 (vtable `0x896A60`), `FUN_004BB990` 0x168, `FUN_004BC280` 0x168
  (vtable `0x896B90`), `FUN_004BCB80` 0x98, `FUN_004BD010` 0xD8, `FUN_004BE350` 0xE0, `FUN_004BEFA0` 0x28 (vtable
  `0x896DB8`), `FUN_004BF100` 0x28 (vtable `0x896DFC`). None allocates the 0x250 object, so the car sim with `+0x228`
  is not made through this table, and its owner is still unidentified.
- The 0x250 allocation (read). The only immediate 0x250 loaded as a size is at `0x007993DD`, inside `FUN_007992F0`. That
  function searches a named list (by `strcmp`, `FUN_00460D20`); on a miss it allocates 0x250 bytes with `FUN_00652AF0`
  (alignment 8), zeroes the words from `+0x8` to `+0x24C` (so `+0x228` is zero), stores a name at `+0x10` and a handle at
  `+0x24C` from `FUN_00793340`. So the 0x250 object is a named table node, not the car sim itself.
  Callers of `FUN_007992F0`: `FUN_0054CCF0` (a constructor with vtable `0x0089B334`, passing itself as the owner and the
  global name `DAT_0091C39C`) and `FUN_00537F60` (not yet read). Which node the car sim is remains open.
- `FUN_00537F60` (read): for each entry of a list it creates a 0x44-byte display object (vtable `0x0089DD2C`), reads
  the `SCROLL_TEXT_%d` strings, and names its node through `FUN_007992F0`. It is a text or HUD owner, not the car sim.
- The other two 0x250 immediates are field offsets, not allocation sizes. `0x0047E23F` is `FADD`/`FST` on `[ESI+0x250]`
  (a timer accumulator). `0x007FE1D8` stores zero to `[ESI+0x250]` in a reset that also clears `+0x248`, `+0x244`,
  `+0x24C`, `+0x254` and `+0x258`.
- Direct search of the dispatcher's callers (`FUN_006F4400` and `FUN_006F4620`): `FUN_006F4400` is called from
  `FUN_0067B520` (hash `0x998C21C0`), `FUN_006A49C0`, `FUN_006B45C0`, `FUN_006BA0B0` and `FUN_006BBC70` (hash
  `0x804C146E`); `FUN_006F4620` from `FUN_006722C0` and two undefined sites. None of these passes the `CarSoundConn` hash
  `0x1DD80B70`, so the car sound connection is created through a path that uses a computed hash, which is not found yet.
- The callers read so far are vehicle-side objects. `FUN_006A49C0` and `FUN_006B45C0` register hash `0xA3F44E2E`
  (`-0x5C0BB1D2`); `FUN_006BA0B0` registers `0x804C146E` and builds a `heliwash` and a `Physics` connection; `FUN_006722C0`
  registers `0x804C146E` through `FUN_006F4620`. None of these hashes is the `CarSoundConn` hash. The two undefined sites
  (`0x0066B35A`, `0x0069384D`) also register `0x804C146E` through `FUN_006F4620`. So every dispatcher caller found
  uses a constant hash, and none computes `CarSoundConn`. The `CarSoundConn` hash is produced only by the static getters
  (`0x004AE550`, `0x00693BE0`), which the pool-type descriptor's slot 10 reaches. The callers of that virtual slot are the
  remaining search, and they are not yet read.
- The pool object itself has vtable `0x00896800`: `FUN_00464D80` stores its descriptor argument at `*this`. Slot 10 is
  `[pool+0x28]`. A direct scan finds 170 indirect calls through `+0x28` (`FF 50/51/52/53/56/57 28`), which are not yet
  classified. The pool id table `DAT_0090DF10` and the name table `DAT_0090DE50` are read only inside the allocator.
  `FUN_004640F0` is the name lookup (string compare with `FUN_00460DD0`). Its callers are `FUN_0073A920` (creates the named
  pools `Main Pool`, `Track Streaming`, `Audio Memory Pool`, `Speech Cache Memory Pool`, `FEngMemoryPool`) and `FUN_00500D80`.
  So the `CarSoundConn` pool is likely the `Audio Memory Pool`, but the slot-10 caller is still not located.
- **The car-sound owner classes (read).** The `BlewByCop` check's `param_1` is a 0x130-byte owner. Its three factories
  are `FUN_004C3830`, `FUN_004C3B20` and `FUN_004C3C90`. Each allocates 0x130 bytes from the pool id `DAT_008EE558`, calls
  the base constructor `FUN_004B9BB0`, and installs a vtable (`0x896FF8` for the first two, `0x897048` for the third).
  They are registered in the owner-record table at `0x008EE5F8` (records at `0x10` stride, factory at `+0xC`), with names:
  `FUN_004C3830` = `EAXCopCar` (name at `0x008966F4`), `FUN_004C3B20` = `EAXTruck` (`0x008966E8`), `FUN_004C3C90` =
  `EAXTrafficCar` (`0x008966D8`). So the `BlewByCop` owner is a cop, truck or traffic car sound object, and its sim is
  reached through `+0x34`. The sim's own class and its `+0x228` writer are not yet read. The base constructor
  `FUN_004B9BB0` (vtable `0x896FA8`) does not set `+0x34`, so the sim is attached after construction, through a later
  call on the owner, which is the next thing to read.
- The owner vtable (read): `0x00896FF8` slot 1 is `FUN_004F4AC0` (the setup), slot 5 is the `BlewByCop` thunk
  `0x004C3E30`, and slot 9/10 are `FUN_004AFA80`/`FUN_004AFA90`. `FUN_004F4AC0` stores the mask at `+0x28`, zeroes
  `+0x1C` and `+0x38`, and calls `FUN_004F2BB0` (per mask bit: finds the component by class id through `FUN_004E5770`,
  calls its slot 5, appends it to the list at `+0x24`) and `FUN_004F2C60` (walks that list and calls each component's
  slot 3, then `FUN_004EB8E0`, and clears `+0x30`). **Neither writes `+0x34`.** So the sim pointer at `+0x34` is stored by
  a different writer, which is still not found.
- Store scan for `+0x34` (read): 507 functions contain `MOV [reg+0x34], reg` (opcode `89`, `disp8 = 0x34`). **None** of
  them references the owner vtables `0x00896FF8`, `0x00897048`, `0x00896FA8` or `0x00896FA0`. So the sim pointer is stored
  by a generic helper that receives the owner as an argument, not by the function that builds the owner. The helper's
  callers are the next step.
- **The attach (read).** Narrowing the 349 store functions to those in vtables leaves `FUN_004E4FD0`, which is slot 3 of
  the owner base vtables `0x00896FA8` (at `0x00896FB4`) and `0x00897048` (at `0x00897054`). `FUN_004E4FD0(this = owner,
  param_1 = sim)` stores `owner+0x34 = param_1`. So `+0x34` is the car sim, passed in by the caller of owner slot 3. It also
  sets `owner+0xA8` to 0, 1 or 2 from the list entries of the sim (types `0x20` gives 2, `0x10` gives 1), and reads the sim's
  tables at `+0x200` and `+0x208`. The caller of owner slot 3 (which creates the sim) is the next read.
- **The `+0x228` owner (resolved).** The car sims are the array at `DAT_00913DB4` (count `DAT_00913DBC`), filled by
  `FUN_004C0A90` from the car-sim constructor `FUN_004C9CB0`. `FUN_004CAEF0` (the body of the `CarSoundConn` factory
  `FUN_004CC790`) mallocs **0x260** bytes, constructs the sim with `FUN_004C9CB0(sim, id, role, ·, ·)`, and stores the sim
  at its `this+0x14`. The role is the float at **record `+0x0C`**, where the record is the car descriptor from the
  `CarSoundConn` registry. `FUN_004C9CB0` stores it at **sim `+0x228`**, and sets `+0x22C = 1`, `+0x22D` from the role.
  So the state word is the car descriptor's role field. The `BlewByCop` gate compares its raw bits with the integer 2,
  so it selects the descriptors whose role field holds the value 2 (raw). The owners of the descriptors are the car
  types `EAXCopCar`, `EAXTruck` and `EAXTrafficCar`. Which car descriptors carry role 2 is in the game data, not the
  binary.
- **Data search for role 2 (read, negative).** The type names are not stored as strings in the install: a scan for the
  `vlt_hash` values of `EAXCopCar` (`0x888A8B77`), `EAXTruck` (`0xD6FD9EA1`), `EAXTrafficCar` (`0x88AAA41B`) and
  `CarSoundConn` (`0x1DD80B70`) finds only three incidental matches in `.VP6`/`.SPS` video. `tools/chunkdump.py` cannot
  parse `SOUND/*.BNDL` at the top level (`SPEEDSOUNDS.BNDL` is not a bChunk; `METALCAR.BNDL` overruns at `0x1000D`), and
  `docs/formats/audio.md` does not document a car-descriptor role field. So the role-2 mapping needs the car-sound
  bundle format, which is not yet decoded.
- **Bundle format (read, negative).** The `SOUND/` and `GAMELOGIC/` bundles begin with a `bnd2` header, not a bChunk
  `VPAK` pack. A byte search of every `.BNDL`, `.BIN` and `.BIG` in the install finds **no** `VPAK` (the attribute
  database format of `blackbox-attrib`). So the pvehicle and car-descriptor attributes are not in a `VPAK` in this
  install, and the role field is in a container format that the repo does not yet read.
- **Inflated-stream search (read, negative).** Every zlib stream in every `.BNDL`, `.BIN` and `.BIG` of the install was
  inflated and searched for the little-endian `vlt_hash` values of `EAXCopCar` (`0x888A8B77`), `EAXTruck` (`0xD6FD9EA1`),
  `EAXTrafficCar` (`0x88AAA41B`) and `CarSoundConn` (`0x1DD80B70`). **No stream contains any of them.** So the car
  descriptors are not stored under these hashes in the compressed bundles either. The role-2 mapping remains unresolved.
- **Vault-marker search (read, negative).** The AttribSys vault chunks `ExpN`, `PtrN`, `DepN`, `StrE` and the pack magic
  `VPAK` were searched in every raw and every inflated stream of the install. **None is present.** So the pvehicle and
  car-descriptor data is not in AttribSys vault form in this install, and the `bnd2` entries use a different record layout.
- **`bnd2` header (read).** `GAMELOGIC/GAMEPLAY.BNDL` (216,879 B): `0x00` `bnd2`; `0x04` u32 `0x00010005`; `0x08` u32 file
  size `0x34F2F`; `0x0C` u32 `0x118` (table offset); `0x10` u32 `0x70`; `0x14` u32 `0x4F30`; `0x18`/`0x1C`/`0x20` u32
  `0x34F2F` (end offsets); `0x24` u32 **9** (entry count); `0x34` container name `GAMEDATA`. The table at `0x118` is 0x20-byte
  records of (hash, flags, size, offset) in the same `0x40000`/`0x400...` flag style as bChunk. The vault inside these
  entries is not decoded yet, so the car-descriptor role field remains unread.
- Classification of the 170 `+0x28` indirect calls (read): they fall in 130 distinct enclosing functions. Of the 95 that
  are functions with a `+0x28` call and a named body, **none** references the pool-id global `DAT_008EE558`, the
  descriptor `0x00896800`, the name-hash global `0x00913BD8`, or the pool tables `DAT_0090DF10`/`DAT_0090DFF0`/`DAT_0090DE50`.
  So the getter is invoked on an object reached through a pointer, not through those globals. The next step is the
  callers of the functions that take that object, which are not yet traced.
- So the only 0x250 allocation is the named node in `FUN_007992F0`. Its caller `FUN_0054CCF0` (vtable `0x0089B334`) has
  no direct caller. The same vtable is set by `FUN_005142F0`, a base constructor that calls `FUN_00572360`. So the node's
  owner is a UI-side class shared through the named table, not the car sim. The 0x250 path does not lead to the car
  sim's `+0x228` owner.
- Earlier reading (superseded in its object identity, kept for the chain): the `BlewByCop` check reads `+0x228` of the
  car's sound-sim object and compares it with the integer 2. `FUN_004C9CB0` (constructor) stores its float argument at `+0x228` (the role word from a record: `FUN_004CAEF0`
  passes `*(record+0x0C)`). The caller of `FUN_004CAEF0` is `FUN_004CC790`, which builds a 0x28-byte car sub-object from a
  record. `FUN_004CC790` has no direct caller and no table entry in any initialised memory (a dword scan finds none), so its
  path is not reached by a static call. The role values for the player's car are therefore not established by this path.
- Other writers of `+0x228` (all read): `FUN_0056C5E0` zeroes an object and sets `+0x200` to 2 (not `+0x228`).
  `FUN_00704E70` writes a game timestamp (`DAT_00925AE8`) to `+0x228` of a different object. `FUN_005E4B10` zeroes
  `+0x228` of a 0x250-byte object. `FUN_006D4DC0` binds a post-processing parameter handle to `+0x228`.
  `FUN_006C3D20` is a shader-parameter name constructor (unrelated). `FUN_0071CB10` and `FUN_007200C0` are game-object
  constructors that store the timestamp `DAT_00925AE8` at `+0x228`; `FUN_007200C0` builds the event-driven object
  that names "AIRacerBusted", "Infraction" and "SoundAI". `FUN_00791F10`, `FUN_00793B90` and `FUN_0079F450` reset
  `+0x228` to 0 (reset routines). `FUN_00864850` is a property setter that writes its argument to `+0x228` only when the
  key is the fourcc `'time'` (`0x74696D65`), so this field is a timestamp in every store read so far.
- Conclusion for the gate: no writer of `+0x228` stores a role value on the car sound sim. Every reachable store is a
  timestamp or a zero reset. The state-2 comparison therefore cannot be a role on this object, and the gate's meaning
  stays **[unconfirmed]**. The `FUN_004C9CB0` role argument remains the only role source, and its caller path is not
  reached statically.
- Interpolator writers (read). `FUN_004B02B0` (the interpolator setup) has 35 callers. The nitrous boost is set only by
  `FUN_004EEF80` (the nitrous update, slot 9), at `0x004EEFDE` (rise) and `0x004EF026` (fall). The other 33 callers set
  unrelated interpolators and do not touch the nitrous object. So the nitrous boost has one writer, and its evaluation
  is still only the list-walker path, which slot `+0x30` (a `RET`) skips. The boost is never applied on this path.
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
