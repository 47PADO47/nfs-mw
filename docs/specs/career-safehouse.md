# Career safe house, native side: entering, the menu, the screens

What the executable itself does when the player drives into a safe house (or car lot, or custom shop) zone, what happens
while the front end is up, and which state changes on the way. The leaving half (Resume Free Roam, the world reload,
the respawn marker, heat) is in [career-safehouse-2.md](career-safehouse-2.md). The rules that live in the Lua graphs
are in [career-scripts.md](career-scripts.md); the front-end context word, the request object and the save are in
[career-flow.md](career-flow.md); the heat numbers are in [ai-pursuit-heat.md](ai-pursuit-heat.md) and
[career-economy-flows.md](career-economy-flows.md). Addresses, Ghidra names and dead ends are in the
[provenance file](../provenance/career-safehouse.md).

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) read through the Ghidra MCP and by
  disassembling undefined routines; `GLOBAL/gameplay.bin` for the zone collections.
- **Evidence tags:** **[exe]** read in `speed.exe`, not tested in game. **[verified]** checked against the install.
  **[community]** from a community summary. **[unconfirmed]** a reading that was not closed.
- **Addresses** are `speed.exe` addresses; no Ghidra auto names appear here.

## 0. Direct answers

| Question | Answer |
|---|---|
| Does entering the safe house reset **heat**? | No native code resets it. The live heat of the player's car is copied into the career car record when the world is torn down (capped at 5.0), and read back (never below 1.0) when the next world creates the car. Section 8 of the second file lists the writers found. **[exe]** |
| Does it reset **cops** or the **pursuit**? | Not explicitly. The whole world (cops, traffic, pursuit, racers) is destroyed when the front end loads, so nothing survives; free roam builds a new cop manager. Entering is only allowed with no pursuit or one in cool-down; in cool-down the engage action first ends the pursuit as *evaded* (section 3.4). **[exe]** |
| Does leaving reset them? | The same: the new world starts without a pursuit; the player's heat is the stored value. `SetWorldHeat` is not called on either side. **[exe]** |
| Where does heat live? | On the live perpetrator of the player's car (float at `+0x1C` of the heat interface) and, between worlds, in the career car record (`+0x0C`). Section 8 of the second file. **[exe]** |
| What is the respawn marker? | A 32-bit collection key (the `respawn` child marker of the zone, attribute `0x2241F4CD` of the zone). Position and heading are read from that marker at the next world load. Stored in the gameplay manager, saved in the gameplay blob. Section 9 of the second file. **[exe]** |
| How does the game tell safe house, car lot and custom shop apart? | The `ZoneType` string of the zone: `safehouse`, `carlot`, `customshop` (section 2). **[exe + verified]** |

## 1. State used by this flow

| Name | Where | Meaning |
|---|---|---|
| Gameplay manager | global at `0x0091E00C` | holds the respawn fields (`+0x108`, `+0x10C`, `+0x110`, `+0x11C`), the permission flags (`+0x2CE`, `+0x2CF`, `+0x2D0`) |
| Race status | global at `0x0091E000` | `+0x1960` event underway, `+0x1964` play mode (0 quick, 1 online, 2 career), `+0x1968` current event node, `+0x196C` current bin |
| Runtime node table | global at `0x0091E004` | event nodes and their state flags (bit 1 = complete); `+0x2C` the pending node, `+0x30` its mode |
| FEDatabase | global at `0x0091CF90` | `+0x10` profile, `+0x12C` context word |
| Settings | profile `+0xA8` | `+0` current car handle, `+4` flags, `+8` stage byte (16 prologue, 15 to 1), `+0xC` cash |
| Front-end manager | global at `0x0091CAE0` | `+0x18` platform id (1 profile/crib, 2 safe house, 3 custom shop, 4 back room, 5 car lot), `+0x1C` previous platform |
| Game flow state | global at `0x00925E90` | 2 unloading the front end, 3 front end, 6 in game, 7 world unload, 8 front-end load |
| Gameplay broadcast | message subject `0x20D60DBF` | the channel the script graphs and the screens use |

Context word bits used here (full list in [career-flow.md](career-flow.md) section 1): bit 0 career, bit 5 customize,
bit 8 career manager, bit 9 rap sheet, bit 15 car lot, bit 17 "Blacklist sequence" (`0x20000`), bit 18 game won.

## 2. Which kind of zone: safe house, car lot or custom shop

The three kinds are told apart **only by a string**, the `ZoneType` attribute of the zone collection in `gameplay.bin`
**[exe + verified]**. The install has three parent collections of kind `menu_gate`, each with spheres:

| ZoneType | Count | Screen opened | Platform | Extra context |
|---|---|---|---|---|
| `safehouse` | 4 | `MainMenu_Sub.fng` | 2 | career (bit 0); free-roam marker flag set (section 4) |
| `carlot` | 3 | `Car_Select.fng` | 5 | bit 15 (car lot) |
| `customshop` | 8 | `CustomizeMain.fng` | 3 | bit 5 (customize); **needs a current car record** |

Rules, exactly:

- The comparison is a plain character-by-character string compare (case sensitive) in the engage component and the screen
  opener; the icon lookup (`0x0056E100`) compares without case. The install's strings are lower case, so both agree.
- Any other string (or none) does nothing: no message, no screen.
- The custom shop opener looks up the current car's record in the stable; if the record is absent (marker byte `0xFF`) it
  returns without opening anything. It also clears the three customize state bytes and the part cursor before the screen
  is created.
- The prompt icon image is chosen by kind: car lot `0x4EAEE18B`, safe house `0x0ED39F69`, custom shop `0x0CF07089`
  (FE image ids; 0 for any other string).
- The zones themselves are spheres with the stored radius (6 or 7 m, see [world-markers.md](world-markers.md)); the
  nearest zone whose sphere holds the car is the entered one. The Lua graph raises the enter/exit hooks (section 3.5).

## 3. Entering from free roam

### 3.1 Permission flags, recomputed every frame

At the end of every update of the race status object (call at `0x0060370D`, routine `0x005EBA50`) the manager recomputes
three bytes. Inputs: whether an event is underway, and the pursuit object of the first player's car. **[exe]**

```
eventUnderway = race_status.event_underway != 0
pursuit       = first player's car -> pursuit object   (may be none)
inPursuit     = pursuit exists
coolingDown   = pursuit exists and pursuit.status == 2
manager.+0x2D0 = !eventUnderway && !inPursuit
manager.+0x2CE = !eventUnderway && !inPursuit          (same expression)
manager.+0x2CF = !eventUnderway && (!inPursuit || coolingDown)
```

The script hook `AllowEngageSafehouse` (`0x00605660`) returns `+0x2CF`. So a safe house can be entered with no event
and either no pursuit or a pursuit in cool-down; a car lot or custom shop only with neither. The Lua graph asks the
hooks before it shows the prompt ([career-scripts.md](career-scripts.md) section 4). **[exe]**

### 3.2 The engage prompt component

The player's HUD owns one component (interface id `0x005659C0`, constructed at `0x005659E0`) that remembers the zone or
the event the player can engage. Its fields (offsets from the component): `+0x0C` the on-screen object, `+0x14` the zone
type string, `+0x18` the event node hash, `+0x1C` a "pending" byte, `+0x1D` a "present" byte. Its virtual methods, by
offset (all **[exe]**):

| Offset | Routine | Effect |
|---|---|---|
| `+0x04` | `0x00569010` | can engage now: `race_status.event_underway == 0` |
| `+0x08` | `0x00569030` | reads the property `0x0280164F` of the prompt object (the engage action requires it true; meaning **[unconfirmed]**, probably "prompt is up") |
| `+0x0C` | `0x00569170` | is the stored zone type equal to the given string |
| `+0x10` | `0x0057BDE0` | **enter zone**: present = 1, store the zone type string, show the prompt object, pick the icon (section 2) |
| `+0x14` | `0x0057BD10` | **enter event trigger**: present = 1, store the node hash, show the prompt, pick the icon for the node's type |
| `+0x18` | `0x0057BEA0` | **exit**: present = 0, clear both stored values, hide the prompt |
| `+0x1C` | `0x005A0640` | set the pending byte |
| `+0x20` | `0x00569050` | **engage event**: if the stored node hash resolves to a node whose flag `0xFF5EE5D6` is set, post a small HUD event (type 3); otherwise post an event request carrying the node hash |
| `+0x24` | `0x00569100` | **engage zone**: if the stored zone type is `safehouse`, `carlot` or `customshop`, post the message `MEnterSafeHouse` (section 3.5) |
| `+0x28` | `0x0058C640` | **open the front end** for the stored zone type (section 4) |

The script hooks reach it through the player's HUD: `EnterGateZone(type)` (`0x00611FB0`) calls `+0x10`,
`ExitGateZone` (`0x00611FF0`) calls `+0x18`, and the events behind `EnterEngagableTrigger` (`0x00604F80`, handler
`0x006407D0`) and its exit (`0x00640870`) call `+0x14` and `+0x18`. **[exe]**

### 3.3 The engage button

The HUD input handler (`0x0057C660`) maps controller actions to codes; code `0x2E` is the engage action. Skipping it
when the context is online or LAN (bits 3 or 6) or a debug switch is on, it does, in this order **[exe]**:

```
on engage:
    pursuit = player's pursuit object
    if pursuit exists and pursuit.status == 2 (cool-down)
       and component flag (+0x08) is true
       and stored zone type == "safehouse":
            pursuit.EndPursuitEnteringSafehouse()          # section 3.4
            return
    if event_underway == 1 and current node is not flagged 0x2B1F54F6:
            post a HUD event (kind 0, argument 1)
    else if the controller's busy byte (+700) == 0:
            if component present and its flag (+0x08) is true:
                component.engage_event(+0x20)(stored node hash)
                component.engage_zone(+0x24)(stored node hash)   # the argument is ignored
    else:
            post a HUD event (kind 1, argument 1)
```

The meaning of the HUD events (a refusal message of some kind) is **[unconfirmed]**. The other codes (`0x2D`, `0x2F` to `0x31`) open
other overlays and are not part of this flow.

### 3.4 Entering while a pursuit is cooling down

Pressing engage on a safe house zone with the pursuit in cool-down (status 2) **[exe]**:

1. `EndPursuitEnteringSafehouse` (pursuit interface slot `+0x130`, `0x0040B040`) writes three fields of the pursuit:
   status = 4 (evaded), evade level = 1.0, and a byte "enter the safe house when done" = 1. Nothing else.
2. The next pursuit update runs the normal **evade** handling (routines around `0x00410F90` to `0x00411236` and
   `0x00411240`):
   - only when no event is underway: request the post-pursuit results screen: the request constructor (`0x00641FE0`) copies the pursuit statistics of
     the player's pursuit and perpetrator into a results record (global at `0x0091D3D8`) and stores the byte from step 1
     as the field at `+0x24` of that record ("enter safe house after close"); it then opens `PostRace_MilestoneRewards.fng`;
   - broadcast a type 0x14 message with the pursuit owner's id on the gameplay subject;
   - apply the evade heat factor to every car of the stable (`pursuitlevels/default`, 0.95; routine `0x00590160`), then
     the strike bookkeeping of [career-economy-flows.md](career-economy-flows.md) section 4 and 6.
3. When that results screen closes (its destructor, `0x0058E320`), and the stored byte is set, the byte is cleared and
   the message `MEnterSafeHouse` is posted with the payload `safehouse`.

So an evade-into-the-safe-house costs exactly what a normal evade costs (heat x 0.95 on all cars, strike decay,
`EvadeCount`) and nothing more. There is no extra heat or cop reset.

### 3.5 Script side

`MEnterSafeHouse` is the C++ name of the message the Lua graph `menu_gate` handles as `entersafehouse`
([career-scripts.md](career-scripts.md) section 4). A native listener registered on the gameplay subject (`0x00632720`) wraps it and queues it for the script engine
(queue at `0x0091E0D0`); that is how the graph receives it. It has exactly two native senders: the engage component (step `+0x24`,
any of the three zone kinds, payload = the zone type string) and the results screen close of section 3.4 (payload
`safehouse`). The message is **not** sent by "the front end when the player confirms". The graph plays the gate's NIS
(if any) and calls the hook `DoZoneMenuAction(zone)` (`0x00612030`):

```
DoZoneMenuAction(zone collection):
    key = zone.attribute 0x2241F4CD (u32; 0 if absent)         # the zone's respawn marker
    manager.+0x108 = key                                        # last visited zone, any kind
    hud = first player's prompt component (give up if none)
    if hud.zone type == "safehouse":
        manager.+0x10C = key                                    # last safe house
    hud.open_front_end()                                        # +0x28, section 4
```

Note that the safe-house test uses the type remembered by `EnterGateZone`, not the collection. Only a safe house updates
`+0x10C`. **[exe]**

### 3.6 Other ways in

| Way | Routine | Effect |
|---|---|---|
| Pause menu "Jump to Safe House" (item id `0xE5C9C609` of `Pause_Main.fng`) | `0x00603220`, then a request for platform 2 | in career mode with a current bin and a current event node flagged `0xFF5EE5D6` and not flagged `0x4393F69B`: clears the state of every event of the bin and refreshes the bin; then the request of section 4 |
| Script `JumpToSafeHouse` | `0x006052B0` | request platform 2, screen `MainMenu_Sub.fng` |
| Script `JumpToCarLot` | `0x00605250` | sets the time-of-day preset index to 0, request platform 5, `Car_Select.fng` |
| Script `DoSafeHouseIntro(marker)` | `0x00605560` | `+0x108` and `+0x10C` = the key of the marker collection passed (its key field, not a hash), `+0x110` = 1, stage byte = 16, the intro movie name `storyfmv_saf25` with next screen `SafeHouseRivalBio.fng`, time-of-day index 0, request platform 2 with screen `FEAnyMovie.fng` (`WS_FEAnyMovie.fng` on wide screens) |

The pause path is **[exe]** for the call chain; the exact meaning of the two node flags is **[unconfirmed]**.

## 4. The request and the world unload

All of the above end in a *front-end request object* (16 bytes: platform at `+8`, optional screen name at `+0xC`). The
work is done in its destructor (`0x00623F80`); the object is allocated from the event pool, filled, and released.
**[exe]** The routine, in order:

```
if flow_state != 6 (in game): do nothing
platform  = request.platform; set front-end manager platform (and remember the previous)
if platform == 3: context |= 0x20
mode = race_status.play_mode
mode 0 (quick):   context = 4 (or keep main menu when bit 1 set); next screen MainMenu_Sub.fng / MainMenu.fng
mode 1 (online):  context |= 0x40 if LAN else 0x08; online screen
mode 2 (career):
    context = 1
    if requested screen is "SafeHouseRivalChallenge.fng" or equals the default FE screen (FEAnyMovie):
        context |= 0x20000                                   # Blacklist sequence
    else if platform == 5:   context |= 0x8000; screen = Car_Select.fng
    else if race_status.node == none:
        if stage == 16: context = (context & ~1) | 0x100; platform = 1 ; screen = MainMenu_Sub.fng
        else:           screen = MainMenu_Sub.fng
    else (an event node is current):
        if node flag 0x8CB01ABF:  as the stage-16 case above
        else if not final-event:  selected stage = stage; screen = SafeHouseRaceSheet.fng
        else if node complete:    mark game won (context |= 0x40000), platform = 2, screen = GameWon.fng
        else:                     as the stage-16 case above
    if platform == 2 and race_status.node == none:  manager.+0x110 = 1     # next load uses the safe house marker
if the request names a screen: that screen overrides the choice
clear the HUD overlay state; remove FadeScreen.fng if present; if Pause_Main.fng is up, unpause (resume the game clock)
start the game flow transition (state 6 -> 7 world unload -> 8 front-end load -> 3)
```

The world is therefore **unloaded completely** before the safe house screen exists: the player's car object is
destroyed (and writes its heat, section 8 of the second file), every AI vehicle, cop, traffic car and pursuit goes with
it. **[exe]**

## 5. Inside the safe house

The scene is `CAREER_SAFEHOUSE.BIN` (platform 2) and the screen `MainMenu_Sub.fng` (class constructed at `0x00570660`,
message handler `0x00531EA0`). While the front end is up the game clock of the world is not running because the world
does not exist. The preview car in the background is the current car handle of the settings (`0x005A3B60` loads it into
the preview slot). **[exe]**

### 5.1 The menu, built by `0x00532220`

Items are created in this order (label hashes in the FE data; the English labels are in [career-text.md](career-text.md)):

| # | Item | Condition | Action on select |
|---|---|---|---|
| 1 | Resume Free Roam | always | opens the confirmation dialog; the accept message `0xD05FC3A3` is handled in section 5.2 |
| 2 | Blacklist N | always | stores the current stage as the displayed stage (global `0x0091CA3C`), opens `SafehouseReputationOverview.fng` |
| 3 | Car Select | always | opens `Car_Select.fng` (message `0x0C407210`) in the safe house context |
| 4 | Rap Sheet | settings flag bit 4 | sets context bit 9 (`0x200`), opens `RapSheetLogin.fng` |
| 5 | Save | the memory-card layer is present | requests the save dialog (message `0x2251` through `0x00527940`, **[unconfirmed]**) |

After the items the screen fills `CAREER_DATA` (the career completion percentage, [career-rapsheet-2.md](career-rapsheet-2.md) section 4), `BOUNTY_DATA` (sum of the bounty of the stable,
`0x00590500`) and `CASH_DATA` (settings cash). **[exe]**

### 5.2 Resume Free Roam

Section 6 of the second file.

### 5.3 The Blacklist sequence (after a rival is beaten, and the first visit)

When the request carries context bit 17 a stepper (`0x005302C0`) walks a counter and opens one screen per call. Each
call adds 1 first; the counter starts at -1. Corrected order **[exe]** (the older text in [career-flow.md](career-flow.md) section 7.3 had the
autosave condition on the wrong step):

| Step | Screen | Condition |
|---|---|---|
| 0 | `SafeHouseRivalChallenge.fng` | always |
| 1 | `SafeHouseMarkers.fng` | always (the marker choice) |
| 2 | `SafeHouseRegionUnlock.fng` | only when the stage byte is 8 or 12; else skipped |
| 3 | the memory-card autosave flow: `MC_Main.fng` (`0x00527940`, mode flags `0x4000B2`), with `SafeHouseRivalBio.fng` stored as the screen to open when the flow ends | only when autosave is possible (below); else skipped |
| 4 | `SafeHouseRivalBio.fng` | always (reached directly when step 3 was skipped) |
| 5 | the rival movie `blacklist_<NN>` in `FEAnyMovie.fng` (`WS_` variant on wide screens) | always |
| 6 | see below | |
| 7 | none: broadcast and end | |

Step 6 by stage: **15** clears bit 17 and sets bit 0 and the settings flag bit 5 (`0x20`, "opening seen") and opens
`SafehouseReputationOverview.fng`, ending the sequence. **13** with the rap sheet flag (bit 4) still clear sets the flag,
sets the counter to 5 and plays the rap sheet movie; when it ends, step 6 runs again and, with the flag now set, leaves
the front end. Any other stage leaves the front end (the same leave as Resume Free Roam, section 6 of the second file). Step 7 broadcasts a message on the gameplay subject,
creates a type-0 event, clears bit 17 and ends the sequence; the player is then in the safe house menu. **[exe]**

*Autosave possible* (`0x005184D0`): context bits 3 (online) and 6 (LAN) clear, the memory-card layer present, and either
the profile's autosave option (profile `+0x38`) on or the card manager's manual-save byte set, and the current event node
(if any) not flagged `0xFF5EE5D6`; and the game is not in its final-event state. **[exe]**

**The first visit** (stage 16, from `DoSafeHouseIntro`): the request screen is the movie, the movie plays
`storyfmv_saf25`, then `SafeHouseRivalBio.fng`. The bio screen constructor (`0x00561320`) sees bit 17 and stage 16 and
posts the *enter bin* event with bin 15 (`0x0062C2D0`): it walks the stage byte down from 16 to 15, updates the gameplay
manager for each bin passed, activates the bin's event nodes and grants every reward the bin lists (through the reward
routine at `0x0062AA90`, which fills the popup queue). That is the only native place that turns stage 16 into 15 (the other two callers of the routine are the script function
`EEnterBin` and the event of the same name, [career-scripts.md](career-scripts.md) section 5). Which step the stepper
resumes at after the first-visit bio is **[unconfirmed]**: the counter is -1 when the front end is created
(`0x0054EA80`) and the navigation code that advances it was not followed. **[exe]**

### 5.4 Markers, region unlock, rap sheet, Car Select

- **Markers** (`SafehouseMarkers.fng`, class at `0x007B9A80`): reads the first six entries of the pending popup queue
  ([career-save.md](../formats/career-save.md) 3.2), keeps the entries whose reward type belongs to one of four
  category ids, shuffles up to four of them and shows them for choosing. The type table is in
  [career-text.md](career-text.md) section 7.1. Picking a marker applies it through the reward routine and clears the
  queue entry. The exact selection rule beyond this is **[unconfirmed]**.
- **Region unlock** (`SafehouseRegionUnlock.fng`, class at `0x0055B020`): a display screen. It starts an asynchronous
  load of `Global\Rivals.bin` and shows one of its pictures (`RIVAL_13_BG` at stage 12, `RIVAL_09_BG` at stage 8). It
  changes no game state itself; the region gates follow from the stage byte and the `Barriers` list of the bin
  ([career-regions.md](career-regions.md)). **[exe]**
- **Rap Sheet**: the item sets context bit 9 and pushes `RapSheetLogin.fng`; the family (`RapSheetMain`, `RapSheetRS`,
  `US`, `VD`, `CTS`, `TEP`, `PD`, `Rankings`) has constructors at `0x00570F60` to `0x005712C0`. The screens were not
  read beyond their registration, so what they show is **[unconfirmed]**.
- **Car Select**: sets the current car handle in the settings when a car is chosen; see
  [career-car-lot.md](career-car-lot.md). The next world load builds the player's car from that record.

### 5.5 What the safe house does *not* do

No routine of the screens above, the request, the stepper or the world load clears heat, the pursuit counters, the
bounty of the current pursuit, the strikes or the cop manager (the writers of heat are listed in section 8 of the second
file; the others were searched by their fields). The only heat/strike changes tied to the visit are the evade handling of section 3.4 (before the visit) and the
shop and impound screens (fine payment sets the car's heat to 1.0; releasing an impounded car sets it to 0), described in
[career-economy-flows.md](career-economy-flows.md). **[exe]**
