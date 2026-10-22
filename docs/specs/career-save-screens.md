# Career save screens: the memory-card layer, the Save screen and the save write

What the PC executable does when the player saves: the safe house **Save** item, the alias manager, the `MC_Main`
screen and its dialogs, the memory-card layer underneath, and what is written to disk in which order. The autosave,
the load path, create and delete are in [career-save-screens-2.md](career-save-screens-2.md). The byte layout of the
file is in [career-save.md](../formats/career-save.md); the context word and the safe house are in
[career-flow.md](career-flow.md) and [career-safehouse.md](career-safehouse.md). Addresses, Ghidra names, method and
dead ends are in the [provenance file](../provenance/career-save-screens.md).

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) read through the Ghidra MCP and by
  disassembling routines the project does not define; the screens `MC_*.fng` and `Autosave_Overlay.fng` dumped with
  `nfsmw dump-screen`; `MEMCARD/LOCALE_ENGLISH.loc`; the three saves on this machine (layout only, read only).
- **Evidence tags:** **[exe]** read in `speed.exe`, not tested in game. **[verified]** checked against the install or
  the saves. **[unconfirmed]** a reading that was not closed. Nothing here was observed while the game ran.
- **Addresses** are `speed.exe` addresses; no Ghidra auto names appear here. Game text is cited by its label name from
  the language table and paraphrased.
- **Words:** *the manager* is the single memory-card manager object (created at start-up); *the library* is the EA
  memory-card library underneath it (the strings `2.04.00` and `2.08.04` in the locale file are its versions); *the
  profile* is the live `UserProfile`; *FEDatabase* is the profile database object (`0x0091CF90`).

## 0. Direct answers

| Question | Answer |
|---|---|
| What does the safe house **Save** item do? | It opens `MC_Main.fng` in the "save" state. The screen asks one Yes/No question (save the alias named X?), then writes `Documents\NFS Most Wanted\<alias>\<alias>`. The library may ask a second question (overwrite?) and a third notice (save complete). Section 5. **[exe]** |
| Which routine writes the file? | The library's save state machine at `0x007F4F11`. The game serializes the profile first (`0x0058FD10`, called by `0x005188F0`), then the library writes **header (0x1C), descriptor (8), profile (0xF848)** in one open/write/close. Section 6. **[exe]** |
| Atomic? | No. The file is opened with truncate and written in place; there is no temporary file, no rename and no flush call. **[exe]** |
| Is a write error reported? | Only a failure to **open** the file. Short writes and a full disk are ignored: the status is reset to success after the close. **[exe]** |
| What gates autosave? | Section 8 of part 2: not forced, not online or LAN, layer present, option on (or "switched off by an error" byte), not the final event `1.8.1`, current event not a boss race, plus the one-shot skip after a resume. **[exe]** |
| Does load recompute or ignore the descriptor/profile CRC? | **Ignores** a mismatch (the error is overwritten with success, checked in the disassembly). The header CRC, magic and length are enforced. The profile MD5 is enforced. Part 2, section 9. **[exe]** |
| What changes when saving? | The profile name is stored, the "has an alias" byte is set, the "unsaved changes" byte is cleared, and the live race-bin objects are flushed into the blob. No counter, no flag, no respawn key and no stage is changed. Part 2, section 12. **[exe]** |
| Rename? | Does not exist. Delete does (state `0x30`). Part 2, section 10. **[exe]** |

## 1. The layers

1. **The manager** (object of `0xD4` bytes, global `0x0091CB20`, built at start-up by `0x005435D0`). It holds the pending
   and active operation codes, a snapshot of the profile, the data buffers and about thirty flag bytes (section 1.2).
   Its per-frame update is `0x00563D10`, called from the front-end/world update (`0x005642C0`).
2. **The library** (object `0x304` bytes, `0x007F4CB7`). It owns a queue of at most three operations, the PC
   "hard disk" device, the file I/O and a set of **prompts** whose text comes from `MEMCARD/LOCALE_ENGLISH.loc`
   (section 10 of part 2). It reports to the manager through a table of twenty callbacks (`0x0089CD78`).
3. **The screens**: `MC_Main.fng` / `InGame_MC_Main.fng` (all dialogs and the keyboard), `MC_List.fng` (the alias
   list, 9 rows), `MC_ProfileManager.fng` (the alias manager, class `FEPkgMemcard`), `MC_Bootup.fng` (start-up),
   `Autosave_Overlay.fng` (the save icon). `MC_DeleteProfile.fng` is registered but nothing opens it (section 11 of
   part 2).

**Layer present.** Two globals (`0x008F8708`, `0x008F870C`) start at 1 and the executable never clears them (the only
writers are the record/replay filter `0x00667460`). Every "memory-card layer present" test below is therefore true.
**[exe]**

### 1.1 Operations

The manager keeps a *pending* code (`+0x38`, set by screens, executed by the update when no operation is active and the
dialog flag is clear) and an *active* code (`+0x40`).

| Code | Active meaning | Started by | Library call |
|---|---|---|---|
| 2 | select device (unused state `0x80`) | `0x00518840` | op 1 |
| 3 | **save** | `0x005188F0` | op 8 (write) |
| 4 | **set quiet mode** (autosave on/off) | `0x005268B0` | op `0x80` |
| 5 | **load** | `0x005189F0` | op `0x10` (read) |
| 6 | **delete** | `0x00518AC0` | op `0x20` |
| 7 | **list** `root\*` | `0x00562B80` | op `0x40` |
| 8, 9 | device query (no caller) | `0x00518870`, `0x005188B0` | op 1 |
| 10 | device monitor | `0x00563FBE`, `0x00563820` | op `0x100` (its callback shows the list) |

Pending codes: 5 load, 6 delete, 7 list. The list operation is requested by the screens; the others by the flows below.

### 1.2 Manager flags (the ones that matter)

| Offset | Name used here | Meaning |
|---|---|---|
| `+0x14` | prompt open | a library prompt is waiting for an answer |
| `+0x16` | quiet armed | set when the quiet-mode operation reports "quiet" |
| `+0x1A` | interactive | 1: library prompts are shown; 0: the "silent mask" of section 4.3 is on |
| `+0x1E` | autosave run | a background autosave is in progress |
| `+0x1F` | pending | an autosave was requested (set when the event is consumed) |
| `+0x20` | saving | an autosave write is in progress |
| `+0x23` | screen pushed | `MC_Main` was pushed by the flow starter (cleared at exit) |
| `+0x24` | switched off by error | autosave was turned off by a failure; the next autosave asks first |
| `+0x25` | re-enable | the "continue with autosave?" answer was Yes |
| `+0x26` | dialog visible | the dialog panel of `MC_Main` is showing |
| `+0x27` | setup pending | the quiet-mode operation of a manual save is running |
| `+0x2A` | armed | the in-game HUD update has run since `pending` was set |
| `+0x2B` | skip once | the next autosave check returns false and clears this byte |
| `+0x50` | second player | the flow loads or saves player 2 (flag bit 17) |
| `+0xBC`, `+0xC0` | buffer is shared / size | size is `0xF848` (`63,560`) |

FEDatabase bytes used: `+3` **profile loaded** (an alias is active), `+4` **unsaved changes** (set by option screens, cleared
by a save), `+8` default stable hash, `+0x12C` context word, `+0x128` profile snapshot pointer. **[exe]**

## 2. Where the files are

- **Root.** Documents folder (shell folder id `0x8005` = personal folder + create flag) + `\` + the string at `0x00901748`
  (`NFS Most Wanted`; a mod can change it). Both directories are created (errors ignored) every time the library's
  "device ready" callback runs (`0x00559AC0` calls `0x006CBF00`), which also hands the full path to the library.
  **[exe + verified]**
- **File.** `<root>/<alias>/<alias>` with forward slashes; the folder is created before the file is opened (an existing folder
  is fine). The alias string is cut to 31 characters (`strncpy` into 32 bytes). **[exe + verified]** (`NAME\NAME`,
  `mwo\mwo`, `stock\stock` on this machine).
- **Which name is written.** Always the profile's own stored name (profile `+0x00`), not the folder name the file was loaded
  from. A renamed folder therefore saves into a *new* folder named after the inner name. **[exe]**
- **Empty name.** When a name is empty the profile gets the label `COMMON_PLAYER_1` ("Player 1") or `COMMON_PLAYER_2`
  and its "custom name" byte (`+0x20`) is 0; otherwise that byte is 1 (`0x0056DA30`). **[exe]**
- **Listing.** The list operation enumerates every entry of the root (folders and files, not `.` and `..`). The game keeps
  only names **shorter than 8 characters and without a space** (`0x00563820`), which also hides `Settings.ini` that lives in
  the same root. An alias name that is 8 or more characters, or contains a space, is never listed and so can never be
  loaded or deleted from the screens. There is no limit on the number of aliases (the "too many" notice is never raised,
  part 2 section 11). **[exe + verified]**
- **Not a save.** `Settings.ini` in the root is the graphics/input settings file, written by other code. **[verified]**

## 3. The mode word

`0x00527940` starts the screens: it stores seven arguments and the **mode word** in globals (`0x0091CBAC` word; `0x0091CBB0`
screen A; `0x0091CBB4` screen B; `0x0091CBBC/C0` a callback and its argument; `0x0091CBD0/D4` two messages), clears the
library's silent masks, and opens `MC_Main.fng` when the front end runs (`0x00925E90 == 3`) or `InGame_MC_Main.fng` when
a world runs (exit routing 1 and 3 push it over the calling screen, 2 replaces the calling screen). It sets the manager's "second player" byte from bit 17 and "screen pushed" to 1.

| Bits | Meaning |
|---|---|
| `0x0000000F` | exit routing: 0 none (start-up sequence moves on by itself), 1 return to the calling screen, 2 go to screen B, 3 return then go to B |
| `0x000000F0` | **state**, table below |
| `0x00000100` | at exit forward the exit message to screen B |
| `0x00000400` | at exit post message D4 (cancel) or D0 (ok) to screen B: D0 when the exit message is `0x461A18EE` |
| `0x00000800` | at "back": if the profile has a custom name post "done" instead of "back" |
| `0x00001000` | on destroying the screen reset all the globals |
| `0x00004000` | at init post `0x5AFE12F4` to screen A |
| `0x00008000` | EA-login wording (online; not used by the career) |
| `0x00010000` | Challenge Series wording |
| `0x00020000` | second player (no "lose changes" question) |
| `0x00040000` | "save your changes" wording |
| `0x00080000` | new-career wording |
| `0x00200000` | "stable" wording (shop exit) |
| `0x00400000` | at exit continue the Blacklist stepper (`0x005302C0`) instead of routing |
| `0x0F000000` | runtime: tag of the dialog on screen (section 4.2) |

Bits `0x200` and `0x2000` (set in `0x2251`) were not found tested anywhere **[unconfirmed]**.

| State | Role |
|---|---|
| `0x10` | **load**: confirm if an alias is loaded, list, pick |
| `0x20` | start-up: load the only alias or choose or create |
| `0x30` | **delete**: list, pick |
| `0x40` | EA-login save (online) |
| `0x50` | **save** the loaded alias |
| `0x60` | **create** an alias (confirm, name, save) |
| `0x70` | like load (reached from create) |
| `0x80` | device select (no caller) |
| `0x90` | autosave error: ask whether to continue autosave |
| `0xA0` | enable autosave: ask |
| `0xB0` | **autosave with the screen up** (Blacklist stepper, leaving to the title, shop exit) |
| `0xD0` | pre-save listing (no flow sets it) |

## 4. The `MC_Main` screen

### 4.1 Messages it handles (`0x00564630`, which first calls the alias-manager handler `0x00564070`)

| Message | Handling |
|---|---|
| `0x35F8620B`, `0x3A2BE557` (init complete) | the state step of section 4.4 |
| `0x0C407210` (select) | alias manager: `0x00563380`, the dialog result (section 4.2) |
| `0x461A18EE` ("done") | free the snapshot, then `0x00519730`: record the exit message, then play the leave script (when the dialog panel is visible) or post `EXIT_COMPLETE` (`0xE1FDE1D1`) |
| `0x8867412D` ("back"), `0xDC12AF2E` ("failed or cancelled") | close the list sub-screen; if career-manager context and an alias is loaded and autosave is on and the state is not `0x10` (and not the failed message): re-arm quiet mode; else if state `0x60` and the message is the failed one: switch autosave off and show the "autosave disabled" notice; else exit as above |
| `0xA4BB7AE1` | post `0x461A18EE` to every object (exit) |
| `0xB57FDB17` | show the "no card, autosave disabled" notice (tag 0xB) |
| `0xC6C6B68F` | **begin the save** (action 8 of section 4.5) |
| `0xFE202E3B` | ask "save the alias?" (action 4) |
| `0x5A051729` | the alias list is ready (section 9.2 of part 2) |
| `0x54B3AC6C` | push `MC_List.fng` |
| `0x15457DE1` | close `MC_List.fng` |
| `0xF35D144E` | a load failed the profile checks: show the "damaged" notice (tag 0xD) |
| `0xDA5B8712` | the typed name was accepted (create flow) |
| `0xE1FDE1D1` (exit complete) | the exit routine of section 4.6 |

The yes/no buttons are the objects `0xB8A7C6CC` (**No**, focused first) and `0xB8A7C6CD` (**Yes**, or the
only button of a one-button notice); a third `0xB8A7C6CE` exists. A library prompt is answered by button position
(1, 2, 3) through `0x005195D0`. **[exe]**

### 4.2 Dialog tags (bits 24..27 of the mode word)

A dialog is built by `0x00543CC0` (Yes/No, tag in its second argument) or `0x00543BE0` (one button, label `COMMON_OK`). When a button is pressed
the tag is saved, cleared from the word and `0x00563380` acts. "Yes" means button `0xB8A7C6CD`.

| Tag | Dialog | Yes | No |
|---|---|---|---|
| 1 | create an alias? (wording by bits 19, 16, 15, else generic) | snapshot; reset the profile unless bits 18/21; with bit 19 start a new career's settings; state `0x20` becomes `0x60`; then the autosave notices (action 10) | with bit 19 restore the snapshot; leave |
| 3 | notice (no aliases found) | leave | leave |
| 4 | **save the alias named X?** | action 12: set quiet mode off, then (message `0xC6C6B68F`) save | with state `0x60` switch autosave off; post `0xDC12AF2E` |
| 5 | create while an alias is loaded: unsaved changes are lost | snapshot; reset the profile unless bits 18/21; action 10 | restore the snapshot; leave |
| 6 | load while an alias is loaded: unsaved changes are lost | clear the list and request the list operation | leave |
| 7 | notice (autosave disabled; also "loaded" text, never shown) | post `0x461A18EE` | same |
| 8 | autosave information, part 1 | action 11 | same |
| 9 | autosave information, part 2 | post `SAVE_DONE`; action 3 (one button) | same |
| 0xA | continue with autosave? (PS2 wording on PC) | `re-enable = 1`, autosave on, state `0xB0`, run the state step | autosave off; leave |
| 0xB | notice (no card; autosave disabled) | if state `0xA0` and not EA: exit routing := 1 (return); post `0x7E998E5E` and `0x461A18EE` | same |
| 0xC | enable autosave? | set quiet mode **on** (arm) | autosave off; post `0x7E998E5E`, `0x461A18EE` |
| 0xD | the alias is damaged | leave | leave |

### 4.3 The silent mask

The library can answer its own prompts. The manager sets or clears **mask bits** (`0x007F3CA0`: 0 clear, 1 auto-answer,
2 force-ask); `0x007F4B97` evaluates them. The prompt types and their mask bits:

| Prompt | Type | Bit | Auto answer | Buttons |
|---|---|---|---|---|
| overwrite existing save? | 7 | `0x100` | 2 (Yes) | No, Yes |
| save complete | 8 | `0x400` | 1 | Continue |
| unable to save `<alias>` | 9 | `0x800` | 1 | Continue |
| load complete | 10 | `0x4000` | 1 | Continue |
| unable to load `<alias>` | 11 | `0x8000` | 1 | Continue |
| delete `<alias>`? | 12 | `0x10000` | 2 | No, Yes |
| deleted | 13 | `0x40000` | 1 | Continue |
| unable to delete | 14 | `0x80000` | 1 | Continue |

When the library's own quiet byte is 1 (`+0x6A`, set by the quiet-mode operation) bits `0x100` and `0x400` are forced on.
`0x00518600` is the **silent mask** used by background autosaves: it sets bit 1 and every bit of the table except `0x800`
(so an error is still shown) and clears bits 2, 4, `0x800`. `0x00518710` (the opposite mask) has no caller. At the
start of every flow and of the list-ready step the manager clears all bits. **[exe]**

### 4.4 The state step (`0x00562C30`, run at init complete)

0. If the layer were absent: post `0x461A18EE` and stop (never happens, section 1). Set the message text to a space.
1. Bit `0x4000`: post `0x5AFE12F4` to screen A. Bit `0x400000`, or bit 16 with state `0xB0`: post `MEMCARD_ON` to the screen.
2. By state:
   - `0x10`/`0x70`: with an alias loaded and not bit 17: dialog tag 6. Otherwise list (action `0x00559D60`).
   - `0x20`: if the device status is not 0 or `0xB`, leave; else list.
   - `0x30`: list. `0x40`/`0x60`: no list is taken; `0x5A051729` ("list ready") is posted to all objects at once, which
     leads straight to the create/save questions (section 9.2 of part 2).
   - `0x50`: copy the **profile name** into the screen's alias buffer (`+0x40`) and run action 6 (dialog tag 4).
   - `0x90`: show the autosave-error dialog via `0x00559E90` (section 8.4 of part 2).
   - `0xA0`: with bit 15 re-arm quiet mode; else dialog tag 0xC.
   - `0xB0`: no alias loaded: switch to state `0x60`. Else if the autosave gate (section 8.1 of part 2) passes: show the
     text `MC_SAVEING_THE_PROFILE_PC` ("Saving %s. Please don't turn off your PC.") in the message panel, copy the profile
     name, and run the autosave with **force**. If the gate fails: state becomes `0x50` and the step repeats.
     (The text is set without formatting, so the `%s` may be shown literally **[exe, unconfirmed in game]**.)

### 4.5 Actions (`0x00544370`, "go to step n")

| n | Action |
|---|---|
| 1 | dialog tag 5 |
| 2 | dialog tag 1 (wording: bit 19 new career; bit 16 challenge; bit 15 EA login; else `MC_CREATE_NEW_PROFILE_MENU`) |
| 3 | the name-entry stage: shows the one Accept button of the keyboard panel **[unconfirmed]** |
| 4 | "save the alias?" dialog (`0x00544210`: `MC_SAVE_PROMPT`, or the "stable" variant with bit 21), tag 4 |
| 6 | "save the alias named X?" dialog (`0x00543EB0`: `MC_CONFIRM_SAVE_THE_PROFILE`; bit 18 `MC_SAVE_PROFILE_CHANGES`; bit 21 `MC_SAVE_STABLE_PROMPT`; bit 15 EA wording). The alias name is filled in twice. Tag 4 |
| 8 | **write**: store the typed or kept name in the profile (`0x0056DA30`), then start the save operation (`0x005188F0`) |
| 9 | notice `MC_TOO_MANY_PROFILES`, tag 3 (no caller) |
| 10 | post `SAVE` to the screen, notice `MC_AUTOSAVE_WARNING_PC`, tag 8 |
| 11 | notice `MC_AUTOSAVE_WARNING2_PC`, tag 9 |
| 12 | start the quiet-mode operation with value **off** (`0x005268B0(0)`) |

### 4.6 Leaving (`0x00562FF0`, at exit complete)

1. Route the result messages by bits `0x100`/`0x400` (`0x0051ABC0`).
2. If the context word has bit 8 (career manager) and the front end runs: reset the context to 0, then
   - no alias loaded, or state `0x10` ended with "back", or the last dialog tag was 1, 3 or 5: mode low nibble := 1,
     restore the snapshot, context := bit 8 again (back to the manager);
   - settings flag bit 0 clear: start a **new career** (`0x005A3950(1)`); else **resume** (`0x0057F570`).
3. Bit `0x400000`: continue the Blacklist stepper. Else by low nibble: 1 pop the screen (the SMS mailbox special case),
   2 go to screen B (with the second-player byte as argument), 3 pop and then go to B. Challenge Series (bit 16) has its own path.
4. Clear the globals, hide the autosave HUD flag, clear the monitor bit, and if the device monitor was started
   (`+0x2C`) stop it (operation 10). **[exe]**

## 5. Manual save, step by step (safe house **Save**, alias manager **Save**)

The safe house item exists when the layer is present; it is not conditional on the autosave option. Its select handler
(`0x005321C0`) calls the starter with mode `0x2251` (state `0x50`, return to the caller). The alias manager's Save item
(`0x00542FD0`) uses the same mode; it is hidden when no alias is loaded.

1. `MC_Main.fng` is pushed over the safe house menu (the world is not running, so not the `InGame_` variant).
2. At init the state step copies the profile name and shows **"Are you sure you want to save the alias named X?"**
   (`MC_CONFIRM_SAVE_THE_PROFILE`, No focused). **No**: `0xDC12AF2E` is posted, the screen closes and the safe house menu
   is shown again; nothing was written.
3. **Yes**: action 12 starts the quiet-mode operation with value off. For its duration every library prompt is
   auto-answered. When it reports back (`0x005271E0`) the interactive byte is set, **all mask bits are cleared** and the
   message `0xC6C6B68F` is posted.
4. `0xC6C6B68F` runs action 8: the profile name is stored and the **save operation** starts. The profile is serialized
   *now* into a new `0xF848`-byte buffer (section 6.1), before any prompt.
5. The library's save state machine (section 6.2) runs one state per frame. Because the masks are clear:
   - if `<root>/<alias>` already exists it shows the **overwrite** prompt (No/Yes). No ends the operation as
     cancelled (nothing is written; the error callback posts `0xDC12AF2E` and the screen closes);
   - after the write it shows **save complete** (Continue), or **unable to save X** (Continue) on an open failure.
6. The completion callback (`0x00526AC0`) clears the manager's operation, sets *profile loaded* = 1, clears *unsaved
   changes*, clears "switched off by error". Because the screen is interactive: if the profile's autosave option is on
   it re-arms quiet mode (`0x005268B0(1)`, which ends by posting the "done" message); otherwise it posts `0x461A18EE`
   at once. The screen leaves by section 4.6 (pop to the safe house menu).
7. Failure (open error): the error callback (`0x00526F60`) frees the buffer and posts `0xDC12AF2E`; the screen closes.
   The autosave option is **not** changed by a failed manual save. **[exe; the prompt order is reconstructed from the
   library and was not seen in game]**

So the player normally sees up to three panels: the confirmation, the overwrite question (every save after the first),
and "save complete". There is no progress indicator on the manual path; the dialogs are the indication.

## 6. What is written

### 6.1 Serialization (`0x005188F0`)

1. Size = `0xF848` (`0x0057FD90(0)`). Set the manager's buffer fields; the "info" object is built (descriptor size 8,
   profile size `0xF848`, a label text "Profile", the alias as wide characters).
2. Allocate the buffer (64-byte aligned) and call `UserProfile::SaveToBuffer` (`0x0058FD10`) with the live profile. The
   order of the blocks is in [career-save.md](../formats/career-save.md) section 2 (version 16 zero bytes, settings blob
   with its own MD5, default stable hash, name, playlist, options, stable, flags, statistics, quick-race selections, race
   settings, controller bindings, MD5 of bytes `[0x10, size-0x10)`).
3. Before the blob is written the gameplay manager **flushes** every tracked collection that has live state (the race-bin
   objects) into its stored bytes (`0x005F6BA0` with the "keep dirty" argument). The blob also takes the two spawn keys
   (gameplay manager `+0x108`, `+0x10C`) and the list of pending notifications (manager `+0x2A8`, count `+0x2AC`). None of
   these writes changes live gameplay state. **[exe]**
4. Descriptor: dword `0x10D` (profile type id) and the profile size.

### 6.2 The library's write (`0x007F4E57` prepares, `0x007F4F11` runs)

**Prepare** (when the operation is accepted): copy the alias (31 chars), compute the **descriptor CRC** and the **profile
CRC**, fill the 0x1C-byte header (magic `0x4D433032`, total `0x1C + 8 + size`, descriptor size, profile size, the two CRCs)
and compute the **header CRC** over its first 0x18 bytes. The CRC is the one of [career-save.md](../formats/career-save.md)
section 1 (`0x007F73E1`).

**Run**, one state per update call:

| State | Action |
|---|---|
| 9 | build `<root>/<alias>`; find it; **exists** when a directory entry is returned. Exists: raise the overwrite prompt (type 7) and go to 0xA. Else go to 0xB |
| 0xA | wait for the answer: **1 (No)**: status "cancelled" (2), code 0, go to 0xE. 2 (Yes): go to 0xB |
| 0xB | `mkdir <root>/<alias>` (result ignored); open `<root>/<alias>/<alias>` with write-only, create, **truncate**, binary, permission read/write; **open failure: status 1, code 7**. Else write the header (0x1C), the descriptor, the profile, close, then **set status 0 and code 0 unconditionally**. Go to 0xC |
| 0xC | status 0: prompt type 8; status 1: prompt type 9 with the alias; go to 0xD |
| 0xD | wait for the answer (auto when masked), go to 0xE |
| 0xE | report (status, code, alias) to the controller, which calls the manager's callbacks |

A separate **check** operation (op 4, `0x007F4414`) runs states 9 to 0xC without writing (`+0x300 == 0`): it is the same
overwrite question and nothing else. No caller in the career uses it. **[exe]**

**Consequences.** (a) Write order is header, descriptor, profile; a crash after the truncating open leaves a short or empty
file, which the loader refuses (length check). (b) A full disk or a write error is **not** reported: the status is reset
to success after the close, so the player gets "save complete" (or nothing, when masked). (c) The only error is "cannot
open the file or folder" (read-only file, invalid characters in the alias, missing documents folder), code 7.
(d) There is no free-space check on the PC (the disk-space query `0x007F48A4` is only used by operation 1, which no
career flow starts). **[exe, disassembly checked]**

## 7. Progress indicator and HUD

- **Foreground (screen up):** the dialog panel only; state `0xB0` also shows the "Saving..." line of section 4.4.
- **Background autosave:** `Autosave_Overlay.fng` is pushed (`0x00518C50`) when the run starts and not in state `0xB0`. It
  shows a 46-pixel icon with a spinning ring in the upper right; the animation starts with the message `SAVE`
  (`0x001C8ACE`) and ends with `SAVE_DONE` (`0x40E73793`, `0x00518DF0`). The icon moves to the left side
  (`LEFT_WIDE`/`LEFT_NORM`; `_WIDE` or `_NORM` follows a profile option byte at `+0x34` **[unconfirmed]**) while the current event is a race-type node (flag `0x8CB01ABF`,
  section 8.1 of part 2) and to the right (`RIGHT_*`) otherwise; with the SMS mailbox open it hides the incoming-message
  icon. **[exe]**
- While an `MC_Main` screen is up in a world the HUD is hidden and shown again when it closes (the pair of bytes at
  `0x0091CAE0`; meaning **[unconfirmed]**).

## Note: the prompt's pad

`MC_Main.fng` has package responses for Left and Right (`0x9120409E`, `0xB5971BF1`) but none for Accept (`0xC519BFC3`) or Back
(`0x911C0A4B`), so the game never hears those two as messages from this package. The rewrite reads them from the pad
(`ScreenLogic::pad_pressed`); reading them as messages left the prompt answering only the highlight. **[verified: dump of the
package; scripted run in the career flow]**
