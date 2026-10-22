# Busted sequence: from the meter filling to the career screen

What the original does after the busted meter fills: the timing, the arrest scene, the on-screen packages, what the
player can pay or choose, and what each choice does to the career. The meter and the chase are in
[ai-pursuit.md](ai-pursuit.md) (section 5) and [ai-pursuit-heat.md](ai-pursuit-heat.md) (sections 4 to 6); this file is
the sequence after that point. The rewrite's current state is in the last section.

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) through the Ghidra MCP; the install at
  `D:\Need For Speed Most Wanted Black Edition` (FEng packages, NIS bundles, MOVIES, strings table); the existing
  specs above for the values they already cite. The community decompile (dbalatoni13/nfsmw) was not re-read here;
  where a value comes from it, the tag says so.
- **Evidence tags:**
  - **[verified]** checked in the install files (screen dump, strings table, chunk tree, file list) or in game data.
  - **[decomp]** read in `speed.exe` through Ghidra in this pass. Not tested in game.
  - **[community]** taken from the decomp summary already in the repo specs (the cited values and rules), not re-checked here.
  - **[unconfirmed]** a guess, or a rule whose use or name is not proven.
- **Addresses** are `speed.exe` addresses. Ghidra's auto names are only in the provenance file.

## 0. Summary

1. The meter fills, the pursuit ends busted, and two messages go out. The gameplay one (name `MPerpBusted`) reaches
   at least four listeners. A second, cutscene-channel listener is registered on a different subject. **[decomp]**
2. About 3 s later (the 3 s is **[community]**; the binary's timer is not located) the cutscene listener runs one
   handler. It picks an arrest scene from a fixed list of 26 bundles, looks for an arrest spot next to the player, and
   plays the scene with the player's car bound as the actor. **[decomp]**
3. If there is no spot and the player is in free roam, the game pushes the front-end screen `Infractions.fng`. If the
   player is in a race, the game posts `EPause(0, 1, 0)` instead: the race pauses. **[decomp]** ([nis-player.md](nis-player.md))
4. The arrest is an NIS (cutscene), not an FMV. No busted movie exists in the install. **[verified]**
5. After the NIS the game shows the infractions screen (`Infractions.fng`, pay with cash or a marker), an in-game
   dialog (`InGameBusted.fng`, not named anywhere in `speed.exe`) and an in-game overlay (`BUSTED_OVERLAY.fng`, a
   surveillance-camera look). The code that shows the overlay or the NIS-end path was not found. **[unconfirmed]**
6. `PostBusted.fng` is named in `speed.exe` but is not in the install, so the Black Edition does not ship it.
   **[verified]**
7. Paying fines, using a Get Out of Jail Free marker, or having too little cash changes the car's impound strikes,
   unserved infractions and impound flag. The strings confirm this; the exact rule for the marker is in conflict
   (section 7). **[verified text]** and **[community]**.

## 1. Install files that take part **[verified]**

| File | Size or content | Used for |
|---|---|---|
| `NIS/Scene_ArrestXX_BundleB.bun` (26 files) | `ArrestF02`, `F02b`, `F06`, `F06b`, `F07`, `F07b`, `F14`, `F14b`, `F18`, `F18b`, `F23`, `F23b`, `M01`, `M01b`, `M04`, `M04b`, `M06`, `M06b`, `M07`, `M07b`, `M14`, `M14b`, `M16`, `M16b`, `M19`, `M19b`. About 1.3 to 1.7 MB each | the arrest NIS scenes (section 4) |
| `NIS/` arrest names `ArrestM23`, `ArrestM23b` | **no bundle file** | named in `speed.exe` and in the scene table, never reached by the chooser (section 4.1) |
| `FRONTEND/FrontB.lzc` | `Infractions.fng` (57 objects, 2 buttons), `RapSheetPD.fng` (56), `RapSheetMain.fng`, `RapSheetLogin*.fng`, `RapSheetCTS/US/VD/RS/TEP` and others | front-end infractions and rap sheet |
| `GLOBAL/InGameB.bun` | `BUSTED_OVERLAY.fng` (32 objects), `InGameBusted.fng` (3 objects, 2 buttons), `PostRace_Pursuit.fng` (95), `InGameBounty.fng` (111) | in-game overlay, in-game bust dialog, pursuit summary |
| `MOVIES/*.vp6` (32) | attract, logos, PSA, 15 blacklist, 5 tutorials, 9 story FMVs | **no busted movie** (section 5) |
| `speed.exe` | the strings for the arrest names, `BUSTED_OVERLAY.fng`, `Infractions.fng`, `PostBusted.fng`, `MPerpBusted`, `AIRacerBusted` | the code below |

## 2. Order of events and timing

Times are game time from the moment the busted meter fills (`busted_timer > 5.0`, **[community]**, see
[ai-pursuit.md](ai-pursuit.md) 5.2). The steps after the bust are **[decomp]** unless tagged.

| Step | When | What happens | Evidence |
|---|---|---|---|
| 1 | T0 | The pursuit is busted. The cops get their lights off and a brake goal; the player's controls are forced. | [community] |
| 2 | T0 | The gameplay message `MPerpBusted` is delivered to its listeners (section 2.1). | [decomp] |
| 3 | T0 + 3 s | The cutscene-channel listener runs the arrest handler at `0x0044dc70`. The 3 s is the busted HUD time in the community summary; the timer itself is not in the code read. | [decomp] for the handler, [community] for 3 s |
| 4 | T0 + 3 s | The handler picks a scene name (`0x0044d770`) and sets it as the NIS scene. | [decomp] |
| 5 | T0 + 3 s | The handler looks for an arrest spot next to the player (`0x0044da10`). | [decomp] |
| 6a | spot found | The NIS activity is created (name hash of `NISActivity`), the NIS component is found, and the player's perp handle is passed to its play call. The NIS plays the chosen scene. | [decomp] |
| 6b | no spot, free roam | `Infractions.fng` is queued as a front-end screen (mode 2). No NIS, no arrest camera. | [decomp] |
| 6c | no spot, race | `EPause(0, 1, 0)` is posted: the race pauses ([nis-player.md](nis-player.md)). | [decomp] |
| 7 | during the NIS | The NIS runs its own camera and event track (section 4.3). The port's procedural arrest camera covers this in the rewrite. | [decomp], [unconfirmed] for the camera |
| 8 | NIS end | Not located. The screen the player sees next is open (see Next leads). | [unconfirmed] |
| 9 | after | The infractions screen (`Infractions.fng`) offers pay with cash or a marker (section 6.3). | [verified] text, [unconfirmed] route |

If step 6a fails to create the NIS activity, the handler returns with nothing shown. There is no fallback camera or
fallback screen in that path. **[decomp]** The rewrite should choose its own fallback.

### 2.1 Who listens to the bust **[decomp]**

The gameplay message is one message type (its name hash is cached at `0x004080d0` from the string `MPerpBusted`,
at `0x00890d98`). Five subscriptions use that type:

| Listener | Subject | Registered in | What it does |
|---|---|---|---|
| `0x00642910` | broadcast `0x20d60dbf` | registration table `0x00648590` | forwards the message to the script event queue |
| `0x00700f70` | broadcast `0x20d60dbf` | speech system constructor `0x007200c0` | speech / cop comment side (not read further) |
| `0x0070ad10` | (register-passed, not resolved) | `0x007200c0` | not read |
| `0x004dfc20` | broadcast `0x20d60dbf` | constructor `0x004f8830` | not read |
| `0x0044dc70` | `0xfea34c0a` | arrest listener constructor `0x0044f160` | the cutscene handler (section 4) |

The broadcast subject is `0x20d60dbf`; `0xfea34c0a` is a separate subject that the arrest handler listens on. The
name "cutscene channel" for `0xfea34c0a` matches the community summary and is **[unconfirmed]**. The message sender was
not found (Next leads).

## 3. What the player sees on the chain

| Screen or effect | Package | Shown when | Tag |
|---|---|---|---|
| Arrest NIS | `NIS/Scene_Arrest*` | an arrest spot is found (free roam or race) | [decomp] |
| Overlay | `BUSTED_OVERLAY.fng` (`GLOBAL/InGameB.bun`) | not located | [unconfirmed] |
| In-game bust dialog | `InGameBusted.fng` (`GLOBAL/InGameB.bun`) | not located; no name or hash of it is in `speed.exe` | [unconfirmed] |
| Infractions and payment | `Infractions.fng` (`FRONTEND/FrontB.lzc`) | free roam with no spot, and presumably after the NIS | [decomp] for the first, [unconfirmed] for the second |
| Pursuit summary | `PostRace_Pursuit.fng` (`GLOBAL/InGameB.bun`) | race or pursuit end (the pursuit-end card) | [verified] content |
| Rap sheet, one pursuit | `RapSheetPD.fng` (`FRONTEND/FrontB.lzc`) | from the rap sheet, not directly after a bust | [verified] content |
| `PostBusted.fng` | **not in the install** | never | [verified] |

## 4. The arrest scene

### 4.1 Choosing the scene **[decomp]**

The chooser (`0x0044d770`) fills a 32-byte name buffer and sets the NIS scene by the hash of the final name (through
the NIS manager object at `0x008ee320`, which the animation spec also names as the camera pack owner). It reads two
inputs:

- a **heat** value (0 to 3, or other) read through the perp car's interface (`GetHeat`; earlier versions of this page called it a class). Classes 0 and 1 take one path; class 2
  and class 3 (or any other value) take the other.
- a **variant** index, 0 to 7, from a random helper (`0x0045da70` called with 0.99, rounded by `0x007c4b80`). The exact
  rule is **[unconfirmed]**.

Result, from the name tables in the chooser:

| Class | Variant index | Scene |
|---|---|---|
| 0 or 1 | 0, 1, 2, 3 | `ArrestM01`, `ArrestM16`, `ArrestF02`, `ArrestF18` |
| 0 or 1 | 4, 5, 6, default | `ArrestM01b`, `ArrestM16b`, `ArrestF02b`, `ArrestF18b` |
| 2 or 3 or other | 0 to 6 | `ArrestM06`, `ArrestM19`, `ArrestF06`, `ArrestF07`, `ArrestM06b`, `ArrestM19b`, `ArrestF06b` (index order **[unconfirmed]**) |
| 2 or 3 or other | above 6 | `ArrestF07b` |

Notes:

- For class 2 and class 3 the chooser also writes names from two more tables (`ArrestM04`, `ArrestF23`, `ArrestM07`,
  `ArrestM14`, `ArrestF14` and their `b` variants). Each write copies over the same buffer, so only the last one
  survives. The buffer is overwritten by the third table, so these six names and their `b` variants are not reachable
  as the final scene in this chooser. **[decomp]** (the copy semantics of the helper are assumed from its arguments)
- `ArrestM23` and `ArrestM23b` are in the scene table and the strings, but the chooser never selects them and no
  bundle exists for them. **[verified]** (file list) and **[decomp]** (chooser).
- So 8 scenes are reachable from class 0 or 1 and 8 from class 2 or 3. The `b` names share a flag bit with their base
  in the scene table; the meaning of the bit is **[unconfirmed]**.

### 4.2 The arrest spot **[decomp]**

The spot finder (`0x0044da10`) takes the perp car's position, runs a query on the road or track (the helper is
`0x007862d0`, not decoded), and if it finds a spot it returns the midpoint of a two-point segment as the arrest point,
with a height that is the average of the two. It also writes a float between 0.25 and 1.25 (from `1 - d + 0.25`, where
`d` is a distance-like value from `0x005cb5d0`); its use is **[unconfirmed]**. If there is no spot the handler takes
the "no spot" path (section 2 step 6b or 6c).

### 4.3 The NIS bundle **[verified]**

Example `NIS/Scene_ArrestF02_BundleB.bun`, read with `tools/chunkdump.py`:

- Scene header name `ArrestF02`; description text "Cop walking perp in hand cuffs, cop shoves perp".
- Three `AnimSceneEntityData` records (three actors). The handler hashes the name `car1` but does not use the result
  in the decompiled code, and passes the perp handle to the play call. The three actors are probably a cop, the perp and
  a third entity (**[unconfirmed]**; `docs/models.md` names `FCop01.BIN`, `Perp01.BIN`, `Cuffs01.BIN` as the models of
  the cop and perp scenes).
- Three `TexturePack` chunks, three `GeometryPack` chunks (about 175 KB, 145 KB and 39 KB of geometry), three
  `EAGLSkeletons` chunks, two `EAGLAnimations` banks (61 KB and 1.4 KB), one `CarpEventSequence` (2 KB), and one
  `AnimScene (NisScene)` root.
- No camera chunk in the bundle. The camera for the scene is not in this file.

### 4.4 Camera **[decomp + verified]**

- The chooser always asks for camera track 0; the scene's track comes from the NIS camera pack of `GLOBAL/InGameB.bun`,
  keyed by the hash of the scene name ([nis-player-2.md](nis-player-2.md), [nis-scenes.md](nis-scenes.md)). The
  notes below are the earlier reading.
- The arrest NIS has no camera data in its own bundle (section 4.3).
- The docs record ICE camera packs `0x8003B200` to `0x8003B203` in `GLOBAL/InGameB.bun` (NIS, FMV and replay
  cameras; [animation.md](../formats/animation.md)). The chooser of a camera track for arrests was not located, so the
  link from a scene to a pack is open.
- `NisCameras` is named in `speed.exe` but its one reference is in an init or teardown function, not in the arrest
  path.
- The community summary says the arrest uses a scene and a camera track chosen by the cutscene system. That matches the
  data, but the track choice is not confirmed.
- The rewrite's procedural arrest framing is a guess. See [camera-views.md](camera-views.md).

### 4.5 Freeze and fallbacks

- The freeze of the cops and the player's controls during the NIS is **[community]**; no code in the arrest handler
  does it.
- If the NIS cannot be created, the handler does nothing (section 2). There is no black fade, no overlay and no
  screen in that path.

## 5. Movies

- **No busted movie.** The `MOVIES/` folder has 32 files (attract, logos, PSA, 15 blacklist, 5 tutorials, 9 story
  FMVs). None is a bust or an arrest. **[verified]**
- The arrest is an NIS (`NIS/Scene_Arrest*`), which plays in-engine. **[decomp]**
- The NIS folder has FMV-named scenes (`Scene_FMV*`), but none is a bust. **[verified]**
- The `BUSTED_OVERLAY.fng` package is a FEng screen, not a movie, so it does not play video. **[verified]**

## 6. Front-end and in-game packages

### 6.1 `BUSTED_OVERLAY.fng` (`GLOBAL/InGameB.bun`, 32 objects) **[verified]** content

- A surveillance-camera look: a dark base (1046 by 540, alpha 0 at start, with a 240-tick fade script), a noise tile,
  a TV-frame mask, corner crosshairs, a glow light and a "REC" label with a blinking glow (7600-tick script).
- Text: "ROCKPORT POLICE" and "VID CAM 01" (fixed), and "SUSPECT: WHITE MALE 185LBS. 20-25 YRS: APPREHENDED FOR EXCESSIVE
  SPEEDING" (fixed sample text, scrolled by a 9600-tick script). There is no data binding in the package.
- Two messages drive it: one runs the group script `0x5079c8f8` (also used by the loading screen's stripe), the other
  fades the base image (`0x02c856ed`). The message names are not resolved.
- **[unconfirmed]** The screen is not shown by the in-game overlay router (`0x004c2340`, section 6.5). Its show path is
  not located. The name is only in the table of that router, and the router does nothing for it.

### 6.2 `InGameBusted.fng` (`GLOBAL/InGameB.bun`, 3 objects, 2 buttons) **[verified]** content

- Text: "You been busted. Now pay up!" (the package's wording; the strings table has "You've been busted! Pay your fines
  to get out of jail.").
- Two buttons: "Use Marker" (get out of jail, at 107, 159) and "Pay Infractions" (at 107, 130). Each button's press
  message is `BUTTON_PRESSED` (`0x0c407210`, checked by the FEng name hash). "Pay Infractions" runs a 20-tick script
  and a button response.
- The string `InGameBusted.fng` is not in `speed.exe`, and its FEng name hash is not found there. So nothing in the
  binary names this screen. **[verified]** (search). Its use is open (Next leads).

### 6.3 `Infractions.fng` (`FRONTEND/FrontB.lzc`, 57 objects, 2 buttons) **[verified]** content

- A ticket layout: "CITY OF ROCKPORT", "Violation:", a ticket number, "Unserved Infractions", "Infractions this Pursuit",
  "TOTAL COST:", "Cash:", and the heading "Payment" (the label is the same key as the text "Payment for infractions is now due").
- Images: `BUSTED_ENG.tga`, `Post_Busted_Backing.tga`, `Infraction_Ticket.tga`, `Marker_Icon_Misc_Jail.tga`,
  `Generic_Alert.tga`, `BaseStripe.tga`, `Crosshair_Corner.tga`.
- Two payment buttons: "Pay with Cash" and "Pay with Marker". Each button press runs a payment script and a screen
  change to the cash or the marker panel. Two package responses lead to a game action and a sound (the game action
  names are not resolved).
- The numbers in the package ("20,000", "60,000", "80,000", "99", "0") and "#01AO692JM" are **placeholders**. The
  runtime values must come from the career data (section 6.6). **[verified]** that they are sample text.

### 6.4 `PostRace_Pursuit.fng` and `RapSheetPD.fng` **[verified]** content

- `PostRace_Pursuit.fng` (in `InGameB`): labels "Length", "Escape Within", "Consecutive Pursuits Evaded", "Cops
  Damaged", "Cops Destroyed", "Cost to State Achieved", "Roadblocks Dodged", "Unserved Infractions", "INFRACTIONS",
  and "Continue". This is the pursuit-end summary. It is listed in the in-game overlay router at index 32 (section 6.5).
- `RapSheetPD.fng` (`FRONTEND`, rap sheet, "ROCKPORT PD"): "CASE_FILE", "PURSUIT ID", "PURSUIT DETAIL", "PURSUIT LENGTH",
  "TOTAL COPS INVOLVED", "COPS DAMAGED", "COPS DESTROYED", "ROADBLOCKS DODGED", "SPIKE STRIPS DODGED", "HELICOPTERS
  INVOLVED", "COST TO STATE", "BOUNTY ACCUMULATED", "INFRACTIONS RECORDED", and the buttons "Accept", "Back", "Leave".
  The values are placeholders ("PURSUIT ID: 56XK3", "XX:XX:XX").

### 6.5 The in-game overlay router `0x004c2340` **[decomp]**

A name-keyed router with 39 names. Each name has a hash and a row index. For the in-game screens it either shows or
hides a group (the flag masks at `0x00911fe8`, `0x00911ff0`) or does a fixed action (pause menu, movie, race start).

- The names in the table include `BUSTED_OVERLAY.fng` (index 7), `PostRace_Pursuit.fng` (32), `InGameBounty.fng`
  (35), `InGameRaceSheet.fng` (36) and the pause and movie screens.
- Index 7 (`BUSTED_OVERLAY.fng`) is never acted on: in both branches of the router, the case list leaves it with
  `break` (no action), and the mask path skips it. So this router does not show the overlay. **[decomp]**
- `Infractions.fng` and `InGameBusted.fng` are not in the table.
- The router's three callers (`0x00572360`, `0x00586290`, `0x0060dbd0`) are generic helpers. `0x0060dbd0` is the
  script function `StartRace`. None of them is the arrest path.

### 6.6 Data the screens read **[decomp]** (keys) and **[verified]** (labels)

The career code reads per-car values by formatted keys (for example `BOUNTY_%d`, the `%d` being a slot number). The keys
found in the code, and what the labels show:

| Key family | Where read | What it holds (by name) | Shown as |
|---|---|---|---|
| `CAR_%d`, `BOUNTY_%d`, `FINES_%d`, `UNSERVED_%d`, `TO_DRIVE_%d`, `EVADED_%d`, `BUSTED_%d` | `0x00546a50` (build the list of cars with infractions) and `0x0052d640` (fill empty slots) | car handle, bounty, fines, unserved infraction count, status, evaded count, busted count (strikes?) | "BUSTED: %$d", "FINES DUE: %$d", "BOUNTY: %$d", "TIMES BUSTED: %$d" (strings) |
| per-car status | `0x0052d640` | impounded flag (byte at +4 of the car record) gives `< VEHICLE IMPOUNDED >`; otherwise a second flag gives `< CONFIRMED TO DRIVE >`, else `< SUSPECTED TO DRIVE >` | labels `0x35e4e01f`, `0x2089554c`, `0xd3efe2e5` (checked by label lookup) |
| `INFRACTION_TYPE_%d`, `UNSERVED_QTY_%d`, `TOTAL_QTY_%d` | `0x005466d0` | one row per infraction type (eight rows, one per bit of the infraction mask): type, unserved count, total count | "Served Infractions", "UNSERVED INFRACTIONS: %$d" |
| `CAR_USED_%d`, `BOUNTY_%d`, `PURSUIT_ID_%d`, `PURSUIT_LENGTH_%d`, `BUTTON_%d` | `0x0052e0c0` | the five most recent pursuits (rap sheet) | `RapSheetPD.fng` |

The names of the keys match the labels. The mapping from a key to the rule that sets it is **[unconfirmed]**; the
community summary (ai-pursuit-heat.md 6) gives the same fields by name.

## 7. Choices on the bust screen and what they change

Sources: the strings table (**[verified]** text) and the community career rules (**[community]**, from
[ai-pursuit-heat.md](ai-pursuit-heat.md) section 6). Binary code for the choices was not found.

| Choice | Condition | Effect | Tag |
|---|---|---|---|
| Pay with cash | cash at least the fines due | pay the fines; unserved infractions cleared; one impound strike added to the car (this is the bust's strike); car heat set to 1.0 | text "You also receive an Impound Strike ... if it's the last strike, your car gets impounded" [verified]; heat [community] |
| Pay with cash, last strike | the strike reaches the limit | the car is impounded: "Your car has been impounded. You must choose a new car to race." | [verified] text, [community] rule |
| Pay with marker (Get Out of Jail Free) | a marker is in stock | the fines are not paid and no strike is added. **Conflict:** the marker text says "postpone your debt and prevent an impound strike", and "It will be like the pursuit never happened". The community summary says the infractions are waived. | [verified] text, [community] rule; open |
| Not enough cash and no marker | cash below the fines | the car is impounded and the remaining cash is taken ("your remaining cash has been confiscated") | [verified] text, [community] rule |
| Sergeant Cross bust | a bust by the cop Sergeant Cross (a named pursuer, "Corvette C6" in the strings) | the car is impounded even if the fines can be paid ("Sergeant Cross will impound you car even though you can pay your fines") | [verified] text; mechanism [unconfirmed] |
| Release from impound | at the safe house | pay a fee for court processing, storage and the outstanding fines; needs cash or an "Impound Release" marker ("This car cannot be released...") | [verified] text |
| Extra impound strike marker | a marker is spent | raises the limit on one car by 1, up to 5 | [verified] text, [community] rule |
| All cars impounded and no cash | every car is impounded and the player cannot pay | game over for the Most Wanted goal: "All your cars have been impounded by the police. You've failed to become the Most Wanted." | [verified] text |

Default impound limit: three strikes ("3 strikes and they impound your car", **[verified]** text, MaxBusted 3 in
[community]).

## 8. Modes

| Mode | Bust route | Evidence |
|---|---|---|
| Career, free roam | spot found: NIS, then the infractions screen. No spot: `Infractions.fng` at once (the screen with the fines). | [decomp], text [verified] |
| Career, race (race bust) | the handler takes the race path: no `Infractions.fng`; a request object is made (community: the race pauses) | [decomp], [community] |
| Quick race | the race path (the current race pointer is set). No career records are written. The community summary: heat base 1, max 5, no career fine. | [decomp], [community] |
| Challenge series | the race path, plus the challenge goals: "Don't Get Busted for 1 / 1.5 / 2 Minutes!", "Evade ... with a minimum of %s infractions". Busted fails the goal. | [verified] text; fail rule [community] |
| Online race | no bust: the community rule skips the bust in an online race. The online path of the handler is not checked here. | [community] |

The free-roam rule (the current race pointer is null, so the "no spot" branch goes to `Infractions.fng`) is the main
difference. Whether the race path in career (the `Career` race) keeps the infraction records is **[unconfirmed]**.

## 9. Fines and infraction types

The eight infraction bits, with the fine amount in the install's `infractions` table (**[verified]**, see
[ai-pursuit-heat.md](ai-pursuit-heat.md) section 4):

| Bit | Infraction | Fine |
|---|---|---|
| 1 | speeding | 150 |
| 2 | racing | 350 |
| 4 | reckless driving | 1000 |
| 8 | assault on police | 350 |
| 16 | hit and run | 300 |
| 32 | damage to property | 100 |
| 64 | resisting arrest | 300 |
| 128 | off road | 75 |

The `INFRACTION_TYPE_%d` rows give one row per bit. The placeholder "TOTAL COST" on `Infractions.fng` (80,000) is sample
text, not a fine total: the fines are the sum of count times fine per type **[community]**.

The four pursuit-end card texts pick a reason line by type (**[verified]** strings):

- "SUSPECT APPREHENDED FOR EXCESSIVE SPEEDING - PURSUIT DURATION ... INFRACTIONS ISSUED ...",
- "SUSPECT APPREHENDED FOR DANGEROUS DRIVING - ... POLICE VEHICLES DAMAGED ... IMMOBILIZED ...",
- "SUSPECT SECURED IN CUSTODY - ... SPIKE STRIPS DODGED ... ROADBLOCKS DODGED ...",
- "SUSPECT DETAINED FOR RECKLESS DRIVING - ... COST TO STATE ...".

The mapping from the last infraction (or the bust cause) to these lines is **[unconfirmed]**; the names suggest speeding,
dangerous driving (assault and hit and run), custody (resisting arrest after roadblocks or spikes), and reckless.

## 10. Port status

What the rewrite has, what is built but not wired, and what is missing. The original's behaviour is in sections 0 to 9.

### 10.1 Has (in the port)

- The meter, the bust timer, the procedural arrest framing, the post-reset blink, the end card (`hud/pursuit_end.rs`).
  See [ai-pursuit.md](ai-pursuit.md) and [camera-views.md](camera-views.md).
- Infraction log and fines per chase (`blackbox-pursuit::infraction`, the `[Fines]` table of the Bartender file).

### 10.2 Wired (free roam)

The bust sequence runs in the world from the bust to the payment. It is not checked in the running game (no window
runs were allowed in this pass); its parts have unit tests.

- **The bust sequence** (`scenes/world/ai/police/busted/flow.rs`). A state machine: the arrest shot (3 s), the busted
  overlay (3 s), then the infractions ticket. `Police::start_bust` starts it when the busted meter fills, with the
  chase's infraction log as the ticket; `Police::tick_bust` runs its clock each frame; `reset` runs at each new chase.
  A bust starts once per chase. The ticket is handed over once (`take_request`). Cash ends the sequence; a marker needs
  one in stock (the port has none, so the marker is refused). The race branch exists in the flow, but the port has no
  race mode, so every bust takes the free-roam branch.
- **Ticket hand-over.** The front end polls the scene each frame in free roam (`frontend/flow.rs` `poll_bust`). When the
  bust has a ticket, the stage goes to `Paused` (the world freezes, as for the pause menu) and `Infractions.fng` opens
  over it. The payment is a `Command::PayBust { marker }`; `flow.rs` `pay_bust` asks the scene (`Scene::pay_bust`), and
  the bust takes it or refuses it. A taken payment resumes the game. The hooks are `Scene::take_bust_ticket` and
  `Scene::pay_bust`, with defaults that do nothing (`viewer/mod.rs`), implemented by the world (`scenes/world/mod.rs`)
  and passed through the pause wrapper (`frontend/scene.rs`). This replaces the shared payment flag the first plan
  named: the scene hooks are the channel the front end already uses for the HUD and the pursuit feed.
- **The end card** is suppressed for a free-roam bust (`TrafficWorld::pursuit_end_card` returns `None` while the
  sequence replaces it), so the ticket is the only bust screen.
- **The busted overlay** (`hud/busted_overlay.rs`). `BUSTED_OVERLAY.fng` from `GLOBAL/InGameB.bun` is loaded into the
  HUD's runtime while `HudState::busted_overlay` is set; the world sets the flag from `busted_overlay()` in
  `scenes/world/hud.rs`. It is unloaded when the flag clears, so its fades run again on each bust. It is drawn over the
  HUD in the full frame, not in the HUD's viewport; the layout at other aspect ratios is not checked. The package has no
  data binding.
- **The arrest scene chooser** (`scenes/world/ai/police/busted/arrest.rs`). Section 4.1's table: 16 scenes reachable.
  The variant comes from a roll in `0..1`, taken from the world's seeded random source. The class is fixed at 0
  (`ARREST_CLASS`): the original reads it through the perp car's interface, which the port does not have. The chosen
  scene is logged and kept (`arrest_scene()`) for the NIS player. An ignored test checks that the 16 bundles exist.
- **The infractions ticket** (`frontend/infractions.rs`, `Infractions.fng`). It shows this chase's count and fines, the
  unserved count and fines (0, the port has no career state), and the total due. The package has no row per infraction,
  so the offences are listed on the "Violation:" line (layout unverified). Pay with Cash and Pay with Marker send
  `PayBust`. Pay with Marker is dimmed and refused with no marker. **[verified]** (package dump): the package posts
  nothing to the game when a payment button is pressed. Its only two game responses (`0x9120409e`, `0xb5971bf1`) are
  the pad's left and right moves, not payments. An earlier version took them for the two payments, so the ticket could
  never be paid and the game stayed frozen under it. The press is the pressed button's own `BUTTON_PRESSED` answer
  (the runtime reports it with the button's GUID, `ScreenLogic::pressed`): holder `0xf72c2` (object `0xb8a7c6cd`, on
  the right) is Pay with Cash and holder `0xf70e8` (object `0xb8a7c6cc`, on the left) is Pay with Marker. **[decomp]**
  The original's handler tells the two apart by those object name hashes. The press also disables the package's input
  until its own timed script re-enables it (response `button 0x101`).

### 10.3 Missing or partly built

- **The NIS player** (built). The arrest scene plays as a NIS in the world (`scenes/world/nis`, from the `Arrest*`
  bundles in `NIS/`). The bust flow holds the car for the arrest shot, hands the chosen scene to the player, and waits
  for the scene to end, or to be refused, before the infractions ticket (`busted/flow.rs`, `arrest_scene_over`). The
  arrest spot is the player's pose. Open items of the player, such as the framing not checked on screen, are in
  [provenance/nis-player.md](../provenance/nis-player.md).
- **The arrest spot** (section 4.2). The spot search is not built. The arrest uses the player's pose as its spot instead.
- **Career state.** Done in a career: see [busted-aftermath.md](busted-aftermath.md) (cash, strikes, impound, markers,
  the notices, the car select and the respawn). Free roam without a career has none of it and resumes driving.
- **The race pause** (section 2 step 6c). The port has no race mode, so the race branch of the flow (it ends after the arrest shot, and
  the end card stays) is not reached yet. The request object the original makes for a race is not decoded and not built.

## 11. Open questions

- Which message the cutscene channel carries, and the 3 s delay: is it a timer in the message queue, a pursuit state,
  or a value in the pursuit object?
- The sender of `MPerpBusted` (the five subscriptions are known; the post call is not).
- What the NIS does when it ends, and which screen follows (`Infractions.fng` or the pursuit summary).
- The rule behind the random variant and the class value in the chooser.
- Whether the Get Out of Jail marker keeps or waives the unserved infractions (text and community disagree).
- Which screen or message shows `InGameBusted.fng` and `BUSTED_OVERLAY.fng`, and whether they are used in the Black
  Edition at all (`InGameBusted.fng` has no name in `speed.exe`).
- The camera track for each arrest scene (the NIS bundle has none).
- The meaning of the flag bit shared by each arrest and its `b` variant.

## How to check it

- Free roam, career: get busted with no spot nearby. The game should show `Infractions.fng` at once. With a spot nearby,
  the arrest NIS should play with the perp in the scene.
- Race (career or quick race): a bust should not show `Infractions.fng`.
- Pay with cash and check the car's status after: impound strike count, unserved infractions cleared.
- Pay with a marker and compare the strike count (resolves the open conflict in section 7).
- Let the cash run short: the car should be impounded and the cash should be taken.

## Next leads

1. Find the sender of `MPerpBusted`: the caller of the message dispatch `0x00665390`, and the code that sets the
   pursuit's busted state. Check the float constants 5.0, 15.0 and 90.0 near the busted timer's update.
2. Find the 3 s delay: search for the float `3.0` (`0x40400000`) near the cutscene subject `0xfea34c0a`, and read the
   queue helper `0x0061fc10` that the broadcast forwarder uses.
3. Decode the NIS end path: the function that contains the `Infractions.fng` reference at `0x006f76d7` (outside the
   NIS listener `0x006f4ea0`, which has no callers), and the NIS activity constructor `0x00640f00` with its callers
   `0x00641520`, `0x00641590`, `0x00643d50`.
4. Decode the arrest spot query `0x007862d0` (what it searches: road, track or the car's path) and the class value read
   from the perp's interface (`0x005d59f0` with the type at `0x004037e0`).
5. Decode the consumers of the two request objects: the one made in free roam for `Infractions.fng` (`0x0062fe40`, vtable
   `0x008a723c`) and the race one (`0x00623a90`, vtable `0x008a7110`).
6. Decode the random helper `0x0045da70` and the rounding helper `0x007c4b80` to get the variant rule.
7. Find the camera track for the arrest: the ICE pack chooser (the packs `0x8003B200` to `0x8003B203` in
   `GLOBAL/InGameB.bun`), and the caller of `0x008ee320` for a NIS.
8. Read `dbalatoni13/nfsmw` `VehicleDB.cpp` (`FEImpoundData`, `FEInfractionsData`) to name the fields of the per-car
   record at the offsets used by `0x0052d640`. This would confirm the strike and the impound fields.
9. Find who uses `InGameBusted.fng`: a name hash table, a data file in the game's bundle, or a dead screen.
10. Map the four pursuit-end card texts to the infraction type that set them (`0x00546fc0`, the pursuit history rows).
