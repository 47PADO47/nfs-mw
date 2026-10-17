# EA Trax jukebox (`EA_Trax_Jukebox.fng`)

The EA Trax screen of the front end: a list of the licensed songs where the player picks which songs the radio
may play in the menus and in the game. Companion of [music-graph.md](music-graph.md) (section 6, the play lists and
the 28-bit masks), [custom-music.md](custom-music.md) (its section 8 names this screen as not done) and
[frontend-menus.md](frontend-menus.md) (section 2.2, the option list where the entry sits).

- **Status:** implemented in the port (pass 5, the I part). The entry `Id::EaTrax` in the audio category opens the
  jukebox; its choices are in `eatrax_jukebox.toml` next to the config file. Section 12 lists the port's decisions.
  Sections 2 to 11 are the R part's findings.
- **Sources read:**
  - The install's package `EA_Trax_Jukebox.fng` (`FRONTEND/FrontB.lzc`), read with `nfsmw dump-screen`. Its objects,
    texts, scripts and message responses are facts of the install **[verified]**.
  - `speed.exe` (the original PC build), read in Ghidra: the strings, the xrefs of the package name, the
    loader and its callers, the option table of the option screens, and the constants of the pad messages.
    Read for behaviour only; no code of it is in the repo. Provenance: [provenance/eatrax-jukebox.md](../provenance/eatrax-jukebox.md).
  - The repository's own specs and provenance: [music-graph.md](music-graph.md), [frontend-menus.md](frontend-menus.md),
    [confirmation-dialogs.md](confirmation-dialogs.md), [../provenance/music-graph.md](../provenance/music-graph.md) (the song table).
  - No decompiled source of the CC0 decomp was read for this pass.
- **Evidence tags** as in the [docs README](../README.md#evidence-tags): **[verified]** checked in the install or by
  a hash match; **[decomp]** read in the binary, not yet tested in game; **[inferred]** a reading of the layout or
  the calls, not a read of the handler; **[unconfirmed]** a guess that needs the handler or a run.

## 1. What the screen does for the player [inferred]

The screen lists the EA Trax songs. Each row shows the artist, the title and the album of one song. The player
can pick which songs play in two lists: the menu list (front end) and the in-game list. The song list and the
play rules are in [music-graph.md](music-graph.md) section 6: 26 songs, a `DefPlay` value per song (`FE` for the
menus, `IG` in game, `AL` both), and one bit per song in each mask. The screen is the only place that sets the
bits; the defaults of the install apply until the player changes them.

Several things on the screen are not yet tied to a rule: the row toggle, the mode label (see section 4), and the
reorder mode (section 4). Section 8 lists what is missing.

## 2. The package [verified]

`EA_Trax_Jukebox.fng` is version `0x20000`, with 22 resources (fonts `FONT_MW_BODY.ffn` and `FONT_MW_TITLE.ffn`,
and the images: the button hints, the keyboard key pictures, the page arrows, the stripes and the grit textures)
and **120 objects**. Its objects, top to bottom:

| Area | Objects | Content |
|---|---|---|
| Header | `String` "EA trax" (two copies, title font) | the screen title |
| Song list | 4 rows, one group of text objects per row | per row: "Trax Artist N", "Trax Title N", "Trax Album N" (placeholder texts, replaced at run time) |
| Row numbers | 4 `String` objects "01" to "04" (title font) | at the left of the rows |
| Row overlays | 4 `Image` objects, 560 by 70 (`BasePoly.tga`), one per row | they receive the mouse messages (section 3.3) |
| Mode line | `String` "Squential" with a stripe and arrow images `L1.tga` and `R1.tga` on either side | the play-order line (section 4) |
| Hint strip | Triangle "Back", Square "Preview", Circle "Re-order", Cross "Accept" | pad hints |
| Keyboard hints | "Re-Order" (key 1), "Prev" (key 2), "Accepteren" (Enter), "Back" (Esc), each a group with a key picture | PC key hints |
| Reorder cursor | 4 `Crosshair_Corner.tga` corner images on the list, and one `REORDER_ARROW.tga` in the list group | shown in the reorder mode |
| Track arrows | 2 `ARROW_SKINNIER.tga` images in the track group, left and right | move across the rows |
| Other | `LEAVE` (`String`), "Racing" (four copies) and "Region" (one) | not wired to any row in this dump (section 8) |

Facts about the list:

- **Four rows at a time.** The row pitch is 80 units. Rows 1 to 4 sit at the title, artist and album heights
  of the package, and the list shows 4 songs at once. 26 songs make 7 pages of 4 [inferred, from the row count].
- **The fourth title is a typo in the package.** The fourth row has no "Title 4" string. The text at its title height
  reads "Trax Album 4", and a second "Trax Album 4" sits at its album height. The fourth title is a placeholder
  either way, so the port must not read that text as the song name.
- **Texts are placeholders.** Every "Trax ..." string is replaced by the game at run time; the install does not
  store song names on this screen.

## 3. Messages

### 3.1 Pad messages sent to the package and the game's responses [verified]

The package receives the pad messages and, for each, sends messages to the game (`game`), to its own objects
(`feng`), and to the sound system (`sound`). The `dump-screen` responses, with the names found in section 3.4:

| Pad message (hash) | Name | Sent to the game | Also |
|---|---|---|---|
| `0x72619778` | PAD_UP | `0x72619778` | `feng` `0x00000704` (UP) to its objects |
| `0x911C0A4B` | PAD_DOWN | `0x911C0A4B` | `feng` `0x00148CD7` (DOWN) |
| `0x406415E3` | PAD_ACCEPT | `0x406415E3` | `feng` `0xADE14EC3`, `feng` `0x97BDB465`, `sound` |
| `0x911AB364` | PAD_BACK | `0x911AB364` | `sound` `0x480C9A58` |
| `0xC519BFC3` | PAD_BUTTON4 | `0xC519BFC3` | `sound` `0x610FB237` |
| `0xC519BFC4` | PAD_BUTTON5 | `0xC519BFC4` | `feng` `0xE77A5CAF` (REORDER_CURSOR), `sound` `0x610FB238` |
| `0x9120409E` | PAD_LEFT | `0x9120409E` | `feng` `0x0018C32A` (LEFT), `sound` |
| `0x5073EF13` | PAD_LTRIGGER (L1) | `0x5073EF13` | `sound` |
| `0xB5971BF1` | PAD_RIGHT | `0xB5971BF1` | `feng` `0x039FEFFD` (RIGHT), `sound` |
| `0xD9FEEC59` | PAD_RTRIGGER (R1) | `0xD9FEEC59` | `sound` |

The pad message ids are the hashes of the upper-cased names (`libs/blackbox-feng/src/ids.rs`). `0xC519BFC3` and
`0xC519BFC4` are `PAD_BUTTON4` and `PAD_BUTTON5` (the ten `PAD_BUTTON` ids are consecutive from `0xC519BFBF`).

### 3.2 Button actions [verified; the meaning is inferred]

The buttons react to the message `0x7EABCA56` (name not found, section 3.4). The `game` message each one sends is:

| Button (group) | Hint | Sends to the game | Notes |
|---|---|---|---|
| Re-Order (key 1) | Circle, "Re-order" | `0xC519BFC4` (PAD_BUTTON5) | also `REORDER_CURSOR` on the screen |
| Prev (key 2) | "Prev" | `0xC519BFC3` (PAD_BUTTON4) | the label says "previous"; the game's meaning is not fixed (section 8) |
| Accept (Enter) | Cross, "Accept" | none on the button; the pad message `0x406415E3` goes to the game | the screen's `Accepteren` text |
| Back (Esc) | Triangle, "Back" | `0x911AB364` (PAD_BACK) | |
| L1 arrow | left of "Squential" | `0x5073EF13` (PAD_LTRIGGER) | [inferred] steps the mode line back |
| R1 arrow | right of "Squential" | `0xD9FEEC59` (PAD_RTRIGGER) | [inferred] steps the mode line on |
| Left arrow | in the track group | `0x9120409E` (PAD_LEFT) | [inferred] moves across the rows |
| Right arrow | in the track group | `0xB5971BF1` (PAD_RIGHT) | [inferred] moves across the rows |
| Square "Preview" | hint only | none: no button group found in the dump | [unconfirmed] what the Square pad button does |

The Square and Circle hints are drawn with the "Preview" and "Re-order" texts. No button group behind the Square
hint was found in the dump. The keyboard keys 1 and 2 are the Re-Order and Prev groups, although the game messages
they send carry the indices 5 and 4. Whether the port should map key 1 to `PAD_BUTTON5` is an open point (section 8).

### 3.3 Row messages [verified]

- The four row overlays receive `0xB30D0683` (MOUSE_OVER) and send `game` `0xB30D0683` to the game. The game
  can therefore see the pointer over a row; which row it was is carried by the message's object, not by the message
  id [inferred from the object routing].
- The four button groups (Re-Order, Prev, Accept, Back) receive `0xB30793C1` (MOUSE_EXIT) and tell their stripe
  to change state through an unnamed feng message `0xB841EA36` [verified by the object scripts].
- The row texts run their HIGHLIGHT script when the unnamed message `0x33BC8B0B` arrives, and their UNHIGHLIGHT
  script when `0x3C2DE6D8` arrives [verified by the object scripts; the two messages are unnamed].
- The list group receives twelve position messages `POS1` to `POS12` (section 3.4). Each one starts a script that
  moves the text objects of one row or one slot. The package does not fill the texts by itself; the game is
  expected to send the song data through these positions [inferred].

### 3.4 Names of the messages [verified by hash]

Names are found by the FEng hash of the upper-cased name (`fe_hash_upper`). A name is given only when the hash of
the candidate matches the id in the package exactly; the rest stay unnamed.

| Id | Name | Used as |
|---|---|---|
| `0x249DB7B7` / `0x7AB5521A` | HIGHLIGHT / UNHIGHLIGHT | script names of the row texts |
| `0xE647B375` | PRESSED | script of the track arrows |
| `0x4F90CF9B` / `0xFB12D252` | ACTIVE / INACTIVE | scripts of the track arrows |
| `0x587C018B` | LEAVE_SCREEN | sent by the game to close the screen; the package's LEAVE text reacts |
| `0x35F8620B` | INIT_COMPLETE | sent once the package is loaded |
| `0xE1FDE1D1` | EXIT_COMPLETE | sent when the package leaves |
| `0x33113AC` | LEAVE | the Leave text |
| `0xB30D0683` / `0xB30793C1` | MOUSE_OVER / MOUSE_EXIT | row overlays |
| `0x550FFCCF` to `0x550FFCD3` | COLOR1 to COLOR5 | colour scripts of the Triangle hint bar (the "Back" group) |
| `0xE77A5CAF` | REORDER_CURSOR | the reorder cursor |
| `0x001B20C2` to `0x001B20CA`, `0x037F3932` to `0x037F3934` | POS1 to POS12 | slot messages of the list |
| `0x001744B3` / `0x0016A259` | INIT / HIDE | the initial and hidden scripts |
| `0x704`, `0x148CD7`, `0x0018C32A`, `0x039FEFFD` | UP, DOWN, LEFT, RIGHT | the feng messages of the pad |
| `0x406415E3`, `0x911AB364`, `0x9120409E`, `0xB5971BF1`, `0x5073EF13`, `0xD9FEEC59`, `0x72619778`, `0x911C0A4B`, `0xC519BFC3`, `0xC519BFC4` | PAD_ACCEPT, PAD_BACK, PAD_LEFT, PAD_RIGHT, PAD_LTRIGGER, PAD_RTRIGGER, PAD_UP, PAD_DOWN, PAD_BUTTON4, PAD_BUTTON5 | pad messages (section 3.1) |

Not named by the three hash passes (about 140,000 candidate names; the port names `0x7EABCA56` MOUSE_LEFT_RELEASED in its ids): `0x7EABCA56` is listed here only as the package's button message (the button activate
message), `0x97BDB465` and `0xF459B307` (the list's update and release messages), `0xCFEBE0DF` and `0xB841EA36`
(state messages of the rows), `0xADE14EC3`, `0x33BC8B0B`, `0x3C2DE6D8`, `0x1A1533D5`, `0x34737EA6` to `0x34737EA9`,
`0xB919404C`, `0x1C3553EF`, `0x3E75F58D`, `0x6B945D70`, `0xC3D2AACE`, `0xBAF366F1`, `0xCCD64A6E`, `0x84378BEF`,
`0x4B680587`, `0x7958EA7F`, `0x672DFAFD`, `0x8CB81F09`, `0x033C9547`, `0x83562D55`, `0x906782D5`, `0xCB1AA27F`, and
the sound events `0x610FB237`, `0x610FB238`, `0x480C9A58`, `0x4A805994`. A name for one of them needs the hash of
a longer name, or the game's own table of names. Do not invent names for them.

## 4. Play order and the reorder mode

- The mode line reads "Squential" between the L1 and R1 arrows. The spelling is the package's. The line is the
  play-order setting, and the two arrows step it [inferred from the layout]. The music graph's default is the
  ordered mode (`PBMode` 0, music-graph section 6), so "Squential" is the default label [inferred].
- **Reorder** (Re-Order key, PAD_BUTTON5): the package passes the pad message on to the game and sends
  `REORDER_CURSOR` to its objects; the reorder arrow of the list group reacts to it [verified: the message routing].
  The reorder is for moving a row up or down. This is about the song order of the list, not the
  masks. Whether it changes the play order of the radio is not known [unconfirmed].

## 5. How the screen is opened and loaded [decomp]

The package name `EA_Trax_Jukebox.fng` is loaded by one routine (called the loader below). It runs as follows:

1. It does nothing when the front-end screen object exists and its mode is 3, 4 or 5 (the modes where the jukebox
   does not run, as in [fe-shadows.md](fe-shadows.md)). A missing screen object does not stop it.
2. It loads the package by name, through the front end's package loader (one of two loader paths, chosen by a
   global). A failed load is ignored.
3. When the load succeeded and the game-flow global is 3, it posts the `Pathfinder5` event with the id -1 to the
   PathFinder track. That is the skip of the front-end song, as in [music-graph.md](music-graph.md) section 7 [decomp].

What calls the loader [decomp]:

- Two destructors (vtables at 0x0089E3C4 and 0x0089E3D0). Pass 2 corrected the screen kind: their vtables hold the
  online rank-list routine at 0x005584D0 ("RANK_%d", "GAMERTAG_%d", "POS%d"), so these are online-ranking screen
  classes, not option screens. Each calls the loader before its base cleanup. So the jukebox package is also loaded
  when those online screens are destroyed [decomp; the reason is not known, unconfirmed].
- The option row dispatcher (the routine that handles the rows of the option screens). It reads each row's option
  id and value. The option with id `0x27` calls the loader when its value is 1.0. The option with id `0x28` sets a
  flag to 1 or 0 by the same test. The option table at 0x008F3428 holds 28-byte records (seven `u32`: the id, a
  bit value, a pointer and zeros) for the ids `0x19` to `0x2C` [verified by a read of the table].
- The option id `0x27` is the row "EA Trax mode" of the audio options (its label hash `0xDCFB6B36`, as in
  [frontend-menus.md](frontend-menus.md) section 2.2) [unconfirmed: the id-to-label link is inferred from the order
  of the ids and the row names; check it in the option table's labels].

So the audio option row (id `0x27`, probably "EA Trax mode") loads the jukebox when it is set to 1. The loader is also
reached from the two online-ranking destructors of this section. No call from the main menu or the pause menu was
found [unconfirmed]. The entry `Id::EaTrax` in the port's option list is still greyed.

## 6. The rows: data and fill

- The rows show songs of the list (music-graph section 6, and the song table in
  [../provenance/music-graph.md](../provenance/music-graph.md)). The 26 songs have an artist, a title and an album
  each, from the music collections' `Artist`, `SongName` and `Album` fields.
- The menu list uses the songs with `DefPlay` `FE` or `AL` (10 songs in the install: 8 `FE` and 2 `AL`); the in-game
  list uses `IG` or `AL` (18 songs: 16 `IG` and 2 `AL`) [verified against the song table].
- How the game fills the four rows (which song goes in which row, and how the list scrolls with L1, R1 and the
  arrows) is not found. The positions `POS1` to `POS12` are the slots the game would use (section 3.3) [unconfirmed].

## 7. Rules for an implementation

These are the rules the port can state now. Each one has an evidence tag.

1. Two masks of 28 bits, one per list, as in [music-graph.md](music-graph.md) section 6. A bit is set for each song
   that may play in that list [inferred from the play-list rules].
2. Each row shows one song of the list, from the first row of the page; four rows per page; the page moves with
   the arrows and L1 and R1 [inferred].
3. Accept and Back: the package sends the pad message to the game in both cases (section 3.1). What the game does with
   them, including whether the masks are kept, is the open handler of section 8 [unconfirmed].
4. The masks are saved where the radio's settings are kept. The place the game writes them to is **not found**. Do
   not persist anything on the screen until it is.
5. Custom tracks (`audio/radio/custom/`) do not appear on this screen: the package has four fixed rows and no spare
   row for them (the status table of `custom-music.md`) [verified by the row count].

## 8. Open points

- **The game-side handler.** No function was found that takes the package's `game` messages (`0x911AB364`,
  `0xC519BFC3`, `0xC519BFC4`, `0x5073EF13`, `0xB5971BF1`, `0xD9FEEC59`, and the others of section 3.1). The routines
  that handle the same pad messages for other screens were read and are not this screen's (the online dialogs,
  the world map). Without this routine the meaning of each button is not confirmed.
- **The mask write.** The place where the song masks are stored is not found. The `TraxMask` and `PlayBits` of
  the spec are the names of this rewrite's own model; the game's names are not known.
- **The key mapping.** Key 1 is `PAD_BUTTON5` and key 2 is `PAD_BUTTON4` in the package, which reads as a swap
  of the hints. Check it in the game before the port maps keys.
- **The "Prev" label.** The button labelled "Prev" sends `PAD_BUTTON4`; its meaning as previous page or previous
  song is not known.
- **Racing and Region.** The strings "Racing" (four) and "Region" are in the package but not wired to a row in the
  dump. They may be placeholders of another screen [unconfirmed].
- **The option id `0x27`.** The label link (section 5) is inferred.
- **The loader from the destructors.** The reason the option screens' destructors call the loader is open (section 5).

## 9. What the R part gives to the I part

The I part (the screen in the port) would need: the handler of the `game` messages, the mask storage, and the
row fill. None of the three is found, so the I part is not started. The port keeps `Id::EaTrax` greyed.
The package contract of sections 2 to 4 is enough to write a screen that shows the rows, the hints and the
pad mapping, but not the toggle rule that the spec's section 7 leaves open.

## 10. Pass 2: the handler and the save block [decomp, partial]

- **Blocker 1 (the game-message handler): still open.** The jukebox object's handler slot was not found. Checks:
  - The two destructors of section 5 are online-ranking classes (their vtable slot 1 is the rank-list routine).
  - The search for the PAD_BUTTON5 constant (`0xC519BFC4`) gives 12 instructions; the functions among them are the
    online lobby, the world map, the online game room (its car select and race list), and the car-select screen
    (`ACCELERATION`, `TOPSPEED`, `HANDLING` bars). None is this screen.
  - The PAD_BUTTON4 search (`0xC519BFC3`) and the RIGHT trigger search (`0xD9FEEC59`) give the same set plus
    unanalysed code near 0x007B.... The unanalysed hits do not start a Ghidra function, so their owner is unknown.
- **The PathFinder and EA Trax object.** The constructor at 0x004F8830 (vtable 0x00899B80) registers its event
  handlers by the hash of the event name: `Pathfinder5` goes to 0x004E7BC0, `EATraxInit` to 0x004DF420, `Event` to
  0x004F0770, `Control` to 0x004DFB80, `PartUpdate` to 0x004DFBE0, `InteractiveDone` to 0x004DFC30 and `Swap` to
  0x004DFC40. The hash 0x20D60DBF is registered with the data at 0x004DFC20, not a handler label.
- **The EATraxInit handler (0x004DF420)** is the target of the `EATraxInit` event that the profile loader posts. It is
  not a Ghidra function yet; the body was not read.
- **The profile block loader (0x005ACB70)** reads the EA Trax save data from a buffer. It copies 8 words into the
  object, 0x3C words to +0x324, 0x21 words to +0x24, 0x2332 words to +0x414, one byte to +0x90DC and 0x2F6 words to
  +0x90E0, and it fills 18 entries of a table in the game-state object. It sets the flag bits 0x20000 and 0x40000 in
  the profile word at +0xAC, and posts `EATraxInit` when its flag argument is set. The song mask is probably in the
  +0x414 or +0x90E0 block; the field is not identified. Its callers are 0x005ACDD0 and 0x005ACE40 (not read).
- **Blocker 2 (the mask writer): still open.** The mask field is not yet pinned to a byte of the save block, so the
  writer is not known.
- **Access stopped.** In this pass the Ghidra calls `create_function` and `read_memory` were refused by the permission
  check ("Modify Shared Resources"). No further Ghidra calls were made, and no function was created. The EATraxInit
  body (0x004DF420), the mask field and the handler slot need those calls or a permission rule for them.

## 11. Pass 3: the model, from the EATraxInit path [decomp, orchestrator-run calls]

This section replaces the open points of section 8 that the handler and the mask writer answered.

- **The handler (0x004DF420, `EATraxInit`).** When the event's field at +0x14 is -1, it plays the song held in the
  EA Trax object at +0x13C (an index times 0x74, plus 0x4C) through 0x00829310, then rebuilds the play lists with
  reset on. Otherwise it rebuilds them with reset off.
- **The list builder (0x004DF330).** Its source is the profile object (the global at 0x0091CF90, whose +0x10 is the
  profile). The profile holds a song table at +0x324: 8-byte entries, each a `u32` song index at +0 and a `u8` mode at
  +4. The count is the global at 0x008F42C4. The modes are:
  - 0: the song is off;
  - 1: list 0 (the front-end list, the menus) gets the song's bit;
  - 2: list 1 (the in-game list) gets it;
  - 3: both lists get it.
  Each list keeps its count. The profile byte at +0x7C is copied into both lists' order flag. With reset on, both
  lists' cursors (+0xC) are set to -1.
- **The list objects.** Two 24-byte records at 0x008F2080 and 0x008F2098: +0 the full mask (low 28 bits), +4 the
  remaining mask, +8 the song count, +0xC the cursor, +0x10 the current song (-1 for none), +0x14 the order flag.
- **Pick the next song (0x004F4CD0)**, with the caller's +0x150 naming the active list. When the remaining mask is
  empty it refills from the full mask. With the order flag set it draws a random remaining song (the generator at
  0x004AECF0 takes the count) and clears its bit. With the flag clear it walks the profile table in table order from
  the cursor + 1 and takes the first entry whose bit is still in the remaining mask. So "sequential" means the profile
  table's order, and re-ordering the table changes the play order. When the last bit goes, the mask refills, and the
  just-played bit is excluded from the first pick of the new round (an xor).
- **Save.** The song table and the order flag sit in the profile save block. The block loader (0x005ACB70) is
  called from 0x005ACDD0 and from 0x005ACE40. Those are called from 0x00526C90, and 0x005ACE40 also from 0x00562FF0
  and 0x00563380 (front-end flow and save screens). 0x00562FF0 rebuilds the lists (reset on) after a load.

So the model is: per song, one of four modes (off, menu list, in-game list, both); a table order (re-order changes
it); and one order flag (sequential or random), which the "Squential" line with L1 and R1 sets. What is still
[unconfirmed]: which key cycles the mode, and what re-order does exactly; the port uses the natural reading (see the
port's record of the choice, `docs/provenance/eatrax-jukebox.md`, pass 4).

## 12. The port (I part)

- **Files.** `settings/eatrax.rs` (the model and its file), `audio/radio/playlist.rs` (`Playlist::with_order`: the
  ordered play follows the table), `audio/radio/mod.rs` (`Radio::set_jukebox`), `audio/radio/glue.rs` (loads the
  jukebox with the radio), `frontend/jukebox.rs` (the screen), `frontend/factory.rs`, `frontend/ids.rs`,
  `frontend/icon_menu.rs` (the icon opens the screen), `frontend/icon_items.rs` (the entry is no longer greyed),
  `frontend/plugin.rs` (reads the song list once, `jukebox::prime`).
- **Model.** Each song has a mode: off, menus (list 0), in game (list 1), or both. The table is in play order. The order
  flag is random or sequential. The masks are the lists' 28-bit masks; the order of each list is the table order.
- **Defaults.** Without the file, each song has the mode its `DefPlay` gives (`FE`: menus, `IG`: in game, `AL`: both,
  other: off), in number order, sequential. So an install with no file plays as before.
- **File.** `eatrax_jukebox.toml`: `random = false` and `songs = [[song, mode], ...]` in table order (mode 0 off, 1
  menus, 2 in game, 3 both). A bad file is the defaults; out-of-range or repeated songs are dropped; songs the file
  leaves out follow in default order.
- **Screen.** Four rows per page with the artist, the title (with the mode in brackets) and the album; the number of
  the row is the table position. Up and down move the highlight. Left and right turn the page. Accept cycles the mode
  of the highlighted song. Re-Order (PAD_BUTTON5) turns re-order on: then up and down move the song, and the highlight
  follows it; off again, the order is kept. L1 and R1 set the order line to Random or Sequential. Prev (PAD_BUTTON4)
  moves the highlight up. Back saves and returns to the audio category. Every change is saved at once.
- **Radio.** The radio reads the file when it loads. The jukebox's order flag wins over the config's `shuffle` for
  the EA Trax songs (the order is set after the custom sources). Custom tracks are not on the screen.
- **Limits.** A change made after the radio has loaded (a drive started earlier in the session) takes effect when
  the radio next loads, not at once. The meaning of the Prev key and of the re-order key is the port's reading of
  section 8 [ours]. Nothing has been listened to.

## Dead ends (don't repeat)

- The routines that compare the pad constants `0x911AB364`, `0xC519BFC3` and `0xC519BFC4`: the hits are the
  online-lobby screens and the world map's controller. Each was read and none is this screen's.
- The music attribute names (`SongName`, `Artist`, `Album`, `DefPlay`, `PathEvent`, `LicensedMusic`): the
  references found are class-descriptor tables of the audio system, and one attribute reader that was not read
  further. None is a reader of the song list for this screen.
- Name hashes: about 140,000 candidate names in three passes (screen, pad, mouse, list and sound words) named about
  50 of the ids tried. The rest are not in the vocabulary.

## Next leads

0. Pass 2 is blocked by the permission check (section 10). With `create_function` and `read_memory` allowed: read the
   EATraxInit handler at 0x004DF420 (the event body: play-list setup and the mask), then the profile block of
   0x005ACB70 for the writer of the mask field, and the callers 0x005ACDD0 and 0x005ACE40 for the save side.
1. Find the `game` message handler of this package. Start from the screen object created by the loader (the
   package's manager, not the option screens), then read the class's message routine for `0x0C407210` (BUTTON_PRESSED)
   and `0x7EABCA56`.
2. Read the routine that the option row dispatcher's callers lead to (the routine that calls the option row
   dispatcher), to see whether the audio screen opens the package on its own.
3. Map option ids `0x19` to `0x2C` to their labels. The table at 0x008F3428 gives the ids and the flag bits; the
   labels are in the option screens' tables.
4. Identify the two destructors that call the loader: which screen they close and why they reload the package.
5. Find the writer of the song masks. Start from the radio's play list at its construction, and from the save data
   that holds the EA Trax option (the "EA Trax" item is in the option list).
6. Name the unnamed ids of section 3.4 with the game's own name table, or with a longer vocabulary (for example
   names with the pad, the song index and the list's scroll words).
7. In game, check the key mapping (section 8) and the meaning of "Squential", L1 and R1, with the package running
   on the original.
