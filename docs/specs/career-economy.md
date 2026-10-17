# Career economy: bounty, cash, infractions, impound strikes and the rap sheet

What a pursuit, a race win, a bust, a purchase or a sale does to the career numbers: the per-car record, the
profile statistics, the cash balance, the impound strike system and the screens that show them. The pursuit itself is
in [ai-pursuit.md](ai-pursuit.md) and [ai-pursuit-heat.md](ai-pursuit-heat.md); the arrest scene before the infractions
screen is in [busted-sequence.md](busted-sequence.md). Addresses to the code are collected in
[../provenance/career-economy.md](../provenance/career-economy.md). Sections 2 to 9 (flows, formulas, open
questions) are in [career-economy-flows.md](career-economy-flows.md).

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) through the Ghidra MCP; the install at
  `D:\Need For Speed Most Wanted Black Edition` (`GLOBAL/attributes.bin`, `gameplay.bin`, `FE_ATTRIB.bin`,
  `LANGUAGES/English.bin` + `Labels.bin`, the gpcore Lua bytecode inside `gameplay.bin`). Read with
  `libs/blackbox-attrib` and `libs/blackbox-text`. The community decompile was **not** re-read for this file; the
  existing specs cite it.
- **Evidence tags:**
  - **[exe]** read in `speed.exe` through Ghidra; not tested in game.
  - **[verified]** checked in install data (attribute values, strings table, script bytecode).
  - **[community]** taken from another spec in this repo, not re-checked here.
  - **[unconfirmed]** a guess, or a rule whose meaning is not proven.
- Hashes are the game's string hash (Jenkins lookup2, initval `0xABCDEF00`, see
  [../formats/attributes.md](../formats/attributes.md)); text labels use the other hash of `text.md`.

## 0. Summary

1. Every owned car has a **career record** (0x38 bytes) inside the player's **car stable**. It holds that car's bounty,
   its evaded and busted counts, two sets of eight infraction counters (unserved and served), its heat, and the impound
   strike state. **[exe]**
2. A finished player pursuit writes into the current car's record (`0x00411240`). The pursuit's infraction kinds are
   added to the **unserved** set for evaded and busted pursuits alike. The **bounty is banked only when the pursuit was
   not busted** (bounty is added to the car's bounty, the evaded count rises). A busted pursuit raises the busted count
   instead and banks nothing. **[exe]**
3. Bounty is a per-car number. The profile "total bounty" is the sum over the live cars plus a "sold history" total.
   There is no separate profile bounty counter. **[exe]**
4. Cash is one `u32` on the profile (`profile + 0xb4`). It rises on a career race win (the event's `CashValue`, every win),
   on a car sale (half the car's `Cost`) and from reward objects; it falls on purchases, fines and the impound release
   fee. Spending clamps at zero. **[exe]**
5. A bust opens the infractions screen. The fine is the sum over the car's **unserved** infractions of count times the
   `infractions` class amount. Paying cash adds one strike; reaching the car's strike limit, or not having enough cash,
   impounds the car. A Get Out of Jail Free marker removes this pursuit's infractions and the bust without a strike. **[exe]**
6. An impounded car counts down five ticks (one per evaded pursuit and one per career race win). After that a release fee
   (25 % of the car's `Cost`) or an Impound Release marker frees it. **[exe]**
7. `AwardPlayerBounty` (script hook `0x00612220`) adds to the car's bounty and then divides the sum by a static dword whose
   image value is 5. This cannot be the real behaviour; see [career-economy-flows.md](career-economy-flows.md) section 3.4. **[exe]**, **[unconfirmed]** for the runtime value.

## 1. State

### 1.1 Where it lives

| Object | Where | Notes |
|---|---|---|
| Front-end database | pointer at `0x0091cf90` | `+0x10` / `+0x14` = profile of player 0 / 1; `+0x12c` = mode flags (bit 0 = career) |
| Profile | allocated `0x9cb8` bytes | constructor `0x005992e0` |
| Career settings | `profile + 0xa8` | `+0` current car handle, `+4` flags, `+0xc` **cash** (`u32`), i.e. `profile + 0xb4` |
| Car stable | `profile + 0x414` | size about `0x8cc8`; see 1.2 |
| Pursuit history | `profile + 0x90e0` | statistics and records, flows section 5 |

Profile flag bits that matter here (`profile + 0xac`, **[exe]**): bit 4 (`0x10`) = the rap sheet is unlocked
(script hook `SetHasRapSheet`), bit 12 (`0x1000`) = the infractions screen has been shown before (the first time gives
markers, flows section 6.5), bit 11 (`0x800`) = game over (every car impounded).

### 1.2 Stable layout **[exe]**

| Offset | Type | Meaning |
|---|---|---|
| `+0x0000` | 200 x 0x14 | `FECarRecord`: `+0` handle (`0xFFFFFFFF` free), `+4` car type key, `+8` u32, `+0xC` flags (high word = category mask, low word bit 1 = owned), `+0x10` u8 customization index, `+0x11` u8 career record index or `0xFF` |
| `+4000` | 75 x 0x198 | customization records (byte at `+0x194` = `0xFF` when unused) |
| `+0x8728` | 25 x 0x38 | `FECareerRecord`; byte 0 = `0xFF` means unused |
| `+0x8ca0` | u32 | sold-history bounty |
| `+0x8ca4`, `+0x8ca6` | u16, u16 | sold-history sums of record `+0x14` (evaded) and `+0x16` (busted) |
| `+0x8ca8` | u16[8] | sold-history **unserved** infractions |
| `+0x8cb8` | u16[8] | sold-history **served** infractions |

Every stable total is the sold-history value plus the sum over the live owned records (a record is live when its
`FECarRecord` has a handle, a category mask, the owned bit and a valid record index). The visitor helper is `0x00580f40`.

### 1.3 `FECareerRecord` (0x38 bytes) **[exe]**

| Offset | Type | Meaning |
|---|---|---|
| `+0` | u8 | own index, `0xFF` = unused |
| `+2` | s8 | **strike limit** ("max busted"); initial 3; extra-strike markers raise it by 1 to a cap of 5 (static dword `0x008f4314`) |
| `+3` | s8 | **strikes** so far (never below 0) |
| `+4` | u8 | **state**: 0 free, 2 impounded (strike limit), 3 impounded (could not pay), 4 release available (countdown over) |
| `+5` | u8 | **countdown**, set to 5 when impounded |
| `+6` | u8 | strike decay counter (counts evades while free) |
| `+0xC` | f32 | the car's **heat**; flows section 4 |
| `+0x10` | u32 | **bounty** of this car |
| `+0x14` | u16 | times **evaded** (label "TIMES EVADED") |
| `+0x16` | u16 | times **busted** (label "TIMES BUSTED") |
| `+0x18` | u16[8] | **unserved** infraction counters |
| `+0x28` | u16[8] | **served** infraction counters |

The state machine and its transitions are in flows section 6.

### 1.4 The eight infraction kinds **[exe]** + **[verified]**

A kind is a bit of the pursuit's infraction mask; its counter index is the bit position.

| Bit | Index | Kind | Fine (`infractions` class) | Reporter (bit raised at) |
|---|---|---|---|---|
| `0x01` | 0 | speeding | 150 | `0x006e9830` (state setter, rising edge) |
| `0x02` | 1 | racing | 350 | `0x006e9890` |
| `0x04` | 2 | reckless driving | 1000 | `0x006e9860` |
| `0x08` | 3 | assault (on a police car) | 350 | `0x00429cc5`, thunk `0x004060d0` |
| `0x10` | 4 | hit and run | 300 | `0x00429e78`, thunk `0x004060e0` |
| `0x20` | 5 | damage to property | 100 | `0x00429b4e`, thunk `0x004060f0` |
| `0x40` | 6 | resisting arrest | 300 | `0x004449a4`, thunk `0x004060c0` |
| `0x80` | 7 | off road | 75 | `0x0042f550`, thunk `0x00406100` |

The fine amounts are the one `u32` field of the eight `infractions` collections, found by hashing the names the
fine routine asks for (`speeding`, `racing`, `reckless_driving`, `assault`, `hit_and_run`, `damage`,
`resisting_arrest`, `off_road`). Values **[verified]**; they match [ai-pursuit-heat.md](ai-pursuit-heat.md) section 4.

The infraction manager (object at `0x0091e014`: `+0` mask, `+4` count) accepts a report only when the race context
(`race + 0x1960`) is 0 (no race event) or the current event has the `PursuitRace` flag. The first report of a kind
also updates the script variable `total_infractions` (number of set bits). **[exe]**
