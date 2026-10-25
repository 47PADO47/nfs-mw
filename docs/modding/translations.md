# Translations

How a mod shows its text in the player's language, and how a mod can be a language pack. The design is
[ADR 0012](../decisions/0012-localization.md); how the game picks a language and looks text up is
[specs/localization.md](../specs/localization.md).

## The `lang/` folder

```
mods/<id>/
  lang/<locale>/*.ftl               the mod's own messages
  lang/<locale>/_core/*.ftl         translations of the game rewrite's own messages
  lang/<locale>/_game/*.ftl         text for game labels, by label name
  lang/<locale>/_mods/<id>/*.ftl    translations of another mod's messages
  lang/<locale>/font.ttf | font.otf the fallback font for characters the game's fonts lack
```

`<locale>` is a language tag: `en`, `de`, `pt-BR`. Files are [Fluent](https://projectfluent.org/) (`.ftl`), UTF-8. A
folder whose name is not a tag is skipped with a warning; a file that does not parse is used as far as it parses, and the
problems are logged.

Only the locales of the player's chain are read: the language, its parent (`pt` for `pt-BR`), then English. A message a
locale lacks falls back along the chain, so a mod should always ship `lang/en/`.

## A mod's own messages

```ftl
# lang/en/mymod.ftl
mymod-turbo = Turbo boost
mymod-laps-left = { $n ->
    [one] One lap to go
   *[other] { $n } laps to go
}
```

Messages live in the mod's own namespace, so ids never clash with the game's or another mod's.

### Text the host keeps: just use the id

Wherever a mod hands the host text that the host keeps and draws later, write a message id instead of the text. If the
mod's catalogue has a message of that id, the host shows the message in the current language every time it draws it, so
a language change applies at once. Text that is no id is shown as written, so mods without a `lang/` folder keep working.
This applies to:

- `bb.menus` rows: the label and each choice of a list;
- `bb.ui` themes: the name shown in the theme list;
- `bb.hud` messages;
- interiors: `label` and `prompt` (read when the world opens).

A car's `name` and `manufacturer` are names and are not translated.

### Text a library builds: `bb.i18n`

For text with numbers or names in it, a library asks for `bb.i18n` 1.0 (add `"bb.i18n" = "1.0"` to `[optional.interfaces]`):

```rust
if let Some(i18n) = ctx.i18n() {
    let text = i18n.t_args("mymod-laps-left", &[("n", I18nArg::Int(2))])?;
    hud.message(&text, MessageStyle::Info, 3.0)?;
}
```

| Call | What it does |
|---|---|
| `t(id)` | The message in the current language; its id when no language has it |
| `t_args(id, args)` | The same with variables: `I18nArg::Int` (selects plurals) or `I18nArg::Text` |
| `locale()` | The tag in use, `en` or `pt-BR` |
| `generation()` | Counts language changes. Text a mod keeps from an earlier count is in the old language: format it again |

At most 16 variables per message.

## Language packs

A mod with only a `lang/` folder and no `[library]` is a language pack. See the example in `mods/lang-pack`.

- **`_core/`** translates the rewrite's own messages: the menus, prompts and HUD text that the original game does not
  have. The English source is `crates/nfsmw/i18n/en/` in the repository; copy a file, keep the ids and translate the
  values. A pack that gives `language-name` (in its `_core/languages.ftl`) makes its language selectable in
  Options → Language even when the install has no table for it.
- **`_game/`** gives text for game labels: each message id is a label name, matched without regard to case, and each value
  is plain text with the game's printf places (`%d`, `%s`). Variables are not allowed. Use it for labels a mod adds, such
  as `CARNAME_<BRAND>_<MODEL>` for a mod car. Do not publish the game's own text or translations of it without the
  rights to it.
- **`_mods/<id>/`** translates another mod's own messages.
- **`font.ttf`** (or `.otf`) is used for every character the game's bitmap fonts do not have, in this language: CJK and
  other scripts. Latin, Greek and Cyrillic work without one. Text is not shaped, so right-to-left and complex scripts are
  not supported.

A full game translation can also replace or add a language table through `files/LANGUAGES/<Name>.bin` in the VFS
overlay. The game lists every language table it finds there.

## Priority

For one locale, the first source that has a message answers:

1. language packs, the mod loaded last first;
2. the owner: the rewrite's built-in catalogue for `core`, the mod itself for its own messages.

For game labels, packs win over the install's table of the same locale. Each locale of the chain is tried in full before
the next one, so a German pack's entry beats the English table, and the German table beats an English pack.

## Testing

`--language <tag>` (or `NFSMW_LANGUAGE`) picks a language for one run. `--language en-XA` is the pseudo-locale: every
built-in and mod message in English is shown accented, longer and in brackets, so text that is not translated stands out.
