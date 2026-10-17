# Career save screens, part 2: autosave, load, create, delete

Continues [career-save-screens.md](career-save-screens.md) (layers, the mode word, the `MC_Main` screen, the manual save
and the file write). This part covers the **autosave** (triggers, gate, run, failure), the **load** path with every
check and message, **create** and **delete**, the other places that open the screens, what saving changes, and notes for
a rewrite. Addresses, Ghidra names and dead ends are in the [provenance file](../provenance/career-save-screens.md).
Tags, address and wording conventions are those of part 1.

## 8. Autosave

### 8.1 The gate (`0x005184D0`, "autosave possible")

The gate is a function of the manager, FEDatabase, the profile and the gameplay manager. A background autosave evaluates
it twice (when the event is disposed and when the run starts); a screen autosave evaluates it in the caller and in the
screen's state step (section 8.5). Order, first failure returns false **[exe]**:

1. `force` is true: return **true** (nothing else is tested).
2. The one-shot byte `skip once` (`+0x2B`) is set: clear it and return **false**.
3. Context word bit 3 (online) or bit 6 (LAN) set: false.
4. Layer not present: false (never, part 1 section 1).
5. Neither the profile's autosave option (profile `+0x38`, default 1) nor the manager's `switched off by error`
   byte (`+0x24`) is set: false.
6. The current event node's id equals the id of `1.8.1` (the final event, `0x0056DC00`): false.
7. The gameplay manager has a current node and that node's boolean attribute `BossRace` (`0xFF5EE5D6`, `0x005FBD90`)
   is true: false.
8. Otherwise **true**.

The "profile loaded" byte is **not** part of the gate; the run (8.3) tests it separately. `skip once` is set by the
resume code (`0x0057F570`) in two cases: resuming into a node that is not `16.1.0` while stage 16, and starting the final
event `1.8.1`. The first autosave attempt after such a resume is therefore dropped (whichever of the three evaluations
comes first consumes it). **[exe]**

### 8.2 Triggers

An `EAutoSave` event (type `0x10`, constructor `0x00620920`) is allocated from the event pool, which posts it. The event is
posted by:

| Poster | When | Condition |
|---|---|---|
| `SetRoaming` (`0x005FE3E0`) | every time free roam starts (after the world loads, after every event ends, after "Resume Free Roam") | none |
| event start (`0x00602010`, called by the `Game_StartRace` hook `0x0060DBD0`) | an event starts | the node exists, carries the node flag `0x8CB01ABF` (`0x005FBD20`) and its name is not `16.1.0`; **or** the current node is `1.8.1` |
| pursuit evaded (`0x00411070`, via `0x00602770(1)`) | the pursuit listener sees a pursuit end as evaded, after it updated `pursuits_in_a_row` and `pursuit_evasion_time` | no event underway (gameplay manager `+0x1960 == 0`) |

The event itself does nothing until it is **disposed** (`0x00620930`): then it evaluates the gate with `force = false` and
if true sets `pending` (`+0x1F`). **[exe]** The earlier version of this documentation said "when a race ends"; the
hook is an event *start*, the post-race save comes from `SetRoaming`. The meaning of the flag `0x8CB01ABF` is not known
(the icon placement of 8.4 reads the same flag, which suggests "race-type node") **[unconfirmed]**. A boss race (flag
`BossRace`) passes the start condition but the gate refuses it, so no save happens before a boss race.

### 8.3 From pending to the write

1. The in-game HUD update (`0x0058CE99`, once per frame while a world runs) copies `pending` into `armed` (`+0x2A`).
   Nothing is armed while the front end is up, so a pending autosave waits for the world.
2. The manager update (`0x00563D10`) tests: `pending` and `armed` and the gameplay manager exists and is **idle**. Idle
   means its notification list (`+0x2AC` entries, the same list that is stored in the blob) is empty, or
   the "a notification can be shown right now" test (`0x005EC040`) is false. When all hold it clears `pending` and `armed`
   and calls the run with `force = false`. **[exe]**
3. **Run** (`0x00562AB0`): if the gate (8.1) is false or the *profile loaded* byte is 0, do nothing. Otherwise:
   - state is not `0xB0`: show the overlay (section 7 of part 1) and zero the mode word;
   - `switched off by error` set: open the screens in state `0x90` (8.4) and stop;
   - else: `autosave run = 1`, `saving = 1`, HUD flag on, then `0x00526800`: with state `0xB0` clear the masks and set only
     bit `0x100`; otherwise apply the **silent mask** (part 1 section 4.3); then `0x005188F0` with the profile's own name.
4. The library runs the write (part 1 section 6.2). The silent mask auto-answers the overwrite question, the "save
   complete" notice, and the other notices except "unable to save". The write takes about six update calls; the file write
   itself is synchronous in state `0xB`.
5. **Completion** (`0x00526AC0`, same callback as the manual save): *profile loaded* := 1, *unsaved changes* := 0,
   `switched off by error` := 0, buffers freed. Because `interactive` is 0 in silent mode and `autosave run` is 1 the
   second branch runs: clear `saving`, `autosave run`, the HUD flag, `interactive := 1`, clear all mask bits, post
   `SAVE_DONE` to the overlay (hides the icon). In state `0xB0` it also posts `0x461A18EE`, which closes the screen and lets
   the caller continue. **[exe]**

### 8.4 Failure of a background autosave

Only an open failure is an error (part 1 section 6.2). The prompt "unable to save X" is not masked, so:

1. The prompt reaches the manager's prompt callback (`0x00560980`). The state is not `0xB0`, `autosave run` is set, the
   prompt has a button: the manager stores the prompt and, with no screen up, opens `InGame_MC_Main.fng` with mode `0x91`
   (state `0x90`) from the world.
2. The state step of that screen (`0x00559E90`) frees the stored prompt and asks, as a Yes/No dialog (tag `0xA`),
   `MC_AUTOSAVE_CARDLESS_PS2` ("there is no memory card (PS2) in slot 1, do you wish to continue with autosave?"). When
   `switched off by error` was set it asks `MC_AUTOSAVE_NO_CARD_PS2` instead (the PS2 wording is shown on PC because
   no PC wording exists). The save icon is hidden and the saving flags are cleared.
3. The error callback (`0x00526F60`) runs when the library finishes: with `autosave run` or `saving` set it frees the
   buffer, clears the flags, hides the overlay, posts `0x8867412D` if the state is `0xB0`, and **sets the profile's
   autosave option to 0**.
4. Dialog **Yes**: `re-enable = 1`, option := 1, state := `0xB0`; the state step shows "Saving..." and runs the save again
   at once with `force = true` (a second failure repeats the question). **No**: option := 0 and the screen closes. From then
   on the gate returns false at test 5 until the player turns the option on in the Options (section 11).

The quiet-mode callback (`0x005271E0`) also matters here: when the operation reports value 1 (**arm**) it stores the
profile option := 1 and clears `switched off by error`; when it reports 0 during a background autosave run (no path does
this on PC) it would switch the option off and set `switched off by error`. Arming is therefore what turns autosave on
after "Yes" to the enable question, after a load with the option on, and after a manual save with the option on.
**[exe]**

### 8.5 Autosave with the screen up (state `0xB0`)

Used when the game wants the save to finish before the next screen.

| Caller | Mode | When | After the save |
|---|---|---|---|
| Blacklist stepper step 3 (`0x005304F5`) | `0x4000B2` (state `0xB0`, go to screen B, continue stepper) | the caller first evaluates the gate with `force = false` and skips the screen when it is false; after the rival challenge/markers, [career-safehouse.md](career-safehouse.md) section 5.3 | exit continues the stepper (`0x005302C0`) |
| main menu, way back to the title screen (`0x00529230`, called at `0x0054559E`/`0x005455C1`) | `0x400B3` (return, then go to B) | *unsaved changes* is 1 (and the layer is present); otherwise it just goes to the screen. The screen B is the title screen, or another one when the context has bit 3 or bit 6 | goes to screen B |
| leaving the customization shop (`0x007BE59D`) | `0x2000B3` (stable wording) | the stable differs from the copy taken when the shop opened (`0x0057FC90`, `0x8CC8` bytes compared) and the layer is present | goes to screen B |

The state step (part 1 section 4.4, state `0xB0`) evaluates the gate itself with `force = false`. If true it shows the
"Saving ..." line, copies the profile name and runs the save with `force = true`. If false (online or LAN context, option
off, final event, boss race) the state becomes `0x50` and the manual question of part 1 section 5 is asked instead
(`MC_SAVE_STABLE_PROMPT` wording with bit 21, `MC_SAVE_PROFILE_CHANGES` with bit 18). For the stepper the gate is evaluated
twice (caller, then step), so the `skip once` byte is consumed by the caller. **[exe]**

## 9. Load

### 9.1 Where the screens open for a load (or a create)

| Source | Mode | Notes |
|---|---|---|
| start-up (`MC_Bootup`, then `MC_Main.fng` in the boot list) | `0x20` | section 9.2 |
| alias manager **Load** (`0x00543030`) | `0x411` | state `0x10`; ok/cancel messages go to the manager |
| "Load career" button of the game-over screen (`0x00532810`) | `0x413` | same state; leaves to screen B |
| quick race, second player's car list (`0x007C2134`) | `0x411` / `0x20411` | loads into player 2's profile (short format) |
| **New career** item with no alias loaded (`0x00532740`) | `0x80063` | state `0x60` with the new-career wording; with an alias loaded it starts the new career directly (`0x005A3950(1)`) |
| main-menu item with no alias (`0x005289D0`) | `0x10063` | state `0x60`, Challenge Series wording |

At start-up `MC_Bootup` lists the root (`0x00518440`), then sends "done"; the handler sets the mode word to `0x20` and
sets bit `0x4000` of the mask, and the next boot screen `MC_Main.fng` runs the state step with state `0x20`. The mask bit
is cleared by the first list-ready step, so the load notices are not hidden. **[exe]**

### 9.2 The list-ready step (`0x00563E90`)

The list operation (part 1 section 1.1) calls back once per entry and once at the end. The end posts `0x5A051729`. The step
first sets `interactive` and clears the masks, counts the kept entries (name shorter than 8, no space), then by state:

| State | 0 entries | 1 entry | 2 or more |
|---|---|---|---|
| `0x10`, `0x30`, `0x70` | notice "no aliases found" (tag 3) | show the list | show the list |
| `0x20` (start-up) | if the last error code was `0xB`: leave; else ask "create an alias?" (tag 1 wording `MC_CREATE_NEW_PROFILE_MENU`) | **load it at once** (no list) | show the list |
| `0x40` | alias loaded: "save the alias?" (action 4); none: create question | - | - |
| `0x60` | alias loaded and not bit 19: tag 5; else tag 1 | - | - |

"Show the list" = start the device-monitor operation; its callback (`0x005191E0`) posts `0x54B3AC6C`, which pushes
`MC_List.fng`: nine rows (`option_name_N`, `option_data_N`) with the title "load" or "delete" by the state. Picking a row
(message `0x406415E3`) copies the name and sets the pending code: **5 load** (state `0x10`) or **6 delete** (`0x30`),
`0x005436F0`. A pick that is a front-end message `0x911AB364` ("back") goes back without an operation. **[exe]**

### 9.3 Load operation (`0x005189F0`) and the library's checks

1. Take the **snapshot**: serialize the current profile into FEDatabase `+0x128` (`0x00599500`), so a failure can be undone.
2. Allocate the `0xF848`-byte buffer, set the alias, mode load (code 5) and request the read (`0x007F3BFA`, library op
   `0x10`). The path is `<root>/<alias>/<alias>`.
3. Library read (`0x007F5255`), one step per update; **errors** are `(status 1, code)`:

| # | Check | Error |
|---|---|---|
| 1 | open read-only binary | code **9** (file or folder missing) |
| 2 | file length query | code 7 |
| 3 | read the 0x1C header (short read) | code 7 |
| 4 | header total size equals the file length, magic `0x4D433032`, header CRC over bytes 0..0x17 equals the word at 0x18 | code **8** |
| 5 | ask the manager for buffers (`0x005192F0`): descriptor size must be 8 and profile size must equal `0xF848` | code **8** |
| 6 | read the descriptor, then the profile (short read) | code 7 |
| 7 | repeat check 4; then compute the descriptor CRC and the profile CRC | mismatch **ignored** |

   In check 7 a CRC mismatch writes `(1, 8)` and the next instruction (`0x007F5690`) writes `(0, 0)`: the file is
   accepted. **[exe, disassembly checked]** (An editor that changes the profile without fixing these two words still loads, as
   long as the MD5 of check 11 is right.)
4. Prompt: success type 10 "load complete" (Continue), failure type 11 "unable to load X" (Continue); both can be masked
   (part 1 section 4.3). Then the report goes to the manager: success `0x00526C90`, failure `0x00526F60`.
5. Game checks in `0x00526C90`, in this order:
   - **8** descriptor type equals `0x10D` and descriptor size equals `0xF848` (and the buffer is the private one);
   - **9** `LoadFromBuffer` (`0x005ACB70`): settings first (the blob is MD5-checked, tag `Game`, version at least 8
     (`0x005F81D0`); a bad blob is **silently skipped** and the old gameplay state stays), then the **default stable hash**
     against FEDatabase `+8` (**mismatch: stop, return false**), then name, playlist (first player only), options, stable,
     flag byte, statistics, selections, race settings, controller bindings, and last the **profile MD5**
     (`0x0057F9F0`, over `[0x10, size-0x10)` compared with the digest at the position where parsing ended). The MD5 is checked
     *after* the data was copied into the live profile.
   - There is no "version too old" error at the container or profile level (the 16 version bytes are not read); only the
     blob version (check 9) is tested.
6. **Failure** of 8 or 9, or of the library: set `interactive := 0`, set all mask bits, **restore the snapshot** (load the
   serialized snapshot back through the same `LoadFromBuffer`, `0x005ACE40`), post `0xF35D144E`, free the buffer. The screen
   shows the notice `MC_CORRUPT_PROFILE_XBOX` (the alias "appears to be damaged and cannot be used"; the Xbox wording, with
   the alias name) with Continue (tag 0xD), then leaves. For library errors 7, 8 or 9 the prompt
   "unable to load X" is shown first (unless masked) and the error callback posts `0x8867412D` ("back").
   `MC_CORRUPT_PROFILE`, `MC_CORRUPT_PROFILE_PS2/NGC` and `MC_BOOTLOAD_SUCCESS` exist in the table but are never used on PC.
7. **Success**, first player: free the snapshot, *profile loaded* := 1, `switched off by error` := 0, then
   - option off: post "done" (`0x461A18EE`; `0xA4BB7AE1` in state `0x20`); the screen closes;
   - option on: free the buffer and **arm quiet mode** (`0x005268B0(1)`), which closes the screen when it reports.
   Second player: free the snapshot and buffer and post "done" without touching the loaded flag.
8. Loading posts the `EATraxInit` event (inside `LoadFromBuffer` when `firstPlayer`), sets settings flags `0x40000` and
   `0x20000` by the bonus-content probes, and merges the flags word (`new = (old | stored) & ~0x800`), as in
   [career-save.md](../formats/career-save.md) section 2.3. **[exe]**

### 9.4 After the screen closes

If the context word has bit 8 (career manager, the Career button path) the exit routine of part 1 section 4.6 starts the
career: settings flag bit 0 clear means **new career** (`0x005A3950(1)`, the steps of [career-flow.md](career-flow.md) section
3), set means **resume** (`0x0057F570`, section 4). A cancelled or failed load returns to the alias manager. **[exe]**

### 9.5 Messages on the load path (English labels, paraphrased)

| Situation | Text label | Notes |
|---|---|---|
| load while an alias is loaded | `MC_CONFIRM_LOAD_LOSE_ANY_CHANGES_TO_THE_PROFILE` | tag 6, Yes/No |
| no aliases | `MC_INFORM_NO_PROFILES_FOUND` | tag 3 |
| library: file missing, short, bad header | "Unable to load %1." (library id 11) | Continue |
| library: success | "Load complete." (library id 10) | Continue |
| checks 8/9 fail | `MC_CORRUPT_PROFILE_XBOX` | tag 0xD |

## 10. Create and delete

**Create** (alias manager **Create**, mode `0x61`, state `0x60`). Step list **[exe]**:

1. List-ready for state `0x60` (no list is taken): an alias is loaded and bit 19 is clear: question "creating a new alias
   loses unsaved changes" (tag 5); otherwise the create question (tag 1) whose wording depends on bits 19, 16, 15.
2. Yes: snapshot; unless bits 18 or 21, **reset the profile to a fresh default** (`0x005AB9C0`: default profile, settings
   reset (stage 16, cash 0), default stable rebuilt, popup queue cleared, the gameplay manager reset); with bit 19 only the
   career settings are initialised (`0x005A3950(0)`). Start-up state `0x20` becomes `0x60`.
3. Two information notices about autosave (`MC_AUTOSAVE_WARNING_PC`, `MC_AUTOSAVE_WARNING2_PC`), each with Continue.
4. Name entry (the keyboard group "Enter Name:" of `MC_Main.fng`; the PC uses the keyboard). The accepted text is copied to
   the screen, stored in the profile (`0x0056DA30`), *profile loaded* := 1 (before anything is written), and the question
   "Do you want to save the alias X?" is asked (tag 4).
5. Yes: quiet mode off, then the save of part 1 section 5 steps 3 to 6. An existing alias with the same name triggers the
   library's overwrite question, which is the only duplicate-name protection.
6. No at any question: with state `0x60` autosave is set off; with bit 19 the snapshot is restored; the screen leaves.
   A cancelled create after step 4 leaves *profile loaded* = 1 with an unsaved default profile. **[exe]**

The name text field is the FNG's; no length check was found in the code. The list shows only names shorter than 8
characters without spaces (part 1 section 2), and the library keeps 31 characters. A character that the file system
refuses fails at the folder or file open (code 7, "unable to save"). **[unconfirmed]** for the field's own limit.

**Delete** (alias manager **Delete**, mode `0x31`, state `0x30`): the list as above; the pick sets pending code 6, then
`0x00518AC0` requests the delete (library op `0x20`):

1. Prompt type 12 "Are you sure you want to delete %1?" (No/Yes; not masked in the interactive flow). No: status 2, the
   screen closes with the "failed or cancelled" message.
2. Yes: for the alias folder, remove every file matching `<root>/<alias>/*` (a failed `remove` is code 7), then remove the
   directory (code 7 if it fails, for instance with a subfolder inside). The prompt type 13 "Save game deleted." or
   type 14 follows. The text of type 14 (the failure) has **no entry** in the locale table (ids go to 13), so that prompt
   has no text.
3. Completion (`0x00519050`): if the deleted alias equals (exact, case-sensitive comparison) the loaded profile's name,
   the profile is **reset to a fresh default** (`0x005AB9C0`) and *profile loaded* := 0; then "done" is posted. **[exe]**

**Rename** does not exist anywhere in the code. The only way an alias changes name is editing the inner name in the file
or copying a folder; see part 1 section 2 for what happens then.

## 11. Other places that touch the save, and code that is never reached

- **Options, gameplay page.** The Autosave row is shown only when `0x00510530` allows it (play mode is free roam or the
  front end, the layer is present, an alias is loaded, the context has no online bit 3). Left/right (`0x00528EA0`) toggles the
  profile option; switching it **on** opens the screens with mode `0xA1` (state `0xA0`): the Yes/No question about enabling
  autosave (tag 0xC); Yes arms quiet mode, No sets the option back to 0. Leaving the gameplay page while the option is
  **off**, an alias is loaded and the page index is 3 opens the same question (`0x00545C1F`; how that handler is reached
  was not followed **[unconfirmed]**). The page's "reset to defaults"
  keeps the old value of the option when it is not available (`0x0051D170`). **[exe]**
- **Unsaved changes.** The two option screens (`0x00545730`, `0x00545F50`) set FEDatabase `+4` when the settings differ
  from the stored copy; a save clears it. Only the main menu's way back to the title uses it (section 8.5). **[exe]**
- **Challenge Series** has its own autosave openings (`0x005A7C97`, `0x005A8285`, mode `0x100B1`) when the context word has bit
  1; not part of the career.
- **Never reached on PC:** `MC_DeleteProfile.fng` (its opener `0x005262C0` is never constructed); the unsaved-change
  comparison `0x00599570` (no caller); the notices "too many aliases", "loaded successfully", "no card" variants (`0x00543E40`,
  `0x00543EA0`, `0x00543F90`); the `0x518710` mask; state `0xD0`; the device-select operation; the UG2 cash prompt is
  separate (`0x00518B40`, settings bit 1).
- **Quit** shows `QUIT_GAME` (unsaved changes will be lost) and offers no save.

## 12. What saving changes

| Item | Change |
|---|---|
| profile name `+0x00` and its custom-name byte `+0x20` | set from the alias (manual save and create only) |
| FEDatabase `+3` profile loaded | 1 |
| FEDatabase `+4` unsaved changes | 0 |
| manager `+0x24` switched off by error | 0 |
| race-bin objects of the gameplay manager | live state flushed into the stored bytes (dirty flags kept) |
| spawn keys, pending notification list, stage, flags, cash, counters, bounty, node states | **read only** |
| save counter, timestamp | none exists |
| profile autosave option | unchanged by a successful save; set to 0 by a failed autosave or a "No" to the questions above; set to 1 by "Yes" to the re-enable question |

Nothing else is written; in particular saving does not touch the heat, the popup queue or the career flags. **[exe]**

## 13. Library texts

`MEMCARD/LOCALE_ENGLISH.loc` (`LOCH`/`LOCI`/`LOCL`): 14 UTF-16 strings and an id table that maps library id *n* to
string *n - 1* for ids 1..13 (id 0 also to string 0). Ids used: 3 "Yes", 4 "No", 5 "Continue", 6 "PC hard disk", 7 overwrite
question, 8 save complete, 9 unable to save %1, 10 load complete, 11 unable to load %1, 12 delete %1?, 13 deleted. String 13
("unable to delete %1") has no id. Strings 0 and 1 are version numbers. The Russian file has the same shape. The prompt
kind and its buttons are chosen by the library: type 7 and 12 use buttons [No, Yes] (answer 1 = No, 2 = Yes); the others use
[Continue]. **[exe + verified bytes]**

## 14. Rules for a rewrite

```
can_autosave(force):                       // section 8.1
    if force            -> true
    if skip_once        -> skip_once = false; false
    if online || lan    -> false
    if !(option_autosave || off_by_error) -> false
    if current_node == "1.8.1"            -> false
    if current_node.is_some_and(|n| n.boss_race) -> false
    true

on_event_disposed():                       // EAutoSave
    if can_autosave(false) { pending = true }
hud_update():  if pending { armed = true }          // only while a world runs
tick():        if pending && armed && gameplay_idle { pending = armed = false; run_autosave(false) }

run_autosave(force):
    if !can_autosave(force) || !profile_loaded -> return
    show_icon(); if off_by_error { ask_continue_autosave(); return }
    match write_alias(profile.name, serialize(profile)):
        Ok            -> profile_loaded = true; dirty = false; off_by_error = false; hide_icon()
        Err(OpenFail) -> option_autosave = false; hide_icon(); ask_continue_autosave()   // Yes: option = true, retry(force)

write_alias(name, buf):                    // part 1 section 6.2
    dir = root/name; exists = dir.is_dir()
    if exists && interactive && !confirm_overwrite() -> Cancelled
    create_dir(dir) (ignore error)
    f = create_truncate(dir/name)?  else OpenFail   // the only error
    write(header 0x1C, descriptor 8 = {0x10D, len}, buf)   // errors ignored
    Ok

load_alias(name):
    snapshot = serialize(profile)
    bytes = read(root/name/name)?                         // codes 9, 7, 8 of section 9.3
    require header.total == len && magic == 0x4D433032 && crc32x(header[..0x18]) == header.crc
    require desc_size == 8 && prof_size == 0xF848         // CRC of desc/profile: do not enforce
    require desc == {0x10D, 0xF848}
    ok = load_from_buffer(bytes)                           // stable hash, then data, then MD5 last
    if !ok { load_from_buffer(snapshot); show("damaged"); return }
    profile_loaded = true; off_by_error = false
    if option_autosave { arm_quiet() }
```

A reimplementation may choose a safer write (temporary file and rename) and should then also report write errors; the
original does neither. Keep the message flow and the autosave rules; the PS2/Xbox wording for the autosave failure is an
artefact that a rewrite can replace with a PC text.

## Open questions

1. The name of the node flag `0x8CB01ABF` and which nodes carry it (decides which events trigger the start-of-event
   autosave).
2. Whether the two-click behaviour of the prompt answer (`0x005195D0` answers a pending library prompt before the game's
   own dialog sees the button) can lose a click in the autosave-failure dialog.
3. What the two bytes at `0x0091CAE0` control (HUD visibility while a save screen is up).
4. Bits `0x200` and `0x2000` of the save mode `0x2251`.
5. The exact behaviour of `0x005EC040` (the notification-ready test) in each game state; section 8.3 states it by its
   effect.
6. Whether the "Saving %s" line of state `0xB0` prints the placeholder literally.
7. Nothing in this part was seen running; every flow is a static reading and the prompt order in particular
   should be confirmed with a trace (`Documents\NFS Most Wanted` can be watched for the truncate/write sequence).
