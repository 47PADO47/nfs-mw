# Career milestones: pursuit goals per Blacklist bin

What the original calls "milestones" (also "challenges" in its scripts): small goals tied to a Blacklist bin that the
player completes in free-roam pursuits (pursuit length, bounty, cost to state, cops tagged, roadblocks, spike strips,
infractions, evasion time), plus the speed traps. This file is the rules: where the goals are stored, what tracks them,
when they complete, what they pay, what is saved, and how they gate the rival of a bin. The numbers (all 55 goals, the
39 speed traps, the per-bin requirements, the 30 tracked statistics) are in
[career-milestones-data.md](career-milestones-data.md). Where each rule was found is in
[../provenance/career-milestones.md](../provenance/career-milestones.md). The pursuit that produces the numbers is in
[ai-pursuit.md](ai-pursuit.md) and [ai-pursuit-heat.md](ai-pursuit-heat.md); the rival fight itself is not covered here.

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) through the Ghidra MCP and a whole-program
  decompile read as understanding only; the install at `D:\Need For Speed Most Wanted Black Edition` (`GLOBAL/attributes.bin`,
  `GLOBAL/gameplay.bin`, `LANGUAGES/English.bin`, the FEng packages). The community decompile was not read.
- **Evidence tags:**
  - **[verified]** checked in the install data (read with the Rust readers in `libs/blackbox-attrib` and
    `libs/blackbox-text`, or with the repo's `list-screens` command), or a name proven by hash equality.
  - **[exe]** read in `speed.exe` through Ghidra; not tested in game.
  - **[community]** taken from the existing repo specs, not re-checked.
  - **[unconfirmed]** a guess, or a rule whose use or name is not proven.
- **Addresses** are `speed.exe` addresses. Ghidra auto names are only in the provenance file.
- **Field names.** The gameplay database stores field names as hashes. Names marked "hash-verified" were recovered by
  hashing a name that the scripts or the exe use and getting the stored hash. Other fields are written as `0xXXXXXXXX`.

## 0. Summary

1. A **milestone type** is a collection of class `milestonetypes` (30 of them). It names a tracked statistic, says
   whether bigger or smaller is better, whether the statistic restarts every pursuit, and which text label to use.
   **[verified]** (data), **[exe]** (use)
2. A **milestone** is a leaf collection of class `gameplay` under the folder tree `milestones/bin_NN`: 55 in total,
   3 or 4 per bin, each with a bin index, a threshold and a bounty reward. **[verified]**
3. The game keeps one **statistics map** (30 entries: current value, best value, flags) and one **array of 55 milestone
   entries** (20 bytes each: type, collection key, state, flags, bin, threshold, value) in a gameplay manager object
   whose pointer is at `0x0091e00c`. **[exe]**
4. Every time a statistic is updated with a better value, a progress message goes out, and **in free roam only** each
   active milestone of that statistic is tested against its threshold. **[exe]**
5. A milestone has four states: 1 locked, 2 active, 3 "at stake" (threshold passed, not banked), 4 complete. Bins are
   unlocked (1 to 2) when the player reaches them. **[exe]**
6. Passing the threshold only shows a HUD message and moves the milestone to "at stake". It is **banked when the
   pursuit ends in an escape** and **falls back to active when the pursuit ends any other way** (busted). **[exe]**
7. Banking pays the milestone's `Bounty` as career bounty (rep points) to the current car, sends the reached message,
   recounts the bin, and in free roam opens the milestone rewards screen and autosaves. **[exe]**
8. Speed traps are a ninth kind: 39 entries in a second 20-byte array, completed by driving through the trap faster
   than its threshold, and counted together with the milestones. **[exe]**, **[verified]** (data)
9. The count of completed milestones plus speed traps of a bin is one of three conditions (races won, bounty,
   milestones) read by the script that unlocks the bin's rival (section 6). **[exe]**, **[verified]** (scripts)
10. State is saved in a 16 KiB block at the start of the profile data and cleared on a new career. **[exe]**

## 1. Data

### 1.1 Milestone types (`milestonetypes`, `GLOBAL/attributes.bin`) **[verified]**

Class hash `0xE4C3D904`, 4 fields, 30 collections (one per statistic, in `attributes.bin`, not in `gameplay.bin`).

| Field (hash) | Name | Type | Meaning |
|---|---|---|---|
| `0x9CA1C8F9` | `CollectionName` (hash-verified) | text, layout offset 0 | the statistic name, same as the collection name |
| `0x16DF392A` | `MilestoneType` (hash-verified) | i32, layout offset 4, default 1 | 1 or 3: bigger is better; 2: smaller is better. 3 is used only by `pursuit_length`; the code treats 1 and 3 alike when tracking |
| `0xAB4A196F` | unknown | bool, set on 9 of 30 | "per pursuit": the statistic is cleared at the start of each player pursuit (section 3.4) |
| `0xDB89AB5C` | unknown | i32, set on 8 of 30 | the text label index `NN` of `BLACKLIST_PURSUIT_MILESTONES_NN` and its `_SHORT` form; absent (0) on the others |

The 30 collections with their values and the code that updates each one are in the data file, table 1.
Only one type has "smaller is better" semantics in the data: `pursuit_evasion_time`. The label indexes used by milestones
are 0 (spike strips), 1 (pursuit time), 2 (evasion), 3 (bounty), 5 (cost to state), 6 (roadblocks), 7 (cops tagged) and
8 (infractions). Labels 4 (consecutive evaded pursuits) and 9 (speed trap) exist in the strings table but no milestone
collection points at label 4. **[verified]**

### 1.2 Milestones (`gameplay.bin`, vault `gpcore`) **[verified]**

The tree: collection `milestones` (hash `0x1D975142`) has the 15 folders `milestones/bin_01` to `milestones/bin_15` as
children (field `Children`, hash-verified). Each folder lists its milestones, and some folders also list a
`taggedcarmarker1`. A milestone is a leaf such as `milestones/bin_01/challenge_1_cops_damaged`, whose parent is a kind
template `milestone_<kind>` (8 of them), whose parent is the base `milestone`. The kind template carries the statistic:

| Template (`milestone_...`) | `MilestoneName` it sets | Label index |
|---|---|---|
| `pursuit_length` | `pursuit_length` | 1 |
| `pursuit_evasion_time` | `pursuit_evasion_time` | 2 |
| `cops_destroyed` | **`bounty_in_pursuit`** (the name does not match: this kind is a bounty goal) | 3 |
| `cost_to_state` | **`cost_to_state_in_pursuit`** | 5 |
| `roadblocks_dodged` | `roadblocks_dodged` | 6 |
| `cops_damaged` | `cops_damaged` | 7 |
| `total_infractions` | `total_infractions` | 8 |
| `tire_spikes_dodged` | `tire_spikes_dodged` | 0 |

Fields of a milestone leaf (inherited from the base `milestone` when not set):

| Field (hash) | Name | Type | Meaning | Read by the exe |
|---|---|---|---|---|
| `0x704F72E8` | `MilestoneName` (hash-verified) | text | statistic name; hashed to give the entry's type key | yes |
| `0x6CE23062` | `BinIndex` (hash-verified) | i32 | bin 1 to 15 | yes |
| `0x8445AF47` | unknown | f32 | **the threshold that completes the milestone** (seconds, points, counts) | yes |
| `0x8E1904C7` | `Bounty` (hash-verified) | i32 | reward in bounty points | yes |
| `0xDDF411F5` | `SpawnPoint` (hash-verified) | collection key | a tagged car marker (position and heading); the rap sheet map pin | yes |
| `0x3B9BBFC2` | unknown | f32 | a second, "harder" number (see below) | **no code reads it** |
| `0x006EC903` | unknown | f32 | a step (negative for evasion time) | **no code reads it** |
| `0x9E8910EF`, `0x3E9156CA`, `0x3E225EC1`, `0x9CA1C8F9` | generic collection fields | | common to all `gameplay` collections | |

The two unread numbers and the threshold look like a three-step ladder: the threshold is the first step, the "goal" the
last, and the step the spacing (for `pursuit_evasion_time` the threshold 120 steps by -30 down to the goal 60; for
`pursuit_length` in bin 1: 780, then 840). The numbers do not fit this for every row (bin 1 bounty: 850000, step
100000, goal 1000000), and the PC code completes at the threshold alone, so **the ladder is [unconfirmed]** and likely a
remnant of another platform. The rewrite should use the threshold only.

`SpawnPoint` is shared: bins 1 to 8 point at `milestones/bin_08/taggedcarmarker1` (map position 1226.2, -273.4, 108.6,
heading -13.0), bins 9 to 12 at `milestones/bin_12/taggedcarmarker1` (4503.0, 779.9, 56.1, heading -77.5), bins 13 to 15
at `milestones/bin_15/taggedcarmarker1` (1678.6, 2514.2, 150.3, heading 179.3). The one exception is the bin 10 bounty
goal, which points at a collection named `milestones/default_spawn_point`. **[verified]** The code only uses the key to
place a pin (section 5); that these are launch points of a pursuit is **[unconfirmed]**.

A template `milestone_challenge` (an activity with `MilestoneName`, `ThreshholdValue`, `Bounty`, `AutoStart`) and a
`milestone_monitor` activity exist with their script state graphs, but **no collection in the install has them as a
parent**, so they are not used by the shipped data. **[verified]**

### 1.3 Speed traps (`speedtraps/bin_NN`, vault `gpcore`) **[verified]**

Collection `speedtraps` (`0x49511906`) has 15 bin folders; the leaves `speedtrap1` to `speedtrap3` have parent
`speedtrap`. Fields used: `BinIndex`, `Bounty`, `ThreshholdSpeed` (hash `0xC3710777`, km/h, hash-verified), `SpawnPoint`
(a tagged car marker next to the trap) and a position. Bins 1 to 10 have 3 traps, bins 11 to 14 have 2, bin 15 has 1:
39 in total. Table 3 of the data file lists thresholds and bounty per bin. Each trap in a bin pays the same bounty.

### 1.4 Rival requirements (`race_bin_NN`) **[verified]**

Each bin collection `race_bin_NN` has three requirement fields, whose names are proven by hash and by the script that
reads them: `RequiredRacesWon` (`0xD617FEDC`), `RequiredBounty` (`0xD3657D92`) and `RequiredChallenges`
(`0x6DD4B98B`). Values per bin are in table 4 of the data file. "Challenges" here means milestones plus speed traps.

## 2. Runtime state **[exe]**

### 2.1 Objects

| Object | Where | Notes |
|---|---|---|
| gameplay manager | pointer at `0x0091e00c`, size `0x2D8` | created by the creator at `0x005fa430`, constructed at `0x005f7d80` |
| statistics map | manager `+0x98` (map header at `+0x9C`, count at `+0xA0`) | 30 nodes, keyed by the hash of the statistic name |
| milestone array | manager `+0xA4` count, `+0xA8` pointer | 55 entries of 20 bytes |
| speed trap array | manager `+0xAC` count, `+0xB0` pointer | 39 entries of 20 bytes |
| bounty-jump key list | manager `+0xB4` count, `+0xB8` up to 20 keys | collection `bounty_jump`, not a milestone list |
| race status | pointer at `0x0091e000` | `+0x1960` play mode (0 free roam, 1 race), `+0x1968` current race parameters (null in free roam) (names ours) |
| bin table | pointer at `0x0091e004` | 0x1C-byte bin views from `+0x24`, count at `+0x20`; `+0x18` u16 completed milestones and speed traps, `+0x1A` u16 completed events |
| profile current bin | player-0 profile (`0x0091cf90` +0x10) byte at `+0xB0` | 16 = none yet, then 15 down to 1 |
| viewed bin | global `0x0091ca3c` | the bin shown on the rap sheet screens |

### 2.2 Statistic node (16 bytes copied to the save)

| Offset | Type | Meaning |
|---|---|---|
| 0x00 | u32 | key: the collection key (hash of the name) of the type |
| 0x04 | f32 | current value, -1.0 = not set |
| 0x08 | f32 | best value, -1.0 = not set |
| 0x0C | u32 | flags: bit 0 bigger is better (the type's `MilestoneType` is not 2), bit 1 per pursuit |

### 2.3 Milestone entry (20 bytes, copied whole to the save)

| Offset | Type | Meaning |
|---|---|---|
| 0x00 | u32 | type key: hash of the milestone's `MilestoneName` |
| 0x04 | u32 | collection key of the milestone leaf |
| 0x08 | u8 | state: 1 locked, 2 active, 3 at stake, 4 complete |
| 0x09 | u8 | flags: bit 0 bigger is better (copied from the statistic), bit 1 forced (only the debug path sets it) |
| 0x0A | u16 | bin index |
| 0x0C | f32 | threshold (field `0x8445AF47`) |
| 0x10 | f32 | value recorded when it completed; 0 before |

### 2.4 Speed trap entry (20 bytes)

| Offset | Type | Meaning |
|---|---|---|
| 0x00 | u16 | flags: bit 0 unlocked, bit 1 armed, bit 2 done |
| 0x02 | u16 | bin index |
| 0x04 | u32 | collection key |
| 0x08 | u32 | a number read from field `0x52DC742C` (use not found) |
| 0x0C | f32 | `ThreshholdSpeed` converted from km/h to m/s (x 0.27778) |
| 0x10 | f32 | speed in m/s recorded when it completed |

## 3. Behaviour **[exe]**

### 3.1 Building the arrays

At manager creation (and again on a new career) the milestone array is built by walking the `milestones` tree depth
first through the `Children` lists and taking every collection whose parent chain includes `milestone`. The speed trap
array is built the same way from `speedtraps` and `speedtrap`. Each entry is then filled from the data: state 1, type
key from `MilestoneName`, bin from `BinIndex`, threshold from the threshold field, value 0, and the bigger-is-better bit
from the statistic. The statistics map is filled from the 30 `milestonetypes` collections with current and best at
-1.0. Entry order is the walk order; the save matches entries by collection key, so order does not matter.

### 3.2 Unlocking a bin

When the player's current bin changes (the career start moves from 16 to 15; a defeated rival moves it down), the
event that does it writes the profile byte and, for every bin from the old value minus one down to the new one, turns
each milestone of that bin from state 1 to 2 and sets bits 0 and 1 of every speed trap of the bin. Nothing turns a
milestone back to 1, so **milestones of older bins stay active and can still progress**. Progress is matched by type
key only, not by bin: one pursuit can advance the milestones of several bins at once. The rival-defeat flow is not
covered here.

### 3.3 Updating a statistic

Every producer (3.4) calls one routine with the statistic name and a new value. In words:

1. Look up the node by the hash of the name. Set its current value.
2. If the best value is set, compare: for a smaller-is-better statistic return when the best is less than or equal to
   the new value; for a bigger-is-better one return when the new value is less than or equal to the best. So only a
   better value goes on.
3. Store the new best.
4. Broadcast the progress message (3.5) with the name and the value.
5. Only if the race status object exists and the play mode is 0 (free roam): for each milestone entry in state 2
   whose type key equals the node key, run the threshold test (3.5).

An "increment" helper adds 1 to the current value (starting at 1 when unset) and calls the same routine.

### 3.4 Who updates which statistic

Only the statistics below have a producer in the PC executable; the other 12 types of table 1 have none (they are
unused data). "Free roam or pursuit race" means the play mode is 0, or the current race is a pursuit race (field
`PursuitRace`, hash `0x2B1F54F6`, true for the challenge races).

| Statistic | Updated | Address |
|---|---|---|
| `pursuit_length` | every pursuit tick of the player's pursuit, in seconds (the pursuit's accumulated time), while the pursuit is not in its cool-down; only in free roam or pursuit races | `0x00443CE7` |
| `pursuit_evasion_time` | once, when the player escapes: set to the value returned by the pursuit interface slot `+0x18` (taken to be the pursuit duration) | `0x004110D8` |
| `pursuits_in_a_row` | once, when the player escapes: the cop manager's escape counter (it counts up here) | `0x004110C2` |
| `bounty_in_pursuit` | each tick: pending rep points of the perpetrator (the normal counter plus the cop-destruction counter) | `0x006013F0` |
| `cost_to_state_in_pursuit`, `cost_to_state` | each tick: the pursuit's property damage value, and the perpetrator's cost to state | `0x006013F0` |
| `cops_destroyed` | running count of wrecked cops, when a cop is destroyed and removed; only in free roam or pursuit races | `0x0043E8D0` |
| `cops_destroyed_in_pursuit` | the pursuit's count of wrecked cops, on each wreck | `0x00418F30` |
| `cops_damaged` | +1 each time the pursuit is told a cop was damaged (for the player's pursuit) | `0x0040AF40` |
| `cops_in_pursuit`, `helis_in_pursuit` | the number of ground cops and of helicopters in the pursuit, when they change | `0x004190C0` |
| `total_cops_in_pursuit` | the pursuit's total cops joined so far, when a cop joins | `0x00423DF0` |
| `roadblocks_dodged` | +1 when the player passes a roadblock that was not yet marked dodged (the roadblock is behind the car, dot product below zero); a "Dodged" message is sent | `0x00439EE0` |
| `tire_spikes_dodged` | adds the number of spike strips of a dodged roadblock to the running value (only free roam or pursuit race) | `0x00439EE0` |
| `total_infractions` | the number of **distinct** infraction kinds committed in the pursuit (bit count of the infraction mask), when a new kind is added; only free roam or pursuit race | `0x005FDDA0` |
| `insurance_claims` | +1 from the collision handler | `0x004299B0` |
| `speedtrap_speed` | the km/h value of the speed trap crossing | `0x0060A7B0` |
| `heat_meter` | the heat value | `0x006F13F0` |

At the start of a player pursuit (the cop joins routine at `0x00423DF0`) the per-pursuit statistics (flag bit 1) have
both current and best set back to -1.0. The other statistics keep their values across pursuits and never reset except
on a new career. Milestone entries are not reset by this.

### 3.5 Threshold test, "at stake", progress message

The progress message `MNotifyMilestoneProgress` (type from `0x005DD5F0`) carries the statistic name and the new value. It
is broadcast on the subject `0x20D60DBF` (the subject used by the gameplay listeners, see
[busted-sequence.md](busted-sequence.md) section 2.1) and also reaches the scripts as a table with `MilestoneName` and
`ValueReached`. The race scripts that follow challenge races listen to it (section 4.3).

The threshold test of an active milestone:

- difference = threshold minus value; for a smaller-is-better milestone the sign is flipped;
- if difference is zero or negative, the milestone has passed: it moves to state 3 and an "at stake" event is created.

So bigger-is-better passes at value >= threshold, smaller-is-better at value <= threshold.

The at-stake event (class `EReportMilestoneAtStake`, constructor `0x00641C20`) shows a two-line HUD message: the short
title of the milestone type and the "milestone complete" text, in the HUD message style `0x8AB83EDB`; and it appends
the milestone to the pursuit summary list (at most 32), which the rewards screen shows later. **[exe]** The summary
list is emptied at the start of the player's pursuit.

### 3.6 Banking, losing, reward

- **Escape.** In the cop manager's end-of-pursuit routine for the player's escape (at `0x00411070`; it is not a function
  in Ghidra's analysis) the escape counter goes up, the two statistics `pursuits_in_a_row` and `pursuit_evasion_time` are
  set, and then **every milestone in state 2 or 3 is processed with "bank"**. The last two steps matter: setting
  `pursuit_evasion_time` can itself pass a smaller-is-better threshold, and the bank step that follows turns it straight
  into a complete one. Banking is the call at `0x00602770` with the flag set.
- **Bust or any other end.** When the player's pursuit is removed from the cop manager's list (`0x0043E8D0`), the same
  loop runs with the flag clear: milestones in state 3 go back to state 2 (not lost for good; state 2 ones are
  untouched). Because the escape routine runs before the removal, escaped pursuits have nothing left to revert.
  **[exe]**; that the removal runs after the escape routine is inferred from the order described in
  [ai-pursuit.md](ai-pursuit.md) section 6, **[unconfirmed]**.
- **Banking one milestone** (`0x006015B0`, only if the state is 3): the recorded value is the current value of the
  statistic (the threshold if the forced bit is set); the state becomes 4; the reached message `MNotifyMilestoneReached`
  is broadcast (name, value); the bin's counters are recomputed (3.8); the milestone's `Bounty` is paid with the same
  routine the scripts call as `AwardPlayerBounty` (`0x00612220`); and the `ChallengeCompleted` message is broadcast
  (`0x00605140`).
- **Payment.** `AwardPlayerBounty` adds the amount to a u32 at `+0x10` of the career record of the player's current car
  (`stable + 0x8728 + index x 0x38`, at most 26 records). The total shown as the player's bounty is the sum of that
  field over all cars plus `stable + 0x8CA0` (bounty of removed cars). The routine divides the new sum by a global that
  reads 5 in the file image (`0x008F5A4C`), which cannot be the shipped behaviour because every other reader
  uses the raw sum; **it is probably an integrity-check artefact, expected 1, [unconfirmed]**. A posted message gives the
  car handle and the new total to the HUD.
- **Free roam.** After banking, the routine at `0x00602770` creates an autosave event (class `EAutoSave`, name found
  next to its virtual table, **[unconfirmed]**) when the play mode is 0, and the escape routine then creates the
  `EShowMilestones` event (`0x00641FE0`), which fills the pursuit summary and pushes the package
  `PostRace_MilestoneRewards.fng`. The rows on that screen come from the summary list, show the short title, the
  recorded value and the reward.
- **Heat.** The data field `MilestoneCompleteHeatAdjust` (0.93 in `pursuitlevels`, see
  [ai-pursuit-heat.md](ai-pursuit-heat.md)) is **not read by any code found in the PC executable** (neither its hash
  nor the name appears). Whether a milestone cools the car is therefore **[unconfirmed]**.
- A complete milestone is final. Nothing sets state 4 back.

### 3.7 Speed traps

The trap trigger script (`stategraph_speedtrap_group_monitor`) runs when the player's car enters the trap trigger: if the
trap is an active one and the car's speed in km/h is above the trap's `ThreshholdSpeed`, it calls the exe's
`NotifySpeedTrapTriggered` (`0x0060A7B0`) and `AwardPlayerBounty` with the trap's `Bounty`; otherwise it shows the "too
slow" message. A second script (`stategraph_speedtrap_activity_monitor`) does the same for a single trap but keeps a record
speed and a re-arm delay (`ResetTime`) for repeated crossings, and shows a debug message. **[verified]** (script bytecode)

`NotifySpeedTrapTriggered` sets the statistic `speedtrap_speed`, posts a message, and looks for the trap entry with
bit 1 (armed) set and the same key: for that entry (`0x00601730`) bit 1 is cleared, bit 2 (done) is set, the speed is
stored, the bin counters are recomputed (3.8), and the map icons are refreshed. In free roam the speed trap photo screen
(`InGamePhotoMaster.fng`) is shown with the speed and the bounty. In a pursuit race the call also writes the speed into
the race's own list. A trap that is already done cannot complete again. **[exe]**

### 3.8 Bin counters

The recount (`0x006010E0`, run after every milestone bank and speed trap completion) sets two u16 counters on the bin
view: the number of milestones in state 4 plus the number of speed traps with bit 2 set, over the entries of that bin
(stored at `+0x18`), and the number of the bin's events that are marked done (at `+0x1A`, the events part is not
decoded here). The scripts read `+0x18` as `GetNumChallengesPassed(bin)` and `+0x1A` as the races-won count
(`0x00604A10`, `0x00604A30`).

## 4. Where scripts take part **[verified]** (bytecode read)

### 4.1 Message names

`MNotifyMilestoneProgress` (`0x005DD5F0`, send helper `0x00632D20`), `MNotifyMilestoneReached` (`0x005DE370`, helper
`0x00632DA0`), `MNotifyChallengePassed` (the one sent by `0x00605140`, getter `0x006046C0`). The two milestone messages
are converted for scripts by `0x00626040` and `0x00626090`, which make a table with `MilestoneName` (the statistic name)
and `ValueReached`.

### 4.2 Hooks the scripts call

`GetNumChallengesPassed`, `GetNumRacesWon`, `GetPlayerBounty`, `AwardPlayerBounty`, `ChallengeCompleted`,
`ChallengeComplete`, `NotifySpeedTrapTriggered`, `IsActiveSpeedTrap`, `AddSpeedTrapToMap`, `ForcePursuitStart` are
registered in the script hook table at `0x0061E750`.

### 4.3 Challenge races

The 36 `challengerace_*` events (race type 8, see [../formats/gameplay-races.md](../formats/gameplay-races.md))
are pursuit races that reuse the milestone statistics. Their data carries `MilestoneName` (same names, from the
template, e.g. `roadblocks_dodged`), `ThreshholdValue` (hash `0x4E90219D`, the goal) and `MilestoneBiggerIsBetter`
(hash `0x0896D043`). In such a race the exe tracks the same statistics but does not test the 55 milestones
(play mode is 1). The race state graph handles the progress message instead: when the name matches and the value is
at or past the goal (above for bigger is better, below for smaller), it sets `PassedMilestone` and shows the HUD message
"Challenge Complete. Now Evade."; when the perpetrator then escapes and the player won, it calls `ChallengeComplete`.
Two variants of this handler exist (the second uses field names `BiggerIsBetter` and `MilestoneThresh` and prints a debug
message); which is live was not decided. The race rules themselves belong to the race specs.

## 5. Screens **[exe]** / **[verified]** (packages)

| Package | In | Role |
|---|---|---|
| `InGameMilestones.fng` | `GLOBAL/InGameB.bun`, 117 objects, 9 buttons | the pause-menu milestones list; **[unconfirmed]** which routine opens it |
| `PostRace_MilestoneRewards.fng` | `GLOBAL/InGameB.bun`, 51 objects, no buttons | the rewards screen of the summary list (3.6) |
| `SafeHouseMilestones.fng`, `SafehouseMilestones_HELP.fng` | `FRONTEND/FrontB.lzc`, 164 and 18 objects | the rap sheet milestones page and its help |
| `Hud_MilestoneBoard` (FEng element) | HUD | four slots `MILSTONE_ICON_n` (sic) with `MILESTONE_ICON_BACKING_n`, groups `MILESTONE_INFO_GROUP` and `MILESTONE_ICON_GROUP`, built for the profile's current bin; with two or more unfinished slots it rotates a highlight between them on a timer; the code that fills the slots was not located |
| `Milestone_TollBooth` (FEng object) | HUD race information element | an object looked up by the race information element beside `CurrentLapTime`; its use is **[unconfirmed]** |

Behaviour of the lists: the rap sheet page (`0x00552440`) lists every milestone of the viewed bin (all states), then
the bin's speed traps, marking state 1 and "not unlocked" rows as locked and state 4 and "done" rows as complete; the
selected row puts a pin on the map from its `SpawnPoint` marker (`0x005481C0`). The row object is `MilestoneDatum`
(virtual table `0x0089C3A4`); each row holds the entry pointer. The pause-menu list builder (`0x005A2390`) builds one row
per milestone of the current bin from: a display kind (3 for pursuit length, 4 for evasion time, both shown as
minutes:seconds; 2 for plain numbers), the short label, the statistic's current value, the threshold, and a status
(1 at stake, 2 complete, 0 otherwise); during a pursuit race it builds a single row for the race's goal. Texts: the
labels `BLACKLIST_PURSUIT_MILESTONES_NN` (long form, one `%s` for the goal) and `_SHORT` (title); `HUD_MILESTONE_COMPLETE`
("Milestone Complete!"), `HUD_CHALLENGE_COMPLETE`, `HUD_MILESTONES`, `BL_MILESTONE_WINS`, `MILESTONES_ACHIEVED`,
`HELP_MILESTONES` and `REP_SHEET_MILESTONE_CONFIRM` exist in `LANGUAGES/English.bin`. The help text says a minimum number
of milestones is needed before the next rival and that launching a milestone from the rap sheet starts a pursuit;
the code path that launches one was not found.

## 6. Gate of the bin's rival (condition only)

While a bin's rival is locked, the bin's script state graph `race_bin` (state `check_rival`, entered on each update
message) reads three numbers and unlocks the rival only if all three are met, with greater-or-equal tests: **[verified]**

- races won in the bin >= `RequiredRacesWon`;
- the player's total bounty (sum over cars, 3.6) >= `RequiredBounty`;
- `GetNumChallengesPassed(bin)` (completed milestones + completed speed traps, 3.8) >= `RequiredChallenges`.

Values per bin are in table 4 of the data file. Twelve other handlers (`rival_locked`, message `NotifyChallengePassed`)
send the player a text message (`EAddSMS` with a message number) when the bin's count reaches 1, 2, 3 or 4 (different
thresholds and numbers per bin; listed in the provenance file). The rap sheet rival screen (`0x0052EE50`) shows the same
three comparisons as ticks. The unlock itself and the rival fight are another spec's.

## 7. Save and reset **[exe]**

The profile buffer starts (after the 16-byte header, see [../formats/saves.md](../formats/saves.md)) with a 16 KiB block
written by `0x005F7410` and read by `0x005F81D0`:

| Offset | Content |
|---|---|
| 0x00 | MD5 of bytes 0x10 to 0x3FFF; the load fails on mismatch |
| 0x10 | u32 `Game` (0x656D6147), 0x14 u32 version 8 (a load needs >= 8) |
| 0x18 to 0x34 | counts: script state records, 8-slot block, statistics, milestones, speed traps, 0x200 (bit count of the 64-byte flag set), bin counters bytes, bounty-jump keys |
| 0x80 | the script state records, then (16-byte aligned) the 8 x 0x20-byte block, the statistics (16 bytes each), the milestone entries (20 bytes), the speed trap entries (aligned, 20 bytes), the 64-byte flag set, the bin counters, the bounty-jump keys, two 4-byte references |

Bin counter record: u16 bin index, u16 length (4), u16 completed milestones and speed traps, u16 completed events. On load
the arrays are first reset to data defaults, then each saved milestone and speed trap replaces the entry with the same
collection key (so adding milestones to the data keeps old saves valid), the statistics map is restored by key (current
and best), and the counters are copied back. On a new career the reset sets all milestones to state 1, all statistics to
-1.0, all speed traps to flags 0 and clears the counters. The blob also holds the script state records, so rival
unlock state is not separate. The details of the unrelated parts are not decoded here.

## 8. The rewrite today

The milestone state exists in `crates/nfsmw-career::milestones` (statistics, the four states, the bin transitions, the
speed traps in `milestones::speedtrap`, and the save block records in `save::milestones_block`); the data readers are in
`nfsmw-data`. The choices are in [provenance/career-core.md](../provenance/career-core.md). **Still open:** the native mod
API now pays bounty: `nfsmw.career` 0.2 has `MilestoneDef` with `reward_bounty` (`BbMilestoneDef.reward_bounty`, in
`crates/nfsmw-modding/src/abi/career_records.rs`), and `nfsmw.career`'s `milestone_complete` pays it to the current car
(`crates/nfsmw/src/modding/backends/career.rs`). The original pays **bounty (rep points), not cash**, and its
threshold is one number: `MilestoneDef.goal` is that number. Race rewards keep `reward_cash` (`RaceDef`), because races
pay cash.

## 9. Open questions

1. What the unread fields `0x3B9BBFC2` and `0x006EC903` and the threshold field `0x8445AF47` are called, and whether the
   ladder idea is real (section 1.2).
2. The divisor read by `AwardPlayerBounty` (3.6).
3. What interface slot `+0x18` of the pursuit returns when `pursuit_evasion_time` is set, and whether the gate in front of
   the escape routine tests "is player" or "has escaped".
4. Which routine fills the HUD milestone board slots and which opens `InGameMilestones.fng`.
5. How a milestone is launched from the rap sheet ("jump to location and start a pursuit").
6. Whether `MilestoneCompleteHeatAdjust` is used anywhere.
7. The 12 statistic types with no producer: whether they were meant for other modes.
8. Which of the two challenge-race progress handlers is live, and the exact meaning of `MilestoneType` 3.
9. The parts of the save block not decoded (8-slot block, bit set, bounty-jump keys).
