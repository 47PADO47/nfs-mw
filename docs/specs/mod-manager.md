# Mod manager

The Mod manager lists the mods the game found, with an on/off switch for each one, on the Mods page of the options.
It is a menu of the game's own front end, reached from Options like the other categories. This spec covers what the
player sees, what a switch saves, and when it takes effect. The mod loader itself is in
[ADR 0005](../decisions/0005-native-mod-plugins.md); the menu screens are in [frontend-menus.md](frontend-menus.md).

Source: `crates/nfsmw/src/modding/manager.rs` (the switches and their saving), `crates/nfsmw/src/frontend/mod_options.rs`
(the rows), `crates/nfsmw/src/ui/mod_icon.rs` (the icon). This is a design of this project, not a behaviour of the
original game: the original has no mod manager.

## 1. What is listed

- The Mods category is listed in the options (and in the pause menu's options) when the game has at least one mod to
  show: a mod that loaded, or a mod the loader refused that has a readable id (`mod.toml` parsed). With no such mod the
  category is not listed, as before.
- The page lists one switch per mod, in the order the mods loaded (refused mods follow, in the order the loader saw
  them). A mod that is both loaded and refused (a duplicate id) is listed once.
- Below the switches, the page shows the rows the mods registered themselves with `bb.menus`
  ([api-nfsmw.md](../modding/api-nfsmw.md)). Their behaviour is unchanged.
- A switch shows the mod's id, then its value (On or Off), and a note when the next start will change what this run did:
  - `(restart to unload)`: the mod is loaded now and its switch is Off;
  - `(restart to load)`: the mod is not loaded now and its switch is On.

## 2. What a switch saves

A switch never changes a running mod. It edits the config file, and the change takes effect at the next start.

| `mods` setting (ADR 0005 section 6) | the switch edits | a mod's switch is On when |
|---|---|---|
| `all` (the default) | `mods_disabled`, the list of ids switched off | its id is not in the list |
| `allowlist` | `mods_allowlist`, the list of ids that may load | its id is in the list |
| `off` | nothing: the page is not listed, since no mod is scanned | not applicable |

- When the config file has a `[mods]` table (`docs/modding/api-config.md`) the list is written inside it (`disabled`, `allowlist`),
  where it wins over the legacy key; the `[mods.<id>]` tables of the mods' own settings are left as they were.
- The list is written with the other keys of the file left as they were, as the other settings are
  (`settings/write.rs`). The key is written even when the list is empty, so the file says what is switched off.
- A switch that cannot be saved (no config folder, an unwritable file, a broken TOML file) changes nothing and logs a
  warning. The row keeps its state.
- Settings are read at start. The page shows the state the file gives at the start of the run and updates each switch
  when the player flips it. A later edit of the file by hand is seen at the next start.

Why a running mod is not unloaded: a normal run loads mods at start and unloads them only at exit, because a mod's
function pointers, threads and statics could still be used after a unload (ADR 0005 section 3, "Unload and hot reload").
Developer runs (`--mod-dev`) reload a mod at a frame boundary; the manager does not use that path.

## 3. The icon

- The Mods category has its own picture: a puzzle piece with a switch cut out of it, 128 by 128 pixels, white on
  transparency (`assets/mods/mod-manager.png`). It is drawn at the full slot size, like the package pictures, because
  the drawing keeps a margin inside its 128 pixels.
- The picture is an original drawing made for this project and bundled into the executable. Its provenance and licence
  are in [assets/mods/README.md](../../assets/mods/README.md). Its fingerprint is pinned in `xtask/src/leak.rs`, so
  replacing the file needs that entry updated too.

## 4. Rules

- The switches are a view of the settings: the rows read the manager's state each time they are drawn, so a switch
  that the player flips shows its new value at once.
- No switch is persisted outside the config file. No mod is trusted with the switches: the front end writes them, and
  mods only see the result at the next start.
- A mod id in a switch is only ever a key in a list. It is not used as a path, a command or a name the game shows
  unescaped; the id comes from the mod's manifest, which the loader checked.

## 5. Tests

- `host::policy` (`libs/blackbox-modding`): `Except` allows every id but the listed ones.
- `settings/mods.rs`: the lists are read from and written to their own keys, the other keys are kept, a broken file is refused.
- `modding/manager.rs`: which list a switch edits in each mode, no duplicates, the rows of loaded and refused mods, a
  switch with no config folder changes nothing, a saved switch is read back at the next start.
- `frontend/mod_options_tests.rs`: the switch text and its restart note; no mods give no switches.
- `ui/mod_icon.rs` and `xtask/src/leak.rs`: the icon decodes to 128 by 128, and only the pinned file passes the media check.

## 6. Open questions

- A mod refused for a reason other than the policy (a missing dependency, an unsupported ABI) is listed, and its switch
  can be turned on, but the next start refuses it again until the reason is fixed. The page does not say why. Showing
  the refusal reason needs a row of its own.
- A switch cannot be applied in the running game. A hot switch would need the unload path that a normal run avoids.
