# Career regions: closed gates, the map variants and the region unlock screen

How the original closes parts of the city while the player is high on the Blacklist, how it opens them as rivals fall,
what the "region unlocked" screen shows and does, and which pictures and texts belong to it. The stage byte
(**CurrentBin**) and the bin records are in [career-blacklist.md](career-blacklist.md); the arrival sequence that opens
the screen is in [career-safehouse.md](career-safehouse.md) section 5.3; the minimap and the markers are in
[hud-minimap.md](hud-minimap.md) and [world-markers.md](world-markers.md). Addresses, Ghidra names, method and dead
ends are in the [provenance file](../provenance/career-regions.md).

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) through the Ghidra MCP and a linear
  disassembly; the install at `D:\Need For Speed Most Wanted Black Edition`: `GLOBAL/gameplay.bin` (the bin and race
  records, read with the repo's `blackbox-attrib` reader), `TRACKS/L2RA.BUN` (scenery groups `0x34109`, override
  records `0x34108`, track path barriers `0x3414D`), `TRACKS/STREAML2RA.BUN` (scenery instances, collision packs read
  with `blackbox-scenery` and `blackbox-collision`), `TRACKS/L2RA/MINI_MAP*.BIN` and `TrackMaps.bin`,
  `GLOBAL/RIVALS.BIN`, and the package `SafehouseRegionUnlock.fng` in `FRONTEND/FrontB.lzc` (read with
  `blackbox-feng`). The community decompile was not re-read.
- **Evidence tags:** **[exe]** read in `speed.exe`, not tested in game. **[verified]** checked in install data.
  **[unconfirmed]** a guess or an unproven rule.
- **Addresses** are `speed.exe` addresses. Ghidra names are only in the provenance file. Terms: **stage** = CurrentBin
  (16 at the start, 1 at the end); **gate** = one of the six barrier groups `BARRIER_LOCKED_10` .. `_15`.

## 0. Direct answers

| Question | Answer |
|---|---|
| What is a "region" in the data? | There is **no region table**. Three things carry the idea: the six *gates* (scenery groups) that wall the city off (section 2), the text attribute `Region` of every race (`college`, `coastal`, `city`; section 6.3) and three picture variants of the minimap and the world map chosen by the stage (section 6). **[exe + verified]** |
| Which gates are closed at which stage? | Stages 16..13: gates 11, 12, 13, 14, 15. Stages 12..9: gates 10, 12, 13. Stages 8..1: none. Read from the `Barriers` list of the bin record of the current stage; bins 8..1 have no list. **[verified]** |
| How is a closed gate enforced? | Physically. A gate is a set of scenery instances (the model `XO_TrackBarrierLocked_1`) that are hidden and non-colliding by default; naming the group turns them into visible walls with collision. The AI road network also gets one barrier line per gate. There is **no trigger zone, no message, no push-back**. **[exe + verified]** |
| When is it evaluated? | At world creation, whenever the stage changes (deferred to the next roaming frame), and again after every race (the race graph's `done` state). Never per frame. **[exe]** |
| What opens the region screen? | The Blacklist arrival stepper shows it only when the stage byte is exactly 12 or 8. It shows the text "Camden Beach borough has now been unlocked." (12) or "Rockport borough has now been unlocked." (8) over the background picture of the rival just beaten (`RIVAL_13_BG` / `RIVAL_09_BG`). It changes no state; the accept action (about 0.5 s of leave animation) moves to the next step. **[exe + verified]** |
| What is in `Rivals.bin`? | A texture pack of 90 textures: six per rival, 15 rivals (section 8). **[verified]** |
| Are "gate zones" involved? | No. `EnterGateZone`, `ExitGateZone`, `AllowMenuGates` and `IsActiveMenuGate` belong to the **menu gates** (safe house, car lot, custom shop prompts), which have nothing to do with closed regions (section 5). **[exe]** |

## 1. The data

### 1.1 The `Barriers` list of a bin record **[verified]**

Attribute `Barriers` (text array, key `0xE244F26B`) on `race_bin_NN` collections and on race collections (section
3.4). Entries are group names, optionally prefixed with `*`. The bin lists in the install:

| Stages | `Barriers` of `race_bin_NN` (in file order) |
|---|---|
| 16, 15, 13 | `_15, _14, _11, _13, _12` |
| 14 | `_12, _13, _11, _15, _14` |
| 12 | `_12, _13, _10` |
| 11, 9 | `_10, _13, _12` |
| 10 | `_12, _13, _10` |
| 8 .. 1 | no attribute (empty list) |

(`_N` is `BARRIER_LOCKED_N`.) Order does not matter. As sets: stages 16..13 close {11, 12, 13, 14, 15}; stages 12..9
close {10, 12, 13}; stages 8..1 close {}. This agrees with [career-blacklist.md](career-blacklist.md) section 6.

### 1.2 Scenery groups (`0x34109` in `L2RA.BUN`) **[exe + verified]**

754 records, each padded to 4 bytes: `+0x00` two junk words (a runtime list node), `+0x08` u32 name hash
(`bStringHash`, seed `0xFFFFFFFF`, multiplier 33, case sensitive), `+0x0C` u16 **group number**, `+0x0E` u16 entry
count `n`, `+0x10` u8 (1 for 751 groups, 0 for three), `+0x11` u8 **drive-through flag** (1 for 126 race groups, 0
for the gates), `+0x14` `n` x u16 indexes into the override records. The record size is `(2n + 0x17) & ~3`. Group 1
(hash `0x454C22D2`, 15,381 entries) is a world-wide group of unknown purpose. The group numbers are what the collision instances
store (section 4.2).

An **override record** (`0x34108`, 6 bytes, 22,340 of them) is `{u16 section, u16 instance, u16 flags}`: the scenery
instance `instance` of streaming section `section`, and the flags that replace the low 16 bits of that instance's
flag word. The shipped flags of the gate instances are `0x8070`. Bit `0x10` of the instance flags removes it from
every view (the visibility rule of [scenery-visibility.md](scenery-visibility.md): `0x70` xor the inverted bits `0x60`
= `0x10`), so `0x8070` = hidden and `0x8060` = shown. **[exe + verified]**

### 1.3 The six gates **[verified]**

Each gate group holds 3 to 7 instances of the scenery model `XO_TrackBarrierLocked_1`, all in one streaming section,
placed edge to edge every 8 to 9 m across a road (each piece about 8 x 2 x 4.4 m). Positions are render-space
`(x, y)` in metres, the frame of the minimap projection (`mx = (x - ox) / W`, `my = (oy - y) / W + 1`, origin
`(-1224.89, -1591.14)`, width `6659.33` m); the last columns are the position on the 1024 x 1024 stitched minimap:

| Gate | Group no. | Section | Pieces | Centre `(x, y)` | Minimap px | Closed at stages | Opened by reaching |
|---|---|---|---|---|---|---|---|
| `BARRIER_LOCKED_13` | 200 | 425 | 5 | (2051, 1127) | (504, 606) | 16..13, 12..9 | stage 8 |
| `BARRIER_LOCKED_12` | 356 | 920 | 7 | (1363, 1344) | (398, 573) | 16..13, 12..9 | stage 8 |
| `BARRIER_LOCKED_11` | 389 | 932 | 5 | (2255, 1340) | (535, 573) | 16..13 | stage 12 |
| `BARRIER_LOCKED_14` | 495 | 1605 | 7 | (2765, 2098) | (614, 457) | 16..13 | stage 12 |
| `BARRIER_LOCKED_15` | 496 | 1605 | 5 | (2792, 2176) | (618, 445) | 16..13 | stage 12 |
| `BARRIER_LOCKED_10` | 624 | 1801 | 3 | (2779, 610) | (616, 686) | 12..9 | stage 8 |

Gate 10 is the odd one: it is *open* at stages 16..13 (it lies deep inside the area the other gates already wall off)
and is closed only at stages 12..9. So **stage 12 opens gates 11, 14, 15** (the eastern border of the starting quarter: the "Camden Beach" side) and
**stage 8 opens gates 10, 12, 13** (the southern border, towards the "Rockport" side). **[verified]** (sets) and
**[verified]** (the picture of the lit area, section 6.1); which name belongs to which side is **[unconfirmed]** beyond
the two unlock texts.

Each gate group also owns exactly **one** track path barrier (`0x3414D`, 24-byte record `{f32 x0, y0, x1, y1; u8
enabled, pad, drive-through, left-handed; u32 group hash}`): a line 28 to 61 m long on the same spot (for example
gate 10: `(2778.0, 624.2)` to `(2779.4, 596.0)`; 12: `(1335.2, 1334.4)` to `(1390.1, 1353.5)`; 13: `(2031.9, 1122.8)`
to `(2071.5, 1132.6)`; 11: `(2247.5, 1319.6)` to `(2262.3, 1362.2)`; 14: `(2743.6, 2078.0)` to `(2787.5, 2120.1)`; 15:
`(2803.6, 2158.6)` to `(2780.2, 2195.0)`). All 1,376 records ship with `enabled` = 0 and drive-through = 0.
**[verified]**

## 2. Which gates are closed: the rule

```
closed_gates(stage):
    record = bin record whose BinIndex == stage        # race_bin_NN; stages 18..21 are not stages
    if record is missing: return {}
    return the names in record.Barriers (leading '*' removed)
```

Equivalent table: `stage >= 13` -> {11, 12, 13, 14, 15}; `9 <= stage <= 12` -> {10, 12, 13}; `stage <= 8` -> {}. The
rewrite should read the list from the gameplay data (so a modded bin works) and keep the table only as a test. The
thresholds 13 and 9 are the same ones that choose the map variants (section 6). **[exe + verified]**

## 3. Applying and clearing barriers

### 3.1 Enabling a list **[exe]**

For each entry of a `Barriers` list, in order:

```
enable_entry(text):
    star = text starts with '*'
    name = text without that '*'
    if name does not start with one of "BARRIER_", "BARRIERS_", "PLAYER_BARRIERS_",
       "SCENERY_GROUP_", "FREE_ROAM" (prefix compare ignores case): return    # silently skipped
    hash  = bStringHash(name)                       # case sensitive
    remember hash in a 256-slot list                # bookkeeping only
    group = scenery group with that hash; if none: return
    for each override record of the group:
        flags = flags & ~0x0410                     # clears "hidden" (0x10) and "turned" (0x400)
        if star: flags |= 0x0400
        write the flags to the instance; if the instance is a movable one and its 0x400 state changed,
        rebuild its matrix turned by 180 degrees      # the meaning of the star is a flip [unconfirmed]
    group_state[group number] = 1 | (2 if star) | (4 if the group's drive-through byte)
    for each track path barrier whose group hash equals hash:
        barrier.enabled = true
        barrier.drive_through = (group's drive-through byte != 0)
```

After the whole list: clear the cached road-barrier flags (a pass over the road segment cache) and recompute them
(section 4.3). `group_state[g] != 0` is the single test the world collision uses (section 4.2).

### 3.2 Clearing **[exe]**

`DisableBarriers` (the script call and the first half of every re-evaluation): every group with a non-zero state has
the hidden bit `0x10` put back on all its instances and its state set to 0; the enabled and drive-through bytes of all
track path barriers are zeroed; on every road segment the barrier flags (`CrossesBarrier`, `CrossesDriveThroughBarrier`)
and the two race-route flags (`RaceRouteForward`, `InRace`) are cleared ([road-network.md](../formats/road-network.md)). Then, if the
group `SCENERY_GROUP_DOOR` exists, it is enabled again without the star (the junk-pile gate of the final race is
closed by default; races that list `*SCENERY_GROUP_DOOR` open it). The door group is not part of the region system.

### 3.3 When it runs **[exe]**

| Trigger | What happens |
|---|---|
| The race-status object is created with the world (the world-load hook) | If no event node is current: clear, then enable the list of the record of the stage byte. |
| The race-status object is reset (a routine reached from the world-loaded path; the caller chain was not followed) | If no event node is current and a bin record is stored: enable its list again, without clearing. |
| The enter-bin routine changes the stage | Sets a "bin changed" byte on the race-status object. |
| Every race-status update in the roaming state (racing state 0) with that byte set | Clears the byte, then **set current bin** with the stage byte: clear (if a record was stored), look up the record of the stage, store it, enable its list. This is the only call that follows a stage change. |
| `basicrace` graph, state `done` | `DisableBarriers`, then `EnableBinBarriers` (enable the list of the stored record). Race barriers are gone, region gates are back. No other script calls either function. |
| A race starts | The race's own `Barriers` list is **added** on top (section 3.4). Nothing is cleared first, so the region gates stay closed during every race. |

The stage is only ever changed through the enter-bin routine (blacklist spec section 5.2) or by loading a save, so a
changed stage is always followed by a re-evaluation before the player drives again. A save loaded at stage 12 gets the
{10, 12, 13} gates at world creation. The re-evaluation therefore happens **at the first roaming frame after the
stage change**, normally while the front end is up and the world is being rebuilt (the front end destroys the world,
see [career-safehouse-2.md](career-safehouse-2.md)), so the player never sees the walls appear or vanish.

### 3.4 Barriers named by races **[exe + verified]**

238 gameplay collections carry a `Barriers` list: the eight bins above and 230 races. Race entries are
`BARRIER_SPLINE_n` (3,720 plain and 3,575 starred entries in total; the star marks a barrier piece that is turned) and
`SCENERY_GROUP_DOOR` (8 races, starred in two). They close roads *during that race* (the race route walls); they are
enabled by the race-start hook and removed by `done`. The 15 challenge-series races (`race_bin_challenge/challenge_01
.. _15`) additionally carry gate lists of their own: challenges 01 to 05 list the five gates {11, 12, 13, 14, 15},
challenges 06 to 15 list {10, 12, 13}. So a challenge pursuit closes those gates for its duration whatever the player's
stage, and `done` restores the stage's set. **[verified]** (lists) **[exe]** (timing).

## 4. What a closed gate is

### 4.1 Appearance **[verified]** (names) **[unconfirmed]** (look)

The pieces are the scenery model `XO_TrackBarrierLocked_1` (the other `XO_TrackBarrier*` models in the world are
`XO_TrackBarrierPlayer_1` for race walls, 233 placements, and `XO_TrackBarrierTraffic_*`, 57). The streaming file holds
the texture names `SFX_TRACKBARRIERLOCK`, `SFX_TrackBarrierNeonLocked` and `SFX_TrackBarrierNeonArrow`, which suggest a
neon "locked" wall with arrows; the renderer treats textures whose name starts with `SFX_TRACKBARRIER` specially
(an extra surface flag byte set to 2). No sign, text or HUD message is shown near a gate.

### 4.2 Collision **[exe + verified]**

Every gate group has one world-collision instance (CARP pack of the same streaming section) whose scenery group
number field (`+0x18` of the 0x40-byte instance) equals the group number, holding 4 or 5 vertical wall segments about
11 m tall that span the pieces (gate 13: five walls over a 40 m box). On disk its instance flags are 0. A collision
query (sphere or ray) accepts an instance only if its group number is 0 **or** the group's state byte is non-zero, so
a closed gate blocks cars, and an open one is ignored. The rewrite already excludes group instances in free roam
(`GROUP_EXCLUSION`, [collision.md](../formats/collision.md)); it must instead include a group when it is enabled.
Nothing pushes the car back and no event fires: the car simply hits a wall.

### 4.3 AI roads and the GPS **[exe]**

After an enable pass the game recomputes `CrossesBarrier` / `CrossesDriveThroughBarrier` on the road segments that
the enabled barrier lines cross ([ai-road-network.md](ai-road-network.md) section 5.3): first clear both barrier flags of
every segment, then for each enabled barrier test the segments found near its endpoints and set `0x10` (full
barrier, drive-through = 0) or `0x20`. Segments of the road(s) under an exemption point are skipped: in a race, the
roads at the race's start points; in free roam, one fixed point (physics position `(-2511, 148, 1783)`, direction `(-0.7, 0, 0.7)`). Racers, the player, GPS and race routes respect the flag; cops and helicopters do not (so cops can
route through a gate line, but the wall stops them physically). All six region gates have drive-through = 0.

## 5. What "gate zone" means (and does not)

| Name | What it is | Region relation |
|---|---|---|
| `EnterGateZone(type)` / `ExitGateZone` | show / hide the prompt of a safe house, car lot or custom shop sphere | none |
| `AllowMenuGates` | returns a permission byte of the race-status manager (`+0x2D0`) | none |
| `IsActiveMenuGate(race)` | true for a race of a gate type (flag bits 13, 14, 15) whose gate record is accepted by the manager | none |
| `menu_gate` state graph | 3 instances: safe house, car lot, custom shop | none |
| `BARRIER_LOCKED_n`, `SCENERY_GROUP_DOOR` | the closed-region and door walls | this spec |

The "gates" drawn as chevrons on `MINI_MAP_UNLOCK_1` are the region gates of section 1.3 (section 6.1).

## 6. Region-dependent presentation

### 6.1 Minimap and world map variants **[exe + verified]**

One stage-to-variant rule is used by the free-roam minimap and by the world map screen:

| Stage | Minimap file (`TRACKS/L2RA/`) | World map texture in `TrackMaps.bin` | Lit area |
|---|---|---|---|
| >= 13 | `MINI_MAP_UNLOCK_1.BIN` | `TRACK_MAP_UNLOCK_1` | north-west quarter |
| 9 .. 12 | `MINI_MAP_UNLOCK_2.BIN` | `TRACK_MAP_UNLOCK_2` | all but the south-west |
| <= 8 | `MINI_MAP.BIN` | `TRACK_MAP` | everything |

Mean tile brightness of the 8 x 8 tile grids (`#` lit, `:` partly lit, `.` black), row 0 at the top, measured on the
install:

```
UNLOCK_1       UNLOCK_2
######:.       ########
#####:..       ########
#####...       ########
#####...       ########
#####...       ########
........       ....####
........       .....##:
........       ........
```

The world map screen computes the same variant from the stage when it opens (`< 9` -> full, `9..12` -> 2, `>= 13` ->
1) and loads the matching texture of `TrackMaps.bin` (the three names exist; 297 textures in the pack). In a race that
is not a pursuit race the minimap is `MINI_MAP_<event id>` ([race-minimap.md](race-minimap.md)); in a **pursuit race** (`PursuitRace` true) it is chosen by
the race's `Region` text instead (section 6.3). Outside a race (racing state 0) the stage rule above applies, reading the career settings byte
whatever the game mode; the function returns no map for the drag HUD (argument 2). **[exe]**

### 6.2 Labels **[verified]**

`REGION_UNLOCK_COSTAL` (`0x29E4B193`, "Camden Beach borough has now been unlocked."), `REGION_UNLOCK_DOWNTOWN`
(`0x2B0BCA2D`, "Rockport borough has now been unlocked."), `TRACK_REGION_COLLEGE`, `TRACK_REGION_COASTAL`,
`TRACK_REGION_CITY` ("College", "Coastal", "City"), eight `NEIGHBOURHOOD_*` names and the `WM_*` world map legend
([career-text.md](career-text.md) section 5.1). `REGION_NORTHAMERICA`, `_EUROPE`, `_JAPAN`, `_ALL` are the car
classes of the car lot, not city regions. The texts `TIP_UNLOCKS_*` and a message from the contact "Rog" sent at bin
13 after the first race win (SMS 35, "You wanna see the ocean?") hint at the coming region; no text announces a closed
gate. No label is shown when a gate is hit.

### 6.3 The `Region` race attribute **[verified]**

Every race has `Region` (text, key `0xCB01E454`): `college` -> 0, `coastal` -> 1, `city` -> 2 (the numbers are the map
variant index of 6.1: `UNLOCK_1`, `UNLOCK_2`, full). Counts in the install: 73 college, 85 coastal, 60 city. They
follow the opening order: **all races of bins 15, 14 and 13 are `college`**; bins 12 to 9 use only `college` and
`coastal`; `city` first appears in bin 8 and later. (Bin 9 has 1 college, 6 coastal.) The quick race track list (builder at `0x007AAC70`) keeps a
race only if its `Region` index equals the region stored in the profile's quick race options (record 11, byte `+0x0D`)
or that value is 3 (all regions), and the race type matches the selected one. **[exe]**

## 7. The region unlock screen (`SafehouseRegionUnlock.fng`)

### 7.1 When **[exe]**

It is step 2 of the Blacklist arrival stepper, shown only if the stage byte is **12** or **8**; at any other stage the
stepper skips to the next step. The stepper runs on the arrival after a rival win, when the stage byte is already
lowered, so stage 12 means rival 13 was just beaten and stage 8 means rival 9 was (the screen's picture key is
`stage + 1`). The screen is the first one
after the marker choice and before the (optional) autosave and the rival bio. The stage check is the only gate: if
the stepper ran twice at stage 12 the screen would show twice. How often the stepper runs per rival is **[unconfirmed]**
(open question 5).

### 7.2 Construction **[exe]**

```
on create:
    screen object = the front-end package SafehouseRegionUnlock.fng
    rival texture helper (in-game variant off), key = none yet
    ask the file system for the pack "Global\Rivals.bin" (asynchronous); on completion call load_picture
    find in the package: the picture object  H_BG  (name hash 0x2CBE1DD0)       -- present
                         the portrait object (name hash 0xC1F62308)             -- absent in this package
                         the graffiti object (name hash 0xF5A2A087)             -- absent in this package
                         (an object only counts if it exists and is an image; absent ones are skipped)
    if stage == 12: set the text of the string object 0xD6C0E097 to label REGION_UNLOCK_COSTAL
    if stage == 8 : set the text of the string object 0xD6C0E097 to label REGION_UNLOCK_DOWNTOWN
    helper.key = stage + 1                          # 13 or 9: the rival just beaten
    mark the present picture objects hidden and dirty until their texture arrives
```

`load_picture` (called when the pack is registered, and again if the key changes) builds the texture names for the
helper (section 8.2), asks the texture streamer to bring them in, and, when they have arrived, sets each present
picture object's texture to the matching name hash, clears its hidden bit and posts the three scripted messages
`0xC0942E85`, `0x8C9D4547`, `0xD22B95D0` to the screen's package (no response exists for them in this package). **[exe]**

The picture of this screen is therefore **`RIVAL_13_BG`** at stage 12 and **`RIVAL_09_BG`** at stage 8 (512 x 512, 32
bit; the package's own default is `RIVAL_15_BG`). It is the rival-specific background, not a map and not a region
picture. **[exe + verified]** On destruction the helper releases its textures and the pack.

### 7.3 Layout of the package **[verified]**

640 x 480 FEng screen. Objects that matter (all hash names are the FEng name hashes): `2CBE1DD0` the full-width
picture (900 x 480, white, depth 1001), `A1C9E674` a black backdrop (905 x 514, depth 1500), the title string
`BL_CONGRATULATIONS` ("CONGRATULATIONS!", gold plus a black shadow copy), the label string `D6C0E097` (white; placeholder
text in the file) and a black twin `F577FA43` with a placeholder label, the button group `Continue`
(`COMMON_CONTINUE`, with the Enter key icon), graffiti and splatter decorations, and an invisible string
`EVENT HANDLER` (`07B7C74A`) that sequences the exit. The code sets the text of `D6C0E097` only; whether the twin
shows its placeholder is **[unconfirmed]** (it is drawn black at +1 px, behind the white text).

### 7.4 Input, timing and exit **[exe + verified]**

FEng ticks are 1200 per second on PC ([feng-runtime.md](feng-runtime.md)).

1. On start every object runs its intro script (`INIT` chains to the `IN` script, 300 to 3,700 ticks; the longest is
   the backdrop poly, 3.1 s). The event handler's `INIT` chains into a 500-tick script whose events fire at its start:
   a message to the package itself (`0xBDFD2DF9`), `INPUT_ENABLE` (`0x8CB81F09`, which enables the button through the
   package response) and a sound message `0x63E5ED6F`. Input is therefore live at once; the intro keeps playing.
2. **Accept** (pad accept, Enter, or a mouse release on the Continue button, which has the same response): the package
   posts `0x97BDB465` (every object switches to its `OUT` script, 200 to 1,200 ticks) and `0xADE14EC3` to itself, plays the click sound `0x7B6B89D7`
   (SOUND_ACCEPT) and locks the button. `0xADE14EC3` starts the event handler's 600-tick script: at its start a
   sound message `0xF786ED9D`, and at tick 600 (0.5 s) it posts `INPUT_ENABLE`, `EXIT_COMPLETE` (`0xE1FDE1D1`) and
   **PAD_ACCEPT (`0x406415E3`) to the game**.
3. The screen's game-side handler reacts to exactly one message, PAD_ACCEPT, by running the arrival stepper (next
   step: the autosave flow if possible, else the rival bio). **Back and Start do nothing**, and nothing times out:
   the screen waits for the player.
4. No NIS, movie, camera or world change belongs to this screen. The audio is the three UI sounds above plus whatever
   the front end music already plays (the ids `0x63E5ED6F` and `0xF786ED9D` are not named in the repo; they are the
   screen-in and screen-out cues by position **[unconfirmed]**).

## 8. `GLOBAL/RIVALS.BIN`

### 8.1 Contents **[verified]**

A bChunk `TexturePack` (21,640,440 B; pack name `RIVALS`, file name `Global\Rivals.tpk`) with the usual info, key,
entry and data-array children ([textures.md](../formats/textures.md)). 90 textures, six per rival `NN` = 01 .. 15
(name hash = `bStringHash` of the name; the entries are 24-byte stream records `{hash, absolute file offset,
compressed size, size, 256, 0}`; blobs are JDLZ or HUFF, both read by `ea-compress`):

| Texture | Size | Use |
|---|---|---|
| `RIVAL_NN` | 512 x 512, 1,048,576 B of pixels | the rival's portrait (rival challenge, bio) |
| `RIVAL_NN_BG` | 512 x 512 | the rival's background picture (also this screen) |
| `RIVAL_NN_GRAF` | 128 x 64, 9,216 B (looks paletted) | graffiti tag |
| `RIVAL_NN_IG`, `_IG_BG`, `_IG_GRAF` | as above | in-game (pursuit/race) variants |

Every decompressed blob is its pixels plus a 156-byte trailer (name, hash, size, width, height). The colour format was
not decoded. The pack is registered through the async file call; the textures are brought in on demand by name, so
only the one picture is read (**[exe]**, the stream-by-name reading is inferred from the entry offsets).

### 8.2 Name building **[exe]**

For key `k` (two digits, zero padded) and helper variant `ig`:
`RIVAL_<k>` / `RIVAL_<k>_IG`, `RIVAL_<k>_GRAF` / `_IG_GRAF`, `RIVAL_<k>_BG` / `_IG_BG`, built only for the picture
objects the screen actually has, in the order portrait, graffiti, background. The region screen has only the
background. The same helper serves the rival challenge, rival bio, in-game bio and marker screens.

## 9. What a rewrite needs (the pure rules)

```
region_gates_closed(stage) -> set<u8>:        # from the bin's Barriers list; fallback table of section 2
map_variant(stage) -> Full | Unlock2 | Unlock1:  stage >= 13 -> Unlock1; stage >= 9 -> Unlock2; else Full
region_unlock_screen(stage) -> Option<(label, picture_key)>:
    12 -> (REGION_UNLOCK_COSTAL, 13)
    8  -> (REGION_UNLOCK_DOWNTOWN, 9)
    _  -> None
picture_texture(key) = format("RIVAL_{key:02}_BG")
barrier_apply(world, list, star_aware)           # sections 3.1, 3.2
on stage change: set pending; at the next roaming frame: clear, then apply the new stage's list
after every race: clear, then apply the stage's list
collision: include a group instance iff its group number is 0 or the group is enabled
```

Tests worth writing: the stage -> set table (all 16 stages), the prefix filter of 3.1, that bins 8..1 yield no gates,
that a race's lists are added without clearing the gates, that `done` restores the stage's set, and that the unlock
screen appears for stages 12 and 8 only.

## 10. Open questions

1. The meaning of the star (`*`) prefix beyond "turn the instance by 180 degrees" and of the group byte at `+0x10`.
2. The ids `0xBDFD2DF9`, `0x63E5ED6F`, `0xF786ED9D` (sound or screen messages) and whether the black twin label of the
   unlock text shows its placeholder.
3. Whether `RIVALS.BIN` textures are streamed individually or the pack is read whole; the pixel formats.
4. The exact look of the gate models (the texture binding of `XO_TrackBarrierLocked_1` was not followed).
5. Which code requests the arrival stepper after a rival win (the unfound mover of CurrentBin, blacklist spec section 5.3).
6. The `PursuitRace` minimap rule was read, not observed; what a quick race does with `Region` was only partly read.
