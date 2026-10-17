# Career economy: flows, formulas and open questions

Continuation of [career-economy.md](career-economy.md) (state layouts and the infraction table). Same sources and
evidence tags. Code addresses are in [../provenance/career-economy.md](../provenance/career-economy.md).

## 2. A player pursuit ends: the order of events

Two pieces of code run when the cop manager removes a finished pursuit (`0x0043e8d0`). The second one is the career
recorder. **[exe]**

1. If the pursuit's slot at `+0x74` of the pursuit interface is clear (the pursuit was not busted, see 2.2), a listener
   method (`0x00411070`) runs for a player pursuit:
   - the "pursuits in a row" counter (object field `+0x50`) rises by one, and the script variables `pursuits_in_a_row`
     and `pursuit_evasion_time` (the pursuit duration) are written;
   - the gameplay manager is told "evaded" (`0x00602770`, argument 1);
   - the strike ticker for player 0 runs (flows 6.3);
   - a script event named `Event` is fired in free roam when a flag object exists;
   - the evade heat factor is applied to every car of the pursuing player (flows 4).

   For a busted pursuit the manager is told "not evaded" instead (argument 0) and none of the above runs.
2. The recorder (`0x00411240`) runs for every finished pursuit:
   1. Return at once when the current race event has the `ChallengeSeriesRace` flag (hash `0x1c650104`). Challenge series
      pursuits never touch the career record.
   2. Find the pursuing player (index 0 or 1) from the pursuit target. Anything else returns.
   3. Find that player's current car (`profile + 0xa8`), its `FECarRecord`, its career record.
   4. `bounty = rep_points_normal + rep_points_destroyed` of the perpetrator (interface slots `+0x44` and `+0x40`).
   5. Add the pursuit to the profile history (flows 5) with that bounty and the number of distinct infraction kinds.
   6. Add the pursuit to the record: the infraction mask (manager `0x0091e014`) goes into the **unserved** counters;
      then, if the pursuit was not busted, `evaded += 1` and `record.bounty += bounty`; otherwise `busted += 1` and
      nothing else.
   7. Tell the HUD side (a second perpetrator interface) the new **total** stable bounty.
   8. If not busted, post a message of kind `0x18` on the broadcast channel `0x20d60dbf` with (vehicle handle,
      **this car's** bounty). Note that `AwardPlayerBounty` posts the **stable total** in the same message
      (section 3.4), so the two senders disagree on the meaning of the value. **[unconfirmed]** which one listeners expect.

### 2.2 What "busted" is here **[unconfirmed]**

The recorder and the manager test slot `+0x74` of the pursuit interface (a byte at `+0xe8` of the interface object).
They treat a non-zero value as busted: the rap sheet labels `+0x14` "TIMES EVADED" and `+0x16` "TIMES BUSTED" and the
recorder raises `+0x14` exactly when the slot is zero. The only writer of that byte that was found stores a value derived
from the race context (zero only while a non-pursuit race event runs), which does not look like a bust flag. The slot
may be set from code that was not found. Until tested in game, model it as "the pursuit ended in a bust".

## 3. Bounty

### 3.1 Counters

| Counter | Where | Unit |
|---|---|---|
| Rep points of the running pursuit | two fields of the perpetrator (normal, destruction), read by slots `+0x44`, `+0x40` | points (rates in [ai-pursuit-heat.md](ai-pursuit-heat.md) 5.1) |
| Script variable `bounty_in_pursuit` | gameplay manager named float, set each update by `0x006013f0` | the sum of the two fields above |
| Car bounty | career record `+0x10` | points, banked at pursuit end |
| Stable total ("TOTAL BOUNTY", statistic 9) | sold history `+ sum of live records` | points |
| Best single-pursuit bounty | pursuit history, statistic 9 best pair | points |

There is no profile-level bounty counter other than the stable total. **[exe]**

### 3.2 Banking

Bounty reaches the career in exactly three ways: the recorder (section 2, evaded pursuits only), the script hook
`AwardPlayerBounty` (3.4), and the sold history (a sold car's bounty stays in the total). **[exe]**

### 3.3 Where scripts get their numbers **[verified]**

`gameplay.bin` scripts call `Game:AwardPlayerBounty(Bounty)` from the milestone progress handler and from the two speed
trap handlers. Typical values of the `Bounty` field: speed traps 4000 to 8000 (bins 15 to 10), milestone rewards from
500 to 220000 (values seen: 500, 5000, 8000, 11000, 16000, 19000, 24000, 30000, 34000, 35000, 46000, 60000, 70000,
76000, 80000, 90000, 94000, 100000, 114000, 136000, 140000, 180000, 220000), 100 for most race rows (200 collections).

### 3.4 `AwardPlayerBounty` and `GetPlayerBounty` **[exe]**

`GetPlayerBounty` returns the stable total for the player bound to the script (the same value as statistic 9).

`AwardPlayerBounty(amount)` as written:

```
rec = career record of the player's current car
rec.bounty = (rec.bounty + amount) / D          // unsigned integer division
post kind-0x18 message (player vehicle handle, stable total bounty)
```

`D` is read through a pointer stored at `0x008f5a50` that points at the dword `0x008f5a4c`, whose image value is **5**.
Findings:

- No instruction in the program writes `0x008f5a4c` or `0x008f5a50`; the only other reference takes the address inside
  the constructor of the gameplay manager (`0x005fa430`), a function dominated by integer-obfuscation arithmetic
  (mixed boolean masks and multipliers, and a trace hook). The data block around the value (`0x008f59f0` to `0x008f5a54`)
  looks like a node of a small set/add/subtract patch table. So `D` may be modified at run time by code that this
  analysis could not follow. **[unconfirmed]**
- Dividing the running total by 5 on every award would erase most of the bounty at each speed trap (awards of 4000 to
  8000) and make the milestone rewards meaningless. The pursuit-end path adds without any division.

The rewrite should therefore treat `D` as **1**: `rec.bounty += amount`. The factor of 5 is **[unconfirmed]** and
needs a run-time read of `0x008f5a4c` in the original to settle. Unit of the caller: points, the same unit as the
pursuit rep points. **[verified]** for the values in 3.3.

The hooks `StorePursuitRepForFE` and `SetRaceCompleteForFE` are empty (`ret`) in the PC build. `AwardCash` and
`AwardPoints` share one handler: it adds the amount to the **racer's score** in the race status (`0x005dc6d0`), not to
the profile cash. **[exe]**

## 4. Heat on the career record

Heat is the float at record `+0xC`. **[exe]**

- Written back when the player's car object is destroyed: `min(heat, 5.0)` (the 5.0 is the static float at `0x00890da4`;
  the value above it is replaced by 5.0). Applies in career mode only.
- Multiplied by a factor from the `pursuitlevels` collection `default` for **every** stable car (not only the current
  one) at three moments: pursuit evaded (layout offset `+0xa8`), career race won (`+0x84`), milestone completed
  (`+0x60`, no caller found). The `default` values given in [ai-pursuit-heat.md](ai-pursuit-heat.md) 3.3 are 0.95, 0.94,
  0.93; the pairing of offset to name follows the callers and is **[unconfirmed]** for the third.
- Set to 1.0 when the fine is paid in cash; set to 0 when a car is released from impound.

## 5. Profile statistics and pursuit history

The history object starts at `profile + 0x90e0`. **[exe]**

### 5.1 The ten statistics

| Idx | Name (the `frontend` collection of the same name) | Career total (`profile + 0x9c24 + 4 * idx`) | Source slot of the pursuit interface |
|---|---|---|---|
| 0 | `pursuit_length` | raw sum | `+0x18` (duration, rounded) |
| 1 | `cops_involved` | raw sum | `+0xa0` |
| 2 | `cops_damaged` | raw sum | `+0x94` |
| 3 | `cops_destroyed` | raw sum | `+0x98` |
| 4 | `tire_spikes_dodged` | raw sum | `+0xb8` |
| 5 | `roadblocks_dodged` | raw sum | `+0xa4` |
| 6 | `helis_involved` | raw sum | `+0x10` |
| 7 | `cost_to_state` | raw sum | `+0xe0` |
| 8 | `total_infractions` | computed: all unserved plus all served counters of the stable | pursuit's number of distinct kinds |
| 9 | `bounty` | computed: stable total bounty | the banked bounty |

Indices 8 and 9 are never stored in the raw array; the accessor (`0x005999f0`) computes them. Raw sums are updated only
for pursuits that were **not busted** (the history routine tests the same slot as the recorder). **[exe]**

### 5.2 History layout (relative to `profile + 0x90e0`)

| Offset | Meaning |
|---|---|
| `+0xa2c` | 5 entries of 0x38 bytes: best pursuits by bounty, sorted descending (a new one is inserted before the first entry with smaller bounty, the rest shift down, the fifth falls out) |
| `+0xb44` | `u32[8]` sums for indices 0 to 7 |
| `+0xb64` | 10 pairs `{car key, best value}` for indices 0 to 9 (best single pursuit) |
| `+0xbb4`..`+0xbd0` | 8 further sums (`+0xbb4` slot `+0xa8`, `+0xbb8` `+0xb4`, `+0xbbc` `+0xbc`, `+0xbc0` `+0xc0`, `+0xbc4` `+0xc4`, `+0xbc8` `+0xc8`, `+0xbcc` `+0xac`, `+0xbd0` `+0xb0`); meaning not matched to a label |
| `+0xbd4` | key of the car of the last pursuit |

Top-5 entry (0x38): `+0` case number text `MW-` plus 8 random characters alternating digit and capital letter (12 bytes
with the terminator), `+0xc` car key, `+0x10` bounty, `+0x14` duration, `+0x18` slot `+0xa0`, `+0x1c` `+0x94`, `+0x20`
`+0x98`, `+0x24` `+0xa4`, `+0x28` `+0xb8`, `+0x2c` cost to state, `+0x30` distinct infraction kinds, `+0x34` slot `+0x10`.

### 5.3 Rankings **[verified]** + **[exe]**

The rankings screen ranks each statistic against 15 thresholds (descending). The rank is the 1-based index of the first
threshold that is at most the value; below all of them the rank is 16. "Career" uses the raw sum, "best" uses the best
single pursuit; the `_in_pursuit` collections hold the thresholds for a single pursuit. Thresholds (rank 1 first):

| Statistic | Career thresholds |
|---|---|
| pursuit_length | 14400, 13200, 12000, 10800, 9600, 8400, 7200, 6000, 4800, 3600, 3000, 2400, 1800, 1200, 600 |
| cops_involved | 1200, 1000, 900, 800, 700, 600, 500, 400, 300, 200, 150, 100, 75, 50, 25 |
| cops_damaged | 500, 450, 400, 350, 300, 250, 200, 150, 100, 75, 50, 30, 20, 10, 5 |
| cops_destroyed | 250, 225, 200, 175, 150, 125, 100, 75, 50, 25, 20, 15, 10, 8, 5 |
| tire_spikes_dodged | 100, 90, 80, 75, 70, 65, 60, 55, 50, 40, 30, 20, 15, 10, 5 |
| roadblocks_dodged | 200, 175, 150, 125, 100, 90, 80, 70, 60, 50, 40, 30, 20, 10, 5 |
| helis_involved | 30, 26, 24, 22, 20, 18, 16, 14, 12, 10, 8, 6, 5, 4, 3 |
| cost_to_state | 3000000, 2500000, 2000000, 1500000, 1000000, 750000, 500000, 400000, 300000, 200000, 150000, 100000, 75000, 50000, 25000 |
| total_infractions | 500, 450, 400, 350, 300, 250, 200, 150, 100, 75, 50, 40, 30, 15, 5 |
| bounty | 10000000, 8000000, 6000000, 4000000, 3000000, 2000000, 1000000, 750000, 500000, 250000, 150000, 100000, 50000, 10000, 5000 |

Single-pursuit thresholds: `pursuit_length_in_pursuit` 1800, 1680, 1560, 1200, 900, 600, 540, 480, 420, 360, 300, 240, 180,
120, 60; `bounty_in_pursuit` 1500000, 1250000, 1000000, 750000, 500000, 300000, 200000, 150000, 100000, 75000, 50000,
25000, 10000, 5000, 1000 (the other `_in_pursuit` tables were not dumped). The rankings screen multiplies statistic 0 by
the static float `0.00025` before display; why is not understood **[unconfirmed]** (the thresholds compare with the raw
integer, so the raw unit is probably not seconds).

## 6. Busted consequences and impound

### 6.1 The infractions screen **[exe]**

Opened after a bust (busted-sequence.md). On construction:

- Sets profile flag `0x1000`. If it was not set before, and the player holds no Get Out of Jail Free marker (type
  `0x11`), **four** markers of that type are added to the first free inventory slots (inventory table `0x0091d928`, 12
  bytes per entry `{type, aux, 1}`). The strings table promises three in another place; the code adds four. A
  first-time help dialog is shown.
- `this_pursuit_fine` = fine of the pursuit's mask; `total_fine` = fine of the car's unserved counters.
- `fine = sum(unserved[i] * amount[i])` with the amounts of flows 1.4; result 0 when all counters are 0.
- Remembers `cash_at_open`.

Two choices:

| Choice | Effect |
|---|---|
| **Pay with cash** (confirm id `0xb8a7c6cd`) | `cash = (fine > cash) ? 0 : cash - fine`; `strikes += 1`; decay counter = 0; all unserved infractions move to served; heat = 1.0; then the outcome test below |
| **Use a marker** (confirm id `0xb8a7c6cc`) | one type-`0x11` marker is removed; this pursuit's infraction kinds are subtracted from the unserved set and `busted -= 1`; no strike, no cash change, heat unchanged; the other unserved infractions stay |

Outcome after paying: `limit_reached = strikes >= limit`. If `limit_reached`: state = 2, message
`MW_BUSTED_STRIKE_LIMIT`. Else if `fine > cash_at_open` (not enough cash): state = 3, message
`MW_BUSTED_INSUFFICIENT_FUNDS` ("your remaining cash has been confiscated"; the cash is already 0). Otherwise the
screen closes. The strike-limit case wins when both apply. Entering state 2 or 3 sets strikes = limit and the countdown
to 5. **[exe]**

The marker-only path never impounds. A car that is already impounded cannot be driven, so a bust happens only in free
cars. **[exe]**

### 6.2 State machine

| From | Event | To |
|---|---|---|
| 0 | cash payment reaches the strike limit | 2 |
| 0 | cash payment, fine larger than cash at open | 3 |
| 2, 3 | countdown ticks reach 0 | 4 |
| 4 | release by cash fee or Impound Release marker | 0, strikes = 0, countdown = 0 |
| 0 | pursuit evaded, decay counter reaches 3 | 0, strikes - 1 (floor 0) |

Release by cash costs `round(0.25 * Cost)` of the car (static float `0x008f4310`; `Cost` is the `frontend`
collection field at layout offset `0x4c`, for example `tt` 35000, `skylinezt` 150000). The fee is paid by
`SpendCash`; heat becomes 0. An Impound Release marker (type `0x15`) is consumed instead of cash. An Extra Impound
Strike marker (type `0x14`) raises the limit by 1 up to 5 for a car in state 0 whose limit is below 5. **[exe]**

### 6.3 Tickers **[exe]**

| Ticker | When | For each live career record |
|---|---|---|
| Evade tick (`0x00580a40`) | player 0 evades a pursuit | state 0: decay counter + 1; when it exceeds 2: strikes - 1, counter = 0; strikes below 0 become 0. State not 0: countdown - 1 if above 0; at 0 and state not 4: state = 4 |
| Win tick (`0x00580990`) | the player finishes first in a career race event (race mode 2) | state not 0: countdown - 1 if above 0; at 0 and state not 4: state = 4 and the script event `0x78` fires (the "released" SMS) |

The evade tick is hard-coded to player 0. So the five-tick countdown runs on evaded pursuits **and** race wins, and
strikes decay only on evaded pursuits. **[exe]**

### 6.4 Losing every car

When the car-select handler's check finds no car in state 0 (`0x005811a0` returns less than 1) and the player owns no
Impound Release marker, it sets profile flag `0x800` and shows the `GAME_OVER` text. Cash is not consulted by this
check; the strings table also has `SB_GAME_OVER` which mentions cash, so another check may exist. **[unconfirmed]**
No code deletes a car because of busts; impound is the only loss. A car in state not 0 cannot be sold
(`CS_NO_DELETE_IMPOUNDED_CAR`). **[exe]**

### 6.5 Markers **[exe]**

Marker inventory entries are `{type, aux, flag}`; the types seen are `0x11` Get Out of Jail Free, `0x14` Extra
Impound Strike, `0x15` Impound Release. Rewards of kind `0x13` are cash (section 7), kind `0x12` a marker/unlock.

## 7. Cash

Cash is the `u32` at `profile + 0xb4` (career settings `+0xc`). **[exe]**

| Flow | Rule |
|---|---|
| Spend | `cash = max(cash - amount, 0)` (`0x0056d7a0`); never negative |
| Career race win | when the finishing racer is placed first in a race with race mode 2: `cash += round(event.CashValue)`, **every** win (not only the first); first completion also grants the event's reward objects |
| Event reward of kind cash | `cash += round(reward.amount)` |
| Sell a car | `cash += Cost >> 1` (half the `Cost`, integer shift); the record's bounty, evaded and busted counts and both infraction sets move into the sold history first |
| Buy a car | `SpendCash(Cost)` |
| Part purchase | per cart item: `delta = item_price - credit`; if negative the player is paid the difference, else `SpendCash(delta)` |
| Release fee | `round(0.25 * Cost)` |
| Fines | section 6.1 |
| Start bonus | a flag-driven `+10000` (`0x0056d7c0`) |

`CashValue` per career bin **[verified]** (race rows of `gameplay.bin`; the dominant value for ordinary events, then the
other values in the same bin): bin 01 25000 (also 200000 for the epic pursuit); 02 20000; 03 18000; 04 16000; 05 14000;
06 12000; 07 10000; 08 8000; 09 6000; 10 5000; 11 4000; 12 3000; 13 2500; 14 2000; 15 1500 (also 5000). Some rows are 0
(pursuit-only events). `opm` rows 1000 to 10000, `collectors` 1500, challenge rows 0.

No cash cap was found in the code read. **[unconfirmed]** The static dword `9900` at `0x008f430c` has no reader.

## 8. Rap sheet screens **[exe]**

| Screen | Content | Data |
|---|---|---|
| `RapSheetMain.fng` | menu of the sheets | profile flag `0x10` unlocks it |
| `RapSheetVD.fng` (vehicle database) | one row per owned car: `CAR`, `BOUNTY`, `FINES` (fine of unserved counters), `UNSERVED` (count), status, `EVADED`, `BUSTED` | status text: impounded when state is not 0; "confirmed to drive" when busted count is not 0; else "suspected to drive" |
| `RapSheetRS.fng` | totals: bounty, fines due, fines paid, unserved, served, times busted | stable sums |
| `RapSheetUS.fng` / CTS | infractions by kind (`INFRACTION_TYPE_%d`), unserved or served toggle | per-kind sums |
| `RapSheetTEP.fng` | per-pursuit entries (top-5 table) and the "clean record" text | history `+0xa2c` |
| `RapSheetPD.fng` | one pursuit's detail | |
| `RapSheetRankings.fng`, `RapSheetRankingsDetail.fng` | section 5.3 | |

The UI glue functions are `0x0052ca00`, `0x0052cbc0`, `0x0052cf40`, `0x0052e750`, `0x00532b90`.

## 9. Rust implementation notes

- Model `CareerRecord { limit, strikes, state, countdown, decay, heat, bounty, evaded, busted, unserved: [u16; 8],
  served: [u16; 8] }` per owned car and a `SoldHistory` with the same sums. Totals are derived, never stored.
- The recorder and the two tickers are plain functions of (record, pursuit result); keep the evade-only bounty rule.
- Treat `AwardPlayerBounty` as `bounty += amount` (3.4).
- Cash is one `u32`; spending saturates at 0.

## 10. Open questions

1. Run-time value of the divisor `D` of `AwardPlayerBounty` (3.4) and what writes `0x008f5a4c`.
2. Whether slot `+0x74` of the pursuit interface is really "busted" (2.2).
3. Unit of `pursuit_length` (the `0.00025` factor, 5.3) and the meaning of the eight extra sums (5.2).
4. The sender of the vtable method `0x00411070`; the player index is fixed to 0 there (co-op/split screen?).
5. Which check produces `SB_GAME_OVER`; whether cash can ever block game over.
6. Message kind `0x18` listeners: per-car or total bounty.
7. The names of the `ChallengeSeriesRace` neighbours `0xa4e6fcfd` (bit 14), `0x637584fe`, `0x4393f69b` in the race parameters.
8. Where the first-time marker count 3 (strings) versus 4 (code) is reconciled.
9. Any cash cap (none found).
