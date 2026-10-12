# Traffic sound events: car-data search log

Companion to [traffic-sound-events.md](traffic-sound-events.md) §1.3. These are the search results for the car-descriptor
role field and the data formats of the install. Each entry records one check; negative results are kept so the search
is not repeated.

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
- **`bnd2` entry contents (read).** The inflated streams of `GAMELOGIC/GAMEPLAY.BNDL` are Genesys (`CgsResource`,
  `Genesys.Gen.*`) game-data resources: gameplay triggers, nitrous and heat parameters, pursuit and cop behaviour. The
  readable strings contain no car type name and no car-sound role. So the `bnd2` gameplay entries do not hold the car
  descriptors. The role-2 mapping still needs the record that the car-sound constructor reads, which is not located.
- **Other bundles (read).** Strings of every `GAMELOGIC` bundle: no car type or car-sound role (`HL_31208.BNDL` has 3,362
  strings, none car-related; `GAMEMODES.BNDL` has `CAR_ONLY` and `RequestTrafficRestart`). Strings of the `SOUND` bundles:
  `GLOBAL.BNDL` (24,578 strings) and `METALCAR.BNDL` (19,698) are Genesys sound data. They define the type
  **`Genesys.Gen.VehicleSound`** (with `.BurstTyre` and `.PlayerWithinVfx`), next to `WorldSound`, `MiscSound`,
  `CollisionSoundSet` and `MixerChannel`. The type names are present but the field names are not stored as adjacent strings,
  so the field schema is binary and not yet decoded. The per-car `VehicleSound` instances, which would hold the role,
  are the next thing to read.
- **Genesys type record (read).** In `GLOBAL.BNDL` (stream at file `0x22DA2`, inflated length 2,784), the type name
  `Genesys.Gen.VehicleSound` is followed by the type key `0x2EDB25E7`, then (key, offset) entries. The key `0xB92285F2`
  repeats five times at offsets `0x40`, `0x5C`, `0x78`, `0x94` and `0xB0` (stride `0x1C`), which reads as an array of one
  sub-record type. The key is **not** the `vlt_hash` of the name (`0x2D83EBD7`), nor CRC-32 (`0x1C100881`) or FNV-1/1a of the
  name, so the Genesys key function is still unidentified and the field names cannot yet be recovered.
- **Hash candidates rejected for the Genesys key (read).** Tested against `0x2EDB25E7` (type) and `0xB92285F2` (sub-record),
  on `Genesys.Gen.VehicleSound`, `VehicleSound`, `Genesys.Gen.WorldSound`, `BurstTyre`, `PlayerWithinVfx`, `WorldSound`,
  `MiscSound` and `Genesys.Gen.MixerChannel` in the original, lower and upper case: `vlt_hash`; Jenkins lookup2 with initial
  values `0`, `0xABCDEF00`, `0x9E3779B9`, `0xFFFFFFFF`, `0x12345678`, `0xDEADBEEF`; Jenkins lookup3 `hashlittle` with
  initial values `0`, `0xABCDEF00`, `0x9E3779B9`; CRC-32; FNV-1; FNV-1a. None matches. The key is not a plain hash of the
  type name in these forms.
- **Where the Genesys key is computed (read).** `speed.exe` has no `Genesys` or `Cgs` string (Ghidra `search_strings`
  returns none), and `GDFBinary_en_US.dll` in the install root has none either (`Genesys`, `CgsCore`, `CgsResource`,
  `VehicleSound` count 0). Both are stripped of names. So the key function can only be found by analysing the code
  itself. The DLL would have to be imported into the Ghidra project to read it, which changes the shared project.
  That import was denied by the permission classifier, so it was not done.
- **Constant scan of the DLL (read, read-only).** `GDFBinary_en_US.dll` contains none of the standard hash constants,
  in either byte order: CRC-32 polynomial `0xEDB88320`, FNV prime `0x01000193` and offset `0x811C9DC5`, the golden ratio
  `0x9E3779B9`, the Jenkins seed `0xDEADBEEF`, the `vlt_hash` seed `0xABCDEF00`, and the Murmur constants `0x5BD1E995`,
  `0xCC9E2D51` and `0x1B873593`. So the Genesys key is not a standard hash computed inside this DLL.
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


## Genesys name hash in `NFS13.exe` (read, read-only)

`NFS13.exe` (install root, 13.6 MB, image base `0x400000`) contains the Genesys type-registration strings
`Dynamically registered Genesys type` (VA `0x102BF04`, referenced from `0xA4E469`) and `Dynamically registered Genesys
property` (VA `0x102BF28`, referenced from `0xA4E20F`). The registration code near `0xA4E204` calls a name helper, then the
hash function at `0x009ED220`.

`0x009ED220` is a 64-bit base-40 string hash over **12 characters** (the loop runs at most three four-character blocks,
`edi = 0, 4, 8`, and pads short names with zeros). For each character `c`: `h = h * 40 + m(c)`, with
`m('_') = 39`, `m(c) = c - 0x54` for `c >= 'a'`, `m(c) = c - 0x34` for `'A' <= c < 'a'` (case-folded), `m(c) = c - 0x2D`
for digits, `m('/') = 2`, `m('-') = 1`, and `m(other) = 0`. The result is returned in `EDX:EAX` (high and low 32 bits).

Checked against the type key `0x2EDB25E7` and the sub-record key `0xB92285F2`: `VehicleSound`, `WorldSound`, `MiscSound`,
`MixerChannel`, `BurstTyre`, `PlayerWithinVfx`, `Genesys.Gen.` (the 12-character prefix), and the lo, hi, lo^hi and lo+hi
combinations of each. **No match.** The hash has 946 callers in `NFS13.exe`, so the Genesys registration call is one of them,
not yet identified. The type-name string `VehicleSoundResourceType` (VA `0xFA10B0`) has no absolute-address reference, so the
type table is built at run time from a data table, not from a direct push.

**Registry (read).** The Genesys types are kept in a hash set keyed by a 32-bit value: `0x00A51F10` (bucket `div` by the
bucket count, linear probe over empty `-1` slots) and `0x00A52430` (the same probe with a load-factor update). The key is
computed by the caller, not by these helpers. The registration function at `0x00A4DF00` calls `0x00A51F10` and `0x00A558A0`
(the grow/rehash path). Further 12-character slices of the names (first, last, middle, upper case) were rejected as well.
The caller that turns a type name into the 32-bit key is still unidentified.

**Callers of the registration function (read).** `0x00A4DF00` is called from `0x00A4E224`, `0x00A4E343` and `0x00A4E3C3`.
The enclosing function of `0x00A4E224` takes the registry key as an argument: at `0x00A4E0CD` it pushes `[esp+0x2C]` (the
key) into the registry lookup `0x00A52500`, and at `0x00A4E0FB` it stores the key through the output pointer. So the 32-bit
key is an argument to the registration function, and the caller that supplies it is the next link, not yet identified.
