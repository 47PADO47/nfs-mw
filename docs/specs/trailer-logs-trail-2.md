# Log trailer: the Ghidra trail, part 2 (companion to trailer-logs-trail.md)

## 25. The vtable and the call sites (narrowed, not finished)

- `FUN_006B9920` stores `&PTR_FUN_008AD2CC` as the `DamageVehicle` vtable. `FUN_0069C030` is at `0x008AD2F4`, which is slot
  `(0x2F4 − 0x2CC) / 4 = 10` of that table. The impact call is therefore `call [reg+0x28]` on a `DamageVehicle` object.
- Byte searches for `ff 50 28` (about 75 hits), `ff 51 28` (3), `ff 52 28` (about 100 hits) show the call sites are too many
  to check without knowing the object's register. Hits near the classifier (`0x0069C3DD`, `0x0069BB55`, `0x006A4060`,
  `0x006AFA2A`, `0x006B1FB8`, `0x006B2987`, `0x006B7195`, `0x006B739F`) are the first to read: they sit next to the
  damage code and are the likeliest callers.
- Next: for each candidate, the caller must load the `DamageVehicle` object into the register and pass the arm and normal
  (`FUN_0069C030`'s `param_1`, `param_2`) that describe the contact.

## 26. The classifier's caller (found) [disassembly]

- The caller is the function whose entry is `0x0069BAF0` (its prologue pushes `0x0087C3B0`). It takes the contact record
  `ESI` from its argument (`[ESP+0x64]`). When the contact state `[ESI+0x1C] & 7` is 2, it computes the arm into a local
  vector (`ECX = [ESP+0x20]`) through `FUN_006E9660(local, [ESI+0x7C])`, then calls the classifier.
- The classifier call is `call [EDX+0x28]` at `0x0069BB55`, where `EDX = [EBP−0x48]` is the `DamageVehicle` object (vtable
  slot 10 = `FUN_0069C030`). The pushes include `ESI+0x20` and `ESI+0x10` (pointers into the contact) and `[ESI+0x5C]`
  (the count that the classifier loops over, its `param_3`).
- So the arm is not taken straight from the contact point: it is `FUN_006E9660`'s output, computed from the contact's
  `+0x7C` input (most likely the contact position transformed into the body frame). This is the input the classifier sees.
- Rejected candidate: `0x006B1FB8` (`call [EAX+0x28]` on `[ESI+0x48]`, returns a float; it is not the classifier).
- Next target: `FUN_006E9660` (the arm transform) and the meaning of the contact fields `+0x7C`, `+0x10`, `+0x20`, `+0x5C`.
  The rewrite's arm (contact point minus body position, rotated into the body frame) must match `FUN_006E9660`'s output for
  the side-zone result to match.

## 27. Correction to section 26: `FUN_006E9660` is a handle, not the arm transform [decomp]

- `FUN_006E9660(this, p)` calls `FUN_00467C30(this, p, 0, 0)` and sets the `this+4` object, the same reference-building
  pattern as `FUN_00452380` (a handle to an object). It transforms nothing. So `[ESI+0x7C]` is a handle (a body or
  collection reference), not a position, and the caller does not compute the arm there.
- The classifier's first argument (`param_1`, the arm, read as three floats) is therefore the pointer `ESI+0x20` pushed at
  `0x0069BB40` (the `LEA EAX,[ESI+0x20]`). The arm is a vector stored in the contact record, written upstream by the
  collision code before this handler runs.
- Next target: the writer of the contact record's vector at `+0x20` (and `+0x10`, the second pointer, which the classifier
  receives as `param_6`). That writer is the place where the contact arm is computed from the contact point and the body
  position, and the rewrite's `DamageHit::capture` (`crates/nfsmw/src/scenes/world/events/damage.rs`) is the equivalent
  computation to compare against.

## 28. The classifier against the rewrite's `DamageHit::zone` (checked) [disassembly]

- The x (lateral) tests match the rewrite: zone 5 (RFRONT) and 4 (LFRONT) above the band, 7 (RREAR), 6 (LREAR) below
  it, using the normal's lateral component against ±0.8, as `DamageHit::zone` does (`side(...)`). Zone 0 and 1 are the
  middle cases in the same way. Checked for the x tests (`0x0069C15D`..`0x0069C23D`).
- The y (longitudinal) tests in the band compare the arm's `y` with half the `y` extent (`[ESP+0x2C]` × 0.5 against
  `[ECX+4]`, `0x0069C247`) and then the normal component in `[ESP+0x20]` against ±0.8 (`0x0069C251`, `0x0069C255`). The
  rewrite uses `normal.y` for these tests. The original's `[ESP+0x20]` is the first component of the normal after its
  transform, i.e. the normal's lateral (x) part, so the rewrite's y test may read the wrong component. The decompile's
  `fVar1` name for this value was misleading: it is not the strength.
- Effect: zones 8 and 9 (TOP, BOTTOM) would be chosen by a different normal component in the original. Side contacts are not
  affected: a pure side contact has `arm.y ≈ 0`, so it never reaches these tests and lands in zone 2 or 3 in both.
- So the classifier's zone for a trailer side contact is the same in the original and the rewrite (zone 2 or 3, scale 0).
  The side-zone difference is not in the zone choice. It must be in the trigger or in what the original sends for such a
  contact (section 23's open point).
- Hypothesis to test (not changed in code): the rewrite's `zone()` y tests should use the normal's lateral component, which
  matches the original's `[ESP+0x20]`. This changes top and bottom zones only.

## 29. The contact handler's class (next target, partly read)

- `FUN_0069BAF0` (the classifier's caller, section 26) has no direct callers: it is a virtual method. Its address is stored
  in five vtable slots at `0x008AB3A8`, `0x008AB3F8`, `0x008AB448`, `0x008AB498`, `0x008AB538` (spaced `0x50`), which points to
  one class seen through several sub-objects (multiple inheritance), not five classes.
- The handler takes the contact record as `param_1` and is called through a virtual slot. The caller is therefore a
  dispatch through one of those vtables. Next: the vtable base of the sub-object used by the collision code, and the call
  site `call [reg+disp]` with the displacement of the slot. The collision code writes the contact's arm and normal into the
  record before the call (section 27); that writer is the place to compare with `DamageHit::capture`.

## 30. The class of the contact handler (constructor found) [xrefs]

- The vtable at `0x008AB3A8` is preceded by the string `Effects` in the image, which suggests an effects class. Its slot 0
  is the contact handler `FUN_0069BAF0`.
- The constructor that stores that vtable address is `FUN_0069B900` (`0x0069B900`, 252 bytes). It has four callers:
  `FUN_0069BD80`, `FUN_006A7E20`, `FUN_006A82F0` and `FUN_006A84F0`. A second function (`FUN_0069BA10`, 168 bytes, six callers
  `FUN_0069BD70`, `FUN_0069BE00`, `FUN_006A79F0`, `FUN_006A82E0`, `FUN_006A83A0`, `FUN_006A85A0`) stores the same address.
- Next: the callers of `FUN_0069B900` (the four above) are the places that create the effects objects for a vehicle, and
  the collision code that passes contact records to them is in or near those callers (`FUN_006A7E20` and `FUN_006A82F0` are
  the first to read).
- `FUN_0069B900` is a base constructor. Its caller `FUN_006A82F0(param)` is a factory: it allocates `0xEC` bytes, runs
  `FUN_0069B900`, then sets a derived vtable (`0x008AB4F8`, with secondary tables `0x008AB4EC` and `0x008AB4E8`), so the
  handler `FUN_0069BAF0` is the base method that the derived class inherits. The factory's argument is the parent (the
  vehicle or world object). Next: the derived class's method that calls the base contact handler with the arm and normal,
  and who calls the factory `FUN_006A82F0` for a vehicle.
- `FUN_006A82F0` has no direct callers, so it is reached through a factory table (a function-pointer table indexed by the
  effect type). The next lookup is that table, not a call site. The chain from the collision code to the contact handler
  currently runs: collision code → factory table → factory `FUN_006A82F0` → derived effects object → base handler
  `FUN_0069BAF0` → classifier `FUN_0069C030`. The collision code's arm and normal writes are at the top of this chain and
  are not yet read.

## 31. The factory is a registry entry (static init) [bytes and xrefs]

- The factory `FUN_006A82F0` is stored in a global at `0x009382A0` by a static initialiser at `0x008881E0`
  (`mov [0x009382A0], 0x006A82F0`, then `mov [0x009382A4], eax`, and more of the same at `0x009382A8` onward). So the
  factory table is a registry of effect types, filled at start-up, not a call table.
- Direct references to `0x009382A0`/`0x009382A4` are writes only (the initialiser). The readers index the registry by an
  effect-type value, so their references are not direct.
- Next: the readers of the registry block (the base at `0x009382A0` minus the stride), found by the effect-type value that
  a collision uses. That lookup is the link between the collision code and the factory.
- Rejected reader: `FUN_0069DA90` references `0x00938298`, but as a float constant (`_DAT_00938298`), inside tyre-force
  physics (slip, friction, suspension terms). It is not the effect registry. The registry block's readers are still unknown.
- Direct searches for the registry base `0x009382A0`: the only match is the initialiser itself (`0x008881F9`). An indexed
  load of the base (`mov eax, [ecx*4+0x009382A0]`) does not occur. So the registry is read through a base pointer computed
  at run time, which direct byte searches cannot find. A dead end for this method.

## 32. Last attempt: the derived effects vtable

- The derived vtable at `0x008AB4F8` (set by the factory `FUN_006A82F0`) has its first slot at `0x008AB4F8` = `0x006A8370`.
  That address is the derived class's own method, most likely the override that calls the base contact handler
  `FUN_0069BAF0` with the arm and normal. Next: decompile `FUN_006A8370` and read which contact fields it writes before the
  call. That is where the arm and normal enter the chain.

## 33. The derived vtable's first slot is the deleting destructor (decompiled)

- `FUN_006A8370(this, flags)` calls `FUN_006A83A0(this)` (the destructor body) and, when `flags & 1`, frees the 0xEC-byte block
  (`FUN_005D0370(pool, this, 0xEC)`). It is the scalar deleting destructor, not the contact override.
- So the derived vtable's slot 0 says nothing about contacts. The contact dispatch is in another slot of the derived table
  (`0x008AB4F8` onward), or in `FUN_006A83A0`, which is the destructor body and may also reset the contact state. Next: read
  the other slots and `FUN_006A83A0`.

## 34. The derived vtable's other slots (one read)

- The derived vtable slot at `0x008AB514` (`0x0069BAC0`) is a component-attach method: it checks the event's type word
  against the global `0x0092C5E0`; when it matches, it finds a component of the parent (`[this+0x34]`, looked up with
  `0x004039C0`) and stores it at `[this+0x4C]`. It sets the parent link. It does not compute the contact arm or normal.
- Slots not yet read: `0x008AB4FC` (`0x00406600`), `0x008AB500` (`0x0068C280`), `0x008AB504` (`0x00400520`), `0x008AB508`
  (`0x00403BF0`), `0x008AB50C` (`0x00403C40`), `0x008AB510` (`0x0068C2A0`). The contact-facing method is one of them, or the
  base handler is reached from `0x0069BAF0` in a different call path.

## 35. The derived vtable, read in full where it matters

- `0x008AB4F8` (slot 0): deleting destructor `FUN_006A8370` (section 33).
- `0x008AB510` (`0x0068C2A0`): state getter. Reads the flag at `+0xDC` and the float at `+0xE8`. Not the contact method.
- `0x008AB514` (`0x0069BAC0`): component-attach (section 34).
- `0x008AB4FC`, `0x008AB500`, `0x008AB504`, `0x008AB508`, `0x008AB50C`: not read. `0x008AB500` (`0x0068C280`) is a thunk: it adds
  `0x90` to `this` and jumps to `FUN_006EEB10`, which releases the child object at `[this+0x3C]`, sets its flag and calls its
  vtable slot 1 (a stop and cleanup, not a contact method).
- Result: none of the derived slots read so far computes the contact arm or normal. The four unread slots point into the
  low-address library range (`0x00406600`, `0x00400520`, `0x00403BF0`, `0x00403C40`), which is most likely RTTI or
  destructor helpers. So the contact dispatch is probably not in this derived table; the base handler `FUN_0069BAF0` is
  called from a caller that was not found.

## 36. The derived vtable: all slots read (except one body)

- `0x00406600`: an "is active" query (returns false, or delegates to `0x006FC060` when `[this+0x18]` is set). Not contact.
- `0x00400520`: zero-filled data, so that vtable slot is unused, not a method.
- `0x00403BF0`: an empty no-op method (`RET 4`).
- `0x00403C40`: calls `0x004523C0` with two arguments and tests `[this+0x8]`. A generic initialiser; its full body is still
  unread, but it does not match the contact arm or normal.
- Conclusion: the derived effects vtable does not contain the contact computation. The base contact handler `FUN_0069BAF0` is
  called from a caller outside this vtable, which was not found in the vtable search.

## 37. The `call [reg]` search, narrowed to the effects range

- `call [eax]` (`ff 10`) hits in `0x0065`–`0x006B`: `0x0065CAE8`, `0x00661D5F`, `0x006681DC`, `0x0066B6CE`, `0x006701E3`,
  `0x00672257`, `0x00674228`, `0x0067BA1A`, `0x0067C018`, `0x0067DD32`, `0x0068279B`, `0x00686CA4`, `0x00686E7B`,
  `0x00687ED1`, `0x006A5065`, `0x006A50B5`, `0x006B457B`, `0x006B8E3B`, `0x006BA038`.
- `0x006A5065` (read): an intrusive-list walk that calls the first virtual of each entry with one argument (`push 1`). It
  passes no arm or normal, so it is a notify, not the contact dispatch.
- Not read: the other `ff 10`, `ff 11` and `ff 12` hits in the range. The contact dispatch has three arguments at least
  (the arm pointer, the body and the count), so a caller with that many pushes before `call [reg]` is the target.
- Read since §37: `0x006B8E3B` (a notify loop over `[this+0x6C]`/`[this+0x74]`, `push 1`), `0x006B8E52` (the same loop's second
  call), `0x006BA038` (a notify on `[this+0x18]`, `push 1`), `0x006B457B` (a list walk notify, `push 1`). All four pass a single
  `1` (a flush or notify flag) and no arm or normal, so none is the contact dispatch.
- Still unread: the `ff 10`, `ff 11` and `ff 12` hits at `0x0067`–`0x0068` (`0x0067DD32`, `0x00686CA4`, `0x00686E7B`, `0x00687ED1`,
  `0x00687DAE`, `0x0068279B`) and `0x006B0454`, `0x006B6F48`, `0x006697AA`. A call that passes the arm pointer and the normal
  pointer (three or more pushes) is the target.

## 38. End of the narrowed `call [reg]` search (effects range)

- Read: `0x0067DD32` (notify, `push 1`, after a pool free), `0x0068279B` (list walk notify, `push 1`), `0x00686CA4` (notify on
  `[this+0xA8]`, `push 1`), `0x00686CA4`'s neighbour `0x00686E7B` (notify, `push 1`, then clears `[this+0xA8]`), `0x00687DAE`
  (notify, `push 1`), `0x00687ECF` (notify, `push 1`, a `[esp+0x2c]` = 5 setup). `call [edx+0x10]` at
  `0x00686C8D` is a destructor-style call with one argument.
- None of these passes the contact arm and normal. Within `0x0067`–`0x006B`, every `call [eax]`, `[ecx]` and `[edx]` hit
  has now been read or is a notify, a free or a destructor call, except `0x006B0454`, `0x006B6F48` and `0x006697AA`, which are
  listed in §37 and not yet read.
- Conclusion for this narrowed search: the contact dispatch is not a `call [eax]`/`[ecx]`/`[edx]` with zero displacement in
  `0x0067`–`0x006B` (apart from the three unread hits). It may use another register or displacement, such as `[esi+0x44]`.

- Last three read: `0x006B0454` (`push edi`, one argument, then `call [edx]`: a match-and-visit that returns a bool, comparing
  `[esi+4]` with `[eax+4]`; not the contact dispatch), `0x006B6F48` (list notify, `push 1`), `0x006697AA` (notify, `push 1`,
  after `call [esi+0x2c]`).
- Zero-displacement forms (`ff 50 00`, `ff 51 00`, `ff 52 00`): `ff 50 00 00` occurs only in data (`0x00909BDB`); `ff 51 00 00`
  occurs in code only at `0x007E11CD`, outside `0x0067`–`0x006B`. So within the effects range, no zero-displacement virtual
  call matches the contact dispatch.
- End point of the search: every `call` through a register in `0x0067`–`0x006B` has been read, and none passes the arm and the
  normal. The contact dispatch therefore uses a non-zero displacement, a register loaded from memory in another way, or a call
  outside this range. That is the next search, and it is open.

## 39. Final pass: all register-indirect calls in the effects range (`0x0067`–`0x006B`)

- Zero-displacement register-indirect calls, all eight register forms: `ff 10`, `ff 11`, `ff 12` read (section 37 and 38),
  `ff 13` has no hit in the range, `ff 16` and `ff 17` have none in the range (the nearest hits are `0x0060D55E`,
  `0x0060D56F`, `0x006151BD`, below it). Every hit in `0x0067`–`0x006B` is a notify, a free, a destructor call or a
  match-and-visit (section 38), and none passes the contact arm and normal.
- Conclusion: within the effects range, the base contact handler `FUN_0069BAF0` is not called by any zero-displacement
  register-indirect call, and it has no direct callers. Its caller is outside `0x0067`–`0x006B` (for example in the collision
  code near `0x0060`–`0x0061`, or in a sub-object's table at a non-zero displacement). That search is the next and last step.

## 40. The `0x0060` record visitors (lead, read)

- `FUN_0060D1E0` and `FUN_0060D260` walk an intrusive list of records, select entries whose type word is 2 or 7 (`+0x10` of
  the record's collection entry), and call a virtual on the object (`call [eax]`) with a pointer into the record (`+0x14`,
  or `+0x14 + 0x10` for type 7). They are generic visitors, not the contact dispatch. Callers: `FUN_0061AE80` (for
  `FUN_0060D1E0`) and `FUN_0061AEF0` (for `FUN_0060D260`).
- Next: read `FUN_0061AE80` and `FUN_0061AEF0`, to see whether either is the collision code that passes the contact arm and
  normal into the visitor.
- `FUN_0061AE80` and `FUN_0061AEF0` (read): registration helpers. Each appends a type-7 entry (a callback and its parameter) to
  a list (via `FUN_00613F40`, `FUN_00618E90` and `FUN_0060BF90`). Their only caller is `FUN_0061E750`, a builder of generic
  record-visitor lists. So this branch is record infrastructure, not the collision code that passes the contact arm and normal.
  Next: `FUN_0061E750` (its callers, to see whether any is the collision code).

## 41. `FUN_0061E750` is the race-script API table (dead end)

- `FUN_0061E750` registers the script functions by name: `Randomize`, `Run`, `SetFlag`, `AddRacer`, `SpawnCop`,
  `NotifyCheckpointReached`, `NotifyLapFinished`, `ShowMessage`, `KnockoutRacer` and about 200 more. It is the race-script
  API. Its only caller is `FUN_006F9EA0`.
- So the `0x0060`/`0x0061` record visitors (section 40) belong to the script system, not to the collision code. The branch is
  closed: no contact arm or normal is passed in it.
