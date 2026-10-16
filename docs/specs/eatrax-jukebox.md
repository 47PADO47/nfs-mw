# EA Trax jukebox (`EA_Trax_Jukebox.fng`)

The EA Trax screen of the front end: a list of the licensed songs where the player picks which songs the radio
may play in the menus and in the game. Companion of [music-graph.md](music-graph.md) (section 6, the play lists and
the 28-bit masks), [custom-music.md](custom-music.md) (its section 8 names this screen as not done) and
[frontend-menus.md](frontend-menus.md) (section 2.2, the option list where the entry sits).

- **Status:** not implemented. The entry `Id::EaTrax` in `crates/nfsmw/src/frontend/icon_items.rs` stays greyed.
  This spec is the R part only (see section 9): the package contract is known, the game-side handler and the
  place where the masks are written are not yet found.
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

Not named (three hash passes, about 140,000 candidate names, found no match): `0x7EABCA56` (the button activate
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

- Two routines of the option screens (their vtables are at 0x0089E3C4 and 0x0089E3D0). Both call the loader and
  then release their own members, so they are destructor-like. The loader is therefore run when an option screen is
  left as well as when the package is opened [unconfirmed: the reason is not known].
- The option row dispatcher (the routine that handles the rows of the option screens). It reads each row's option
  id and value. The option with id `0x27` calls the loader when its value is 1.0. The option with id `0x28` sets a
  flag to 1 or 0 by the same test. The option table at 0x008F3428 holds 28-byte records (seven `u32`: the id, a
  bit value, a pointer and zeros) for the ids `0x19` to `0x2C` [verified by a read of the table].
- The option id `0x27` is the row "EA Trax mode" of the audio options (its label hash `0xDCFB6B36`, as in
  [frontend-menus.md](frontend-menus.md) section 2.2) [unconfirmed: the id-to-label link is inferred from the order
  of the ids and the row names; check it in the option table's labels].

So the audio option row (id `0x27`, probably "EA Trax mode") loads the jukebox when it is set to 1. The loader is also
reached from the two option-screen routines of this section. No call from the main menu or the pause menu was found in
this pass [unconfirmed]. The entry `Id::EaTrax` in the port's option list is still greyed.

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

## Dead ends (don't repeat)

- The routines that compare the pad constants `0x911AB364`, `0xC519BFC3` and `0xC519BFC4`: the hits are the
  online-lobby screens and the world map's controller. Each was read and none is this screen's.
- The music attribute names (`SongName`, `Artist`, `Album`, `DefPlay`, `PathEvent`, `LicensedMusic`): the
  references found are class-descriptor tables of the audio system, and one attribute reader that was not read
  further. None is a reader of the song list for this screen.
- Name hashes: about 140,000 candidate names in three passes (screen, pad, mouse, list and sound words) named about
  50 of the ids tried. The rest are not in the vocabulary.

## Next leads

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
