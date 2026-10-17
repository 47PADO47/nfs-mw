# Career save: how the profile is serialized

The byte-level layout of a PC save, block by block: the file container, the profile buffer and every block inside it, with
sizes, the in-memory source offsets and the checks the loader makes. It extends [saves.md](saves.md) (which has the
location, the MD5 and the community editor offsets). The behaviour that uses these fields is in
[career-flow.md](../specs/career-flow.md); addresses and method are in [career-flow provenance](../provenance/career-flow.md).

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) through the Ghidra MCP; the three saves on this
  machine (`NAME`, `mwo`, `stock`, 63,596 bytes each, read only, parsed with a throwaway script; no save bytes are copied
  here, only layouts and measured counts).
- **Evidence tags:** **[exe]** read in `speed.exe`, not tested in game. **[verified]** checked against the saves on this
  machine (all three unless stated). **[community]** from the editor offsets already in `saves.md`. **[unconfirmed]**
  a guess or a field whose use was not found.
- **Addresses** are `speed.exe` addresses. Ghidra auto names are only in the provenance file. All integers are little endian.
- "Settings" below means the career settings struct, which lives at `profile + 0xA8` in memory.

## 1. File container **[exe + verified]**

The PC memory-card layer writes `Documents\NFS Most Wanted\<alias>\<alias>` as three parts: a 0x1C-byte header, an 8-byte
descriptor, then the profile buffer. (`saves.md` called the 16 bytes at 0x10 "unknown" and listed 0x20 as a second copy of
the size; both are explained here.)

| File offset | Size | Field | Value on all three saves |
|---|---|---|---|
| 0x00 | 4 | magic `20CM` (`0x4D433032`) | |
| 0x04 | 4 | total file size = 0x1C + 8 + profile size | 0xF86C |
| 0x08 | 4 | size of the descriptor | 8 |
| 0x0C | 4 | size of the profile buffer | 0xF848 |
| 0x10 | 4 | checksum of the descriptor | `0x28A9C0BB` (the same in every save) |
| 0x14 | 4 | checksum of the profile buffer | varies |
| 0x18 | 4 | checksum of header bytes 0x00..0x17 | varies |
| 0x1C | 4 | descriptor word 0: profile type id | 0x10D |
| 0x20 | 4 | descriptor word 1: profile size again | 0xF848 |
| 0x24 | 0xF848 | profile buffer (section 2) | |

**The checksum** is a 32-bit CRC with the polynomial `0x04C11DB7`, processed most significant bit first, with a nonstandard
start: the first four bytes are read big endian and complemented to seed the register, the remaining bytes are shifted in,
and the result is complemented. It reproduces all three header words of all three saves **[verified]**.

**What the loader checks** **[exe]**: the magic, that the file length equals the stored total size, and the header
checksum at 0x18. The profile-buffer and descriptor checksums are computed and compared, but the code then resets the error
state to "ok" on the path that follows, so a wrong value in 0x10 or 0x14 does not by itself reject the file
**[exe, unconfirmed in game]**. The profile has its own MD5 (section 2.1), which is checked.

The profile type id 0x10D and the size are also compared by the screen code before the profile is parsed.

## 2. Profile buffer (`UserProfile::SaveToBuffer`) **[exe + verified]**

The buffer size is computed, not stored: `23,033 + 19 x (0x30 + 0x825) = 63,560` bytes (0xF848). 23,033 is the settings
block; 19 is the number of control-binding records; the rest is the sum of the other fixed blocks. There is a second, shorter
size formula (42,372 bytes) used when a second player's profile is loaded; it skips the settings and the playlist
**[exe, unconfirmed use]**.

Offsets are file offsets (buffer offset + 0x24). `Mem` is the offset in the in-memory `UserProfile` (heap object of
0x9CB8 bytes, `FEDatabase + 0x10`). The in-memory profile is laid out so that most blocks are copied straight from it.

| File offset | Size | Block | Mem | Notes |
|---|---|---|---|---|
| 0x0024 | 16 | version | none | zero filled; the MD5 range starts after it |
| 0x0034 | 23,033 | career settings | 0xA8 | section 3 |
| 0x5A2D | 4 | default stable hash | `FEDatabase + 8` | section 2.2 |
| 0x5A31 | 32 | profile name | 0x00 | zero padded text; the alias on disk is the same string |
| 0x5A51 | 240 | music playlist | 0x324 | 60 dwords |
| 0x5B41 | 132 | options | 0x24 | 33 dwords; byte at +0x14 is the autosave switch |
| 0x5BC5 | 36,040 | car stable | 0x414 | section 4 |
| 0xE88D | 1 | career completed at least once | 0x90DC | a flag that unlocks every car in the non-career modes |
| 0xE88E | 3,032 | statistics / pursuit records | 0x90E0 | section 5 |
| 0xF466 | 44 | selected quick-race car, 11 values | `FEDatabase + 0x2C + 0x18 x i` | stable handles, one per race type |
| 0xF492 | 54 | race settings | `FEDatabase + 0x134` | section 6 |
| 0xF4C8 | 916 | controller bindings | global table | section 6 |
| 0xF85C | 16 | MD5 | none | of file bytes [0x34, 0xF85C) |

Block ends line up exactly: the controller block ends at 0xF85C, where the MD5 starts, in all three saves **[verified]**.

### 2.1 MD5

MD5 over the profile buffer after its 16-byte version field, up to the digest. Standard MD5, no key. Matches on all three
saves **[verified]**. The settings blob (section 3.1) carries a second MD5 of its own.

### 2.2 Default stable hash and the load rule

The u32 at 0x5A2D is a checksum of the freshly built default stable (the 0x8CC8 bytes of section 4 as they are before any
career change). The game builds the default stable from the car database at start-up, so the value depends on the installed
car data. The three saves carry three different values. On load the settings are read first, then this value is compared
with the running game's; if it differs the load stops and the stable, name, options and the rest are not read
**[exe]**. A save from a different car set (another mod or a different install) therefore fails at this point.
The settings may already have been copied by then **[exe]**; the screen code restores a snapshot of the profile when a load
fails.

### 2.3 What a load does

`LoadFromBuffer` copies the blocks of the table above back to the memory offsets of the table. Two details **[exe]**:

- The settings **flags** word is merged, not replaced: `new = (old | stored) & ~0x800`.
- After the stable is copied, one of two unlock bits in the flags is set when a static "bonus content present" probe says so
  (bit 18, `0x40000`), and, on the main path only, bit 17 (`0x20000`) plus a pending popup entry of type 2 when the other
  probe says so. What the probes test (a table at `0x008F8408`) was not decoded **[unconfirmed]**.
- The first-player load also posts the `EATraxInit` event.

## 3. Career settings (23,033 bytes at file 0x34)

### 3.1 Gameplay-state blob, 0x4000 bytes (0x34 .. 0x4033) **[exe + verified]**

A snapshot of the gameplay manager (the object that owns the career progress). It is always exactly 0x4000 bytes; the rest
after the used part is zero (the used part ends at 0xC64 to 0xD54 in the three saves).

| Blob offset | Size | Field |
|---|---|---|
| 0x00 | 16 | MD5 of blob bytes [0x10, 0x4000); matches on all three saves **[verified]** |
| 0x10 | 4 | tag `Game` (`0x656D6147`) |
| 0x14 | 4 | version, 8; the loader requires 8 or more |
| 0x18 | 4 | number of saved gameplay objects N (2 in `NAME` and `stock`, 17 in `mwo`) |
| 0x1C | 4 | count of 0x20-byte slots, 8 |
| 0x20 | 4 | count of 0x10-byte entries, 30 |
| 0x24 | 4 | count of 20-byte entries A, 55 |
| 0x28 | 4 | count of 20-byte entries B, 39 |
| 0x2C | 4 | unused, 0 |
| 0x30 | 4 | number of bits in the bitfield, 0x200 |
| 0x34 | 4 | size of the zone block, 164 |
| 0x38 | 4 | count of the trailing dword list, 0 in all three saves |
| 0x3C..0x7F | | unused |

After the 0x80-byte header, in this order (every part after the objects starts on a 16-byte boundary):

1. **N gameplay objects**, each `{u32 key, u32 size, bytes[size]}` padded so the next starts at `(size + 0x17) & ~0xF` from
   its own start. The keys are name hashes of gameplay collections; resolved here: `race_bin_15` and `speedtraps` (the
   two objects every fresh save has, 129 and 20 bytes), and `race_bin_01`, `race_bin_03` and others in `mwo`
   **[verified]**. So these are per-bin / per-activity state snapshots **[unconfirmed meaning of the bytes]**.
2. **8 slots of 0x20 bytes**: two dwords, one byte, then up to 19 text bytes **[exe]**.
3. **30 entries of 0x10 bytes** (four dwords each) from a list in the manager.
4. **A: 55 entries of 20 bytes**, **B: 39 entries of 20 bytes**: two fixed tables of 20-byte records. Their meaning is not decoded.
5. **Bitfield**, 0x200 bits = 64 bytes (one bit per tracked flag).
6. **Zone block**, 164 bytes: a u32 count (20) then 20 records of `{u16 value of the zone field `0x6CE23062`, u16 4, u32 value}`.
   The field is the one `gameplay-races.md` notes on the safe house, car lot and shop zones **[exe]**.
7. **Dword list**, count from header +0x38.
8. **Career-start pair**: one u32 at the next aligned address, another u32 16 bytes later. They are the spawn-point
   collection hashes `DoSafeHouseIntro` sets (see [career-flow.md](../specs/career-flow.md) section 6) **[exe]**.
   Values seen: a plausible hash in two saves, small numbers in the third **[unconfirmed]**.

The blob is the persistent form of the Blacklist, milestone and event progress. Its byte-level meaning is for the
progress specs; this page only fixes the frame so a reader can skip or copy it as a unit.

### 3.2 Settings fields (after the blob) **[exe + verified]**

The fields are written in a different order from their memory order. `Set` is the offset in the settings struct.

| File offset | Size | Set | Field | Notes |
|---|---|---|---|---|
| 0x4034 | 4 | 0x00 | **current car handle** | handle of an FECarRecord in the stable (section 4). On a new career it is the hash of the preset name `M3GTRCAREERSTART`; `0x38A3C42C` |
| 0x4038 | 1 | 0x08 | **stage** | the Blacklist rank being challenged: 16 before the Blacklist (prologue), 15 down to 1. Values seen: 15, 1, 15 |
| 0x4039 | 4 | 0x0C | **cash** | matches the editor's money offset **[community + verified]** |
| 0x403D | 2 | 0x10 | unknown | 0 in two saves, 30,372 in `mwo` **[unconfirmed]** |
| 0x403F | 4 | 0x04 | **flags** | section 3.3 |
| 0x4043 | 600 | 0x12 | SMS table | 150 x 4 bytes |
| 0x429B | 2 | 0x26A | SMS counter | the next delivery sequence number |
| 0x429D | 16 | 0x26C | profile serial text | built from `MW-%d-%.4sXXX` at the first career start |
| 0x42AD | 4,804 | via manager | node state list | 4 + 300 x 16 bytes |
| 0x5571 | 456 | global | hint / tip counters | 57 x 8 bytes |
| 0x5739 | 756 | global | pending popup queue | 63 x 12 bytes |

The flags word is identical in all three saves (`0x001DF7F3`), including the fresh `stock` profile, so the saves do not
show which bit means what. Meanings come from the code (section 3.3).

**SMS table** **[exe + verified]**: 150 records `{u8 id, u8 state, u16 sequence}`. `id` is the record's own index, or 0xFF
when the language table has no `SMS_MESSAGE_<n>` string; 95 of 150 are valid in all three saves. `state` 0 = not delivered,
2 = delivered (set on delivery together with the next sequence number), 4 seen in `mwo` (80 records) **[unconfirmed:
read]**. `sequence` is the order of delivery.

**Node state list** **[exe + verified]**: a u32 count (255, 256 and 280 in the three saves; the cap is 300) then that many
`{u32 node hash, u32 state flags, u32, u32}`; the rest of the 4,800 bytes is zero. The nodes are the career graph's
event nodes (the game looks nodes up by the hash of ids like `16.1.0`). State flag values seen: 0x10 (most nodes in an early
save), 0x4, 0x14, 0xE, 0x1E, 0xD in a late one. Bit 1 (value 2) is tested when the list is saved to refresh a node
**[exe]**; the other bits are **[unconfirmed]**. The two last dwords hold two floats or hashes only on a few entries.

**Hint counters** **[exe]**: 57 entries `{?, u8 cooldown, u8 count, ...}`; a counter above 3 resets the entry. They drive which
loading-screen hint is shown next **[unconfirmed]**. Zero in a fresh save.

**Pending popup queue** **[exe + verified]**: 63 entries `{u32 type, u32 value, u32 active}` of rewards and notices waiting
to be shown after a race. Types seen in code: 1 to 0x10 shop part unlocks (brakes 1, engine 2, nitrous 3, induction 4,
chassis 5, tires 6, transmission 7, body kit 8, hood 9, spoiler 10, rims 11, roof scoop 12, hud 13, vinyls 14, decals 15,
paint 16), 0x11 a Get Out of Jail Free marker, 0x12 a car won (pink slip), 0x13 cash, 0x14 an impound box added,
0x15 a car released from impound; 0x14 and 0x15 are also used as plain notices. Empty in the fresh saves, 90 non-zero dwords
in `mwo`. A second, unsaved queue of 6 entries sits behind it in memory.

### 3.3 Flags word (settings + 4) **[exe]**

Bits found by reading the code that tests or sets them. A bit not listed was not found.

| Bit | Value | Meaning |
|---|---|---|
| 0 | 0x1 | the career has been started (the profile is not "new"); decides New Career or Resume |
| 1 | 0x2 | set when the Underground 2 cash prompt is shown |
| 4 | 0x10 | the Rap Sheet is unlocked (script hook `SetHasRapSheet`; adds the Rap Sheet button) |
| 5 | 0x20 | the opening sequence has been seen (read by `SkipCareerIntro`; set when the rank 15 reputation screen is shown) |
| 6, 7, 8, 9, 10 | 0x40 to 0x400 | set together by two routines, probably "tutorial / first-time hints seen" **[unconfirmed]** |
| 11 | 0x800 | cleared on every load |
| 12 | 0x1000 | set in a screen routine that touches the stable, unknown |
| 13 | 0x2000 | set by an availability routine, unknown |
| 14 | 0x4000 | unlocks cars `0x2CF370F0` and `0x03A94520` (the E3 demo BMW preset); set on the Game Won path |
| 15 | 0x8000 | unlocks car `0xCB6AAF2F` |
| 16 | 0x10000 | unlocks car `0x2CF385B2` |
| 17 | 0x20000 | bonus-content popup already queued |
| 18 | 0x40000 | unlocks car `0x34498EB2`; set from a bonus-content probe |
| 20 | 0x100000 | the scripted E3 demo BMW has been given (set by the add-car routine for preset `0x03A94520`) |

## 4. Car stable (36,040 bytes at file 0x5BC5; memory `profile + 0x414`) **[exe + verified]**

The stable is copied as one 0x8CC8-byte block, so file and memory layouts are the same. Offsets are from the stable start.

| Stable offset | Size | Part |
|---|---|---|
| 0x0000 | 200 x 0x14 | FECarRecord table (section 4.1) |
| 0x0FA0 | 75 x 0x198 | customization records (section 4.2) |
| 0x8728 | 25 x 0x38 | FECareerRecord table (section 4.3) |
| 0x8CA0 | 4 | **bounty base** (see the tail below) |
| 0x8CA4 | 2 | u16 counter A |
| 0x8CA6 | 2 | u16 counter B |
| 0x8CA8 | 16 | u16 x 8, infraction set 1 |
| 0x8CB8 | 16 | u16 x 8, infraction set 2 |

**Map to the editor offsets in `saves.md`** **[verified]**: money 0x4039 is settings +0x0C; name 0x5A31 is the profile name;
cars 0xE2ED = 0x5BC5 + 0x8728, the start of the career records (the editor's "cars" are the 25 career records, not the
200-slot table); bounty 0xE865 = 0x5BC5 + 0x8CA0; infractions 0xE86D = 0x5BC5 + 0x8CA8; pursuit records 0xF2BA =
0xE88E + 0xA2C (section 5).

The tail values are the totals of cars that are no longer in the garage. The game's totals add the live per-car values
(the sum over the owned career records, visited through a small visitor) to these bases **[exe]**. In the three saves the
bounty base is 0, 0 and 43,116,691, counter A 0, 0, 111 (`mwo` is a heavily played save).

### 4.1 FECarRecord, 0x14 bytes, 200 slots

| Offset | Size | Field |
|---|---|---|
| 0x00 | 4 | **handle**; 0xFFFFFFFF = free slot. A small slot number for ordinary cars; a name hash for the preset rides; 0x12345678 for slot 0 |
| 0x04 | 4 | frontend collection key (the car's FE data: price, unlock rank, class) |
| 0x08 | 4 | vehicle collection key (the physics car) |
| 0x0C | 4 | **flags**: low 16 bits = state, high 16 bits = class (below) |
| 0x10 | 1 | index of the customization record, 0xFF = none |
| 0x11 | 1 | index of the career record, 0xFF = none |

Flags, low half **[exe]**: 0x1 on sale in the car lot, 0x2 owned career car, 0x4 quick-race custom car ("My Cars"), 0x8 an
unlockable preset ride (the 14 rival cars and the bonus cars), 0x10 an ordinary preset ride, 0x20 not for sale / hidden,
0x40 marks a car as not counted (used in the owned-car count). High half: 0x10000 North America, 0x20000 Europe,
0x40000 Japan, 0x80000 the "vehicles" root class; the owned cars carry all four (0xF0000). The saves show a few
combinations the creation code would not produce (class 0x80000 together with 0x1), so the creation rule is not complete
**[unconfirmed]**.

How the default stable is built (the game rebuilds it from data on every start, then overlays the save) **[exe]**:
slot 0 is the start car (`M3GTRCAREERSTART`, handle 0x12345678, flags `0xF0020`); slots 1 to 30 are the 25 preset rides of
the `presetride` class plus related entries, flags `0xF0010`, or `0xF0008` for the unlockable ones; the rest are one slot
per vehicle collection that is flagged visible in the front end. Measured occupancy: 149, 100 and 142 occupied slots.
Owned career cars come after, with flags `0xF0002`; quick-race customs with `0xF0004`.

### 4.2 Customization record, 0x198 bytes, 75 slots

The first 0x116 bytes are 139 u16 part ids (0xFFFF = empty), one per part slot, stored through the game's part-id table.
A block at +0x118 holds the visual and tuning values (paint, vinyls, performance package). The last byte of the record
(+0x194) is 0xFF when the record is free. Index 0 to 30 belong to the preset rides. In the saves the owned cars use 31 and up
**[exe + verified]**. The block at +0x118 was not decoded.

### 4.3 FECareerRecord, 0x38 bytes, 25 slots

One per owned career car. Byte 0 is the record's own index (0xFF = free). Created by the add-car routine with: +2 = 3
(the impound strike limit), +3 strikes = 0, +4 state = 0, +5 countdown = 0, +6 decay = 0, everything else 0 **[exe]**.
Values seen in the saves **[verified]**: +2 is 3 in every record; +0x10 is a small number (100 to 400 in `NAME`),
which the busted spec treats as the car's bounty. The fields of the impound system are in the economy and busted specs
([busted-sequence.md](../specs/busted-sequence.md)); this page only fixes the frame. The strike decay and the
countdown are advanced after every race by two routines that walk the table **[exe]**.

## 5. Statistics block (3,032 bytes at file 0xE88E, memory `profile + 0x90E0`) **[exe + verified]**

Named "high scores" in `saves.md`; it is the career statistics and the Rap Sheet data.

| Block offset | Size | Part |
|---|---|---|
| 0x000..0xA0B | | zero in the three saves (not read by the code found) |
| 0xA0C | 4 | number of stored pursuit records (5 in `NAME`, 97 in `mwo`) |
| 0xA2C | 5 x 0x38 | the five most recent pursuit records. File 0xF2BA is the first one **[community + verified]** |
| 0xB44..0xBD7 | | lifetime counters, with the ten category totals read by the Rap Sheet at the offsets 0xB4C, 0xB50, 0xB5C, 0xBB4, 0xBB8, 0xBBC, 0xBC0, 0xBC4, 0xBC8, 0xBCC/0xBD0 |

A pursuit record starts with 8 text bytes (an id of the form `MW-<n>...`), then a u32, then a car key, then the pursuit's
numbers (cost to state, bounty, cops, helicopters, infractions, duration). The ten categories are pursuit length, cops
involved, cops damaged, cops destroyed, tire spikes dodged, roadblocks dodged, helicopters involved, cost to state, total
infractions and bounty, each in an "in this pursuit" and a "career total" form **[exe]**. Exact per-field offsets inside the
record and the counter block are open questions.

## 6. Smaller blocks

- **Race settings, 54 bytes** at file 0xF492: eight zero bytes, three bytes, ten dwords and three bytes copied out of
  an object at `FEDatabase + 0x134` (the quick-race / last-race selections). Two bytes are non-zero in all three saves
  **[exe + verified; meaning unconfirmed]**.
- **Controller bindings, 916 bytes** at file 0xF4C8: a u32 hash of the active input device's name, then 19 records of
  0x30 bytes copied to a global table. On load the records are applied; if the stored device hash differs from the current
  device the game resets the bindings **[exe]**. Two saves hold the same device hash, `mwo` holds 0xFFFFFFFF
  **[verified]**.
- **Options, 132 bytes**: 33 dwords; a fresh profile starts with a block of audio levels as floats (1.0, 1.0, 0.8, 0.8,
  then 1.0), 0xCD padding bytes and small enums. Byte +0x14 is 1 in all three saves and is the autosave switch read by the
  save routine **[exe + verified]**. The option meanings are in [frontend-menus.md](../specs/frontend-menus.md).

## 7. Writing a save from the rewrite (notes)

1. Build the profile buffer in the order of section 2; the size must be exactly 0xF848.
2. Settings blob first: compute its MD5 over [0x10, 0x4000) and write it at the start.
3. The default stable hash must equal the value the running game computes; a save written by the rewrite cannot be loaded
   by the original unless the car data is the same. Reading is the easier goal.
4. Compute the profile MD5, then the three header checksums (descriptor, profile, header).
5. The original saves by writing `<alias>/<alias>` in one pass (open, truncate, write the 0x1C header, the descriptor, the
   profile) **[exe]**. `libs/save-store` uses its own format and slots; this layout is for importing existing saves.

## Open questions

1. The meaning of the 55 + 39 twenty-byte tables, the 8 slots and the 30 entries in the gameplay blob, and the gameplay
   object bytes.
2. The settings u16 at +0x10 (30,372 in one save) and the flag bits 6 to 10, 12, 13.
3. The creation rule for the flag combinations seen in the saves (section 4.1).
4. The inner layout of a pursuit record and of the counter block (section 5).
5. What the two "bonus content" probes test (section 2.3).
6. Whether a wrong per-buffer checksum in the header really loads (the code path suggests so; not tested).
