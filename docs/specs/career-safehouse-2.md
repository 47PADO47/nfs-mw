# Career safe house, native side: leaving, the world that loads, heat, the respawn marker

The second half of [career-safehouse.md](career-safehouse.md) (entering, zones, the menu and its screens). Section
numbers continue from there. Addresses, Ghidra names and dead ends are in the
[provenance file](../provenance/career-safehouse.md). Evidence tags as in the first file: **[exe]**, **[verified]**,
**[community]**, **[unconfirmed]**; addresses are `speed.exe` addresses.

## 6. Leaving: Resume Free Roam

### 6.1 The confirmation and the handler

Item 1 of the menu shows the confirmation dialog ("Are you sure you want to leave the Safe House to resume free roam?").
Its accept message (`0xD05FC3A3`) reaches the screen's handler (`0x00531EA0`, branch at `0x00531F7B`). In order **[exe]**:

```
on accept:
    if debug byte (0x00926126) is 0:
        finished = node "16.2.1" has state bit 1 (complete)
        if not finished:                                     # the prologue is not over
            name = the first of  16.1.0, 16.2.2, 16.2.3, 16.1.1, 16.2.1   whose node is not complete
                   (routine 0x00600C30; the five names are a table at 0x008A37E4, stride 8)
            node = runtime.make_node_state(name)             # 0x00603440
            runtime.pending_node = node; runtime.pending_mode = 2    # 0x00600AB0
            runtime.activate(node)                           # 0x005E39E0: sets the node's "active" byte
                                                             # if it is the pending one
    leave the front end                                      # 0x0056C5B0
    context |= 1                                             # career
```

- "Leave the front end" starts the flow transition: from state 3 it requests the unload of the front end (state 2), then
  the world load runs (section 7). From any other state it uses the alternative route of the same routine.
- When `16.2.1` is complete (the normal case after the prologue) nothing is selected: no pending node, so the world
  loads **as free roam** (section 7.2).
- When the prologue is unfinished the pending node makes the world load straight into that event (its start position,
  no free roam, no autosave); the node's time of day is applied when the node is made pending if the node stores one
  (`0x00600AB0`: if the node has a time of day >= the threshold constant, the time-of-day index is set to it).
- The debug byte (`0x00926126`) skips the whole prologue check.
- Resume Free Roam sends nothing to the script graphs. It does not touch heat, the respawn fields (except through the
  request that opened the front end, section 4: the free-roam flag at `+0x110` stays set until the world has loaded),
  the car, or the cops.

### 6.2 Other exits

| Exit | What it does |
|---|---|
| Blacklist sequence reaching step 6 at a stage other than 15 / 13 | the same "leave the front end" routine (section 5.3); context bit 17 is still set when it runs |
| Message `0xE1FDE1D1` while the screen's last pressed id is `0x911AB364` (a back-style action; the name is **[unconfirmed]**) | platform = 1, context bit 0 cleared; if the career manager layer exists (flag at `0x008F8708`) context bit 8 is set and the screen stored in the object is opened, else `MainMenu.fng` **[exe]**; no world is loaded |
| Rap sheet, Car Select, Save | return to this menu; they do not load the world |
| Selecting a Blacklist race from the race sheet or the challenge screens | sets the pending node and mode through `0x00600AB0` in the screen code (callers `0x0057F674`, `0x0057F6F9`, `0x005A3769`, `0x005A3B1F` and others), then leaves the front end the same way. The node list and what starts the event are in [career-scripts-events.md](career-scripts-events.md) **[exe]**, the individual launch buttons **[unconfirmed]** |

## 7. The world that loads

### 7.1 State sequence

After the transition starts the flow state machine runs: unload the front end (state 2), begin the world load, load the
region around the start position, wait for the simulation, then state 6 (in game). The reverse for entering: state 6 ->
7 (world unload) -> 8 (front-end load) -> 3. **[exe]** (state names; the timing is not specified here).

### 7.2 The race status object

A new race status object is built for every world load (constructor `0x00603720`). Relevant initial values **[exe]**:

| Field | Value |
|---|---|
| `+0x1960` event underway | **1** until `SetRoaming` runs (see below) |
| `+0x1964` play mode | 2 (career) |
| `+0x1968` current node | none |
| `+0x196C` current bin | the bin of the profile's stage byte (`0x005E3980`) |

The constructor then ends with:

```
if runtime.pending_node == none:           # runtime +0x2C
    set_current_bin(stage)                 # 0x00602BB0: bin lookup, activate it, refresh the manager
    SetRoaming()                           # 0x005FE3E0
# else: the pending event is started by the load-complete callback (0x00603500) and SetRoaming is not run
```

### 7.3 `SetRoaming` (also called by the race graphs at the end of an event)

Order of the routine **[exe]**:

1. Remember whether the current node is `16.2.1`.
2. Unless the current node has attribute `0x64273C71` non-zero, reset the camera/HUD director to its default mode
   (`0x004D0D70`).
3. `event underway = 0`, `play mode = 2`, `current node = none`, byte `+0x195C` = 0.
4. Reset two small tables (`0x007652F0`): clear bit 7 of byte 10 in every 22-byte record of the table at `0x009B38C0`,
   and set byte 4 of every 8-byte record of the table at `0x009B38C8` to `0xFF`. What the tables hold is **[unconfirmed]**.
5. For every vehicle in the AI vehicle list (`0x0092D84C`): if its racer flag (virtual `+0x40`) is set call virtual
   `+0x50`; for those with the driver interface, set the driver class back to its default (`+0x70(+0x74())`); if its
   pursuit object reports no pursuit (`+0xA0 == 0`) switch the cop manager on (`+0x2C(1)`). **[unconfirmed]** for the
   meaning of each virtual, **[exe]** for the order.
6. Unless the previous node was `16.2.1` and unless the manager byte `+0x2D4` is set: post the event `EReloadHud`.
7. Post the event `EAutoSave` (always). The autosave rules are in [career-flow.md](career-flow.md) section 8.
8. Clear the rain override (the condition on the node's flag `0x8CB01ABF` is always true here because the node was just
   cleared).
9. Refresh the zone/road-segment links of the gameplay manager (`0x005EDE20`) and clear the byte `0x0090D5EC`.

So a free-roam start always autosaves (if the autosave rules allow) and reloads the HUD; it does not touch heat.

### 7.4 The start position

The world loader asks `0x00733710` for the start point and direction **[exe]**:

```
start = built-in table by track name (L2RA, L2RB, ... debug/fallback positions)   # always overwritten below
if runtime has a current/pending node:     # 0x005DCA00
    start = the node's start position and heading
else if gameplay manager exists:
    start = respawn_marker_position(section 9)                                   # 0x005E4DF0
if debug flag and debug start position given: start = that           # "SkipFEStartPosition"
```

Because the manager always exists in career, the built-in table is dead there. The position and heading come from the
marker collection (section 9). Engine axes are `(-y, z, x)` of the stored vector.

### 7.5 The player's car

The car is created by the player-car routine (`0x006F47A0`) **[exe]**:

```
if the current node forces a car (attribute read by 0x005FC5C0): use that preset and the node's heat parameter
else:
    handle = settings.current_car                       # profile + 0xA8, when the context has bit 0
    record = stable.find(handle)                        # profile + 0x414 table
    vehicle key = record + 8 ; customization = the car record's data (0x198 bytes copy: parts, paint, vinyls ...)
    heat       = max(1.0, record.heat)                  # career only; else the default
the vehicle is created fresh at the start position
```

- The car is **always new and undamaged**: the career record stores no damage, nitrous, speed-breaker or fuel state
  ([career-save.md](../formats/career-save.md)). A resume therefore "repairs" the car. **[exe + verified]**
- Non-career sessions use the slot of the front-end database or the debug preset `bmwm3gtre46` (debug only).
- After the car is added to the player list the routine calls the heat write slot with the value above (default 1.0
  when the context is not career) (`0x006F4BD9`). Section 8.

### 7.6 Cops, pursuit, traffic, time of day

- **Cops and pursuit.** The cop manager, the pursuit objects, all AI cars and traffic are created with the world; none
  survives from the previous one. The first cop appears under the free-roam rules of [ai-pursuit-cops.md](ai-pursuit-cops.md)
  with the player's heat (section 8). No script runs `SetCopsEnabled` or `NoNewPursuitsOrCops` on this path; the hooks are
  only called from event graphs. **[exe]**
- **World heat.** `Game.SetWorldHeat` is not called on entering or leaving. Its two script call sites are in the
  rival-locked graphs (values 1 and 5), section 8.
- **Time of day.** Free roam sets no time of day. The index (`+8` of the lighting object) is set by events that store one
  (`0x00600AB0`, `0x006035F0`), and reset to 0 by `JumpToCarLot` and `DoSafeHouseIntro`. Whether the index is rebuilt with
  the world is **[unconfirmed]** (the day/night presets follow the location otherwise, [time-of-day.md](time-of-day.md)).
- **Rain.** Cleared by `SetRoaming` (step 8).

## 8. Heat: where it lives and who writes it

### 8.1 Storage

| Place | Layout | Lifetime |
|---|---|---|
| Live heat | float at `+0x1C` of the player car's heat interface (interface id `0x004037E0`); read slot `+4` (`0x00409400`), write slot `+8` (`0x00409060`) | the car object |
| Career record heat | float at `+0xC` of the career car record (access `0x0056F7B0`, set `0x0056F7A0`) | the profile; saved |

The write slot stores the value, derives the heat level (the integer part) and, when the level or the "race event is
underway" bit changes, replaces the two per-level tuning objects (looked up in the `pursuitlevels`-style attribute rows
by `level - 1`). **[exe]**

### 8.2 Writers of the live heat

| Writer | When | Value |
|---|---|---|
| Player car creation (`0x006F47A0`) | every world load | `max(1.0, record heat)` in career |
| Race start (`0x0060DBD0`, hook `Game_StartRace`) | an event starts | for each racer: `max(event value (attribute 0xE4211F4F), current heat)` |
| `Game.SetWorldHeat(n)` (`0x00612660`) | scripts: two rival-locked handlers | first player's heat = n (values 1 and 5) |
| Pursuit logic | during a pursuit | growth, clamp, forced starts: [ai-pursuit-heat.md](ai-pursuit-heat.md) section 3 |

### 8.3 Writers of the career record heat

| Writer | When | Value |
|---|---|---|
| Player car destructor (`0x00434FE0`) | the world is torn down (the safe house entry, quitting, loading another world) | `min(live heat, 5.0)`, only if the context has bit 0 and the car has a record |
| Evade, event win, milestone | pursuit evaded (and the cool-down entry of section 3.4), event won, milestone done | `heat x factor` for every car of the stable ([career-economy-flows.md](career-economy-flows.md) section 4) |
| Pay a fine in cash | the infractions screen | 1.0 |
| Release from impound | the release action | 0 |
| New decal or paint | customize | `heat x NewDecal factor` ([ai-pursuit-heat.md](ai-pursuit-heat.md) 3.3) |
| Quick race car select | quick race | 0 |

Two copies of the stable's heat in screens (`0x007AFB60`, `0x007AFC90`) are preview copies and not writers.

### 8.4 One round trip

```
drive: live heat h, record r (current car)           → engage (no pursuit, or cool-down)
  cool-down: the evade handling multiplies every record by 0.95 (and ticks the strikes);
             the live heat h is not changed
unload world: record(current car) = min(h, 5.0)       # overwrites the factored value of the current car
safe house: Car Select may change the current car
resume: new world: new car heat = max(1.0, record(current car).heat)
```

Consequences **[exe]** (reading of the routines; not tested in game):

- Nothing in the safe house or in the engage path lowers the live heat. The evade, win and milestone factors act on the
  records only. The car that is being driven when the world is torn down gets its record overwritten by the live value,
  so the factors survive only on the other cars of the stable.
- Heat above 5.0 is cut to 5.0 at the unload. A new world never starts below 1.0.
- Entering without a pursuit gives the same result as entering in cool-down, minus the factor on the other cars and the
  strike tick.

## 9. The respawn marker

### 9.1 What is stored

All fields are 32-bit keys of **collections** of the gameplay data (class `gameplay`), kept in the gameplay manager
(global at `0x0091E00C`) **[exe]**:

| Offset | Meaning | Written by |
|---|---|---|
| `+0x108` | the respawn marker of the **last visited zone of any kind** (safe house, car lot, custom shop) | `DoZoneMenuAction`; `DoSafeHouseIntro`; the fallback of section 9.3; the gameplay blob load |
| `+0x10C` | the respawn marker of the **last visited safe house** | `DoZoneMenuAction` (only for `safehouse`); `DoSafeHouseIntro`; the gameplay blob load |
| `+0x110` | byte: use `+0x10C` instead of `+0x108` for the next world load | the front-end request (platform 2, no event node) and `DoSafeHouseIntro` set it; the load-complete callback (`0x00603500`) clears it |
| `+0x11C` | explicit override key | no writer found; used first when its collection exists **[unconfirmed]** |

### 9.2 What a key points to

The key is read from the zone collection's attribute `0x2241F4CD` (a reference to the zone's `respawn` child). In the
install the marker collections are the `respawn` children of `menu_gate_safehouse/zone_safehouse_N`,
`menu_gate_carlot/zone_carlot_N` and `menu_gate_customshop/zone_customshop_N`. A marker collection has (at least) **[exe]**:

| Attribute (VLT key) | Type | Meaning |
|---|---|---|
| `0x9F743A0E` | 3 floats | position in game coordinates; the engine position is `(-y, z, x)` |
| `0x5A6A57C6` | float | heading in degrees; the direction vector (0,0,1) is rotated by `heading x (-1/360)` (the constant at `0x008A3874`) |

The values for the install's markers are in [gameplay-races.md](../formats/gameplay-races.md); the start collection
`career_start_l2ra` is described in [career-flow.md](career-flow.md) section 6.

### 9.3 Which marker the next world uses

```
select_respawn_key():                                  # 0x005E4CF0
    if collection(+0x11C) exists: return +0x11C
    key = (+0x110 != 0) ? +0x10C : +0x108
    if collection(key) does not exist:
        key = hash("career_start_" + region)          # region = track name from the loaded region
        +0x108 = key
    return key
```

So: coming out of a **safe house** (flag set by the request, or a visit that stored `+0x10C`) the player respawns at the
safe house's own marker; coming out of a **car lot or custom shop** the flag is clear, so `+0x108` is used (the last zone
entered, which is that shop). A jump from the pause menu sets the flag too (platform 2, no event), so it respawns at the
last safe house; if none was ever visited the collection is missing and the start point `career_start_<region>` is used.
After the world has loaded the flag is cleared, so a later unload that does not go through the request uses `+0x108`.

### 9.4 Saved

Both keys are part of the gameplay blob tail ([career-save.md](../formats/career-save.md)): written at tail offsets 0 and
16 and, on load, accepted only if the collection exists in the loaded data. `+0x110` and `+0x11C` are not saved.
**[exe]**

## 10. Rewrite notes

Minimum model: `respawn_home: Key` (+0x108), `respawn_safehouse: Key` (+0x10C), `use_safehouse: bool`;
`Zone { kind: ZoneKind, respawn: Key }` with `ZoneKind` from the `ZoneType` string.

1. Each frame in free roam compute `allow_zones` and `allow_safehouse` (section 3.1).
2. On engage: in cool-down + safehouse zone -> mark the pursuit evaded (evade level 1.0) and run the normal evade
   handling (heat x 0.95 on the stable, strike tick), then after the results screen enter; otherwise enter at once.
3. On enter: store the zone's respawn (`respawn_home`, and `respawn_safehouse` for a safe house), set `use_safehouse`
   for a safe house, write the player's heat to the current car record as `min(h, 5.0)`, tear down the world, open the
   screen for the kind.
4. Resume Free Roam: if the prologue is unfinished, make the first unfinished prologue event pending; otherwise load a free
   roam world: new car from the current car record with `heat = max(1.0, record)`, position from the selected marker
   (`use_safehouse ? respawn_safehouse : respawn_home`, fallback `career_start_<region>`), then clear `use_safehouse`,
   post the HUD reload and the autosave.
5. Do not reset heat, cops or the pursuit on either side; they are fresh because the world is.

## Open questions

1. What sets the override key `+0x11C` (nothing found), and what the debug byte `0x00926126` is for.
2. The meaning of the HUD events posted when the engage button is refused, and of the property `0x0280164F`.
3. The exact start of the Blacklist stepper after the first-visit bio (the navigation code that advances the counter).
4. Whether the time-of-day index survives a world reload.
5. The remaining individual launch buttons that set a pending node from the front end.
