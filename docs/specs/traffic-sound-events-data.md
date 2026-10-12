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

**The key is stored data (read).** The function containing `0x00A4E224` has entry `0x00A4DFD0` (`sub esp, 0x14`). Its two
callers, `0x00A4E5A6` and `0x00A4EBF0`, do not compute the key. They read it from a record: `0x00A4E54D` loads
`mov edi, [ebx+0x20]` (the key field of a descriptor), and `0x00A4EB8E` loads `ebx = [ecx]` from a list entry. So the 32-bit
Genesys type keys (such as `0x2EDB25E7`) are **values stored in the bundle records**, not outputs of a hash at run time.
That is why none of the hash tests matched. The role-2 car descriptor is therefore a stored record, and it can be read by
decoding the record layout (the `+0x20` key field and the record size), not by reproducing a hash.

**Record layout (read, partial).** The key `0x2EDB25E7` occurs in 80+ inflated `GLOBAL.BNDL` streams, mostly as a
field reference inside schema records. The smallest stream (file `0x2537D`, 144 bytes) is the schema of
`Genesys.Gen.MixerChannel.Priority`: a header of 32-bit words, the name string, then 16-byte entries of the form
`(key u32, 0 u32, value u32, 0 u32)`. At `+0x70` the entry is `key 0x2EDB25E7, value 4`, so the key is a field key with a
value. The role field of a `VehicleSound` instance is therefore an entry `(key, 0, value, 0)` whose key is the role field's
key. The field key for the role is not yet identified, and the instance streams that hold a `VehicleSound` with value 2
are not yet isolated.

**Instance search (read, negative).** All 73 occurrences of the `VehicleSound` key `0x2EDB25E7` in the inflated `SOUND`
bundles are followed by `(0, 4)`: the key is always paired with the value 4, which is a field kind (a reference to a
`VehicleSound`), not a per-car value. No occurrence is followed by the value 2. So a car's role is not stored as the field
that follows this key, and the per-car `VehicleSound` instances are stored under a different field key that is not yet
identified.

**Car-name search (read, negative).** A search of the inflated `SOUND` streams for car model names (BMW, Lamborghini,
Viper, Corvette, Mustang, Cop, RX7, Skyline, Carrera, Ferrari, Porsche, Audi, Mercedes, Subaru) returns only binary
fragments, with no readable car-type name. So per-car instances are keyed by hashed IDs, and the car identity needs the
hash-to-name mapping, which is not in these streams.

**Hash-to-name table search (read, negative).** The car type names are not in `NFS13.exe` as text (`BMWM3GTR`, `COPCAR`,
`FORDCOP`, `CHEVYCORVETTE`, `LAMBOGALLARDO` count 0; `TRAFFIC` count 4). Their `vlt_hash` values (`BMWM3GTR` `0x20D3071F`,
`COPCAR` `0xD4BF6F56`, `FORDCOP` `0x4E9792BC`, `TRAFFIC` `0x7DF94ADA`) do not appear in `NFS13.exe` or in any inflated
stream of `SOUND/`, `GAMELOGIC/` or `VEHICLES/`. So the car IDs are not `vlt_hash` values of the names, and the
hash-to-name table is not stored in these files.

**UTF-16 name search (read, negative).** The car names `BMWM3GTR`, `COPCAR`, `FORDCOP`, `CHEVYCORVETTE`, `LAMBOGALLARDO`,
`TRAFFIC` and `BMWM3` were searched in UTF-16LE in the raw files of the install's root, `SOUND/`, `GAMELOGIC/`, `VEHICLES/`,
`NFS13.exe`, and in every inflated stream of those bundles. **No occurrence.** So the car names are not stored as text in
either encoding, and the ID-to-name table is not in the files searched.

**Car names found on disk (read).** The vehicle bundles are named by vehicle ID: `VEHICLES/VEH_<id>_HI.BNDL`, `_LO.BNDL`,
`_TR.BNDL`. The vehicle ID in the gameplay data (`Players.LocalPlayer.GetVehicleInfo.ID(EQUALS:1085007)`) matches the file
`VEH_1085007_HI.BNDL`, so the file name is the link from ID to vehicle. `VEHICLELIST.BNDL` holds the ID `1085007` as a
little-endian `u32` at offset 4848 of the stream at `0xF76`.

A search of the inflated `VEHICLES/*.BNDL` streams for the `vlt_hash` values of real car names (taken from the repo:
`BMWM3`, `BMWM3GTR`, `COPHELI`, `COPLIGHTBLUE`, `COPLIGHTRED`, `COPLIGHTWHITE`, `CORVETTE`, `MUSTANGGT`, `PORSCHE911`) found
**one hit**: `MUSTANGGT` (`0xB8BBBF63`) in `VEH_122818_TR.BNDL`, stream offset `0x7747`. That stream is a 768,000-byte table of
repeated 32-byte records with float data (a vehicle part or light table), so the name hash is present but the record holds
no sound role. The sound role is still not located, so this is a name source, not the role source.

**`VehicleSoundSet` (read).** `VEHICLES/COMPETITORSOUNDS.BNDL` defines `Genesys.Gen.VehicleSoundSet` and its
`.Transmission` sub-type. Its schema stream (448 bytes, file offset `0x41F`) has field keys `0x0B3FE48A` and
`0xA32A93D6` (each repeated three times at offsets `0x5C`, `0x78`, `0x94` and `0xB0`, `0xCC`, `0xE8`, i.e. arrays of a
sub-record), plus the `VehicleSound` kind key `0x2EDB25E7` with value 4 and the sub-record key `0xB92285F2`. The vehicle ID
`122818` (MUSTANGGT) does not occur in the competitor sound streams. So the sound set is not keyed by the vehicle ID in this
bundle, and its instances (which would link a car to a set) are in other streams, not yet read.

**Instance streams of `VEH_122818_TR.BNDL` (read, partial).** Only `VEH_122818_TR.BNDL` exists for vehicle 122818 (no
`_HI`/`_LO` file). Its inflated streams contain Genesys data: 68 streams hold a generic `0x02000000` word, and a few hold
`Genesys.Gen` strings and the `VehicleSoundSet` field key `0x8A89E1A3` (streams at `0x1DA9`, `0x36CB`, `0x37AD`, `0x388F`,
`0x3A39`, `0x3B61`, `0x3C58`). The values after that key do not follow the `(key, 0, value)` pattern of the schema streams,
so the instance layout differs and is not yet decoded. The role-2 value is therefore not read from these instances yet.

**Stream at `0x3C58` (read).** This stream is a schema, not an instance: it names a `Genesys.Gen.VehicleSound` sub-type
(`...Position`) and repeats the kind-4 entries of `0x2EDB25E7`, with the key `0x8A89E1A3` as one of its fields. So the
vehicle bundle stores the schemas of the sound types. The instances (one per car) are in other streams of the same
bundle, which are not yet isolated. The role-2 value is still unread.

**Stream classification of `VEH_122818_TR.BNDL` (read).** The bundle inflates to 130 streams. Those that name a Genesys
type (`Genesys.Gen`) are schemas: 0x70702, 0xA0702 and 0xB0702 headers (8 streams). The other 101 streams of 64 bytes or more
have no type name, and they are the candidates for the per-car instance data. Their layout is not yet decoded, so the
role value is not read from them.

**Candidate-stream sample (read).** The 384-byte candidate at file offset `0x1E52` of `VEH_122818_TR.BNDL` is a binary table
with an internal offset sequence (`0x60`, `0x78`, `0x90`, `0xC0`, `0xD8`, `0x120`, `0x138`, `0x160`, ...) and float-like
words. It looks like a geometry or animation table, not a sound instance. So the unnamed candidates are at least partly
non-sound data, and the sound instances are still not identified.

**Screen of all `VEH_*` bundles for the `VehicleSoundSet` keys (read, not discriminative).** The keys `0x0B3FE48A`,
`0xA32A93D6` and `0x8A89E1A3` occur in 12,775 inflated streams of the vehicle bundles (`VEH_1085007_HI` alone has many).
Such a count means the keys are common values in the binary tables, not a sound-instance signature. So this screen does not
single out the instance streams, and a different signature is needed.

**Self-ID signature (read).** A vehicle's instance should carry its own ID. Searching each vehicle bundle's inflated streams
for the file's own ID as a little-endian `u32` gives: `VEH_122818_TR` 0 streams; `VEH_1085007_HI` and `VEH_1085007_LO` 2
streams each. The 336-byte stream at `0x499F1` of `VEH_1085007_LO` (ID at byte 92) is a table of bundle pointers
(`0x108E4F`, `0x108E50`, `0x108E79`, ...), so the ID there is incidental, and that stream is a resource table, not the
sound instance. The self-ID signature therefore does not identify the sound-instance stream either.

**Sub-record shape signature (read).** The schema shape `(key, 0, value, 0)` with a `VehicleSoundSet` key appears in 53
inflated streams of `VEH_1085007_HI`. The densest candidate, `0x101AC` (624 bytes, all three keys), names a
`Genesys.Gen.VehicleDamage…` type, so it is a schema. The shape therefore selects schemas as well as any instance data, and
it is not a discriminator for the instance streams.

**Schema cross-check (read).** The non-schema filter (drop streams naming `Genesys.Gen`) left one candidate in
`VEH_1085007_HI`, the 464-byte stream at `0x133AB`. Its name is split across the words (`Genesys`, `VehicleSound`,
`AnimationBehaviour`), so the filter missed it, and the stream is also a schema. With the name-split case included, **no
non-schema sub-record candidate remains in this bundle.** So the per-car instances are not in `VEH_1085007_HI`, and the
sound instances must be in a different bundle (not yet located).

**Vehicle-bundle cross-check (read, negative).** Across all `VEHICLES/VEH_*.BNDL` bundles, with schemas excluded by the
`Genesys` name (the name test is on the full word, so split names are caught), **zero bundles** contain a non-schema
sub-record candidate (the `(key, 0, value, 0)` shape with a `VehicleSoundSet` key). So the per-car sound instances are not
stored in the vehicle bundles in that shape. They must be in another bundle set or a different layout, which is not yet
located.

**`SOUND/` and `GAMELOGIC/` cross-check (read, negative).** The same non-schema sub-record test (the `(key, 0, value, 0)`
shape with a `VehicleSoundSet` key, schemas excluded by the `Genesys` name) was run on every bundle in `SOUND/`
(`GLOBAL`, `METALCAR`, `MIXER_CHANNELS`, `REVERB`, `SPEEDSOUNDS`) and `GAMELOGIC/` (including `WEAPONS/` and `HSM/`).
**Zero candidates in every bundle.** So the per-car sound instances are not stored under this shape in any bundle of the
`VEHICLES`, `SOUND` or `GAMELOGIC` sets. Their layout is different, and it is not yet identified.

**Type-key reference shape (read, negative).** The `VehicleSoundSet` schema header carries two candidate type keys,
`0x77B7F019` and `0xBB4E3D72`. Searching every inflated stream of `VEHICLES/` and `SOUND/` for either value found **no**
occurrence. So an instance does not reference its set by these keys, and the set's type key is not one of these words.

**Per-car list (read, partial).** `VEHICLES/VEHICLELIST.BNDL`, inflated stream at file `0xF76` (13,072 bytes), is a per-car
table with a fixed 112-byte record. Vehicle IDs appear at record starts (`122672` at byte 32, `122675` at 144, `122682` at 256,
...), each followed by a zero word. The rest of each record mixes small integers (for example `0x12`, `0x1FE`, `0x20D`,
`0x1A4`) and floats (dimensions and weights). `COMPETITORSOUNDS.BNDL` has no vehicle IDs with a small neighbour, so the
per-car link is in this list. The sound-set index is one of the small integers in the record, but the field is not yet
confirmed: it needs to vary across cars that use different sets, which is the next check.

**Cross-car comparison (read).** Across the 70 records of the per-car list (stream `0xF76`), word 25 of the record (byte
offset 100 from the ID) is the only categorical field: it takes five values, `18` (42 cars), `8` (11), `0` (11), `1` (5) and
`16` (1). Examples: `MUSTANGGT` (`122818`) = 8, `1085007` = 18, `122672` = 18, `122675` = 18. So field 25 is a candidate
class or sound-set index. **No car has the value 2**, the value the `BlewByCop` gate tests. So field 25 is either not the role
field, or the role-2 value is stored in another place. The role-2 mapping is still unresolved.

**Role-2 check over the per-car list (read, negative).** None of the 28 words of the 70 per-car records equals `2`, so no car
in this list has the raw value the `BlewByCop` gate tests. The role-2 value is therefore not stored in this table in that form.
Word 25 remains a categorical field with no role-2 value. The role-2 mapping is still unresolved.

**All per-car list streams (read, negative).** Thirty-five of the 51 inflated streams of `VEHICLELIST.BNDL` contain
ID-led records (vehicle ID followed by a zero word). The largest, stream `0x3B14` (24,736 bytes), has 103 ID records and 21
words equal to `2`. Those words sit at no consistent offset from the record ID (no offset holds more than three), so they are
incidental. No stream gives a per-car raw role field of 2. The role-2 mapping is still unresolved.

**Traffic attribute bundle (read, negative).** `VEHICLES/TRAFFICATTRIBS.BNDL` inflates to 638 streams, most of them Genesys
data. A search for vehicle-ID-like values with a raw `2` within 16 bytes found 153 matches, but 104 of them are the flag word
`0x00020001` (`131073`), which is a record flag and not a car ID. Of the remaining matches, only 11 are real vehicle file IDs,
and they occur once or twice each, at about the rate chance produces. So the traffic attribute bundle does not link a car ID
to a raw role value of 2 either.

**Role-2 field-key search (read, negative).** Every bundle of `VEHICLES/`, `SOUND/` and `GAMELOGIC/` was inflated and searched
for a `VehicleSoundSet` field key (`0x0B3FE48A`, `0xA32A93D6`, `0x8A89E1A3`) followed by the 16-byte pattern `(0, 2, 0)`, the
shape a per-car role value of 2 would take in the schema layout. **Zero occurrences.** So the role value is not stored as a
keyed field with value 2 in these bundles.

**Runtime check attempt (read, blocked).** With approval, `NFS13.exe` was launched from the install folder (PID 23832, running).
Attaching CheatEngine failed: the Cheat Engine bridge (v12/v99 pipe) is not reachable on this machine, so no memory was read.
The role-2 value remains unread. The check needs the Cheat Engine bridge running before it can attach.

**Runtime check, continued.** Cheat Engine was started (`cheatengine-i386.exe`, PID 11184) and attached to `NFS13.exe`
(PID 23832) successfully. But the addresses used in this document come from Ghidra's `speed.exe` analysis, and `NFS13.exe`
does not share that layout: the bytes at `0x4C0A90` in `NFS13.exe` are a different function, not the car-sim array
writer. So the car-sim array (`DAT_00913DB4`) is not at a known address in `NFS13.exe`, and the `+0x228` read cannot be
done yet. The next step is to locate the car-sim array in `NFS13.exe` (for example through the Genesys name strings in its
data), then read `+0x228` per sim.

**Genesys names in the running `NFS13.exe` (read).** Searching the attached process for the type names found
`Genesys.Gen.VehicleSound` at `0x0F8148D8` (with `.BurstTyre`, `.PlayerWithinVfx`, `VehicleSoundSet` and `.Transmission` nearby).
One writable pointer to the name exists at `0x0F8148D0`, which is a type record field, so the type record starts near
`0x0F8148C8`. Scans for pointers to that record (writable and default protection) found **no** instance references. So the
per-car instances do not hold a direct pointer to the type record in writable memory, and the car-sim array is still not
located.

**Owner-class name in the running process (read, negative).** The owner-class name `EAXCopCar` (from the `speed.exe` owner
table) is not present in the attached `NFS13.exe` memory, so the owner records cannot be reached through that name.

**Sim signature in the running process (read, inconclusive).** The constant tail of the car-sim constructor (`+0x250` = `-1`,
`+0x254` = `0`, `+0x258` = `2`, bytes `FF FF FF FF 00 00 00 00 02 00 00 00`) matches 73 addresses in `NFS13.exe` memory.
Reading the `+0x228` role field at each match's address minus `0x28` gave `5` for one candidate (`0x672CE1D8`) and `0` for
four others. The layout offset is assumed, not verified, and the signature also matches non-sim data, so these values do not
yet identify the car-sim array or the role-2 car.

**Sim confirmation (read, negative on the sample).** The constructor writes `1` at sim `+0x1F0`, which is the signature address
minus `0x60`. Of four sampled signature matches (`0x672CE200`, `0x70BC1B2C`, `0x7217A1D0`, `0x766DB010`), all four hold `-1` at the
signature (as expected), but none holds `1` at `+0x1F0` (the values read were `5`, a pointer, a pointer and `4`). So the sampled
matches are not car-sim objects with this layout. The signature alone does not identify the sim array.

**Full sim-layout check (read, negative).** A single signature requiring `01 00 00 00` at `+0x1F0` and the `-1`/`0`/`2` tail at
`+0x250` (96 bytes apart) matches **zero** addresses in the running `NFS13.exe`. So none of the 73 earlier candidates has the
constructor's layout, and the car-sim object is not found with this layout. The `+0x1F0` and `+0x250` offsets come from the
`speed.exe` constructor, so the layout may differ in `NFS13.exe`, which would also explain the failure.

**Layout derived from `NFS13.exe` (read, negative).** `NFS13.exe` contains no `mov dword ptr [reg+0x250], -1` store (the bytes
`50 02 00 00 FF FF FF FF` occur zero times, and the `C7 8x` encodings do not match either). The `1` store at `+0x1F0` occurs
twice, but not beside the `+0x250` store. So the car-sim class in `NFS13.exe` does not use the `speed.exe` layout, and the
`speed.exe` offsets (`+0x228` role, `+0x250` tail, `+0x1F0` flag) cannot be used to read the running game. The role-2 value
needs a fresh analysis of the `NFS13.exe` car-sim class, which is not yet done.

**`NFS13.exe` has no `speed.exe` car-sound system (read, negative).** The car-sound owner and connection names are absent from
`NFS13.exe` in both forms: as text (`CarSoundConn`, `CarRenderConn`, `WorldBodyConn`, `EAXCopCar`, `EAXTrafficCar`, `EAXTruck`,
`BlewByCop` all count 0) and as their `vlt_hash` values (`CarSoundConn` `0x1DD80B70`, `EAXCopCar` `0x888A8B77`, `EAXTruck`
`0xD6FD9EA1`, `EAXTrafficCar` `0x88AAA41B`, none present). So the `BlewByCop` gate and the `CarSoundConn` chain traced in
`speed.exe` are specific to that build. The running `NFS13.exe` does not use them, which means the runtime check as planned
(reading `+0x228` of those sims) cannot identify the role-2 car in this build.

**The game with the car-sound system is `speed.exe` on the Desktop (read).** `C:\Users\PADO\Desktop\Need for Speed Most Wanted\speed.exe`
(6,049,792 bytes) contains the plain strings `CarSoundConn` and `BlewByCop`, which the install's `NFS13.exe` does not. This is the
build the Ghidra analysis covers. The runtime check therefore must attach to this `speed.exe`, not to `NFS13.exe` (PID 23832,
which is the wrong build for the `BlewByCop` gate).

**Runtime read of the Desktop `speed.exe` (read, in progress).** Attached to `speed.exe` (PID 36252). At the menu, the car-sim
count `DAT_00913DBC` reads `0` and the array pointer `DAT_00913DB4` is `0x00913DC0`, so no car sims exist yet. The sims are
created when a race or drive loads a car. The next read needs a car in the world, which requires starting an event in the game.

**Runtime check of the connection registry (read).** In the running `speed.exe` (PID 36252), the registry record at `0x00913BD8`
holds `0x1DD80B70` (`CarSoundConn`), the factory `0x004CC790`, and a link `0x008EEAF8`. This matches the static analysis.
The car-sound connection objects are created only when a car loads, so their instances remain unread until a race starts.

**`NFS13.exe` car-sound pass, step 1 (read).** The exact `Genesys.Gen.VehicleSound` string is absent from `NFS13.exe`, but the
shorter `VehicleSound` string (VA `0xF9C2A8`) has one code reference, at `0x42B860`. That site is a debug assertion (it pushes the
string and a line number `0x207` and calls the logger `0xACFF30`), not a loader. The routine at `0x42B870` is a generic keyed-table
lookup: it scans a `u32` key array at `[ecx+4]` (count at `[ecx]`) and returns the matching 12-byte entry at `base + index*12`.
Next: find the callers of `0x42B870` to see which table holds the car-sound types.

**`NFS13.exe` car-sound pass, step 2 (read).** The keyed-table lookup `0x42B870` has 11 callers (`0x4A5FFB`, `0x539D16`, `0x539D36`,
`0x57C4D4`, `0x57FEA0`, `0x581BF7`, `0x583B15`, `0x584FA4`, `0x5881E4`, `0x5882D4`, `0x5883C4`). The first, at `0x4A5FFB`, passes a key read
from the object's `+0x30` field and searches the table at `[edi+0x44]`. It then scales float parameters (`+0x5C`..`+0x68`) by the
per-entry value. So this is the audio-parameter lookup, keyed by the object's type key, and the car-type table is one of the 11
callers' tables. Next: identify which caller searches the car-type (sound-owner) table.

**`NFS13.exe` car-sound pass, step 3 (read).** The other ten callers of `0x42B870` build their table the same way: the table
header is `[object + [object+0x188] + 0x1C]`, a per-object indirection at `+0x188`, and the key is pushed by the caller. Callers
`0x57FEA0`, `0x581BF7`, `0x583B15`, `0x584FA4`, `0x5881E4`, `0x5882D4` and `0x5883C4` read the owner through `[+0x184]`/`[+0x28]`,
so these are nested sound objects. The car-type table is one of these, but which one is not yet identified from the call sites
alone.

**`NFS13.exe` car-sound pass, step 4 (read).** The nested-sound initializer at `0x57C120` takes a pair from its argument and
stores the first value at `+0x184` (the owner object) and the second at `+0x188` (the table), and it sets the flag at `+0x18`.
Its callers are `0x57E87B` and `0x5EFC4E`. Next: read those two callers to see which owner and table they pass, which identifies
the car-type table.

**`NFS13.exe` car-sound pass, step 5 (read).** Two callers of the nested-sound initializer `0x57C120`:
- `0x57E87B`: copies a list of 16-bit values from the caller's argument into the object at `+0x1870`, then passes the pair
  pointer from its own argument. The `u16` list is a per-object index table, not yet identified.
- `0x5EFC4E`: works on an array of 0x200-byte slots starting at `+0x69D30`, indexed by an `eax` value. It builds the pair with
  `0x57CA50` at `esp+0x24`, then calls the initializer. So the nested sound object is one per slot in this array.
Next: read `0x57CA50` to see how the pair (owner and table) is built, which identifies the car-type table.

**`NFS13.exe` car-sound pass, step 6 (read).** The pair builder `0x57CA50` loops over rows of 0x90 bytes (four rows, up to `0x240`),
and for each of four literal tags compares 2-byte chunks of the row's name at `[edi+0x82]`. The four tags, read as dwords at
`0xFAD4F8`, `0xFAD504`, `0xFAD510` and `0xFAD51C`, are `Fron`, `Fron`, `Rear`, `Rear`: front and rear position tags. The routine
writes the matched index into a 4-by-4 grid. So this is a front and rear wheel or axle position matcher, not the car-type table.
The car-type table is therefore not in the slot-pair path, and the nested objects under `0x57C120` hold part and wheel data.

**`NFS13.exe` car-sound pass, step 7 (read): the car-type table is `VehicleListEntry`.** The sound and vehicle strings of
`NFS13.exe` include the record type `VehicleListEntry` (VA `0xF9B0AC`, repeated per field), with the fields `Vehicle ID`,
`Livery Vehicle ID`, `Vehicle Name`, `Vehicle Description`, `Vehicle Name Image`, `Vehicle Short Name`, `Vehicle Image`,
`Vehicle Power`, `Vehicle Power RPM`, `Vehicle Year`, `Vehicle Tier`, and `Car Select Sound` (VA `0xF9B3F0`, with `CarSelectSound`
at `0xF9B404`). The list type is `VehicleListResourceType` (VA `0xF9B5E0`). The per-car list I decoded earlier
(`VEHICLES/VEHICLELIST.BNDL`, 112-byte records keyed by vehicle ID) is this table. So the car's sound set is the `Car Select Sound`
field of its `VehicleListEntry`, and the role value is read from that record. Next: locate the field offset of `Car Select Sound`
within the entry, and check it against the role-2 value.

**Field offset of `Car Select Sound` (read, not yet found).** No inflated stream of `VEHICLES/VEHICLELIST.BNDL` contains the name
`VehicleListEntry` or any of its field names as text, so the schema that gives the field order is keyed by hash or lives in
another bundle. The field offset of `Car Select Sound` within the 112-byte record therefore cannot be read from the strings. The
next step is to derive the field-name hashes of the `VehicleListEntry` fields (the `vlt_hash` and the base-40 forms already
tested) and match them against the record's key words.

**`VehicleListEntry` field hashes (read, negative).** The base-40 12-character hash from `NFS13.exe` (`0x009ED220`) was computed for
the field names `Vehicle ID`, `Vehicle Name`, `Vehicle Short Name`, `Vehicle Tier`, `Car Select Sound`, `CarSelectSound`,
`Vehicle Power`, `Vehicle Year`, `Vehicle Power RPM`, `Livery Vehicle ID`, `Vehicle Image`, `VehicleSound`, `SoundSet`,
`Sound Set` and `VehicleSoundSet` (low and high halves). None appears among the 32-bit words of the 70-record per-car table
(stream `0xF76`). So the field keys are not that hash of the field names.

**`VehicleListEntry` field offsets (read).** The registration table at `0x42ADF2`..`0x42B0xx` in `NFS13.exe` registers each field
with its byte offset in the record. The record is `0x70` (112) bytes, as in the list. The field `Car Select Sound` (name at
`0xF9B3F0`, alias `CarSelectSound` at `0xF9B404`) has offset **`0x28`** (byte 40), type `0xF`, count 1. The field `Vehicle Tier`
has offset `0x68` (byte 104), and `Vehicle Power` (or the preceding field) has `0x6A`, `0x6C`.

Checked across the 70 per-car records (stream `0xF76`): byte 40 (`Car Select Sound`) is **0 for 66 cars**, and the other four hold
`14691`, `14692`, `14693` and `14725`. So the sound reference is sparse, most cars use none, and the four non-zero values are the
car-select sound references. Byte 100 (word 25) remains the categorical field (values 18, 8, 0, 1, 16), so word 25 is not the
`Car Select Sound` field. The role-2 value is still not located in these records.

**`VehicleListEntry` field table (read, complete).** The registration calls at `0x42AE27`..`0x42B127` in `NFS13.exe` name each field
and its byte offset: `Vehicle ID`, `Livery Vehicle ID`, `Vehicle Name`, `GamePack ID` (`0x2C`), `Vehicle Description` (`0x0C`),
`Vehicle Name Image` (`0x24`), `Vehicle Short Name` (`0x10`), `Vehicle Image` (`0x1C`), `Online Selection Image` (`0x20`),
`PaintPackGroup` (`0x34`), `TopSpeed` (`0x38`), `Acceleration` (`0x40`), `Power` (`0x44`), `PowerRPM` (`0x6A`), `Year` (`0x6C`),
`Tier` (`0x68`), `CarSelectSound` (`0x28`), `ManufacturerId` (`0x14`), and the rollout fields (`0x48` to `0x60`). The record size is `0x70`.

None of these fields is named as a sound-owner role. The only sound reference in the entry is `CarSelectSound` at `0x28`, which is
zero for 66 of the 70 cars. The role-2 value is therefore not a field of the entry itself. It should sit in the sound set that
`CarSelectSound` points to, and the four non-zero references (`14691`, `14692`, `14693`, `14725`) are the candidate sets to check.

**`CarSelectSound` target values (read, negative for role 2).** The four non-zero `CarSelectSound` values (`14691`, `14692`,
`14693`, `14725`) occur 348 times across the inflated `VEHICLES/` and `SOUND/` streams (`14691` 222 times, `14725` 74, `14693` 35,
`14692` 17), including in the per-car list and in vehicle bundles such as `VEH_1085007_HI`. None of the occurrences has a raw `2`
within 16 bytes. So these references do not put a role value of 2 beside them, and the role-2 value is not in the sound-set
references in this form.

**Definition search for the `14691`-range values (read, negative).** The values `14691`, `14692`, `14693` and `14725` were searched in
every inflated `VEHICLES/` and `SOUND/` stream for an occurrence within 32 bytes of a Genesys object key (`0x2EDB25E7`,
`0xB92285F2`, `0x0B3FE48A`, `0xA32A93D6`, `0x8A89E1A3`). **Zero** definitions were found, so these values are not stored
beside the sound-set schema keys. They are references to a table whose entries are not in these streams.

**Remaining bundle sets (read, weak lead).** The `14691`-range values were searched in the inflated streams of the other 46 bundles
(`GAMELOGIC/**`, the root `*.BNDL` and `*.BIN`). Seven occurrences were found, all in `GLOBALEFFECTS.BNDL` (`14691` five times, `14692`
twice). The effects bundle is not a sound-set table, so these are most likely coincidental values. The table that defines the
`CarSelectSound` targets is still not located.

**`NFS13.exe` sound-set loader, step (read).** The string `_VEHICLESOUND` (VA `0xFAD464`) is used at `0x57BE31`. The routine near
`0x57BE00` builds a resource name: it copies a prefix (`_VEHICLESOUND` or the alternative at `0xFAD474`) and appends the caller's
string with `0x00EDA4D0`, then calls `0x00ACEEA0` with the result. So this is the vehicle-sound resource-name builder, the entry
point to the per-vehicle sound-set loader. Next: read `0x00ACEEA0` to see the resource it opens, and whether that resource holds the
`CarSelectSound` targets.

**`NFS13.exe` sound-set loader, step (read).** The resource-name builder `0x57BE00` passes its result to `0x00ACEEA0`, a thin
wrapper that calls the resource opener `0x00A1D640` with the same argument (`0x00ACEEB0` wraps `0x00A1D5C0` the same way). So the
chain is: `_VEHICLESOUND` name builder, then `0xACEEA0`, then `0xA1D640`. Next: read `0x00A1D640` to see which file it opens, and
whether that file holds the `CarSelectSound` targets.

**`NFS13.exe` sound-set loader, step (read): `0x00A1D640` is a case-insensitive CRC-32.** The routine folds uppercase ASCII to
lowercase (`0x41`..`0x5A` gain `0x20`) and runs the table-driven reflected CRC-32 (table at `0x010D0848`). The loader therefore finds
a sound set by the CRC-32 of its lowercased name, not by a file path. The `CarSelectSound` targets (`14691`, `14692`, `14693`,
`14725`) are then indexes that a CRC-32 lookup maps to. Next: list candidate lowercased set names, compute their CRC-32, and match
the results against the stored keys.

**CRC-32 key match (read, negative).** The case-insensitive CRC-32 of 45 candidate sound-set names (the car names `bmwm3`, `bmwm3gtr`,
`copheli`, `coplightblue`, `coplightred`, `coplightwhite`, `corvette`, `mustanggt`, `porsche911`, each alone and with the prefixes
`_vehiclesound_`, `vehiclesound_`, `_vehiclesound` and `vehiclesound`) was searched for in every inflated `VEHICLES/` and `SOUND/`
stream. **Zero hits.** The sound-set names are therefore not these car names, or the keys are stored in a form I have not found.

**Source of the sound-set names (read, not found).** Three places were searched for the names that the CRC-32 lookup needs:
- `NFS13.exe`: no lowercase sound or car set names (a lowercase search of sound and car words returned nothing).
- `VEHICLES/VEHICLELIST.BNDL` inflated streams: 49 readable strings, all Genesys or nitrous and upgrade terms; no car names.
- `GDFBinary_en_US.dll`: none of the car display names (`M3 GTR`, `Mustang`, `Corvette`, `Porsche`, `BMW`, `Traffic`) in ASCII or
  UTF-16. The word `Cop` appears twice in UTF-16.
So the sound-set names are not stored as readable text in any file searched. Their source is not located.

**Sound-set names in the running `speed.exe` (read, negative at the menu).** Cheat Engine searched the attached process for `vehiclesound`
and `_VEHICLESOUND` (case as the loader builds them). Both return zero addresses at the menu. The loader builds the lowercased set names
when a car loads, so this search needs a race running to be conclusive.

**Numeric sound-set name (read).** The name builder near `0x57BDxx` formats a number with the `%d`/`%u`/`%lld` conversion
(`0x00427F80`) and prefixes `_VEHICLESOUND`. So a sound-set name is the prefix plus a decimal number, and its key is the
case-insensitive CRC-32. Four prefixes (`_vehiclesound`, `vehiclesound`, `_vehiclesound_`, `vehiclesound_`) with numbers 0 to 29999
were tested against the inflated `VEHICLES/` and `SOUND/` streams. That gives 3,807 hits, from many numbers. **But the four
`CarSelectSound` targets (`14691`, `14692`, `14693`, `14725`) give zero hits** under all four prefixes. So the `CarSelectSound`
values are not CRC-32 keys of these names: they are IDs, and their sound sets are keyed differently.

**Function that builds the sound-set name (read).** The enclosing function of the `_VEHICLESOUND` name builder has a `sub esp, 0x338`
prologue at `0x0057BC70` (the bytes before it are padding). It has no direct `call` callers, so it is reached through a virtual slot
or a function pointer. Its `CarSelectSound` ID argument arrives on the stack (`[esp+0x35C]` at the name builder). The next step is to
find the vtable or table that holds `0x0057BC70` and the ID source for that call.

**The owner of the sound-set name builder (read).** The address `0x0057BC70` is stored once, at `0x00F9A3C4`, which is slot 0 of
a vtable at `0x00F9A3C4`. The constructor at `0x00425360` installs that vtable (`mov [eax], 0xF9A3C4`), together with sub-object
vtables at `+0x18` (`0xF99700`), `+0x28` (`0xF98EF8`), `+0x50` (`0xF98DB0`) and `+0x98` (`0xF98DAC`), and clears a block of 16 pairs from
`+0x428`. So the name builder `0x0057BC70` is the first virtual method of a large object that owns the sound-set lookup. Next: read that
object's other methods, and find which one passes the `CarSelectSound` ID.

**Sound-object vtable (read).** The vtable at `0x00F9A3C4` has 18 method slots: slot 0 `0x0057BC70` (the name builder), 1 `0x00573F70`,
2 `0x00531A70`, 3 `0x00531A80`, 4 `0x00531A90`, 5 `0x005719C0`, 6 `0x005719D0`, 7 `0x00531AC0`, 8 `0x00531AD0`, 9 `0x00531AE0`,
10 `0x00425620`, 11 `0x00A874D0`, 12 `0x00A81F90`, 13 `0x00A8CC20`, 14 `0x00425600`, 15 `0x00425610`, 16 `0x00A8B180`, 17 `0x00A8B200`.
Slot 18 onward is text, so the vtable ends at slot 17. The callers of slot 0 are indirect (`call [reg]`), and they are not yet identified.

**Classifying the indirect callers of slot 0 (read, method unreliable).** A byte scan for indirect calls (`FF 10`, `11`, `12`, `13`, `16`,
`17`) within 0x100 bytes of a `[reg+0x28]` load found 23 candidate sites. The four in the sound-code range (`0x570B9F`, `0x578158`,
`0x57EE0C`, `0x57FC01`) disassemble from mid-instruction offsets, so they are false matches, not real calls. A reliable classification
needs instruction-boundary disassembly of the whole binary, which this scan does not provide. The caller of slot 0 is still unidentified.
