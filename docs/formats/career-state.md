# Career state in memory

Where the original keeps the career while the game runs: the profile, the car stable, the per-car career
records, the rap sheet counters and the progress tables of the script manager. This page is the shared layout
table; the behaviour that reads and writes these fields is in the career specs listed at the end. The
serialized form (save file order and offsets) is in [career-save.md](career-save.md) and
[saves.md](saves.md). For the tag meanings, see [evidence tags](../README.md#evidence-tags).

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) read through the Ghidra MCP; the
  four career specs and their provenance records below. Nothing here was tested in game.
- **Tags:** every layout is **[exe]** unless stated. Names of fields are ours; the original has none in the
  binary (the build is stripped).
- **Not here:** no decompiled code, no game data. Addresses are `speed.exe` addresses.

## 1. Roots

| Object | Address | Notes |
|---|---|---|
| Front-end database | pointer at `0x0091cf90` | `+0x10` and `+0x14` are the user profiles for player 0 and 1 (the accessors accept only 0 and 1) |
| Profile | heap block of `0x9CB8` bytes | constructor `0x005992e0`; `0x9b0c..0x9c24` and `0x9c24..0x9cb4` are zeroed there |
| Race status ("GRaceStatus" in the script registry) | `0x0091e000` | `+0x1960` state (0 = free roam), `+0x1964` mode (2 = career), `+0x1968` current race, `+0x196C` current bin; racers `0x194` bytes each |
| Script manager (milestone and speed-trap tables, stat map) | `0x0091e00c` | see section 5 |
| Bin view and race records | table at `0x0091e004` | see section 6 |

## 2. Profile fields used by the career

| Profile offset | Type | Meaning |
|---|---|---|
| `+0x90e0` | block | pursuit history: top-5 pursuits by bounty at `+0xa2c` (`0x38` each, a "MW-" case id), best pairs `{car, value}` x 10 at `+0xb64`, 8 extra sums at `+0xbb4` |
| `+0xa8` | struct | **career settings**, section 3 |
| `+0x414` | struct | **car stable** (`FEPlayerCarDB`), section 4 |
| `+0x9c24` | u32[8] | **rap sheet sums**, section 7 |
| `+0x9c44` | pairs | best pairs per rap sheet stat |
| `+0x9c94` | u32[8] | extra sums |
| `+0x9cb4` | u32 | key of the last car |

## 3. Career settings (profile `+0xA8`)

| Offset in settings | Type | Meaning |
|---|---|---|
| `+0x00` | u32 | current car handle (the key of the car record) |
| `+0x04` | u32 | flags, below (profile `+0xAC`) |
| `+0x08` | u8 | **CurrentBin**: 16 after the first career intro, then 15 down to 1 (profile `+0xB0`) |
| `+0x0C` | u32 | **cash** (profile `+0xB4`), clamped at 0 when spent |
| `+0x10` | u16 | not identified (file offset `0x403D`) |
| `+0x12` | 150 x 4 bytes | SMS inbox: `{id, state, u16 sequence}` per entry |
| `+0x26A` | u16 | SMS counter |
| `+0x26C` | 16 bytes | serial id |

The file packs these fields without padding (car handle at `0x4034`, bin at `0x4038`, cash at `0x4039`,
flags at `0x403F`); the in-memory struct is aligned. The 16 KiB gameplay-manager blob that comes before them in the
file is section 5 and [career-save.md](career-save.md).

Flags (`settings + 4`):

| Bit | Meaning |
|---|---|
| `0x1` | a career exists |
| `0x10` | the player has a rap sheet |
| `0x20` | the reputation overview was shown |
| `0x800` | game over |
| `0x1000` | the infractions screen was seen |
| `0x4000` | **career complete** (winning the last race); unlocks every rival challenge |
| `0x8000` | event 19.8.69 done |
| `0x10000` | every challenge-series race done |
| `0x20000` | first safe-house visit done |

## 4. Car stable (profile `+0x414`)

| Offset in stable | Size | Meaning |
|---|---|---|
| `+0x0000` | `FECarRecord[200]`, stride `0x14` | owned cars |
| `+0x0FA0` | `[75]`, stride `0x198` | customization records |
| `+0x8728` | `FECareerRecord[25]`, stride `0x38` | per-car career records |
| `+0x8CA0` | u32 | bounty of sold or removed cars |
| `+0x8CA4` | u16 | sold history of record `+0x14` (pursuits evaded) |
| `+0x8CA6` | u16 | sold history of record `+0x16` (times busted) |
| `+0x8CA8` | u16[8] | infractions **unserved**, sold history |
| `+0x8CB8` | u16[8] | infractions **served**, sold history |
| `+0x8CC8` | | end |

Stable totals are "sold history plus the sum over live records", computed by a visitor helper (`0x00580f40`).
The accessors: car by handle `0x0056ecc0`, career record by index `0x0056f120` (0xFF byte 0 = free), total
bounty `0x00590500`, total evaded `0x00590550`, total busted `0x005905a0`, infraction count by mask and set
`0x005903b0`, set total `0x00590430`.

### FECarRecord (`0x14`)

| Offset | Type | Meaning |
|---|---|---|
| `+0x00` | u32 | handle; `0xFFFFFFFF` = free slot |
| `+0x04`, `+0x08` | u32 | cleared on init (car type and related ids; not decoded) |
| `+0x0C` | u32 | flags; bit 1 = owned; the mask `0xF0000` selects a stable (a record counts only if both are set) |
| `+0x10` | u8 | `0xFF` on init (not decoded) |
| `+0x11` | u8 | index of the car's FECareerRecord, or `0xFF` |

### FECareerRecord (`0x38`)

| Offset | Type | Meaning |
|---|---|---|
| `+0x00` | u8 | own index; `0xFF` = free |
| `+0x02` | u8 | impound strike **limit**, starts at 3 (cap 5) |
| `+0x03` | s8 | strikes |
| `+0x04` | u8 | impound state: 0 free, 2 limit reached, 3 cash confiscated, 4 releasable |
| `+0x05` | u8 | countdown in events (starts at 5) |
| `+0x06` | u8 | decay counter (every third evade lowers the strikes) |
| `+0x0C` | f32 | heat |
| `+0x10` | u32 | **bounty** earned in this car |
| `+0x14` | u16 | pursuits evaded |
| `+0x16` | u16 | times busted |
| `+0x18` | u16[8] | infractions unserved (counter index = log2 of the infraction bit) |
| `+0x28` | u16[8] | infractions served |

The initializer clears `+0x03..+0x06`, `+0x08`, `+0x0C`, `+0x10`, `+0x14`, `+0x16` and `+0x18..+0x37`.
Meaning of the rules: [specs/career-economy.md](../specs/career-economy.md).

## 5. Script manager (`0x0091e00c`)

| Offset | Meaning |
|---|---|
| `+0x98` | stat map: nodes of 16 bytes `{u32 key, f32 current (-1 = unset), f32 best, u32 flags}`; flag bit 0 bigger is better, bit 1 per pursuit |
| `+0xA4` / `+0xA8` | milestone table (begin and end), entries of 20 bytes |
| `+0xAC` / `+0xB0` | speed-trap ("challenge") table, entries of 20 bytes |
| `+0xB4` / `+0xB8` | bounty-jump keys |
| `+0x108` / `+0x10C` | respawn hashes |
| `+0x264` | 64-byte flag set |
| `+0x2A8` | list |

Milestone entry (20 bytes): `u32` type key, `u32` collection key, `u8` state (1 locked, 2 active, 3 at stake,
4 done), `u8` flags (bit 0 bigger is better, bit 1 forced), `u16` bin, `f32` threshold, `f32` value.
Speed-trap entry (20 bytes): `u16` flags (bit 0 unlocked, bit 1 armed, bit 2 done), `u16` bin, `u32` collection
key, `u32` field `0x52DC742C`, `f32` threshold (m/s), `f32` recorded speed (m/s). Rules:
[specs/career-milestones.md](../specs/career-milestones.md), tables: [specs/career-milestones-data.md](../specs/career-milestones-data.md).

## 6. Bin view and race records (`0x0091e004`)

- Bin view: 20 entries of `0x1C` bytes; `+0x18` u16 = completed milestones plus speed traps of the bin,
  `+0x1A` u16 = completed events.
- Race records: up to 300 of 16 bytes at `+0x3C`: `u32` key (hash of the event id text), `u32` flags, `f32`
  best, two `u16`. Flags: 1 completed outside career, 2 completed in career, 4 available, 8 unlocked by a script.
  "Races won" counts records with flag 2; boss races are not counted.
- Counters are recomputed from these tables (`0x006010e0`), they are not stored as flags.

## 7. Rap sheet sums (profile `+0x9c24`)

`u32[8]` raw sums over **non-busted** pursuits: pursuit length, cops involved, cops damaged, cops destroyed,
tire spikes dodged, roadblocks dodged, helicopters involved, cost to state. The accessor `0x005999f0` also
answers index 8 (total infractions, both sets) and 9 (total bounty) from the stable. Thresholds of the 15 ranks
are in [specs/career-economy-flows.md](../specs/career-economy-flows.md).

## 8. Open

- The settings field at `+0x10` and flag bits 6 to 10, 12 and 13.
- Where the "career was completed at least once" flag is kept.
- The fields of `FECarRecord +0x04..+0x10`.
- Whether the divisor read by `AwardPlayerBounty` is 1 at run time (the file image holds 5):
  [specs/career-economy-flows.md](../specs/career-economy-flows.md).

## Related

[specs/career.md](../specs/career.md) (overview), [specs/career-economy.md](../specs/career-economy.md),
[specs/career-blacklist.md](../specs/career-blacklist.md), [specs/career-milestones.md](../specs/career-milestones.md),
[specs/career-flow.md](../specs/career-flow.md).
