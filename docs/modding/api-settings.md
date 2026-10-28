# Mod API: settings (`bb.settings`)

**Delivered (`bb.settings` 1.0): options in the game's gameplay settings.** A mod adds an option to the Gameplay page of the
options, or adds values to an option the game already has. The game keeps the player's choice with its other settings and
calls the mod back when it changes. The rules below are the same in the host (`blackbox_modding::host::options`, pure
functions with tests), the SDK and this page.

Status: built and unit tested (the table, the SDK round trip, the merge rules, the game's port and its menu row). Not yet run
in the game window.

## Concepts

- **Option.** A row of the settings page with an id (`a-z`, `0-9`, `_`, `.`, `-`; at most 128 bytes), a label and a kind.
- **Kind.** `List` (one of several values; each value has an id, which is saved, and a label) or `Toggle` (on or off).
- **Menu.** Only the gameplay settings take options in this delivery (`BB_SETTING_MENU_GAMEPLAY`).
- **The game's own options.** The game declares its options by id and kind. `nitrous_variant`, `tire_smoke_variant`, `collision_spark_variant`, `xenon_spark_variant` and `wind_variant` are lists
  with one value, `original` ("Original"): one such option for every kind of effect that has variants; see
  [effect-variants.md](effect-variants.md) (the mechanism, the profile of each kind and how to add a kind) and
  [nitrous-variants.md](nitrous-variants.md). The game's other gameplay rows (`hud_layout`,
  `ui_style`, `ui_theme`, `speed_units`, `minimap`, `language`, `rear_view_mirror`, `turn_signals`, `cop_health_bars`) are
  declared too, so that a clash of kinds is seen, but their values are the game's own and a mod cannot add to them.

## `add_option`: exactly what it does

| The game ... | Result |
|---|---|
| has no option with that id | the option is **appended** to the page, after the game's rows, with the label and the values the mod gave |
| has it, with the same kind (list and list, toggle and toggle) | the mod's values are **appended after the option's own values**, in the order given; a value id the option already has is skipped (the game's own value, its label and its place stay). A toggle has no values, so nothing is added |
| has it, with another kind | **nothing changes**, the game logs a warning (`mod <id>: the option "x" is List in the game, not Toggle ...`), and the call is `Error::Exists` |
| has it, as a row whose values the game fixes (built-in enum rows) | nothing is added, a warning is logged, and the call succeeds |

An option a mod made is removed when the mod unloads, if nothing of another mod is in it; the values a mod added to the game's
option are removed with it. After that the option is as the game had it. A mod that is not loaded leaves the menu exactly as
before.

### Visibility

A **list with one value is not shown** in the menu; a list with more than one value is shown. A toggle is always shown. This is
the one rule for every list: the game's "Nitrous variant", "Tire smoke variant", "Collision spark variant" and "Restored spark variant" have the single value "Original" until a
mod adds another, so they are invisible without such a mod.

### The chosen value

The game saves the choice under `[mod_values.nfsmw]` in the config file, as the text `option_<id> = "<value id>"`
(`"on"` or `"off"` for a toggle). A saved value that the option does not have (its mod is not loaded) reads as the first
value (the default of a toggle), without changing the file, so the choice comes back when the mod does.

## Errors

| Status | When |
|---|---|
| `ARG` | the option id or a value id is not plain; a list has no value, a toggle has one; a value has no label; a value id is given twice; more than 64 values; an unknown kind or menu; a short struct |
| `EXISTS` | the option exists with another kind |
| `NOT_FOUND` | `get` of an id no option has; `remove_option` of a registration that is gone |
| `NOT_IN_CALLBACK` / `WRONG_THREAD` | the call is not made from the mod's callback on the main thread |

A call that fails changes nothing.

## SDK

```rust
use blackbox_modding::sdk::{Ctx, SettingValue};

if let Some(settings) = ctx.settings() {
    // Add a value to the game's "Nitrous variant" list. With one more value the row shows.
    let id = settings.add_option(
        "nitrous_variant",
        "Nitrous variant",
        &[SettingValue { id: "mw2012", label: "NFS Most Wanted 2012" }],
    )?;
    // A list of the mod's own, with a closure that runs when the player picks a value.
    settings.add_option_with(
        "flame_color",
        "Flame colour",
        &[SettingValue { id: "blue", label: "Blue" }, SettingValue { id: "red", label: "Red" }],
        |_ctx, value| { /* value is "blue" or "red" */ },
    )?;
    // An on/off option.
    settings.add_toggle("extra_hud", "Extra HUD", false, |_ctx, on| { /* ... */ })?;
    let chosen: String = settings.get("flame_color")?; // the value id, or "on" / "off"
    // settings.remove_option(id)?;  Registrations left are removed when the mod unloads.
}
```

`ctx.settings()` is `None` when the game does not offer `bb.settings`; ask for it with `[optional.interfaces]` in the manifest
(`"bb.settings" = "1.0"`) and cope with `None`. A label may be a message id of the mod's own catalogue ([translations.md](translations.md)).

## ABI

`BbSettings` (`abi::iface::settings`): `head`, `ctx`, `add_option`, `remove_option`, `get`, in that order; fields are only ever
appended. `BbSettingOption` is size-prefixed (`size`, `kind`, `menu`, `reserved`, `id`, `label`, `values`, `value_count`,
`default_on`, `pad`) and `BbSettingValue` is `{ id, label }`. The callback is `fn(user, BbStr)` with the value id. The ids of
the kinds (`TOGGLE` 0, `LIST` 1) and of the menu (`GAMEPLAY` 0) are never reused. The layout is locked by a snapshot test
(`abi/iface/tests.rs`). **Versioning:** a new interface is a new id at 1.0: no table of an existing interface changes and the
ABI minor (`0.1`) stays, so every mod built before loads as it did, and a mod that wants `bb.settings` lists it under
`[optional.interfaces]` or `[requires.interfaces]` (ADR 0005 sections 1 and 7).
