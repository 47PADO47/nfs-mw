# Movie subtitles

How the game times and draws the lines of the FMV subtitles: which file belongs to which movie, what the records
mean, when a line appears and clears, when subtitles are on at all, and which FEng objects show the text. The file
and string-table layout is in [text.md](../formats/text.md#subtitles). The movie container is in
[video.md](../formats/video.md).

Evidence tags as in [docs/README.md](../README.md#evidence-tags). **[decomp]** is the CC0 decompilation. **[verified]**
is measured on the install. **[unconfirmed]** is a guess or a gap.

**Method.** Read the decomp's `SubTitle.cpp`, `SubTitle.hpp`, `SubtitleInfo.hpp`, the movie player, and the five
screens that own a subtitler. Parsed all 30 `SUBTITLES\` files, the 32 movie headers and `LANGUAGES\English.bin` and
`Labels.bin` with a private Python script. The script is not in the repository. Object and script names were found by
hashing candidate names; the hash function is checked against the 4,546 keys of `Labels.bin`.

**Copyright.** The subtitle text is game dialogue. This page gives label names, counts and times, not the lines.

## 0. Summary

1. **There is no header.** A subtitle file is a bare array of 8-byte records. The last record has start time 0xFFFF.
   The first record is an ordinary cue at time 0 (or 5.4 s in the tutorials), not a header. **[verified]**
2. **The time unit is one tenth of a second.** The clock is wall time since the movie's first frame was shown,
   not a video frame count. **[decomp]**, consistent with the install data (section 3). **[verified]**
3. **Lines show only when the game language is not English**, except in the five tutorial movies, which always
   show subtitles. No option in the options screens controls this. **[decomp]**
4. **A record's hash is the upper-case hash of a label name.** The label is looked up in the current language's
   string table. Hash 0x001A20BA is the clear record. **[decomp + verified]**
5. **Tutorial records are base labels.** Each line is a label made from the base name plus "_A" or "_B". For the
   10 base hashes in the install, the "_A" label exists for all 10 and the "_B" label for 9. **[decomp + verified]**
6. **Two text objects and one box draw the lines.** Line 1 is `SUBTITLES_TEXT`, line 2 is `SUBTITLES_TEXT_2`.
   A background box follows the top edge of line 1. In ordinary movies a cleared line moves the box to Y = 6000.
   **[decomp]**; the object names are **[verified]** by hash.
7. **Not recovered:** the box's object name (hash 0x8BD49BCC), the tutorial show script (0xBCBF0306), the message
   sent on a tutorial clear (0xDBDF2888), the font and the position. They need the FEng package. **[unconfirmed]**

## 1. Files **[verified]**

- `SUBTITLES\<movie>`, no extension, 30 files. Every file has a movie of the same name in `MOVIES\` (with the
  suffix `_english_ntsc.vp6`). `attract_movie` and `ealogo` have no subtitle file.
- Sizes are multiples of 8 bytes. Totals: 317 records, of which 30 are terminators, so 287 cues.
  Of the cues, 118 are clear records and 169 carry a label hash.
- Only `*_english_ntsc.vp6` movies are installed, so the movie audio is English. The subtitles are probably
  translations for the other languages' players (see section 4.1). **[unconfirmed]**
- Groups: 15 `blacklist_NN` (the Blacklist movies), 5 `*_tutorial` (bounty, drag, pursuit, speedtrap, tollbooth),
  `psa`, and 9 `storyfmv_*` movies.

## 2. Record layout

Each record is 8 bytes, little-endian on disk. The decomp swaps bytes only on big-endian targets **[decomp]**.

| Offset | Type | Field | Meaning |
|---|---|---|---|
| 0x00 | u16 | start time | Tenths of a second since the movie's first frame. 0xFFFF ends the file. |
| 0x02 | u16 | pad | 0x0087 in the first record of each of the 30 files, 0 in every other record. The decomp ignores it. Meaning **[unconfirmed]**. |
| 0x04 | u32 | label hash | Hash of the label (section 5). 0x001A20BA means clear. |

Facts about the records **[verified]**:

- The first record starts at time 0 in 25 files, and at 54 (5.4 s) in the five tutorials. In the 25 files that
  start at time 0, the first record read as one u32 is 0x00870000. A reader might take that for a chunk id. It is
  not one.
- Each file has exactly one terminator, and it is the last record. Its hash is the clear value.
- Start times are sorted and no two records share a time.
- The last record before the terminator is a clear record in all 30 files.
- A line has no duration field. It stays on screen until the next record.

## 3. Clock and cue timing

### 3.1 Algorithm **[decomp]**

1. When the subtitle screen is built, the file is loaded and both text objects are set to empty and given the
   show-subtitles script. Nothing is timed yet.
2. The movie player starts the clock when it hands the first frame to the platform, just before it unpauses the
   movie. The clock stores the current millisecond ticker value at that moment.
3. On each screen tick, the code takes the ticker difference since the last tick, multiplies it by 0.001 (so it is in
   seconds, though the variable is named as milliseconds), and adds it to the total elapsed time.
4. The total is multiplied by 10 and truncated to 16 bits. That is the "current time" in tenths of a second.
5. If the next record's start time is at or below the current time, that record is processed and the pointer moves
   on. At most one record is processed per tick. Two records due on the same tick appear on consecutive ticks.
6. The movie-finished message (value 0xC3960EB9) frees the data, so no further cues run. The skip button sends the
   same value to the subtitler before it dismisses the movie.

The 16-bit wrap happens after 6,553.5 s. The longest movie is about 189 s, so it never matters.

### 3.2 Pause

The clock reads a pause flag from the movie player. In the decomp that flag is only ever set to false (in the timer
reset). Nothing sets it to true. So the subtitle clock never pauses, even if the movie is paused. **[decomp]**
If a movie can be paused in practice, its subtitles would run ahead of the picture. **[unconfirmed]**

### 3.3 Unit check **[verified]**

The unit is tenths of a second, as the code says. The test is that every movie's last clear falls inside its length.
If the unit were frames at 29.97 fps, the last clears would sit about a third of the way through each movie.

| Movie | Records | Text cues | Clear cues | Last record | Movie length | Gap after last record |
|---|---|---|---|---|---|---|
| `blacklist_01` | 8 | 3 | 4 | 130 (13.0 s, clear) | 932 frames = 31.10 s | 18.10 s |
| `storyfmv_bus12` | 33 | 20 | 12 | 1,483 (148.3 s, clear) | 4,677 frames = 156.05 s | 7.75 s |
| `drag_tutorial` | 4 | 2 | 1 | 333 (33.3 s, clear) | 1,024 frames = 34.17 s | 0.87 s |

- Movie length is frame count × scale ÷ rate from the `MVhd` block, with rate 982,047 and scale 32,767
  (29.97 fps) in every movie. **[verified]**
- Over all 30 files the gap runs from 0.26 s (`speedtrap_tutorial`) to 18.18 s (`storyfmv_cro06_coh06a`). The median is
  17.2 s. No last record is past the end of its movie.
- The five tutorials and `psa` end 0.3 to 1.3 s before the end of the movie. A movie that is designed to end with its
  last line cleared would look like this. A frame unit would not. **[unconfirmed]** that this is the design intent.

## 4. When a line shows and clears

### 4.1 Gate: language and tutorial flag **[decomp]**

Before any record is read, the subtitler decides whether it runs at all:

1. If the current language is English (the first entry of the language list, value 0), and the movie is not a
   tutorial, the subtitles are not loaded. Nothing is shown.
2. Otherwise the file is loaded. An unknown language (value -1) also passes.
3. The five tutorial names (`drag_tutorial`, `speedtrap_tutorial`, `tollbooth_tutorial`, `bounty_tutorial`,
   `pursuit_tutorial`) always pass. They are matched by name.

The result: an English install shows no FMV subtitles except the tutorials. The gate suggests the story subtitles are
meant for the other languages. **[decomp]**, with the purpose **[unconfirmed]**.

No options toggle reads any subtitle setting. A search of the decomp finds no other user of the subtitle code.
**[decomp]**

### 4.2 Ordinary movies

For each record when it is due:

1. If the hash is 0x001A20BA, or the lookup gives an empty string, the text is emptied and the box moves to
   Y = 6000. Y = 6000 is taken to be off screen. **[unconfirmed]**
2. Otherwise line 1 is set to the label's text in the current language. The box is moved to the top edge of line 1.
3. **Missing label.** The lookup falls back to the engine's default string, `FENG: Default string error.`. That is not
   empty, so it would show on screen. In English all 159 non-tutorial labels resolve, so this does not happen there.
   **[decomp + verified]**. Other languages are not checked. **[unconfirmed]**
4. Line 2 is never set for ordinary movies.

### 4.3 Tutorial movies

For each record when it is due:

1. **Clear record.** The subtitler queues a package message (0xDBDF2888) and does not change the text itself. The
   effect of that message is not read. **[unconfirmed]**
2. **Text record.** Both lines are first hidden (script HIDE, 0x0016A259). Then:
   - The label base + "_A" is looked up. If it exists, line 1 is set to it and the show script 0xBCBF0306 runs.
   - The label base + "_B" is looked up the same way for line 2.
   - A line whose label is missing stays hidden.

The five tutorials, all with two text records:

| Movie | Text record times | Lines found (A / B) |
|---|---|---|
| `bounty_tutorial` | 5.4 s, 14.6 s | 2 / 2 |
| `drag_tutorial` | 5.4 s, 14.0 s | 2 / 2 |
| `pursuit_tutorial` | 5.4 s, 17.0 s | 2 / 2 |
| `speedtrap_tutorial` | 5.4 s, 15.6 s | 2 / 2 |
| `tollbooth_tutorial` | 5.4 s, 13.0 s | 2 / 1 |

The second record of `tollbooth_tutorial` has no line B, so only line A shows. **[verified]**

## 5. Label hashes and text lookup

### 5.1 Hash function **[decomp + verified]**

- The label hash is the upper-case hash. Start from 0xFFFFFFFF. For each byte, multiply the value by 33 and add the
  byte, after converting letters to upper case. This is the function the FEng code calls `FEHashUpper`.
- It reproduces all 4,546 keys of `Labels.bin` **[verified]**. Example: `SUBTITLE_BL_1` gives 0xB5ECDE68.
- The case-sensitive variant (`bStringHash`, the same loop without the upper-case step) reproduces only 4,255 of
  them. The count of 4,255 in [text.md](../formats/text.md) is for that variant. The upper-case form is the right one.
- The same loop, started from a given seed instead of 0xFFFFFFFF, is used for the tutorial lines. The base hash is the
  seed, and "_A" or "_B" is the rest of the input.

### 5.2 Tutorial base labels **[verified]**

Each tutorial text record's hash is the hash of a base label that is not stored in the tables. Seeded with that
base, the suffixes give the stored labels:

| Base label | Base hash | Label A | Label B |
|---|---|---|---|
| `TUTORIAL_BOUNTY_PAGE1` | 0x3C4746A0 | `TUTORIAL_BOUNTY_PAGE1_A` | `..._PAGE1_B` |
| `TUTORIAL_DRAG_PAGE1` | 0x37E9C69D | `TUTORIAL_DRAG_PAGE1_A` | `..._PAGE1_B` |
| `TUTORIAL_TOLLBOOTH_PAGE2` | 0x545F4697 | `TUTORIAL_TOLLBOOTH_PAGE2_A` | not stored |

The pattern is `TUTORIAL_<MOVIE>_PAGE<n>`, where `<n>` runs with the text records of each tutorial.

### 5.3 Table lookup

- The language table is `English.bin` (4,586 keys) or the table of the chosen language. Keys are sorted, so the lookup
  is a binary search. The layout is in [text.md](../formats/text.md#0x00039000-language-strblocks-decomp--verified).
- The lookup returns the decoded string, or the default string when the key is missing (section 4.2, step 3).
- The language is an index. English is 0, French 1, and so on. The default before the language is set is -1.
- `Labels.bin` holds 151 `SUBTITLE_*` names. They include `SUBTITLE_BL`, `SUBTITLE_BL_<n>`, `SUBTITLE_BL_NUM_<n>`,
  and `SUBTITLE_STORY_<scene>_<n>`. For example, the line of `storyfmv_bus12` at 13.0 s is `SUBTITLE_STORY_4_10`.
  **[verified]**

## 6. Which screens use the subtitler **[decomp]**

| Screen (decomp file) | Movies | Subtitles |
|---|---|---|
| `FEMovieScreen` (`Frontend/MenuScreens/Loading/`) | Five front-end movies (`ealogo`, `eahd_bumper`, `psa`, `intro_movie`, `attract_movie`), each in two package variants (10 entries) | Flag set for `psa`, `intro_movie` and `attract_movie`. The install has a file only for `psa`. |
| `FEAnyMovieScreen` (`Frontend/MenuScreens/Common/`) | Movies set by name, such as the safe-house and rap-sheet movies (the decomp sets `storyfmv_saf25` and `storyfmv_rap30`) | Loaded if the file exists. None in English. |
| `InGameMovieScreen` (`Frontend/MenuScreens/InGame/`) | In-game movies | Same as above. |
| `FEAnyTutorialScreen` and `InGameTutorialScreen` | The five tutorials | Always, in any language. |

`FEMovieScreen` passes its flag to the subtitler. The other screens call it for every movie.

## 7. Drawing **[decomp; names verified by hash unless stated]**

- **Where the objects come from.** The subtitler finds each object by hash in the screen's own FEng package.
- **Line 1** is the text object `SUBTITLES_TEXT` (hash 0x599B8442).
- **Line 2** is the text object `SUBTITLES_TEXT_2` (hash 0x2E8DA933). It is used only in tutorials.
- **Box** is the background object with hash 0x8BD49BCC. Its name is not recovered. **[unconfirmed]**
- **Scripts.** SHOWSUBS (hash 0x89D97D9D) is run on both text objects at load. HIDE (0x0016A259) and the show
  script 0xBCBF0306 are used in tutorials. The show script name is not recovered. **[unconfirmed]** The
  effect of each script is in the package, not in the code.
- **Position and font** come from the package. The code never moves the text. It moves only the box, to the top
  edge of line 1. The font is the package's text font. **[unconfirmed]**
- **Language.** The text object takes the string of the current language. Nothing else depends on language.

## 8. Checks run

- Parsed all 30 subtitle files and the headers of their 30 movies. Counts are in section 1.
- Every non-tutorial text label resolves in `English.bin` (159 of 159).
- Every tutorial base hash is absent from the tables. Every tutorial line A label is present (10 of 10), and line B
  is present in 9 of 10 cases.
- The time unit check is in section 3.3.
- Checked the hash function on the 4,546 keys of `Labels.bin` (section 5.1).
- Checked the object and script names by hashing candidate names. Hits: `SUBTITLES_TEXT`, `SUBTITLES_TEXT_2`,
  `SHOW` (0x001CA7C0), `HIDE` (0x0016A259), `SHOWSUBS`. No hit for the box or the tutorial show script.

## 9. Open questions

1. The box object's name (hash 0x8BD49BCC) and the tutorial show script's name (0xBCBF0306). Both need the FEng
   package that holds the subtitle objects. Ghidra or a package dump is needed. **[unconfirmed]**
2. The tutorial clear message 0xDBDF2888. What it hides or shows. **[unconfirmed]**
3. The font, colour and position of the two lines. The FEng package defines them. **[unconfirmed]**
4. The meaning of the 0x87 pad in the first record. **[unconfirmed]**
5. Whether the other languages' tables have every label. Only English was checked. **[unconfirmed]**
6. Whether a movie can be paused in practice, and so whether the clock drift in section 3.2 matters. **[unconfirmed]**

## 10. Sources

Decomp (dbalatoni13/nfsmw, CC0-1.0), read through the GitHub API:

- `src/Speed/Indep/Src/Frontend/SubTitle.cpp`, `SubTitle.hpp`, `SubtitleInfo.hpp`
- `src/Speed/Indep/Src/Frontend/MoviePlayer/MoviePlayer.cpp` and `.hpp` (clock start, pause flag, movie-finished value)
- `src/Speed/Indep/Src/Frontend/MenuScreens/Common/FEAnyMovieScreen.cpp` and `FEAnyTutorialScreen.cpp`
- `src/Speed/Indep/Src/Frontend/MenuScreens/Loading/FEMovieScreen.cpp`
- `src/Speed/Indep/Src/Frontend/MenuScreens/InGame/InGameMovieScreen.cpp` and `InGameTutorialScreen.cpp`
- `src/Speed/Indep/Src/Frontend/Localization/Localize.cpp` and `Localize.hpp` (current language, default string, `eLanguages`)
- `src/Speed/Indep/Src/Frontend/Localization/WideCharHistogram.cpp` (string unpacking)
- `src/Speed/Indep/bWare/Src/Strings.cpp` (`bStringHash`, seeded form)
- `src/Speed/Indep/Src/FEng/FEList.cpp` (`FEHashUpper`)
- `src/Speed/Indep/Src/Frontend/FEngFrontend.hpp` (`FEMSG_MOVIE_FINISHED` = 0xC3960EB9)
- `src/Speed/Indep/Src/Frontend/FEngHashes/ScriptHashes.hpp` (`FEHASH_SHOW`, `FEHASH_HIDE`)
- `symbols/hashes.txt` (name table, used only to look up names)

Install (read only), measured **[verified]**:

- `SUBTITLES\*` (30 files)
- `MOVIES\*_english_ntsc.vp6` (`MVhd` headers, the 30 movies that have subtitles)
- `LANGUAGES\English.bin` and `LANGUAGES\Labels.bin`

Repository docs: [text.md](../formats/text.md), [video.md](../formats/video.md), [career-text.md](career-text.md).
