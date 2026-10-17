# Career flow: starting, resuming, the safe house and saving

How the original gets from the main menu into a career, what a new career starts with, what the safe house front end does,
and when it saves. The car lot and the car rules are in [career-car-lot.md](career-car-lot.md); the byte layout of the
saved state is in [career-save.md](../formats/career-save.md). Addresses, Ghidra names and dead ends are in the
[provenance file](../provenance/career-flow.md).

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) through the Ghidra MCP; the install at
  `D:\Need For Speed Most Wanted Black Edition` (`GLOBAL/gameplay.bin`, `GLOBAL/FE_ATTRIB.bin` and the language table); the
  three saves on this machine, read only.
- **Evidence tags:** **[exe]** read in `speed.exe` through Ghidra, not tested in game. **[verified]** checked against the
  install files or the saves. **[community]** taken from a community summary (the extra-options mod docs in this repo), not
  re-checked. **[unconfirmed]** a guess or an unused/undecoded path.
- **Addresses** are `speed.exe` addresses. Ghidra auto names are only in the provenance file.
- The Lua state-graph scripts in `gameplay.bin` (compiled Lua 5.0) hold a large part of the rules (heat reset, intro
  scenes, unlock timing). They are not decoded, so every rule that lives there is marked.

## 0. Summary

1. The game keeps one **front-end context word** (`FEDatabase + 0x12C`, section 1). The main menu buttons only set bits in
   it; the next screen class and the game mode are chosen from the bits. **[exe]**
2. **Career** on the main menu sets bit 8 (the career manager = the alias/profile screens). After a profile is loaded, the
   decision is one bit: if the settings flag "career started" (bit 0 of the flags word) is clear it is a **new career**,
   otherwise **resume**. **[exe]**
3. A new career resets the settings, makes the current car the preset `M3GTRCAREERSTART`, sets cash to 0, stage 16,
   and starts the prologue event `16.1.0` (the D-Day opening race). **[exe + verified]**
4. A resume goes straight to the **safe house front end** (scene `CAREER_SAFEHOUSE.BIN`, screen `MainMenu_Sub.fng`), unless
   the stage is still 16 (prologue) or the final event is next. **[exe]**
5. The safe house menu has Resume Free Roam (with a confirmation), Blacklist N, Car Select, Rap Sheet (once unlocked) and
   Save. Cars are bought and sold in the car lot and the safe house Car Select. **[exe]**
6. The game **autosaves** when free roam starts and when a race ends (except in the opening race), if the autosave option is
   on and nothing is waiting on screen. **[exe]**
7. Cash starts at 0. A first profile gets +10,000 if an Underground 2 profile folder exists. **[exe]**

## 1. The front-end context word **[exe]**

`FEDatabase + 0x12C` is a bit set that says where the player is in the front end. Bits found, with how they are set:

| Bit | Value | Meaning | Set by |
|---|---|---|---|
| 0 | 0x1 | career | main menu Career (when no profile screen is needed); the resume and new-career code |
| 2 | 0x4 | quick race (also "2 players" together with `FEDatabase[0] == 2`) | Quick Race button |
| 3 | 0x8 | online | Online button; also set when returning from an online race |
| 4 | 0x10 | options | Options button |
| 5 | 0x20 | My Cars / customize | My Cars button; the customization shop request |
| 6 | 0x40 | LAN | LAN Play button |
| 7 | 0x80 | alias manager | Alias Manager button |
| 8 | 0x100 | career manager (profile screens in career) | main menu Career |
| 9 | 0x200 | Rap Sheet | the Rap Sheet button in the safe house |
| 10 | 0x400 | quick race main | quick race screens |
| 15 | 0x8000 | car lot | request for the car lot |
| 17 | 0x20000 | a post-race Blacklist sequence is on screen | rival challenge screen |
| 18 | 0x40000 | game won | the Game Won screen request |

The context also builds the **text/attribute name prefix** used to look up per-screen data (`career_`, `carlot_`,
`quickrace_`, `quickracemain_`, `online_`, `options_`, `customize_`, `career_manager`, `main_fe`).

Other front-end state:

- `FEDatabase[0]` is the number of players (1 or 2); `[3]` is 1 once a profile has been loaded.
- The front-end manager (global at `0x0091CAE0`) has the current **platform** (the 3D scene behind the menus) at `+0x18`
  and the previous one at `+0x1C`:

| Platform | Scene file | Tag |
|---|---|---|
| 1 | `FRONTEND/PLATFORMS/PLATFORMCRIB.BIN` | `QRACE` |
| 2 | `.../CAREER_SAFEHOUSE.BIN` | `CAREER` |
| 3 | `.../CUSTOMIZATION_SHOP.BIN` | `CSHOP` |
| 4 | `.../CUSTOMIZATION_SHOP_BACKROOM.BIN` | `CSHOP` |
| 5 | `.../CAR_LOT.BIN` | `CARLOT` |

- The top-level state machine value (`0x00925E90`) is 3 while the front end runs and 6 while a world is running; 2 is
  the unload step. The named steps are `LoadingFrontEnd`, `UnloadingFrontEnd`, `BeginGameFlowLoadTrack`,
  `BeginWorldLoad`, `LoadingRegion`, `WaitForSimulation`, `FinishedGameLoading` (strings in the exe).

## 2. From the main menu to a career

Main menu buttons, in order, with the context bit each sets **[exe + verified labels]**:

| Button (English) | Sets |
|---|---|
| Career | bit 8 (or bit 0 when no profile layer is available) |
| Challenge Series | opens the series screen; asks to create a profile first if none is loaded |
| Quick Race | bit 2 |
| My Cars | bit 5 |
| LAN Play | bit 6 |
| Online | bit 3 |
| Alias Manager (only with the profile layer) | bit 7 |
| Options | bit 4 |

A demo build (a different check) shows only four of them; not relevant to the PC retail.

**Career then goes to the alias manager** (`MC_ProfileManager.fng`, see [save-alias-manager.md](save-alias-manager.md)).
When a profile is created or loaded and the front end is idle (`0x00925E90 == 3`), the per-frame front-end update clears the
context word and decides:

1. profile settings flag bit 0 clear -> **new career** (section 3);
2. bit 0 set -> **resume** (section 4).

If no profile is loaded, or the pending memory-card action is a save/load flow, the same update routes to the load or
restore screens instead. A failed load restores a snapshot of the profile that was taken when the profile screens opened.

## 3. New career **[exe]**

1. The settings are reset (`career-save.md` section 3): current car 0, flags 0 then bit 0 set, **stage 16**, **cash 0**,
   the 150 SMS slots marked valid or 0xFF by whether the language table has the text, the SMS counter 0.
2. The current car of the profile and the quick-race selection slot for the current race type are both set to the hash
   of the preset `M3GTRCAREERSTART` (`0x38A3C42C`). That preset is slot 0 / slot 30 of the stable
   ([career-save.md](../formats/career-save.md) section 4.1).
3. A profile serial text `MW-<n>-<4 letters>XXX` is generated (random seed `0x19B3`).
4. The settings flags get the "career started" bit; if the bonus-content probe says so, a popup of type 2 is queued and
   flag bit 17 is set.
5. The front-end context is set to career (`0x1`).
6. The gameplay graph node **`16.1.0`** is activated (state 2) and the world load begins (`UnloadingFrontEnd` when
   leaving the front end, otherwise `LoadingRegion`). Node `16.1.0` is the collection `16_1_0_partial_dday` of
   `gameplay.bin`: the opening D-Day race with the call from Mia and the Razor boss spawn **[verified]**.

Prologue events in `gameplay.bin` **[verified]**: `16_1_0_partial_dday`, `16_1_1_circuit`, `16_2_1_d_day_intro`,
`16_2_2_sprint`, `16_2_3_sprint`. Stage 16 means "none of the 15 Blacklist rivals is the target yet". The code that lowers
stage from 16 to 15 was not found (it is in the progress logic or the scripts) **[unconfirmed]**.

**Starting cash** is 0 **[exe]**. Community: the first three races pay 30,000 in total **[community]**. When the new
profile is created and a folder `NFS Underground 2` exists (the check lists the directory and needs more than two entries,
in the folder named by a shell-folder id `0x801C`), a dialog is shown and **+10,000** is added; this is the
`AwardedCash 10000` of the extra-options mod **[exe + community]**.

**Developer flags** (set only by an unreached debug boot menu; all 0 in retail): `0x00926124` unlock everything,
`0x00926125/6` skip the prologue (stage 15, add the E3 demo BMW and a second preset car), `0x00926127` show hidden vehicles
in the stable, `0x00926129` include the start car in the default stable **[exe, unconfirmed whether reachable]**.

## 4. Resume career **[exe]**

Resume checks the state of three nodes with the gameplay graph:

- **Stage 16** (prologue) and node `16.2.1` not yet complete: current car := `M3GTRCAREERSTART`, re-activate the saved
  prologue node and start the world (so a prologue always restarts from the opening).
- **Final event pending**: settings flag bit 14 clear and node `1.2.3` complete -> activate node `1.8.1` (the last event)
  and start the world; flag bit 14 also unlocks the extra cars of [career-car-lot.md](career-car-lot.md).
- **Otherwise**: platform := 2 (safe house), context bit 8 cleared, bit 0 set -> the front end shows the **safe house**.

So a normal resume lands in the safe house menu, not in the world.

## 5. Game modes

- The play mode of the running world is `GRaceStatus + 0x1964`: **0 quick race, 1 online/LAN, 2 career** **[exe]**. The
  script hook `IsCareerMode` returns `mode == 2`. `SetRoaming` sets it to 2 (the career free roam). It is copied from the
  active event node at event start.
- The mode of the **session** is chosen when the world loads: online -> `OnlineGame`; else context bit 0 set (and not the
  2-player quick race) -> `CareerGame`; else `QuickGame` (the three strings are keys into a registry of game-mode
  factories) **[exe]**.
- When the front end is opened from a world the request object decides the screen from the play mode: mode 0 returns to
  the main menu (or `MainMenu_Sub.fng`), mode 1 to the online menus, mode 2 to the career screens (section 6).

## 6. Opening the front end from the world

A request object (front-end screen request: platform id and screen name) is created by the script hooks `JumpToSafeHouse`
(platform 2, `MainMenu_Sub.fng`), `JumpToCarLot` (platform 5, `Car_Select.fng`, after resetting the focus car), by the
pause menu's "Jump to Safe House", and by the zone entry handler for `safehouse`, `carlot` and `customshop` markers
(`customshop` opens `CustomizeMain.fng` and needs a current car) **[exe]**. When it runs in game (`0x00925E90 == 6`):

1. In career mode it sets the context to career (bit 0).
2. Platform 5 (car lot): bit 15 set, screen `Car_Select.fng`.
3. A request for `SafeHouseRivalChallenge.fng` or the front end's pending screen: bit 17 set (the Blacklist sequence).
4. Free roam (no event node): if stage is 16 the context drops career and sets bit 8 (prologue safe house is the profile
   page); else the screen is `MainMenu_Sub.fng`. Also, when the platform is 2, a free-roam flag in the gameplay manager
   (`+0x110`) is set so the next world load starts at the **safe house start point** (below).
5. With an event node active: `SafeHouseRaceSheet.fng` (the Blacklist race sheet) unless the current event is the final
   one: then, if node completion says the game is won, the settings flag bit 14 is set, context bit 18 is set, platform 2 and
   the screen is **`GameWon.fng`**.
6. Fade and pause screens are removed, the world is unloaded.

**Script hooks** in the registry at `0x0061E750` that matter here **[exe]**:

| Hook | Does |
|---|---|
| `IsCareerMode` | play mode == 2 |
| `SkipCareerIntro` | true when the debug flag is set, or settings flag bit 5 ("opening seen") is set |
| `DoSafeHouseIntro(name)` | stores the hash of `career_start_<region>` in the manager's two start slots, sets the free-roam flag, sets stage to 16, opens the front end (request mode 2) |
| `JumpToSafeHouse`, `JumpToCarLot` | open the front end as above |
| `AllowEngageSafehouse` | reads a boolean of the manager (+0x2CF) |
| `SetHasRapSheet` | sets settings flag bit 4 |
| `SetWorldHeat`, `PlayTutorial`, `IntroduceRival`, `Fade`, `ShowWinningPostRaceScreen` ... | other hooks, not covered here |

**The safe house start point** is the gameplay collection `career_start_<region>`. The install has one: `career_start_l2ra`
(kind `career_start`), position (981.90, 2148.45, 152.29) in game coordinates and heading 53.08 **[verified]**; the world code
maps the position to engine axes as (-y, z, x). The exe looks the collection up by the stored hash, builds
`career_start_%s` when the stored hash has no collection, and falls back to the manager's explicit override.

## 7. The safe house front end

### 7.1 Entering **[exe + verified]**

The safe house is a world marker (`zone_safehouse`, kind `safehouse`; 4 zones in the install, see
[gameplay-races.md](../formats/gameplay-races.md) section 7). Driving into the gate fires the `menu_gate` state graph
(`initial` -> handler `entersafehouse` -> `nis`) and the message `MEnterSafeHouse`, which the script layer receives. The
script then runs the NIS and calls the request above. What the scripts do to heat, pursuit state and the car when the gate
triggers is **not decoded**; no heat reset was found in `speed.exe`.

### 7.2 The menu **[exe + verified labels]**

`MainMenu_Sub.fng` with context bit 0 builds the safe house menu (the same package is the options page in another context):

| Button | Action |
|---|---|
| Resume Free Roam | confirmation "Are you sure you want to leave the Safe House to resume free roam?", then leaves the front end |
| Blacklist <n> | opens the reputation overview (`SafehouseReputationOverview.fng`) at the current stage |
| Car Select | opens `Car_Select.fng` in the safe house context |
| Rap Sheet | only when settings flag bit 4 is set; sets context bit 9, opens `RapSheetLogin.fng` |
| Save | only when the profile layer exists; opens the save dialog |

The screen also fills three text bindings: career (the stage), bounty (the stable's total bounty) and cash. The other
safe house screens in the factory table are `SafehouseRaceSheet`, `SafehouseRivalChallenge`, `SafehouseRivalBio`,
`SafehouseMilestones`, `SafehouseRegionUnlock`, `SafehouseBounty`, `SafehouseMarkers`, the RapSheet family and
`SMS_Mailboxes`/`SMS_Message`.

### 7.3 Arrival sequence after a rival is beaten **[exe]**

When the front end opens with the "Blacklist sequence" flag, a stepper runs these screens in order:

1. `SafeHouseRivalChallenge.fng`.
2. `SafeHouseMarkers.fng` (choose the Get Out of Jail Free markers) - only if autosave is possible.
3. `SafeHouseRegionUnlock.fng` - only at stage 8 and 12.
4. The memory-card autosave screen (`MC_Main.fng`) if autosave is possible.
5. `SafeHouseRivalBio.fng`.
6. The rival's movie (`blacklist_<NN>` in `FEAnyMovie.fng`).
7. At stage 15 the reputation overview, and settings flag bit 5 ("opening seen") is set; at stage 13, if the Rap Sheet is
   not unlocked yet, it is unlocked (flag bit 4) and the rap sheet movie flow is entered.
8. A message is posted to the gameplay broadcast and the sequence ends.

The details of ranks and rivals are in the Blacklist specs; only the order is recorded here.

### 7.4 What else the safe house does

- **Car change**: Car Select (context bit 0, no bit 15) lists the owned career cars; picking one sets the settings'
  current car handle. See [career-car-lot.md](career-car-lot.md).
- **Selling** happens in this screen (section 3 of the car lot file).
- **Garage capacity**: 25 career records exist in the save; the game lets the player own at most **10** purchased cars
  (the extra-options mod's `PurchasedCarLimit` default). The add-car routine itself refuses past 25 **[exe]**.
- **Saving**: section 8.
- **Heat, pursuit cooldown, bounty payout**: not found in the C++ of the safe house screens. Bounty and impound rules
  are in the economy and busted specs.

## 8. Saving

- **Where:** `Documents\NFS Most Wanted\<alias>\<alias>`; the folder name is the alias; created if missing. The root is the
  user's documents folder plus the name `NFS Most Wanted` (a string at `0x00901748` that mods can change) **[exe + verified]**.
- **Autosave switch:** the profile option at options +0x14 (profile +0x38), default 1. Autosave is skipped online, when the
  memory-card layer is off, when no profile has been loaded (unless the manual save flag is on), and while the current event
  is the final event `1.8.1` or an event whose gameplay data has one particular boolean field set (hash `0xFF5EE5D6`)
  **[exe, field meaning unconfirmed]**.
- **Trigger:** an `AutoSave` event (type 0x10) is created by `SetRoaming` (every time free roam starts) and when a race ends,
  except while the current node is the opening race `16.1.0`. The event only raises a "save pending" bit in the
  memory-card manager; the save runs on a later frame when the gameplay manager has no pending notification
  (`GManager` list empty) **[exe]**. The overlay `Autosave_Overlay.fng` is shown.
- **Manual save:** the Save button and the MC screens. A save serializes the profile
  ([career-save.md](../formats/career-save.md)) into a buffer and writes it in one pass; a snapshot of the saved buffer is
  kept to detect unsaved changes (the game compares a fresh serialization with it when asking "save the alias?").
- **Load:** reads the file, checks the container, then the profile (MD5, default stable hash). A failure shows the
  "corrupt" or "load failed" text and restores the snapshot.

## 9. Touch points with other areas

| Area | Where this flow meets it |
|---|---|
| Bounty, cash, infractions, impound | the stable tail and the career records; cash spend in the shop checkout and the car lot; `AwardPlayerBounty` hook |
| Blacklist, rivals | stage byte, node list, the arrival sequence, rival cars unlocked by stage (car lot file) |
| Milestones | node list and the pending popup queue |
| Pursuit / heat | per-car heat lives in the career record; resets of heat are in scripts |
| Frontend menus | screens and button labels |

## 10. Rewrite

The rewrite has no career state yet. Needed first: the settings struct and the stable of
[career-save.md](../formats/career-save.md), the context word of section 1 as an enum, the new/resume decision of
section 2 and a safe house screen with the five buttons of section 7.2.

## Open questions

1. What the scripts do on entering a safe house (heat, pursuit state, car repair).
2. Where stage changes from 16 to 15 and from n to n-1 (the progress logic).
3. The exact set of conditions that block autosave.
4. What the developer flags and the "bonus content" probes read.
5. Whether the `0x0000801C` shell-folder id in the Underground 2 check is a documents folder or a different one.
