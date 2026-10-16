# Provenance: EA Trax jukebox

Record for [specs/eatrax-jukebox.md](../specs/eatrax-jukebox.md). Holds the addresses and names of the original
binary that the spec does not carry, how they were found, and the lessons of the pass.

## Sources and how they were read

- **The install's package** `EA_Trax_Jukebox.fng` (`FRONTEND/FrontB.lzc`, the install at `D:\`). Read with the
  port's `nfsmw dump-screen EA_Trax_Jukebox.fng` (text output, kept in the scratchpad, not committed). The dump gives
  the 120 objects, the 22 resources, the scripts, the message routing ("message X goes to ...") and the package's
  responses ("package response X: game(...), feng(...), sound(...)"). No game data is in the repo.
- **`speed.exe`** (the original PC build), read in Ghidra through the MCP tools. Only the program `/speed.exe`
  was used. No function was created and nothing was saved in the shared project. All the reads were `get_functions`,
  `get_xrefs_to`, `search_strings`, `search_byte_patterns` and `read_memory`.
- **Repo docs read for context:** `docs/specs/music-graph.md`, `docs/specs/frontend-menus.md`,
  `docs/specs/custom-music.md`, `docs/specs/confirmation-dialogs.md`, `docs/specs/fe-shadows.md`,
  `docs/provenance/music-graph.md` (the song table), `libs/blackbox-feng/src/ids.rs` and
  `libs/blackbox-feng/src/hash.rs` (the pad ids and the name hash). The worktree copy of the world-map record
  (`.claude/worktrees/...`) was read for the meaning of 0x911AB364 and 0x5073EF13, which the world map shares.
- No decompiled source of the CC0 decomp was read.

## Name hashes

The FEng keys are the Black Box string hash (`bstring_hash`, seed `0xFFFFFFFF`, `h = h*33 + byte`) of the
upper-cased name. Three passes of a Python script (kept in the scratchpad) generated candidate names from word
lists and their pairs with `_`, with numbers 0 to 12, and with prefixes (`PAD_`, `BUTTON_`, `SONG_`, `TRAX_`,
`MOUSE_`, `COLOR`, `REORDER`). The candidates were about 47,700, 79,200 and 16,500 names.

Names found (each matched the id exactly): BUTTON_PRESSED `0x0C407210`, BUTTON_HIGHLIGHT `0xABC08912`,
BUTTON_UNHIGHLIGHT `0x55D1E635`, BUTTON_RELEASED `0x936A6A7F`, INIT_COMPLETE `0x35F8620B`, EXIT_COMPLETE `0xE1FDE1D1`,
LEAVE_SCREEN `0x587C018B`, LEAVE `0x033113AC`, PAD_ACCEPT `0x406415E3`, PAD_BACK `0x911AB364`, PAD_LEFT `0x9120409E`,
PAD_RIGHT `0xB5971BF1`, UP `0x00000704`, DOWN `0x00148CD7`, LEFT `0x0018C32A`, RIGHT `0x039FEFFD`, PRESSED `0xE647B375`,
ACTIVE `0x4F90CF9B`, INACTIVE `0xFB12D252`, SHOW `0x001CA7C0`, HIDE `0x0016A259`, INIT `0x001744B3`,
OFF `0x0000CCFA`, HIGHLIGHT `0x249DB7B7`, UNHIGHLIGHT `0x7AB5521A`, MOUSE_OVER `0xB30D0683`, MOUSE_EXIT `0xB30793C1`,
REORDER_CURSOR `0xE77A5CAF`, COLOR1 to COLOR5 `0x550FFCCF` to `0x550FFCD3`, POS1 to POS9 `0x001B20C2` to `0x001B20CA`,
POS10 to POS12 `0x037F3932` to `0x037F3934`, TOGGLE_INFO `0x279EB82C`, SOUND_LEFT `0x48122792`,
SOUND_RIGHT `0x4AC5E165`, SOUND_ACCEPT `0x7B6B89D7`, SOUND_EXIT `0x480EA221`.

The pad ids `PAD_BUTTON4` `0xC519BFC3` and `PAD_BUTTON5` `0xC519BFC4` follow from the ids in
`libs/blackbox-feng/src/ids.rs` (ten consecutive ids from `PAD_BUTTON0` `0xC519BFBF`); the Ghidra search agreed.

Not named: `0x7EABCA56`, `0x97BDB465`, `0xF459B307`, `0xCFEBE0DF`, `0xB841EA36`, `0xADE14EC3`, `0x33BC8B0B`,
`0x3C2DE6D8`, `0x1A1533D5`, `0x34737EA6` to `0x34737EA9`, `0xB919404C`, `0x1C3553EF`, `0x3E75F58D`, `0x6B945D70`,
`0xC3D2AACE`, `0xBAF366F1`, `0xCCD64A6E`, `0x84378BEF`, `0x4B680587`, `0x7958EA7F`, `0x672DFAFD`, `0x8CB81F09`,
`0x033C9547`, `0x83562D55`, `0x906782D5`, `0xCB1AA27F`, `0x610FB237`, `0x610FB238`, `0x480C9A58`, `0x4A805994`,
`0x13FD3296`, `0xB86367CB`. The three passes do not cover the
names of the game (`SongName`-style keys, screen-specific words). Leave them unnamed until a longer vocabulary or the
handler gives a name.

Note on the hash pass: a 32-bit match among about 140,000 candidates is a chance match in about one case in 30,000
per target. The names above also agree with the package's objects (the "PAD" names with the pad responses, the
"POS" names with twelve slot scripts), which is the stronger evidence.

## Addresses (speed.exe)

Strings (with the xrefs found):

| Address | String | Xrefs |
|---|---|---|
| 0x0089B5C4 | `EA_Trax_Jukebox.fng` | the loader (0x00517098, in the loader routine at 0x00517070); data at 0x005456D7 (no function found there); a table entry at 0x008F45F0 |
| 0x008A1320 | `EA_TRAX_JUKEBOX.FNG` (upper case) | 0x005864B8 in the routine at 0x005864B0 (the screen-kind test of `confirmation-dialogs.md`) |
| 0x0089B2D8 | `EA_TRAX.fng` (the chyron package) | 0x005856AF in the routine at 0x00585680; 0x00513797 in the routine at 0x00513780 |
| 0x00897414 | `EA_Trax.fng` | not read |
| 0x008A0B40 | `UIEATraxScreen` | 0x00871CA2, in the unwind funclet at 0x00871CA0, which calls the routine at 0x00652B40 |
| 0x0089CF10 | `OMEATrax` | 0x0086EAFA, in the unwind funclet at 0x0086EAF8 |
| 0x0089CF8C | `AOEATraxMusicMode` | 0x0086EBAA, in the unwind funclet at 0x0086EBA8 |
| 0x00899B70 | `EATraxInit` | 0x004F88EF (routine at 0x004F8830), 0x005ACD8F (routine at 0x005ACB70), 0x0051E0CC |
| 0x008A164C, 0x008A1658, 0x008A1664 | `TRAX_POS_1`, `TRAX_POS_3`, `TRAX_POS_2` | no xref found (packed name table, not read further) |
| 0x00897A18, 0x00897A28, 0x008979A0, 0x008A7758 | `LicensedMusic`, `Music`, `AudioSystem`, `PathEvent` | class-descriptor tables at 0x00885570 to 0x008855D7 and 0x008852F0 to 0x00885317; `PathEvent` at 0x00625BBE (routine at 0x00625B90) |

The loader and its callers:

- **Loader, 0x00517070** (145 bytes). Guard: the front-end screen object at the global `DAT_0091CB10` is missing, or
  its mode at `+0x11C` is not 3, 4 or 5. Then it loads the package by name: through `0x005B5B70` on the manager at
  `+0xD8` when the global `DAT_0091D218` is below 1, otherwise through `0x00572070` and `0x00591D90`. If the load
  returns 0 and the game-flow global `DAT_00925E90` is 3, it hashes `Pathfinder5` (`0x005CC240`), builds an event
  with id -1 (`0x00405DA0`) and posts it (`0x0065FAF0`). The mode numbers 3 to 5 agree with `fe-shadows.md`
  (its routine `0x00517070`, the jukebox check). One conflict: `fe-shadows.md` says the loader runs only when
  `DAT_00925E90` is 3. This decompile gates only the `Pathfinder5` post on that global; the load itself is not gated
  by it. Check in game before relying on either reading.
- **Caller 0x00542890** (169 bytes): it sets the vtable at 0x0089E3C4 and, if a byte at `+0x4C` is set, sends the
  message 0x18883F75 through the object at 0x007A1FA0. It then calls the loader, releases the member at `+0x30`
  (`0x004655F0`) and runs the base cleanup (`0x00572360`). Destructor-like. Its caller is `0x0054EEF0`.
- **Caller 0x005429A0** (123 bytes): sets the vtable at 0x0089E3D0, calls the loader, releases the member at `+0x40`
  and runs the base cleanup. Destructor-like. Its caller is `0x0054F270`.
- **Caller 0x00559400** (626 bytes): the option row dispatcher. It loops over the rows of an option object (up to
  five lists), reads each row's option id (`*row == id`), and matches it in the option table at 0x008F3428. For id
  0x27 with value 1.0 it calls the loader (`0x00517070`). For id 0x28 it sets the byte at `DAT_0091CAE0 + 0x2C` to 1
  when the value is 1.0 and to 0 otherwise. It is called by the routine at 0x005642C0 (559 bytes), whose callers are
  0x006617E0, 0x00663C90 and 0x00663D30. The routine at 0x005642C0 also calls `0x00516FB0`, `0x00542530`,
  `0x00542C50`, `0x00563D10`, `0x00573020`, `0x00595ED0`, `0x005A8C10` and `0x005A8C20`; those were not read.

The option table at 0x008F3428 (read as 448 bytes; 16 records of 28 bytes): records with id `0x19` (bit 1),
`0x1A` (2), `0x1B` (4), `0x1C` (8), `0x21` (0x10), `0x22` (0x20), `0x23` (0x20), `0x24` (0x40), `0x25` (0x200),
`0x26` (0x400), `0x27` (0x800), `0x28` (0x1000), `0x29` (0x2000), `0x2A` (0x4000), `0x2B` (0x8000), `0x2C` (0x100).
Each record carries a second word (a pointer in the 0x0089 range) and zeros. The labels of the ids are not in this
table; the id `0x27` = "EA Trax mode" is inferred from the order and the Audio row names of `frontend-menus.md`.

## Searches for the game-side handler (not found)

The package's `game` responses include the pad constants. Searching the binary for them:

- `PAD_BUTTON4` (bytes `C3 BF 19 C5`): 25 hits; `PAD_BUTTON5` (`C4 BF 19 C5`): 26 hits; `PAD_BACK`
  (`64 B3 1A 91`): about 110 hits. Most are inside screens other than this one.
- The hits at 0x00557E30 (in the routine at 0x00557E20, 1126 bytes), 0x0055B3CA (in the routine at 0x0055B0C0, 2044
  bytes), 0x00562864 (in the routine at 0x005625CD, 1117 bytes) and 0x00587F26 (in the routine at 0x00587D30, 974
  bytes, not read) were looked at:
  - 0x00557E20 is the online-lobby message routine (`OLX_Message.fng`, `OL_Feedback.fng`, `OL_FriendDialogue.fng`,
    `OL_MAIN.fng`). It answers `0xC519BFC3` by opening its feedback package and `0xC519BFC4` by opening its friend
    dialog. Not this screen.
  - 0x0055B0C0 is the world map's controller: `0xC519BFC3` toggles its option list and sends `0x5C28136D` or
    `0x947E6205` (the world-map record already has this). Not this screen.
  - 0x005625CD is an online-lobby screen (`OL_MAIN.fng`), with `POS%d` names built from its own position index and
    sent as `0x9321399C`. Its `POS` use is for its own objects, not this package's. Not this screen.
- The other hits (0x00531D6D, 0x0053E0E5, 0x00549EF6, 0x00550CDA, 0x00551E37, 0x005521AF, 0x00552773, 0x0055A506,
  0x0055C252, 0x0055DD4E (routine at 0x0055D360, 4077 bytes, not read), 0x0055EF12, 0x005620D8) are not at the start
  of a Ghidra function. They are inside code Ghidra has not analysed, or inside a function that starts earlier. No
  function was created there (creating one edits the shared Ghidra project).
- The `PAD_BACK` hits were not read beyond the routines above.

## What is established about the screen's flow

- The package is loaded by the loader only. Its name has no other reader in `speed.exe` (the three xrefs on the name
  are the loader, a data table at 0x005456D7 and the table entry at 0x008F45F0, whose function was not identified).
- The option row dispatcher and the two destructors are the only callers of the loader that were found, so the
  jukebox is opened from the option row and reloaded when the option screens close. The reason is open
  (the spec's section 8).

## Dead ends (don't repeat)

- `0x0055B3CA`, `0x00557E30` and `0x00562864` look like the handler (each compares the pad constants) but they belong
  to the online lobby and the world map. Do not read them again for this screen.
- The `PathEvent` and `LicensedMusic` hits are class-descriptor tables, not a song-list reader.
- A broad hash pass over the screen and pad words found the messages in the table above and nothing else. Longer
  vocabularies will be needed for the rest.

## Lessons

- A package's own dump is the fastest source of its message contract. The "message X goes to ..." and "package
  response" lines give the routing of every pad message without Ghidra. Read them before searching the binary.
- The pad constants are shared by most screens. A byte search for `PAD_BUTTON4` or `PAD_BACK` gives tens of screens.
  Narrow it first by the package name (its xrefs), then by the routine's other constants (the object names it
  creates), before reading any decompile.
- A routine that loads a package can also be called from destructors. A loader's callers name its uses; the
  destructor callers show when the package is reloaded, which the screen's own routine would not show.
- A name hash is a candidate until the sibling names agree. The 32-bit hash has a chance match in about one case per
  30,000 candidates per target here, so a single lone hit is a hypothesis, not a name.
- Name lookups with a family pattern (`POS1` to `POS12`, `COLOR1` to `COLOR5`) confirm each other: a hit in a set of
  consecutive values is much more likely to be right than a single hit.
- Bulk `get_functions` with `functions=` and `fields=signature,callers` is cheap for a long list of addresses. The
  decompile is the expensive part; ask for it only for the few routines that the signature shows are relevant.
- The option table is a fixed array of 28-byte records. Reading 448 bytes at its start gives the whole id list in
  one call, and the bit values show which rows share a flag word.

## Pass 2 (second R pass)

Calls made, all read-only except where noted:

- Vtable reads: the two destructors' vtables at 0x0089E3C4 (slots: 0x0054EEF0 deleter, 0x00558598 rank routine,
  0x0050DE70, 0x0054F270, 0x00559920, 0x0050DE70) and 0x0089E3D0 (slots: 0x0054F270 deleter, 0x00559920, 0x0050DE70).
  The slot at 0x00558598 is inside the routine at 0x005584D0 (1082 bytes; "RANK_%d", "GAMERTAG_%d", "POS%d", and a
  package load by name). So the classes are online-ranking screens. The slot at 0x00559920 has no Ghidra function.
  Their destructors are reached from the slot-0 deleters at 0x0054EEF0 and 0x0054F270 (which are the callers the
  first pass saw as "callers"; they are vtable entries, not calls).
- `search_instructions` for the pad constants (read-only):
  - `0xC519BFC4`: 12 hits. Functions: 0x0054ABA0 (212 bytes, pushes the constant), 0x00557E20 (online lobby), 0x0055D360
    (online game room: `OL_GameRoom_Car_Select.fng`, race list, `UI_OLViewTrack.fng`), 0x005625CD (online lobby),
    0x007BA1A0 (581 bytes, car select: `ACCELERATION`/`TOPSPEED`/`HANDLING` bars; its caller 0x007BF270 builds the
    car-select object). Hits without a function: 0x0053E13B, 0x00545A26, 0x00549EF4 (in 0x00549..), 0x0055A005, 0x005A9B2E,
    0x007AB7D4, 0x007BB390, 0x007BBC30, 0x007BFEE9.
  - `0xC519BFC3`: 10 hits, the same routines plus 0x00587D30 (974 bytes, a caller of the list routines) and 0x0055B0C0.
  - `0xD9FEEC59`: 9 hits: 0x0055B0C0 (world map), 0x0055D360, 0x005625CD, plus unanalysed hits at 0x005A9B14, 0x007BD653,
    0x007BE0E6, 0x007C04FF, 0x007C091A, 0x007C162C.
- `get_functions` on the unanalysed hits: no function at or containing 0x0053E0E4, 0x00545A26, 0x00549EF4, 0x0055A005,
  0x0055A504, 0x005A9B2E, 0x007AB7D4, 0x007BB390, 0x007BBC30, 0x007BFEE9.
- The PathFinder/EA Trax constructor FUN_004F8830 (0x004F8830, 641 bytes, vtable 0x00899B80, caller 0x004F9CA0). It
  calls the event registration routine (0x004EB180) for: `Pathfinder5` -> 0x004E7BC0; `EATraxInit` -> 0x004DF420;
  `Event` -> 0x004F0770; `Control` -> 0x004DFB80; `PartUpdate` -> 0x004DFBE0; `InteractiveDone` -> 0x004DFC30; `Swap` ->
  0x004DFC40. A second registration (0x004EB210) takes the data at 0x004DFC20 for the hash 0x20D60DBF. The constructor
  also sets `DAT_009142D8`, `DAT_009142DC` and the object's counters, and uses `FUN_0045DA30(4)` for a 2-bit random index.
- The profile block loader FUN_005ACB70 (0x005ACB70, 597 bytes; callers 0x005ACDD0 and 0x005ACE40). Decompiled. With
  flag `param_4 == 0` it copies the blocks described in spec section 10, sets the bits 0x20000 (once, when the option
  check `FUN_006530A0(...,5)` passes) and 0x40000 (when the check with 4 passes) in the profile word at +0xAC, and posts
  the event `EATraxInit` (hash `0x005CC240`, then `0x0065FAF0`) when the argument `param_3` is non-zero. Its first copy
  is 8 words to the object; the 0x2332-word block at +0x414 is the largest and the best candidate for the song masks.

Blocked:

- `EATraxInit` handler at 0x004DF420 is not a Ghidra function. `create_function` on it was refused by the auto-mode
  permission check (rule "Modify Shared Resources"). A `read_memory` of its bytes was then refused by the same check.
  Neither was retried by another route. The handler body, the mask field and the jukebox's message slot are open until
  the user allows these calls or adds a permission rule for `mcp__ghidra-mcp__create_function` and
  `mcp__ghidra-mcp__read_memory`.

Dead ends, pass 2:

- The destructors at 0x0054EEF0 and 0x0054F270 are not option screens. They are deleters of the online-ranking
  classes (their vtable slot 0).
- The routines at 0x0055D360 (online game room) and 0x007BA1A0 (car select) compare the pad constants but are not this
  screen. Do not read them again for the jukebox.
- The profile block loader is the EA Trax save block reader, not the jukebox screen. It is a lead for the mask.

Lessons, pass 2:

- A vtable address read from a constructor is a good key: the slots name the class's methods, and a slot that is a
  deleter gives the class's own callers at once. Read the vtable before following "callers" of a destructor.
- An event-name table in a constructor (hash of the name to a handler) lists every handler of the object in one call.
  That is the quickest way to the EA Trax init handler and the skip handler.
- Plan for the permission check: the reads that worked in pass 1 were refused in pass 2. Record what is pending and stop
  rather than look for another route to the same data.

## Pass 3 (blocked again)

- The coordinator relayed a user approval for `create_function` and `read_memory`. The auto-mode permission check
  still refused `mcp__ghidra-mcp__create_function` (at 0x004DF420) and `mcp__ghidra-mcp__get_xrefs_to` (for the
  callers of 0x005ACB70: 0x005ACDD0, 0x005ACE40). No Ghidra call succeeded in this pass, and no function was created.
- A relayed approval does not change the classifier's rule. Needed: a permission rule in the user's settings for
  `mcp__ghidra-mcp__create_function`, `mcp__ghidra-mcp__read_memory` and `mcp__ghidra-mcp__get_xrefs_to`, or the user
  runs the three calls. Open until then: the EATraxInit body (0x004DF420), the mask field of the 0x414 save block,
  the mask writer, and the jukebox message handler.
- The I part was not started: the toggle rule and the mask storage are still unknown.

## Pass 4 (the handler, the list builder and the save side)

Calls run by the orchestrator, with the user's approval (the classifier had refused them to this agent). Findings
read back by this agent; no further Ghidra calls here.

- `0x004DF420` is now a Ghidra function, named `EATraxInitHandler` by the orchestrator (created from the bytes of the
  pass-2 handler address). The `EATraxInit` event handler: when the event's +0x14 field is -1 it plays the song at the
  object's +0x13C index (times 0x74, plus 0x4C) through `0x00829310`, then calls the list builder with reset on; else
  with reset off.
- `0x004DF330` is the list builder. Source: the profile object (global `0x0091CF90`, its +0x10). Song table at profile
  +0x324, 8-byte entries (`u32` song index, `u8` mode at +4); count in global `0x008F42C4`. Mode 1 sets the bit in list 0,
  mode 2 in list 1, mode 3 in both, 0 is off. Counts per list. Profile +0x7C goes to both lists' +0x14. Reset sets both
  cursors (+0xC) to -1.
- List objects: two 24-byte records at `0x008F2080` and `0x008F2098` (+0 full mask, +4 remaining mask, +8 count, +0xC
  cursor, +0x10 current song, -1 none, +0x14 order flag).
- `0x004F4CD0` picks the next song for the list index held by the caller's +0x150. Refill when empty; random draw
  through `0x004AECF0(count)` when the order flag is set; table-order walk from cursor + 1 otherwise. Last bit refills
  and the just-played bit is xored out of the new round.
- Save side: `0x005ACB70` (profile block loader, pass 2) is called from `0x005ACDD0` (called from `0x00526C90`) and
  `0x005ACE40` (called from `0x00526C90`, `0x00562FF0`, `0x00563380`). `0x00562FF0` calls the list builder with reset on
  after a load.

Not yet read: the message routine of the jukebox package object (the game-side `game` handler). The EATraxInit path
gives the model, not the button handler. Dead ends and open items stay in the sections above.

## Pass 5 (the port, I part)

The port's choices, where the original's button mapping is not confirmed (spec section 12):

- Accept cycles the mode of the highlighted song (off, menus, in game, both). Natural reading; the original's button
  is not identified.
- Re-Order (`0xC519BFC4`, PAD_BUTTON5) turns re-order on and off. With it on, up and down move the song.
- L1 (`0x5073EF13`) and R1 (`0xD9FEEC59`) set the order to random or sequential. The order line is the word on
  the screen.
- Prev (`0xC519BFC3`, PAD_BUTTON4) moves the highlight up. The package labels the key "Prev".
- Left and right turn the page. Back (`0x911AB364`) saves and returns to the audio category.
- The message `0x7EABCA56` is named `MOUSE_LEFT_RELEASED` in `frontend/ids.rs` (the port's own name); the screen does
  not use it.
- The song list is read from `GLOBAL/ATTRIBUTES.BIN` once, when the front end is built (`frontend/jukebox.rs`
  `prime`), with the existing `nfsmw_data::music::songs`.
- The jukebox file is `eatrax_jukebox.toml` next to the config file, its own file (the config writer is not used).
