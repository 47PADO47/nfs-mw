# Localization

How the game picks its language and where every piece of player-facing text comes from: the original's string tables,
the rewrite's own catalogue, and the catalogues of mods. The design decision is [ADR 0012](../decisions/0012-localization.md);
the file layout of the tables is in [text.md](../formats/text.md). Mod authors start at
[modding/translations.md](../modding/translations.md).

Evidence tags as in [docs/README.md](../README.md#evidence-tags). **[decomp]** here is the CC0 decompilation and a static read of
`speed.exe` in Ghidra (tables and their users, no code copied). **[verified]** is measured on the install.

## 1. The original

### 1.1 Language ids and files

The game keeps one table of `{id, name, file, ...}` records, 20 bytes each, and walks it to name a language. **[decomp]**
(Table `0x008F40F8`, reader `FUN_00585BA0`.) <!-- leak-check: allow -->

| Id | Name | File |
|---|---|---|
| 0 | ENGLISH | `LANGUAGES\ENGLISH.BIN` |
| 1 | FRENCH | `LANGUAGES\FRENCH.BIN` |
| 2 | GERMAN | `LANGUAGES\GERMAN.BIN` |
| 3 | ITALIAN | `LANGUAGES\ITALIAN.BIN` |
| 4 | SPANISH | `LANGUAGES\SPANISH.BIN` |
| 5 | DUTCH | `LANGUAGES\DUTCH.BIN` |
| 6 | SWEDISH | `LANGUAGES\SWEDISH.BIN` |
| 7 | DANISH | `LANGUAGES\DANISH.BIN` |
| 12 | POLISH | `LANGUAGES\POLISH.BIN` |
| 13 | FINNISH | `LANGUAGES\FINNISH.BIN` |

Ids 8 to 11 have no file on PC. The registry table below names them Korean (8), Chinese (9), Japanese (10) and Thai (11).
**[decomp]** The voice languages (speech in the install's own language) are ids 0 to 4; the others use text-only phone calls
([career-phone.md](career-phone.md) section 7).

### 1.2 Which language runs

At start-up the game reads the string value `Language` under `HKLM\SOFTWARE\EA Games\Need for Speed Most Wanted` and compares it,
ignoring case, with a 16-row table at `0x008FB578` of `{name, language id, region}` (`FUN_006CB680`). **[decomp]** <!-- leak-check: allow -->

| Registry value | Id | Region |
|---|---|---|
| English US | 0 | 0 |
| English UK | 0 | 5 |
| French | 1 | 6 |
| German | 2 | 7 |
| Italian | 3 | 8 |
| Spanish | 4 | 9 |
| Swedish, Danish, Dutch, Polish, Finnish | 6, 7, 5, 12, 13 | 1 |
| Korean | 8 | 2 |
| Chinese (Traditional), Chinese (Simplified) | 9 | 4, 11 |
| Japanese | 10 | 3 |
| Thai | 11 | 12 |

The region picks regional files. In the install it shows only in the credits: `CREDITS\NA_ENGLISH.TXT` (region 0) and
`CREDITS\UK_ENGLISH.TXT` (region 5). **[verified]** The boot screen `LS_LangSelect.fng` exists in the package lists
([boot-profile-prompt.md](../provenance/boot-profile-prompt.md)); its trigger is not followed and the rewrite does not show it
(section 6).

### 1.3 What a language changes

- **The string table.** Every label of every FEng package and every `Text::Game` is looked up in the language's `.bin`.
  **[decomp]**
- **Six fonts.** `English.bin` holds six `FEngFont` chunks after the string table: `font_arial`, `font_mw_title`,
  `tac_numbers`, `font_conduitmditctt38bi`, `font_arial12` and `font_impact36` **[verified]**. `tac_numbers` is byte for byte the
  one in `InGameB.bun` **[verified]**. The other five are in no bundle the rewrite reads, which answers
  [custom-fonts.md](custom-fonts.md) open question 4: the ARIAL fonts live in the language file. A language's file carries the
  glyphs that language needs. **[unconfirmed]** for files other than English: no other language is installed here.
- **Subtitles** show only when the language is not English ([subtitles.md](subtitles.md) section 4.1). **[decomp]**
- **Movies** are named `<name>_english_ntsc.vp6` in this install. **[verified]** The suffix for other languages is built at run
  time and not read here. The rewrite tries `_<language>_ntsc`, then `_<language>_pal`, then `_english_ntsc`. **[unconfirmed]**
- **Phone calls**: voice languages hear the call, the others read it ([career-phone.md](career-phone.md)). **[decomp]**
- **The save-game locale file** `MEMCARD\LOCALE_<NAME>.loc` (ids 14 and 15 use `ENGLISH.loc`, `FUN_00518260`). **[decomp]** The
  rewrite does not use these files. <!-- leak-check: allow -->

### 1.4 Label hashes

A label's hash is `bStringHash` of its name in upper case. `Labels.bin` names all 4,546 keys this way
([career-text.md](career-text.md) section 1). **[verified]** `blackbox_text::label_hash` hashes the name as written, which is the
same for the upper-case names the code uses. Names that come from files a person writes (a mod's `_game/*.ftl`) are hashed in
upper case, so `carname_mymod_car` and `CARNAME_MYMOD_CAR` are one label.

## 2. Sources of text in the rewrite

| Source | Keys | Where |
|---|---|---|
| Game tables | label hashes | `LANGUAGES/<Name>.bin`, through the VFS, so a mod's `files/LANGUAGES/Russian.bin` is a game table too |
| Rewrite catalogue | Fluent message ids, namespace `core` | `crates/nfsmw/i18n/<locale>/*.ftl`, built into the executable |
| Mod catalogues | Fluent message ids, namespace `mod:<id>` | `<mod>/lang/<locale>/*.ftl` |
| Language packs | any namespace, and game labels | `<mod>/lang/<locale>/_core/`, `_game/`, `_mods/<id>/` |

The rewrite catalogue holds only wording the rewrite wrote itself: text for options, prompts and screens the original does not
have. It never holds game strings or translations of them ([licensing.md](../licensing.md)). Translations of game labels come
from the install's `.bin` files or from language-pack mods.

## 3. Choosing the language

The **locale** is a BCP 47 tag (`en`, `de`, `pt-BR`). The choice, first match wins:

1. `--language <tag>` on the command line.
2. `NFSMW_LANGUAGE`.
3. `language = "<tag>"` in `config.toml`.
4. The install's registry `Language` value (section 1.2), mapped to its tag (`English UK` is `en-GB`).
5. The system locale.
6. `en`.

Steps 4 and 5 only pick a language that is **available**: one with a game table in the install or the overlay, or one the
rewrite catalogue or a loaded language pack declares (`language-name` in its `_core/languages.ftl`). Steps 1 to 3 always win; a
tag nothing provides falls back through the chain below and logs a warning.

The **chain** is the locale, its parents (`pt-BR`, `pt`), then `en`. The pseudo-locale `en-XA` (section 7) has the chain
`en-XA`, `en`.

| Tag | Game table |
|---|---|
| `en` (and `en-US`, `en-GB`) | `English.bin` |
| `fr` | `French.bin` |
| `de` | `German.bin` |
| `it` | `Italian.bin` |
| `es` | `Spanish.bin` |
| `nl` | `Dutch.bin` |
| `sv` | `Swedish.bin` |
| `da` | `Danish.bin` |
| `pl` | `Polish.bin` |
| `fi` | `Finnish.bin` |

Any other `LANGUAGES/*.bin` with a language chunk (fan translations, `Russian.bin`) is listed by its file stem and gets the tag a
small built-in table knows (`ru` for `Russian`), or the lower-case stem. `Labels.bin`, `Largest.bin` and `LanguageTextures.bin`
are never languages.

## 4. Looking text up

### 4.1 A game label

For each locale of the chain in turn, the first of:

1. the language packs' `_game/` entries for that locale, the last-loaded mod first;
2. the game table for that locale.

If every source misses, the text is the one of label `0x9BB9CCC3`, the original's default-string error
([confirmation-dialogs.md](confirmation-dialogs.md)), and the miss is logged once per label. FEng objects keep their own text
instead, as the original does ([frontend.md](../formats/frontend.md)).

Values of `_game` entries are plain text with the game's printf placeholders (`%d`, `%s`, `%$d`); they are not Fluent patterns.

### 4.2 A catalogue message

For each locale of the chain in turn, the first of: the language packs that translate the message's namespace (last-loaded mod
first), then the owning catalogue (the rewrite's for `core`, the mod's own for `mod:<id>`). If none has it, the text is the
message id itself and the miss is logged once.

Arguments are Fluent variables. Plural and ordinal categories are CLDR's (`NUMBER($n, type: "ordinal")`).

### 4.3 Numbers, times and units

Number formatting is in the catalogue, so translators control separators and order (`crate::i18n::fmt`):

| Message | Arguments | English |
|---|---|---|
| `group-separator` | none | `,`: put between each three digits of a whole number, the game's `%$d` |
| `format-ordinal` | `$n` | `{ $n }st` / `nd` / `rd` / `th` |
| `format-percent` | `$n` | `{ $n }%` |

Times keep the original's fixed formats (`m:ss.cc`, `mm:ss.cc`), which do not change with the language. Speed and
distance unit words stay game labels (`KM/H`, `MPH`, `FT`, `M`).

## 5. Fonts

1. The FEng fonts are, first match by name wins: the selected language's `.bin`, `English.bin`, then the bundles
   (`GLOBALA.BUN`, `InGameB.bun`, `FrontB.lzc`). The user's custom fonts still win while they are on
   ([custom-fonts.md](custom-fonts.md)).
2. A FEng string with a character its font has no glyph for is drawn in TrueType fallback fonts instead, line by line, at the
   FEng font's line height and justification: each character from the first of the language pack's
   `lang/<locale>/font.ttf` (or `.otf`) and Ubuntu Light (Latin, Greek and Cyrillic) that has it. Before this, such a
   character was dropped. Line breaks apply; word wrap does not, and a line wider than the box is squeezed.
3. egui text (subtitles, mod messages) uses the same fallback fonts after egui's own.
4. Text is not shaped: Arabic, Hebrew, Thai and the Indic scripts are not supported. CJK works with a pack's font.

## 6. Changing the language

Options has a **Language** row. Each language is listed by its own name: the catalogue's `language-name` for that
locale, else the name the built-in table gives the game table (`Deutsch`). Picking one saves `language` and applies it at once:
the string tables, the language fonts, every FEng screen (they relabel when the resolver changes) and every egui text. Mods read
the change through `bb.i18n.generation`.

The rewrite does not show `LS_LangSelect.fng`: the registry value and the system locale choose the first language.

## 7. Testing aids

- **`en-XA`** is a pseudo-locale made from `en` at load time: vowels and some consonants accented (Latin-1, so the game's
  fonts draw them), each message padded by a third with `~` and wrapped in `[ ]`. On screen, rewrite text without the
  brackets is not from the catalogue. Game labels are left as they are.
- `cargo xtask i18n-check` checks the catalogues against the code (section 8).
- `nfsmw strings --match-catalog` lists English catalogue messages whose text is also a game label, the first step of moving a
  string to the original's translations.

## 8. Rules for code

- Player-facing text is a `Text::Game` label when the original has one with the same meaning; otherwise a catalogue message
  through `t!` or `Text::key`. `Text::Literal` is for text that is not language: names, chat, room names, file names.
- Logic never compares displayed text. Prompts, banners and results carry an enum or an id.
- No `Debug` output on screen.
- The catalogue is written in normal case; a theme that shows capitals calls `to_uppercase`.
- The scope is every menu, the HUD, LAN, co-op and every dialog. The F6 garage, the F8 graphics lab, the performance overlay,
  the console, console commands, the CLI and logs stay in English.

## 9. Open questions

| Question | Where to look |
|---|---|
| Do the other languages' tables have every English label ([subtitles.md](subtitles.md) open question 5)? | A non-English install |
| Do the other languages' `.bin` files carry the same six fonts, and does each need its own `LanguageTextures.bin`? | A non-English install |
| The movie suffix of the other languages | The movie player, a non-English install |
| What `Largest.bin` is: the longest text per label across languages? | Compare with two installed languages |
| When `LS_LangSelect.fng` shows | `FUN_00460DD0` and the boot flow | <!-- leak-check: allow -->
