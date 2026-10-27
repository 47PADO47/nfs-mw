# Mod API: configuration (`bb.config`)

**Delivered (`bb.config` 1.0): a mod's settings from its own file, from the game's main config, or from both.** A mod keeps
its settings in a TOML file in its folder, and the player may override any of them, key by key, from a table in the game's
main config file. The host merges the two and hands the mod the result as TOML text. The mod parses it with the TOML
library it likes: the SDK has no TOML dependency.

Status: built and unit tested (the table, the SDK round trip, the merge rules, the manifest key, the game's port, both
shapes of the main config's `mods` key). Not yet run in the game window.

## Where a mod's settings live

| Source | Where | Notes |
|---|---|---|
| The mod's own file | `config.toml` in the mod's folder, or the file named by `config` in the `[mod]` table of `mod.toml` | Written by the mod author (a template) or by the player. Either source may be missing. |
| The main config | the table `[mods.<mod id>]` in the game's config file (`config.toml` in the game's config folder: `%APPDATA%\nfsmw\config\` on Windows, `~/.config/nfsmw/` on Linux) | One table per mod, with the same keys the mod's own file has. |

Two files are called `config.toml`: the game's, in the config folder, and each mod's own, in `<config folder>/mods/<id>/`.

### The `config` key of `mod.toml`

```toml
[mod]
id = "example.hello"
name = "Hello"
version = "0.1.0"
config = "settings/hello.toml"   # optional; the default is config.toml
```

The path is relative to the mod's folder and written with `/`. A manifest is refused, with a message, when the path is empty,
absolute, has a drive (`C:`), uses `\`, or has an empty, `.` or `..` part. A symbolic link that leads out of the mod's folder
is refused too, when the file is read (the call fails and the host logs why). The file need not exist.

### The `[mods.<id>]` table

Mod ids are lower case `a-z 0-9 _ - .`. A table header splits a bare name at its dots, so an id with a dot is **written
quoted**:

```toml
[mods.nfsmw-2012-content]      # an id with dashes only: bare
role = "both"

[mods."example.hello"]         # an id with a dot: quoted
volume = 7
[mods."example.hello".colours] # a sub-table
sky = "blue"
```

`[mods.example.hello]` is not the mod `example.hello`: TOML reads it as a mod called `example` with a table `hello`. The host
looks the table up by the exact id.

The `mods` key of the config file has **two shapes**, because a TOML key and a table cannot share a name:

```toml
# 1. The legacy string: the mode only. A file in this shape has no [mods.<id>] tables.
mods = "all"                    # "off", "allowlist" or "all"
mods_allowlist = ["example.hello"]
mods_disabled  = ["example.other"]

# 2. The table: the mode, the lists, and one table per mod.
[mods]
mode = "all"                    # "off", "allowlist" or "all"
allowlist = ["example.hello"]   # the same list as mods_allowlist
disabled  = ["example.other"]   # the same list as mods_disabled
[mods.example-hello]            # the configuration of a mod
```

Precedence, one rule: **a list in the `[mods]` table (`allowlist`, `disabled`) wins over the legacy key
(`mods_allowlist`, `mods_disabled`)**; the legacy key is read only when the table does not have the list (an empty list in
the table counts as having it). The mode has one source, the `mods` key, whichever shape. The environment (`NFSMW_MODS`) and
`--no-mods` still override the mode, as before. The mod manager on the Mods page writes the lists into the `[mods]` table when
the file has one and into the legacy keys otherwise, and neither it nor the settings screens touch the `[mods.<id>]` tables
when they save. The row values of `bb.menus` stay under `[mod_values."<id>"]`.

## The effective configuration

The mod's file is **deep-merged under** the main config's table: the main config wins key by key.

| Case | Result |
|---|---|
| a key only one source has | kept |
| a key both have, scalars | the main config's value |
| a key both have, tables | merged recursively, by the same rules |
| a key both have, arrays | the main config's array **replaces** the file's whole (arrays are not joined) |
| a table in one source, a scalar in the other | the main config's value replaces the file's, whichever kind it is |

The host computes the result when the mod calls `read`, serialises it, and writes it to a sink. A mod that has neither source
gets an empty text. If a source is not valid TOML, or the file is outside the mod folder, `read` fails with `Host`, and the host
logs the reason with the mod's id.

**When it is read.** Both files are read at the moment of the call, not cached. A mod reads once, in `Mod::load`, so the
settings the mod runs with are the ones on disk when it loaded. An edit of either file, or a save of the game's settings, is
seen at the next start of the game, or at the next developer reload (`--mod-dev`) of the mod; never by a mod that is already
running. A mod should not call `read` every frame.

## SDK

```rust
use blackbox_modding::sdk::{Ctx, Level};

if let Some(config) = ctx.config() {
    let text: String = config.text()?;          // the merged TOML; "" when there is no source
    let settings: MySettings = toml::from_str(&text)?;   // the mod's own TOML library
    ctx.log(Level::Info, &format!(
        "settings from {}{}{}",
        if config.has_file() { "the mod's config file" } else { "" },
        if config.has_file() && config.has_main_section() { " and " } else { "" },
        if config.has_main_section() { "the game's config file" } else { "" },
    ));
}
```

`ctx.config()` is `None` when the game does not offer `bb.config`; ask for it with `[optional.interfaces]` in the manifest
(`"bb.config" = "1.0"`) and cope with `None` by reading `ctx.mod_dir()?.join("config.toml")` yourself.

| Method | Meaning |
|---|---|
| `text() -> ModResult<String>` | the merged configuration as TOML text |
| `has_file() -> bool` | the mod's own file exists |
| `has_main_section() -> bool` | the game's main config has a table for the mod |

## Errors

| Status | When |
|---|---|
| `ARG` | `read` without a sink, `sources` without an `out` pointer |
| `HOST` | a source is not valid TOML, the file cannot be read, or it is outside the mod folder (logged by the host) |
| `NOT_IN_CALLBACK` / `WRONG_THREAD` | the call is not made from the mod's callback on the main thread |

## ABI

`BbConfig` (`abi::iface::config`): `head`, `ctx`, `read`, `sources`, in that order; 32 bytes, `ctx` at 8, `read` at 16, `sources`
at 24. Fields are only ever appended.

| Function | Signature |
|---|---|
| `read` | `(ctx, sink: Option<BbSinkFn>, sink_user)`: writes the merged TOML to the sink during the call |
| `sources` | `(ctx, out: *mut u32)`: writes the `BB_CONFIG_*` bits of the sources that exist |

`BB_CONFIG_FILE` = 1 (the mod's own file exists) and `BB_CONFIG_MAIN` = 2 (the main config has a table for the mod); the ids
are never reused. `sources` does not parse anything, so it answers even when `read` would fail. The ids are `IFACE_CONFIG`
(`"bb.config"`) and `IFACE_CONFIG_VERSION` (1, 0). The layout is locked by a snapshot test (`abi/iface/tests.rs`).
**Versioning:** a new interface is a new id at 1.0: no table of an existing interface changes and the ABI minor stays, so every
mod built before loads as it did (ADR 0005 sections 1 and 7).

## For a game that implements it

`InterfaceRegistry::add_config(backend)` offers the interface. The backend (`ConfigBackend`) has one method,
`main_section(owner) -> Option<String>`: the TOML text of the game's own table for the mod (NFS:MW: `[mods.<id>]` of the config
file), or `None`. The host reads the mod's file from the mod's folder (`HostCtx::mod_dir`, the manifest's `config`), checks that
it stays inside, merges the two and serves the result. The pure rules are in `blackbox_modding::host::config`
(`check_relative`, `resolve_file`, `merge`, `effective`).
