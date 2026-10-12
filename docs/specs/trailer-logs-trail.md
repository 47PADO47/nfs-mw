# Log trailer: the Ghidra trail (companion to trailer-logs.md)

## 6. Ghidra trail (dead ends, so they are not repeated)

- `FUN_0079cf20` sends the collision stimuli; `FUN_0079df40` wraps it; its callers are `FUN_0079e080`,
  `FUN_00537c80` and `FUN_00512ef0`.
- `FUN_0079e080` routes a record to the stimulus senders and calls the callback at `+0x18` of an object. It does
  not scale by strength. Its only caller is `FUN_0079ee50`.
- `FUN_0079ee50` is a generic event post: it sends `USERSET0` and registers `FUN_0079e080` as its callback. Its
  callers are `FUN_00521be0`, `FUN_005360f0` and `FUN_005a4820` (the dialog handler). It is not the damage path.
- `FUN_00795c50` sends a `MINSIZE` stimulus, a different event.
- `FUN_00798630` reads `FORCE` (the strength) from a damage record.

The impact-level computation is still not found. Unread candidates: the callers of `FUN_00537c80` and
`FUN_00512ef0`, and the code that writes the callback at `+0xF8` of the vehicle object.

## 7. What the data rules out (install, `real_install/trailer_logs.rs`)

- No ancestor of `TRAILERLOG` gives a non-zero `VisualScale` on the side zones: `trailers` has the same row, and
  `default` has no `damagespecs`. So the original cannot be scaling side hits by `VisualScale`, whatever the
  impulse. The `floor(impulse / strength × VisualScale)` rule in `world-sequencers.md` is therefore wrong for side
  zones, or the original gets its side-hit level from somewhere else.
- The sequence still releases the logs from `IMPACT_1` on `DZ_LEFT`, so a side hit must reach `IMPACT_1` in the
  original by some other route. The obvious candidate is a minimum level of 1 for any hit above a threshold. That is
  a hypothesis: it is not in the decompile, and it would also change rear hits below the strength.
- Not changed in code. A change to `events.rs` would alter every car's damage, so it waits for evidence from the
  original (a trace of a side hit, or the decompiled level function).

## 8. Dead end: indirect calls through `+0xF8`

- `search_instructions` for `call … +0xf8]` returns 500 hits across the binary (the operand filter is too loose).
  Most belong to unrelated objects, so the damage callback can't be picked out of them without knowing the object's
  type first. Next: find the vehicle object's construction (the store of its vtable or callback pointer), then the
  calls on that object only.

## 9. Candidate writers of `+0xF8` (byte pattern `c7 86 f8 00 00 00`, a store of an immediate at `[esi+0xF8]`)

- Hits in functions (unverified as the vehicle's): `FUN_0042C6C0`, `FUN_00442D00`, `FUN_004DB7F0`, `FUN_004ECF40`,
  `FUN_0055E700`, `FUN_0059B8A0`, `FUN_00600660`, `FUN_0066A170`, `FUN_0074D1F0`, `FUN_00694160`, `FUN_00755800`.
- Several take `this` and a float (for example `FUN_004ECF40(this, float)`, `FUN_0066A170(this, float)`), which fits
  a setter of a time or force parameter more than a callback pointer.
- Not yet read. Next: decompile `FUN_004ECF40`, `FUN_0066A170` and `FUN_0055E700` and see which one stores a function
  pointer to `FUN_0079D100` (the damage dispatcher) at `+0xF8`. (`FUN_0079D100` has no callers, so it is reached through
  a pointer.)
- Ruled out (decompiled): `FUN_004ECF40` and `FUN_0066A170` keep a state or timer value at `+0xF8` (0, 1, 2 states; an
  accumulating time) and store no function pointer. `FUN_0055E700` is a menu-state setup that steps `+0xF8` through
  1, 2, 4, 8. So the byte pattern `c7 86 f8 00 00 00` is mostly a state field, not the damage callback. The callback at
  `+0xF8` in `FUN_0079CF20` is read from a different object (its parameter), so its writer is elsewhere. Not found.

## 10. The global event manager and its callback list (decompile)

- `FUN_0079CA30` returns a fixed global at `0x009B9690` (the event manager's state), used by 23 functions, including the
  collision stimulus sender `FUN_0079CF20`.
- `FUN_0079C410(obj)` removes a callback entry from the global manager's list at `FUN_00794F80() + 0x20`. It looks for an
  entry whose callback is `FUN_0079BDE0` and whose data pointer is `obj`, and unlinks it. So damage objects register
  `FUN_0079BDE0` (with themselves as data) in that list; `FUN_0079BDE0` is the per-object callback.
- `FUN_0079F450` resets the event state (it calls `FUN_0079C410` on the singleton).
- So the `+0xF8` callback in `FUN_0079D100` is not stored on the singleton directly. The next step is the registration
  of `FUN_0079BDE0` (the function that pushes an entry at `+0x20`), and what `FUN_0079BDE0` does with `+0xF8`.
- Correction to section 10: `FUN_0079BDE0` is not the damage callback. It handles the messages with tags `game`
  (`0x67616D65`), `play` (`0x706C6179`) and `0x24727570` / `0x2472666E`, and reads per-opponent `OPPO%d`, `STAT%d` and
  `DATA%d` records. That is the opponent and stats exchange. So the callback list in section 10 holds a different
  kind of entry, and the damage path registers elsewhere. Not found.

## 11. The car's callback is a message dispatcher at `0x0079D140` (label, no Ghidra function)

- `FUN_0079DEC0` (the car object's setup: it writes `+0x100`..`+0x10D` and the name at `+0x2C`) registers the callback
  `0x0079D140` with the car object as its data, through `FUN_0079B0E0`.
- `0x0079D140` is a switch on a message index: it checks the index against 7 and jumps through a table at `0x0079D3CC`
  (8 cases). Case handling calls `FUN_0079CF20` (the collision stimulus sender), `FUN_00798800` (the other sender), and
  reads `[obj + 0xF8]` as a data pointer, copying 0x40 bytes from it (`FUN_00839770`). So `+0xF8` is data, not the
  callback. My earlier reading of `+0xF8` as a function pointer was wrong; section 9's candidates were not the callback.
- Next: read the 8 jump-table targets (`0x0079D3CC`) and find which message is the collision/impact one, and where the
  impact level is computed (it feeds `FUN_0079CF20`'s `FUN_00798630`, which reads `FORCE`).
- Jump table at `0x0079D3CC` (8 entries, message index → handler): 0 → `0x0079D1A8`, 1 → `0x0079D22E`,
  2 → `0x0079D24A`, 3 → `0x0079D275`, 4 → `0x0079D302`, 5 → `0x0079D3BA`, 6 → `0x0079D399`, 7 → `0x0079D336`.
  Case 0 is the block that reads `[obj + 0xF8]` as data (section 11), so it is the first handler to read for the impact
  path. Not yet read: cases 0 to 7 in full, and which index a collision sends.
- Handler reads (disassembly):
  - Case 1 (`0x0079D22E`): calls `0x00516C50` (the UI notification routine) with the label hash `0xF9E94D58` and
    flags `0xFF`, then sets `0x0091CA90 = 1`. A screen message, not impact.
  - Case 2 (`0x0079D24A`): the same UI notification with label `0x4FBEE42A` and the screen object at `0x008F32B0`.
    Also not impact.
  - Case 0 (`0x0079D1A8`): reads the caller's two arguments, calls `0x0078C790` twice, and when the first returns zero
    copies 0x40 bytes from `[obj + 0xF8]` (`0x00839770`). The 0x40-byte copy is the likely impact record. Its contents
    are not decoded yet.
  - Cases 3 to 7 start at `0x0079D275`, `0x0079D302`, `0x0079D3BA`, `0x0079D399` and `0x0079D336`: 3 calls `0x00591E90`
    on the screen object and then `0x0079BAC0`; the rest are not read yet.
- Correction to section 11 (case 0): `FUN_0078C790(obj, p)` returns `FUN_00837690(obj + 0xF8, p)` only when `obj + 0xF8`
  is non-zero and a game-state value at `DAT_0091CF90 + 0x19C` is above 1 (more than one player). `+0xF8` is therefore a
  pointer to a sub-object (a stream or link), not a record. Case 0's 0x40-byte write goes through
  `FUN_00839770` → `FUN_00838E70(stream, buf, 0x100000, 0x40, 0, 0)`: a stream write, most likely the networked
  per-opponent record (the `OPPO`/`STAT`/`DATA` exchange seen in `FUN_0079BDE0`). So case 0 is probably not the
  impact record, and the impact level is still not located.
- Cases 4 and 7 (`0x0079D302`, `0x0079D336`): pass the message's arguments to the screen-level handlers
  (`0x0078CC70`, `0x0078CCD0`) and case 7 shows a UI label (`0x67BFA959`). Not impact.
- The dispatcher's entry (`0x0079D140`) is the useful part: it checks the car object's flag at `+0xC`, and when set
  calls `FUN_0079CF20` (the collision stimulus sender, which reads `FORCE` through `FUN_00798630`). When clear it calls
  `FUN_00798800`. So the impact stimulus is gated by `+0xC`, and the level is computed before this entry, in the caller
  that writes the impact data. That caller is the next target (the writer of `+0xC` and of the impulse it uses).

## 12. The car's creation event (`crea`)

- `FUN_0079DEC0` calls `FUN_0079B0E0(obj, record, dispatcher, obj)`, the only caller of `FUN_0079B0E0`.
- `FUN_0079B0E0` builds a parameter string: `MAXSIZE` (4), `MINSIZE` (the record's byte at `+4`) and `SYSFLAGS`
  (`0x40000` when the record's byte at `+0xD` is 1). It then sends the `crea` event (`0x67637265`) with
  `FUN_0079AA50` as the handler.
- So the car's size class (`MINSIZE`) and system flags come from its record at creation time. This is the same
  `MINSIZE` the scenery-LOD code uses, not the damage path. The impact level is still not found. Next: the record
  fields that feed `+0xC` (the impact gate), which are the record's bytes at `+0xC`/`+0xD` of the car object.

## 13. The object is the session, not the car (correction to sections 11 and 12)

- The only caller of `FUN_0079DEC0` in the session start (`FUN_0055CA20`, the 2,329-byte function that begins a game
  session: it sets the scroll area, the status entries `STATUS_%d`, and the vehicle lists) is `FUN_0055CA20` through
  `FUN_0079DEC0((int)puVar5)`, where `puVar5 = FUN_0079BAC0()`. The second caller is `FUN_00513390`, which calls
  `FUN_0079DEC0(FUN_0079BAC0())` when a session has no active vehicles.
- So the object that receives the dispatcher, the `crea` event, the `+0xC` gate and the `+0xF8` pointer is the session
  object returned by `FUN_0079BAC0()`, not the car. Sections 11 and 12 described a session-level object, and the
  `+0xF8` pointer is a session sub-object (a stream or link).
- Next: read `FUN_0079BAC0` (the session accessor), then find where the session's impact data is written when a
  collision reports a damage zone. That writer is the impact-level computation.
- `FUN_0079BAC0()` returns the fixed global `0x009B9440` (the session state; it initialises on first use through
  `FUN_0079BA30`). The dispatcher's object is therefore this global, and the damage-zone impact data is a field of it.
  The writer of that field, when a collision reports a zone, is the next target. It is reached by the collision code
  that calls the session's stimulus path, not through the car.

## 14. Ruled out: the online session layer

- `FUN_0079D3F0`, `FUN_0079D550` and `FUN_0079D750` use the session singleton, but they are online-session code:
  `FUN_0079D3F0` writes per-player records (`0x84` bytes), `FUN_0079D550` builds a room and venue description
  (`VENUE`, `NFSMW`, `NAME%d`, `TEAM%d`, `ROOM%d`), and `FUN_0079D750` tears the session down (it calls
  `FUN_0079C410` on the event manager and `FUN_0079B780` on the session). None writes a damage zone.
- So the impact writer is not in the session layer. The damage-zone classification (front, rear, left, right, from the
  contact arm and normal, `world-sequencers.md`) must be in the collision code itself, which calls the stimulus path.
  Next: find the collision routine that computes the contact arm and normal (a function that takes two bodies and a
  contact point), and the call it makes with a zone index.

## 15. The zone classifier and the impact formula (decoded) [decomp]

- `FUN_0069C030` is the collision impact function. Its inputs are the contact arm (`param_1`, a vector), a
  body index (`param_2`) and the vehicle `this`. Its zone index (`uStack_4`) is chosen from the arm and the normal, with
  the box half-extents (`0.5 × extent`) and the normal threshold `0.8`:
  - the vertical arm `z` against `±0.5 × height` (above or below, the first split);
  - then the longitudinal arm `x` against `±0.5 × length` (front or rear, 0–7);
  - then the lateral arm `y` against `±0.5 × width`, with the normal's lateral component against `0.8` (corners 4–9,
    sides 1–3). The result is the zone 0..9.
- The zone's record is `FUN_0068CAB0(this, zone)`: a 16-byte record inside the vehicle structure at `this+0x78`, at
  offsets `+8, +0x38, +0x18, +0x30, +0, +0x48, +0x28, +0x20, +0x40, +0x10` for zones 0..9. The float at `record+4`
  is the scale used for the impact.
- Impact: `scale = record+4`, then `value = (impulse / strength) × frame_term × scale`, where the strength is read from
  `this+0x78`'s `+0x54` (or the `+0x58` term for the cap) and `FUN_0069C030` accumulates `value` into `this+0x88`.
  When the accumulator passes `1.0`, an event is raised (`FUN_00627400(0xC)` then `FUN_00625970`). This is a running
  accumulator, not the rewrite's per-hit `floor(impulse / strength × VisualScale)`.
- Discrepancy with the rewrite: the rewrite reads the damage record's first float (`VisualScale`, `0` for the trailer's
  side zones, `docs/specs/trailer-logs.md` §2), but the original reads the `+4` float of the vehicle's own per-zone
  record. That record is built from the damage specs at load time, so its values need not equal the raw damage record.
  Open: the writer of `this+0x78`'s zone records (the vehicle loader), and whether the trailer's side-zone `+4` values
  are non-zero.

## 16. The damage component and the zone table (decoded so far) [decomp]

- `FUN_006B9920` builds the `DamageVehicle` component (its string is passed to `FUN_006FBEA0`). It sets `this+0x70`
  with `FUN_006A5360(this+0x70, FUN_006A5700(this+0x34, 0))`, the damage-spec index of the vehicle.
- `FUN_006A5360` → `FUN_006963D0(…, index)` looks up the `damagespecs` collection (hash `0xC1F0B434`, computed with the
  AttribSys hash) and `FUN_00452380` makes a reference to it: a pointer to the collection object and a refcount. It
  stores no values. So the zone table holds references to the attribute objects, and the `+4` float is read from the
  referenced object.
- `FUN_0068CAB0(this, zone)` returns `*(this+0x78) + offset`, with offsets for zones 0..9 of `+8, +0x38, +0x18, +0x30,
  +0, +0x48, +0x28, +0x20, +0x40, +0x10`. `FUN_0069C030` reads the float at `+4` of that record as the zone's scale.
- Hypothesis (not yet tested): the classifier's zone numbers (0..9, from `FUN_0069C030`) do not match the rewrite's
  zone order (`DAMAGE_ZONES`: FRONT, REAR, LEFT, RIGHT, LFRONT, RFRONT, LREAR, RREAR, TOP, BOTTOM). The trailer has
  `VisualScale` 1 on LREAR and RREAR (the rear corners) and 0 on the pure side zones. If the original's classifier sends
  a side contact to a corner number, the logs release from side hits; the rewrite sends the same contact to a zone with
  scale 0. The test: map the classifier's numbers 0..9 to the DZ names from the offsets above, then check which zone a
  pure side contact lands in.
- Still open: the mapping from the offsets (`+8` … `+0x10`) to the DZ names, which needs the writer of `this+0x78`'s
  records. This is the next target.

## 17. The zone-name table and its chain (mapping step) [decomp + read_memory]

- The `DZ_*` names are not hashed in code (no AttribSys hash of any of the ten names occurs in `speed.exe`). They are
  string pointers. The strings: `0x008B5E38` = `DZ_LFRONT`, `0x008B5E44` = `DZ_RIGHT`, `0x008B5E50` = `DZ_LEFT`,
  `0x008B5E58` = `DZ_REAR`, `0x008B5E60` = `DZ_FRONT`.
- The static initialiser at `0x0088D770` builds a table at `0x009B3960`, 16-byte entries, from pairs of adjacent names:
  entry 0 = (FRONT, REAR), entry 1 = (REAR, LEFT), entry 2 = (LEFT, RIGHT), entry 3 = (RIGHT, LFRONT), entry 4 begins
  (LFRONT, …). Each entry stores `call(name_k)` at +0, `call(name_k+1)` at +8 and `name_k`'s pointer at +0xC. The table
  continues past entry 4 (not yet read: RFRONT, LREAR, RREAR, TOP, BOTTOM would follow in the same chain).
- The table is read by `FUN_00768BB0(out, index)` (copies one 16-byte entry), called from `FUN_0069C030`'s neighbour loop
  with the body index, not the classifier's zone number. So this is zone-neighbour data for multi-zone contacts, not the
  classifier's own numbering.
- Still open: the classifier's zone numbers 0..9 → `FUN_0068CAB0` offsets (+8, +0x38, +0x18, +0x30, +0, +0x48, +0x28,
  +0x20, +0x40, +0x10) → DZ names. The record at each offset is inside the vehicle structure at `this+0x78`, and its
  writer is the remaining target. The hypothesis in §16 stands and is untested.

## 18. The zone records at `this+0x78`: ruled out as the collision reactions

- `FUN_0069C030` has no direct callers: it is reached through the vehicle's vtable. Its `this` owns a car spec at `+0x78`
  (read at `+0x58` and `+0x60`, a speed threshold and a scale).
- The zone records at `+8 … +0x10` are not the collision reactions. The rewrite's `CollisionReactions` holds four
  records (front, front-side, rear, rear-side), but the classifier uses ten zone offsets. So the reaction blocks cannot
  be the source of the zone scale.
- Still open: the writer of the ten zone records in the car spec at `+0x78` and their DZ names. The next step is the car
  spec's loader (the function that fills the struct whose pointer is stored at the vehicle's `+0x78`).

## 19. `this+0x78` is the `damagespecs` object (correction to §§16 and 18) [decomp]

- `FUN_0069C030` is a virtual method of the damage component: the four vtable slots that hold its address
  (`0x008AD2F4`, `0x008AD378`, `0x008AD3EC`, `0x008AD6D4`) sit with the `DamageVehicle` vtables that `FUN_006B9920`
  writes (`0x008AD2A0`–`0x008AD2CC`).
- `FUN_006A5360(this+0x70, index)` calls `FUN_006963D0`, which sets the collection reference through `FUN_00452380`,
  and then, when the object pointer at `(this+0x70)+8` is zero, stores a default object there (`FUN_006269B0()`). That
  pointer is `this+0x78`. So `this+0x78` is the `damagespecs` object itself, not a separate car spec.
- So the zone records at `+8, +0x10, …, +0x48` are fields of the `damagespecs` object, at fixed offsets. The classifier's
  zone numbers select the field by offset: zone 0 → `+8`, 1 → `+0x38`, 2 → `+0x18`, 3 → `+0x30`, 4 → `+0`, 5 → `+0x48`,
  6 → `+0x28`, 7 → `+0x20`, 8 → `+0x40`, 9 → `+0x10`. `FUN_0069C030` reads the float at `field+4` as the zone's scale.
- Open (now precise): the offset-to-name table, i.e. which `DZ_*` field of the `damagespecs` class lives at each of those
  offsets. That is the class's attribute layout, and it decides the mapping. With it, the original's classifier zone can
  be compared with the rewrite's `DZ_*` lookup for a side contact.

## 20. The offset-to-name table, and which float the classifier reads (mapping step done) [install probe + decomp]

- Install probe `damagespecs_layout_offsets_of_the_dz_fields` (`trailer_logs.rs`): the layout offsets of the `DZ_*`
  fields of the `damagespecs` class are exactly the offsets `FUN_0068CAB0` returns:
  `DZ_LFRONT` 0x0 (zone 4), `DZ_FRONT` 0x8 (zone 0), `DZ_BOTTOM` 0x10 (zone 9), `DZ_LEFT` 0x18 (zone 2), `DZ_RREAR` 0x20
  (zone 7), `DZ_LREAR` 0x28 (zone 6), `DZ_RIGHT` 0x30 (zone 3), `DZ_REAR` 0x38 (zone 1), `DZ_TOP` 0x40 (zone 8),
  `DZ_RFRONT` 0x48 (zone 5). Each field is 8 bytes.
- So the classifier's zone numbers are these DZ names: 0 FRONT, 1 REAR, 2 LEFT, 3 RIGHT, 4 LFRONT, 5 RFRONT, 6 LREAR,
  7 RREAR, 8 TOP, 9 BOTTOM.
- `FUN_0069C030` reads `field + 4` (the second float, `HitPointScale`). Probe `damage_both_floats_of_a_car_and_the_trailer`:
  the second float is 0 for every zone of `BMWM3GTR` and of `TRAILERLOG`. The first float (`VisualScale`) is non-zero on
  cars (0.7 to 7.5). So `FUN_0069C030`'s accumulator is driven by `HitPointScale`, which is zero for all cars: it cannot be
  the path that sends the impact stimuli that release cargo. It is most likely the hit-point (visual damage) path.
- The sequencer's `IMPACT_n` level is therefore the other route, and it reads `VisualScale` (the first float), as the
  rewrite does. For the trailer, `VisualScale` is 0 on the side zones (LEFT, RIGHT, LFRONT, RFRONT, FRONT, TOP, BOTTOM),
  1 on REAR and on the rear corners (LREAR, RREAR). So under the sequencer's rule a contact releases the logs only when its
  zone is REAR, LREAR or RREAR.
- This narrows the open question to one point: what the original sends for a trailer side contact that the rewrite sends
  as zero. Either the level for a side zone is not `floor(impulse/strength × VisualScale)` (the rewrite's formula), or the
  side contact is classified as a rear zone by the original's classifier (zone 1, 6 or 7). Testable: the classifier's
  rule for the contact arm (sections 15 and 16) against a trailer side contact.

## 21. The remaining test, and why it is blocked

- The test: classify a trailer side contact with the original's rule (`FUN_0069C030`, sections 15 and 20) and see whether it
  lands in zone 1 (REAR), 6 (LREAR) or 7 (RREAR), the zones with `VisualScale` 1. The inputs are the trailer's collision
  half-extents (from `PhysicsData::car("TRAILERLOG").bounds`) and a side contact's arm and normal.
- Blocked: the C decompile of `FUN_0069C030` reuses one local for the contact arm and for a frame value (`pfStack_8`), so the
  branch conditions can't be read reliably from it. `get_function_pcode` returns about 1 MB for this function, too large to
  read in one pass. The next attempt is to read the p-code in slices, or to disassemble the branch tests directly with
  `disassemble_bytes` from the function's start.

## 22. The classifier's threshold test (disassembly, slice 1 of 4) [disassemble_bytes]

- Entry `0x0069C030`: the vehicle's `+0x78` struct is read at `+0x58` (compared with 0 at `0x00890968`), and the strength
  is read at `+0x60` and multiplied by `0x00894200` (1000.0) (`0x0069C0EF`..`0x0069C0F2`), then compared with 0 (`0x0069C0FC`).
  So the strength is `+0x60 × 1000` and the function returns early when it is not positive. This matches the decompile.
- The arm vector is transformed through the vtable at `+0xA4` (`0x0069C144`) and its z component is compared with half of
  the extent at `[ESP+0x30]` (`0x0069C14A`..`0x0069C15D`, multiplied by `0x008933D4` = 0.5).
- The decompile's later tests (`-0.8 <= fVar1`, `fVar1 <= 0.8`) reuse `fVar1`, which holds the strength at that point. The
  disassembly still has to confirm whether those compares load the strength or the contact normal. If they load the
  strength (a value near 20,000), the `0.8` tests are always true or false, so the zone choice would not depend on the
  normal at all. This is the open question that decides the side-zone result.
- Next: slices 2 to 4 (`0x0069C1C0` onward, about 900 bytes), reading which `[ECX+offset]` each FCOM loads at the `0.8`
  tests (`0x0069C173`, `0x0069C19E`).

## 23. The classifier, verified and run on a trailer side contact [disassembly + constants]

- Constants read from the image: `0x008AB588` = −0.8, `0x0089119C` = +0.8, `0x008933D4` = 0.5, `0x00894200` = 1000.0.
  The threshold tests load `[ESP+0x1C]` (the contact normal's lateral part), not the strength (which is in `[ESP+0x14]`).
  So the decompile in section 15 is right: the `±0.8` tests are on the normal.
- Gates: `SUPPRESS_DIST` (`+0x58`, the damage object's field at offset `0x58`) is compared with the distance from the view
  (`FUN_006ED550`), and the function returns when the impact is farther than that. `FORCE` (`+0x60`) × 1000 is the strength.
  So the classifier's own gate is the suppression distance, which `VehicleEvents` already applies (`suppress_distance`).
- Zones (axes as the classifier uses them: `x` lateral, `y` longitudinal, `z` vertical; `h*` are the extents):
  - `z` within ±`hz/2`: `y` within ±`hy/2` → zone 2 if `x ≤ 0`, else 3 (LEFT, RIGHT); outside that band → 8 or 9 (TOP, BOTTOM).
  - `z` below −`hz/2`: `x` within ±`hx/2` or `|normal| < 0.8` → 1 (REAR), otherwise 6 (LREAR) or 7 (RREAR).
  - `z` above `hz/2`: `x` within ±`hx/2` or `|normal| < 0.8` → 0 (FRONT), otherwise 4 (LFRONT) or 5 (RFRONT).
- Result for a pure side contact on the trailer (`x = ±hx`, `y ≈ 0`, `z ≈ 0`, normal lateral ≈ ±1): zone 2 or 3, LEFT or
  RIGHT, whose `VisualScale` is 0. So the original's classifier does not send a pure side contact to a zone that releases
  cargo. The rewrite sends the same contact to the same zone (0), so the two agree here.
- Conclusion: the zone choice is not the difference. Rear and rear-corner contacts (zones 1, 6, 7) release in both. For a
  side hit to release cargo in the original, something else must be in play: the arm or normal the caller passes in, or a
  step that is not in `FUN_0069C030` (the caller of the impact function is reached through the vtable; it was not read).
  The next test is that caller: who calls the impact function with the arm and normal, and what it passes.

## 24. Finding the classifier's caller through the vtable (blocked)

- `FUN_0069C030` is at the vtable slot `0x008AD2F4` (also `0x008AD378`, `0x008AD3EC`, `0x008AD6D4`). The table around
  it mixes several classes' pointers (`0x008AD2C0` onward is a run of unrelated function pointers), so the class is not
  obvious from the table alone.
- No data references to the nearby table addresses (`0x008AD2DC`, `0x008AD2E0`, `0x008AD2EC`, `0x008AD2F4`). The
  constructor that stores a vtable pointer was not found this way.
- Next: find the vtable base by the constructor that writes it (code that stores a table address into `[this]`), or search
  for call sites `call [reg+disp]` with the displacement of the slot. Either would give the caller.


Sections 25 to 41 are in [trailer-logs-trail-2.md](trailer-logs-trail-2.md).
