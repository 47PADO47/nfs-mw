# 0012 — Localization

- **Status:** accepted (2026-10-24).
- **Spec:** [specs/localization.md](../specs/localization.md).

## Question

The original ships a string table per language; the rewrite reads only `English.bin` and writes about 800 strings of its own in
Rust (options, prompts, LAN, HUD banners). Mods can only pass literal text. How should every menu, HUD and dialog text be
translated, preferring the original's own translations, while letting mods translate their UI?

## Options

### A. Game tables only

Pick the `.bin` of the chosen language and keep the rewrite's own strings in English. No work for the strings the original has,
but every screen the rewrite added stays English, and mods have no way in.

### B. One catalogue for everything

Copy the game's text into a catalogue of our own. It cannot be done: the game's text and its translations are not ours to commit
([licensing.md](../licensing.md)).

### C. Two key spaces in one lookup

Game labels stay label hashes, looked up in the language's `.bin`. The rewrite's own strings are messages of a Fluent catalogue
built into the executable. Mods add catalogues for their own messages and, as language packs, translations of the rewrite's
messages and of game labels for languages the install lacks. One `Strings` object answers both kinds of key over a locale chain.

## Decision

Option C.

1. **Fluent.** `fluent-bundle` (Apache-2.0 / MIT) gives CLDR plurals and ordinals, selectors and the tooling translators use
   (Weblate, Pontoon). A hand-made TOML format would have to grow the same features.
2. **The original first.** A game label is answered by a language pack for that locale, then the game table, then the rewrite's
   fallback; a catalogue message by a language pack, then its owning catalogue. Each locale of the chain (`pt-BR`, `pt`, `en`)
   is tried in turn. A string the original has is a `Text::Game` label at the call site, so all installed languages work without
   translation work.
3. **The engine part is generic.** `libs/blackbox-i18n` (no Bevy, no game names) holds locales, the chain, the layered
   catalogue, the `Strings` lookup and the pseudo-locale. `nfsmw-data` knows the game's language files and ids. `crates/nfsmw`
   owns selection, the built-in catalogue, the fonts and live switching.
4. **Mods.** A mod's `lang/<locale>/*.ftl` is its own namespace. `_core/`, `_game/` and `_mods/<id>/` translate others. A
   content-only mod can be a whole language pack, with a fallback font. `bb.i18n` 1.0 gives libraries `text`, `locale` and
   `generation`. Text a mod hands the host to keep (menu rows, theme names, HUD messages, interior prompts) that names a
   message of the mod's catalogue is shown as that message, resolved at each draw. No existing table changes, so the ABI
   version stays and built mods keep working.
5. **Live switching.** The current language is process-wide (`crate::i18n::current`), like the custom fonts' state, and a
   change swaps it and bumps a generation counter. FEng packages relabel when their resolver is set again, and egui and the
   presenter's fallback text follow the counter.
6. **Scope.** Menus, HUD, LAN, co-op and dialogs. The F6 garage, the F8 lab, the performance overlay, the console, the CLI and
   logs stay English.

## Consequences

- A translation of the rewrite's own strings is a catalogue file, in the repository or in a mod. A translation of the game's
  labels into a language the original lacks is only ever a mod.
- Logic must not read displayed text; the places that did are changed to enums as part of the work.
- Text is not shaped. Right-to-left and complex scripts need a shaping library later.
