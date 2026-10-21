# Save slots and the alias manager

Status: the alias manager is built as an icon row of Create, Load, Save and Delete: see [career-parity-menus.md](career-parity-menus.md). The research below is kept; its section 4 questions are answered there. Earlier status: research done, screen logic not built. The storage side exists: `libs/save-store` (slots as `<name>.sav`, an
atomic write, an alias index in `aliases.toml`). Nothing calls it yet. Read from the install with `nfsmw dump-screen`
and `nfsmw strings`. Tags follow [frontend-menus.md](frontend-menus.md). Addresses are the install's own name hashes.

## 1. What the original has [verified]

The original calls a save an **alias**, and the memory-card screen the **alias manager**:

- `MC_ProfileManager.fng` (FRONTEND, title "alias manager", label `0x1B76F2CE`) is the screen. It has a row of icon
  buttons, an Accept and a Back.
- `MC_DeleteProfile.fng` is the delete confirmation.
- The Back label is `0x61D596D7`, the Accept label `0x2B65A216`.

## 2. The strings the screens use [verified, `nfsmw strings`]

| label | English text | use |
|---|---|---|
| `0x253FF67A` | "%s has been loaded successfully." | the load-complete alert (the second screenshot) |
| `0x2D9830FE` | "Loading the alias %s. Please do not turn off your Xbox." | while loading |
| `0x39B3CCBA` | "Are you sure you want to save the alias named %s?" | save confirmation |
| `0x83F4BB3E` | "Do you want to save the alias %s?" | save question on leaving |
| `0x87C7577E` | "Loading a new alias will result in the loss of any unsaved changes to the current alias. Are you sure you want to load a new alias?" | load confirmation |
| `0x7209349F` | "Creating a new alias will result in any unsaved changes ... Are you sure you want to create a new alias?" | create confirmation |
| `0x7D90C187` | "EA Login cannot be saved without an alias. Would you like to create an alias?" | no alias yet |
| `0x4505588D` | "This alias is corrupt and cannot be used." | a corrupt slot |
| `0x821E4E41` | "Load failed. Save game data ... appears to be corrupt." | a failed load |
| `0x4F68196E` | "Are you sure you want to remove this car? ..." | removing a car from an alias |
| `0x8098A54C` | "Continue" | the alert's one button (already used by `frontend/confirm.rs`) |

`%s` is the alias name. The rewrite fills it in before it shows the text (`ui::Text::Custom`).

## 3. The screen's layout [verified]

`MC_ProfileManager.fng` has 11 buttons, sorted here by their x in the package (the order is not known to be
the slots' order):

| x | GUID | name hash |
|---|---|---|
| 343 | `0x000E4621` | `0xB51549B9` |
| 273 | `0x000E461C` | `0xB51549B8` |
| 204 | `0x000EC47F` | `0x4B4E3890` |
| 134 | `0x000E4612` | `0x4B4E388F` |
| 64 | `0x000E460D` | `0x4B4E388E` |
| -5 | `0x000E4608` | `0x4B4E388D` |
| -75 | `0x000E4603` | `0x4B4E388C` |
| -145 | `0x000E45FE` | `0x4B4E388B` |
| -214 | `0x000E45F9` | `0x4B4E388A` |
| -284 | `0x000E45F4` | `0x4B4E3889` |
| -354 | `0x000DD372` | `0x4B4E3888` |

The package's message graph sends `0x0C407210` (the screen's "buttons are live" message) to all of them.
`0x97BDB465` (`INIT`-like, the screen's start) reaches every button and the title. The title is the string
"profile manager" (`0xB5C74226`, `0x03C458C1`).

The offscreen render (`nfsmw view-screen MC_ProfileManager.fng --screenshot`) shows a **horizontal carousel**: one
large icon in the middle with a car-select style frame, left and right arrows, the title "alias manager", and an
"Option title" line under the icon. So the 11 buttons are the carousel's items, not a grid of slots. Only the centre
item is shown at a time. The item's text is the "Option title" object (`0x5E7B09C9`), set by the logic.

## 4. What is not known yet [open]

1. **What the carousel items are.** Likely the aliases plus one "new alias" item; the count of 11 is the package's
   item buttons, of which the scroller shows the ones in the window. The logic has to fill the items and set the
   title for each. To check: read the per-item text with the logic running, or in the package's scripts.
2. **The slot contents.** The original's save format (the alias file) is not read. `save-store` stores bytes, so the
   game's own save format will go in a later step.
3. **Where the rewrite gets the aliases from.** It should read `save-store`'s alias index and name each slot by its
   alias. The slot names the rewrite uses are the alias index's own slot names.

## 5. The original's code [decomp, speed.exe v1.3, Ghidra]

- The screen table: `0x008F4868` holds one record per screen, eight words each. Word 0 is the name pointer
  (`0x0089E418` = "MC_ProfileManager.fng"). Word 1 is the factory for its logic, `0x005599C0`.
- `FUN_005599C0` allocates `0x14C` bytes and calls the constructor `FUN_0054F420`.
- `FUN_0054F420` sets the vtable to `PTR_FUN_0089EA94`, whose RTTI name is **`FEPkgMemcard`**. It sets the screen's
  header text (`HEADER_TEXT` `0x42ADB44C`) from the label `0xBCB18F38`.
- So the original's alias manager is one C++ class, `FEPkgMemcard`, with its own vtable. Its virtual methods handle the
  screen's messages. The vtable slots (in order): `0x0054F490`, `0x00543280`, `0x00573810`, `0x00573870`,
  `0x00543300`, `0x00559D40`, `0x00564070`, `0x00519450`, `0x005442D0`, `0x00519400`, `0x00543410`, `0x0054F7C0`.
- `FUN_00564070` (slot 2, `HandleMessage`-like, 587 bytes) dispatches on the message id:
  - `0xC407210` (the "items live" message) goes to `FUN_00563380`, the item-select handler.
  - `0x35F8620B` (`INIT_COMPLETE`) goes to `FUN_00562C30`, the state-machine step.
  - `0xE1FDE1D1` (`EXIT_COMPLETE`) goes to `FUN_00562FF0`.
  - `0xDA5B8712` copies the selected alias name into the object at `+0x40`, calls `FUN_0056DA30` with it (the alias
    store), then `FUN_00544370(this, 4)` (next state).
  - `0xC9D30688` (accept) checks the memory-card state and either shows an error or goes on.
- **The state.** Global `DAT_0091CBAC` holds the screen's state in its high nibble: `0x10`/`0x70` (a dialog is up),
  `0x20`, `0x30`, `0x40`/`0x60` (list), `0x50` (a name is chosen), `0x80`, `0x90`, `0xA0`, `0xB0`, `0xD0`.
  `FUN_00562C30` branches on that nibble and sends the next state; `FUN_00563380` does the same for item selection.
  Its values are `0x5000000`, `0x1000000`, `0x3000000`, `0x4000000`, `0x6000000`, `0x7000000`, `0x8000000` (the
  question's target states).
- **The dialogs.** `FUN_00543CC0(this, label, flags)` raises a confirmation with a game label. It uses
  `0x87C7577E` (load a new alias?), `0x750EB45C` (a second answer). `FUN_00516BE0(screen, label, ...)` shows a
  one-line notice with a label: `0x8867412D` (the "back to the list" notice), `0x461A18EE`, `0xDC12AF2E`,
  `0x40E73793`, `0x5AFE12F4`. `0x5A051729` and `0x7E998E5E` are the per-item text updates.
- **The alias engine.** `DAT_0091CF90` is the alias manager's data object (`+3` = "an alias is loaded", `+0x10` =
  the alias data). The routines that read and write it are `FUN_00599500`, `FUN_005AB9C0`, `FUN_005ACE40` and
  `FUN_005A3950`. Not yet read. These are the save and load calls the rewrite has to match.
- **The item source.** `FUN_0056DB00(DAT_0091CF90 + 0x10)` returns the selected alias's name. The items come from the
  alias engine, not from the package's scripts.

## 7. The save buffer [decomp, speed.exe v1.3]

- Size: `FUN_00599500` allocates `count * 0x13 + 0xF4B8` bytes, with `count = FUN_00628CC0() = 0x30`, so **63 560 bytes**.
  It is read with `FUN_0058FD10`'s counterpart and written with `FUN_005ACB70` (the write path also runs `FUN_0057F9F0`,
  which takes the checksum and the record's length minus `0x20`).
- Layout, as `FUN_0058FD10` writes it (word counts are dwords, so `0x2332` is `0x8CC8` bytes):

  | offset (bytes) | size | source in the object | content |
  |---|---|---|---|
  | 0 | 4 | zeroed | header |
  | 4 | 4 | `DAT_0091CF90 + 8` | an id |
  | 8 | 32 | `this + 0x0` | the alias name block (8 dwords) |
  | 40 | 240 | `this + 0x324` | a block of `0x3C` dwords |
  | 280 | 132 | `this + 0x24` | a block of `0x21` dwords |
  | 412 | 36 296 | `this + 0x414` | the career block, `0x2332` dwords |
  | 36 708 | 1 | `this + 0x90DC` | a flag byte |
  | 36 709 | 3 032 | `this + 0x90E0` | a block of `0x2F6` dwords |
  | the rest | | `DAT_0091CF90 + 0x2C`, 18-byte records | per-slot records, then the checksum from `FUN_0057FFB0` |

- The name default: when the stored name is empty, the game shows "Player 1" (label `0x7B070984`) or "Player 2"
  (`0x7B070985`), and `FENG: Default string error.` (`0x9BB9CCC3`) when the table lacks it. `FUN_0056DA30` does this.
## 8. The checksum [decomp, speed.exe v1.3; unverified against a real save]

- `FUN_0057F920` writes the checksum and `FUN_0057F9F0` checks it. Both start the **MD5** state with the standard
  constants (`0x67452301`, `0xEFCDAB89`, `0x98BADCFE`, `0x10325476`), feed `FUN_0065B6E0` the data, and finish with
  `FUN_0065B760`. The digest is 16 bytes.
- The write path (`FUN_005ACB70`) reads and writes the data at file offset `0x10` with length `file_len - 0x20`. So the
  file is: a 16-byte header, the body, and a 16-byte MD5 digest of the body at the end.
- `FUN_0057F9F0` returns 1 only when the four dwords of the stored digest equal the computed ones.
- `FUN_0057F7B0` writes the buffer's first part: a zeroed `0x1000`-dword block, then the `this` fields in order
  (the byte at `this+8`, a dword at `this+0xC`, a word at `this+0x10`, a dword at `this+4`, `0x96` dwords from
  `this+0x12`, a word at `this+0x26A`, four dwords from `this+0x26C`). Then it calls `FUN_0056D810`, `FUN_0056D8C0`,
  and copies `0xBD` dwords from `DAT_0091D928`.
- Not yet verified: that the MD5 is over the body alone, and the exact header. Checking this needs a real save file,
  which the install does not have (the saves are on a memory card). Until one is found, the format is a hypothesis.

## 8a. Corrected file layout [decomp, `FUN_005F81D0`, the top-level loader]

Section 8's guess about the digest's place was wrong. The loader reads the file like this:

- Bytes `0..16`: the **MD5 digest**. The loader hashes bytes `16..end` (`FUN_0065B6E0` over `param_1 + 4` dwords to
  the end of the file) and compares the 16 bytes with the digest at the start. A file that does not match is refused.
- Byte `16`: the magic `"Game"` (`0x656D6147`, little-endian). The file is refused without it.
- Byte `20`: a version or size word that must be **greater than 7**.
- Byte `24`: `n6`, a count. Then `n6` entries, each a key dword, a length dword and the data, rounded up to 16 bytes.
- After the entries, sections in this order (each a count and fixed-size records, 16-byte aligned):
  - a table of `dword[7]` records of `0x20` bytes (`FUN_005E4C90`),
  - `dword[8]` records of `0x10` bytes (`FUN_005F6430`),
  - `dword[9]` records of `0x14` bytes (`FUN_005DDBD0`),
  - `dword[10]` records of `0x14` bytes (`FUN_005DDCB0`),
  - a bit array of `dword[12]` bits (copied into the object at `+0x264`),
  - `dword[14]` dwords (`FUN_005F50C0`),
  - and two 32-bit values read through `FUN_005E2580`, which the loader stores in the device object at `+0x108` and
    `+0x10C` when they are not zero.
- The object's entries are looked up with `FUN_005FB710(key)`: it scans `count` entries and returns the one whose
  `FUN_005FAB40` value matches the key.
- `FUN_005AC600` (the no-callback path of `FUN_005AC7D0`) hashes two strings with `FUN_00578DE0` and reports through
  `FUN_005AC310(5, 0xD, ...)`. It looks like a diagnostic, not a data transform, so `FUN_005AC7D0` is not a
  transform of the file's bytes. Ghidra shows `FUN_005AC7D0` as returning nothing, while its callers use a value,
  so this decompile is not reliable for that function yet.
- `FUN_005AC7D0` is the offset call used by the reader and the writer. Its disassembly (not the decompile, which shows
  no return value) gives the behaviour:
  - When a callback is installed (`DAT_008FBB0C`), it calls the callback with the two keys and the packed arguments, and
    returns nothing.
  - Otherwise, when `DAT_008FBB10` is zero, it **returns** `FUN_005AC600(key, key)`. That function looks the key up
    through `FUN_005783E0` and `FUN_00578DE0` and reports it through `FUN_005AC310(5, 0xD, ...)`. So with no callback the
    reader's cursor step is a table lookup of the key `0x6A02D7C6`, which every field uses.
  - The table that `FUN_005783E0` returns is `DAT_008FBAD4`. Its static constants are read: the mask at `+0x14`
    (`0x5CAC5AC5`), the four multipliers and adders at `+0x18`–`+0x34`. `FUN_00578DE0` mixes the key with them. A
    transcription of that mix (`scratchpad` program, not committed) gives `0x1C800000` for the key `0x6A02D7C6`. That
    value is far too big to be a byte offset into a 63 KB file, so either the transcription is wrong or the table is
    changed at run time. Nothing in the binary writes `DAT_008FBB0C` or `DAT_008FBB10` (only reads them), so the callback
    path never runs in the shipped code, and the static table is the value the game uses.
  - **Correction (call chain, Ghidra xrefs):** `FUN_005ACB70` is the **load** entry, not the writer: it calls
    `FUN_005AC950` (the buffer reader) only on its `param_4 == 0` path, and `FUN_005AC950` is the only caller of
    `FUN_005F81D0` (the file parser). `FUN_005ACB70` is called from `FUN_005ACE40` (the one that clears the buffer and
    then calls it with `param_3 = 1`) and from `FUN_005ACDD0` (twice). So the parser runs only through those three
    callers. Whether those callers run in the shipped game (and from which menu) is not checked yet.
  - So the mix result `0x1C800000` is either a transcription error, or the parser is not reached on a normal load. Both
    are open; the runtime check (run the game with a save) settles it.
  - Open: whether the table is rewritten at start-up (check the writers of `DAT_008FBAD4`), and the exact operator
    precedence of the mix. Until those are settled, the field offsets after the header are not confirmed.

The digest check is implemented in `libs/save-store/src/digest.rs` (`check_prefixed`, `with_prefix`, `md5`), with
tests on synthetic data (`md5` uses the `md-5` crate; its empty-input digest is checked against RFC 1321). The `"Game"`
magic and the version check are in `crates/nfsmw/src/save_format.rs` (`check_header`), with tests for the magic, an old
version, and a short body. Those tests were written but not run: another session's settings edit broke the `nfsmw` test
build at the time. Checking
the whole layout against the original's saves still needs a real save file.

## 9. The read path and the two lists [decomp]

- `FUN_005AC950` reads the buffer back with the same field order as the writer. Each field's offset comes from
  `FUN_005AC7D0(0x6A02D7C6, 0x6A02D7C6)`, which is called before every field, so the field list is the writer's list.
  The read path also reads the `0x4000` block at the start of the body through `FUN_005F81D0`, when the memory-card
  device object `DAT_0091E00C` is present. `FUN_005AC7D0`'s meaning (an offset table lookup or a running cursor) is not
  read yet.
- `FUN_0056D810` writes a list of up to **300 entries** (a count dword, capped at 300, then 16 bytes per entry from the
  list at `DAT_0091E004[0xF]`). It skips entries flagged `0x2` when `DAT_009260B2` is set, through `FUN_005FB710`. The
  entry is four dwords; the list is the car list of the alias.
- `FUN_0056D8C0` writes a fixed table of `0x39` pairs of dwords (`0x72` dwords, 456 bytes) from `DAT_0091CD10`.

Still unread: `FUN_005AC7D0` (the offset lookup), `FUN_005F81D0`, and `FUN_005FB710`. The four fields the rewrite
cannot yet match are the `0x4000` device block, the car list entry flags, the `0x39` table, and the offset table.

What this leaves open: the alias engine

What this leaves open: the alias engine (`FUN_00599500`, `FUN_005AB9C0`, `FUN_005ACE40`, `FUN_0056DA30`) and the
exact item ordering. The state machine is clear enough to port its shape (the states above and the message
transitions), but the save format and the item source are the next reverse targets.

## 10. Our save envelope (a default, not the original's format)

Chosen so that saves can be written before the original's body is decoded. It is the original's digest-and-magic
shape, with our own payload: bytes `0..16` the MD5 of everything after them, `16..20` the magic `"Game"`, `20..24`
the version (`8`), then the payload. `crates/nfsmw/src/saves.rs` does `encode`, `decode`, `save` and `load` over
`save_store::SaveStore`, with tests for the round trip, a missing slot, a changed byte, and a missing magic.

It does **not** read the original's saves; a file from the original would fail the decode at the digest or the
payload. It can be replaced when the original's body is decoded.

The first payload is the **alias index**: `saves::save_aliases` and `saves::load_aliases` write and read
`save_store::Aliases` as TOML text (`alias = "slot"` lines) in a slot. The payload is validated on load: text that is
not TOML, a value that is not a string, or a slot name that is not valid is refused. Tests cover the round trip and the
refusals.

**The screen (moved, see section 13):** the rows now sit on the alias manager list, which the main menu's Alias
Manager icon opens (`alias_options.rs`, `Category::AliasManager`). They were first put in the Control Options list
(`input_options.rs`, now removed from there;
`InputSetting::SaveAlias`). Pressing it sends `Command::SaveAlias`, and `Frontend::save_alias` (`flow.rs`) calls
`saves::save_new_alias` on the `saves` folder next to the config file. Each press makes a new named alias, `Save N`, with the
slot `save-N`. The index is saved in the slot `aliases`, and the alert says "Save N saved." (or the error) in an
`AlertLogic` alert, titled "save/load" (`SAVE_LOAD_TITLE`, `0x774E4F02`).
It saves the alias index and, in each `save-N` slot, the settings (see below).
Each alias now holds **every setting** (decided, by the user). `saves::save_new_alias` takes the current `Settings`
and writes them into the alias's slot as TOML: the config's keys (`write::merge` over `saves::partial_of`), plus the nine
settings the config file does not carry (`skip_intro`, `always_rain`, `rain_amount`, `game_speed`, `animation_speed`,
`car_scale`, `no_rev_limiter`, `show_hud_on_look_back`, `headlights_key`), which live in the extra-options layer. A load
(`saves::load_latest_alias`, the highest `Save N`) reads them back, and `Frontend::load_alias` makes them the current settings and marks them
changed, so they reach the config file like any other change. The round trip is tested with non-default values.

Several aliases exist, but Load reads only the latest: choosing one needs a list screen, which is not built. The names are fixed (`Save N`), because the game has no text entry here.

**Career data is not in an alias yet**: the rewrite does not read or write career data, so there is nothing to save.
`settings/mod.rs` made `file` and `write` `pub(crate)` for this. Load is wired to the alias manager row "Load Alias".


## 11. The alert [drawn, checked offscreen]

The alert is `AlertLogic` in `Dialog.fng`. Its title is the package's `HEADER_TEXT` object (`0x42ADB44C`, which the
package ships as "populate me"); `AlertLogic::start` sets it to `Alert::title`, which defaults to "save/load". The
offscreen render (`nfsmw view-screen Dialog.fng --screenshot`, which now opens an alert with the placeholder message
"Load complete.") shows the title, the message, and one Continue button, like the second screenshot. Only the offscreen
render is checked: the game's real flow has not been run. The placeholder message is in `factory.rs` for that check; the
real messages come from the callers (the save code sends "Alias saved.").

## 12. Checked live [run, not seen by a person]

Reproduce the Save Alias check with the scripted pad (the frontend runs for real, with the install's packages):

    nfsmw view-screen Options.fng --category controls --ui-script "wait 2;down;down;down;down;down;down;down;down;down;wait 1;right;wait 3" --screenshot out.png

The screenshot shows the save/load alert ("Alias saved.", one Continue button) over the Controls category. The file is
written to `<config folder>/saves/aliases.sav`; on the test machine that was `AppData\Roaming\nfsmw\config\saves\`, and it
holds the 16-byte digest, `Game`, the version `8`, then the settings as TOML.

**Load Alias** is a row of the alias manager list (`Setting::Alias(AliasSetting::Load)`, section 13; the checks below were
run when it was the eleventh Control Options row, `Command::LoadAlias`, `Frontend::load_alias`). It reads
the alias index with `saves::load_aliases` and shows "Alias loaded." when the default alias is there, "No alias is saved
yet." when the slot is missing, or the error. Checked live the same way (`--ui-script` with ten `down`s, then `right`).
It restores **only the alias list**: no game state is loaded, because no game state is saved yet.

Still not checked live with a real keyboard: the Save and Load flow. The remap capture is checked with the scripted `key` step (see `controls-remap.md`).
Note for anyone repeating these checks: `cargo test` does not rebuild `target/debug/nfsmw.exe`; run `cargo build -p nfsmw`
first, or the screenshot shows the old binary. The options icon menu needed longer waits than `view-screen` does, so opening the
Controls category from the main menu by script is not reliable yet.

## 13. Where the rows live now (user request) [built, unit-tested; not run live]

The user asked for the save and load entries to leave the controls menu and sit under the main menu's alias manager,
as in the original. Done:

- Main menu: the Alias Manager icon (`icon_items.rs`) is enabled. Pressing it (`icon_menu.rs`) opens `Options.fng` with
  `Category::AliasManager`. Its title is the original's "alias manager" label `0x1B76F2CE` (section 1).
- The list has two rows, Save Alias and Load Alias (`alias_options.rs`, `Setting::Alias`). They reuse the option
  screen and the same `Command::SaveAlias` / `Command::LoadAlias` as before.
- Back from that list returns to the main menu, not to the option categories (`widget_menu.rs`, `finish`).
- Control Options no longer has the two rows; `InputSetting` holds the eight value rows only.

Tests: `options_tests.rs` (row counts per category), `icon_items.rs` (Alias Manager enabled in the main menu),
`controls_tests.rs` (value rows only). The full `nfsmw` suite passes (1417 tests).

Not yet checked: that the original's main menu gates this icon on the memory card. Reversing so far [decomp, speed.exe
v1.3, Ghidra]:

- The screen name `MC_ProfileManager.fng` (`0x0089E418`) is referenced by the screen table entry (`0x008F4868`, the
  table's xref list is empty, so the table is read by index, not by a direct reference) and by two data references at
  `0x0055A170` and `0x005435A9`. Ghidra reports no function at either address, so the function that owns them is not
  yet identified. Next step: find the function boundary (`create_function` or the containing code), then read the
  main menu's item handler for the memory-card test. Nothing here is verified yet.

## 6. Plan

1. Pick the slot buttons with the screenshot check above, and write `ProfileLogic` (the same grid focus as
   `controls_screen.rs`).
2. Accept on a filled slot asks "Loading the alias %s" then shows the load-complete alert, with the alias's name.
3. Save and load call `save-store` (`SaveStore::write` and `read`, `Aliases::assign`). A corrupt index shows the
   "corrupt" alert.
4. Back asks the save question when there are unsaved changes (`0x83F4BB3E`).
