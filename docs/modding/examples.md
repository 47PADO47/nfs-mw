# Mod examples

Five small mods, each a folder you can copy, using the 0.1 SDK: the generic `libs/blackbox-modding` interfaces and,
in the last one, the NFS:MW extension in `crates/nfsmw-modding`. The catalogue is in [api.md](api.md) and the pages it
links.

The one example CI builds is [`mods/hello`](../../mods/hello). The five below are sketches written
against the same APIs. They are **not built by CI**; each heading says so. Where a sketch goes past what the SDK
wraps, it says so.

## Build and install

Each Rust mod is a `cdylib` whose `Cargo.toml` follows [`mods/hello/Cargo.toml`](../../mods/hello/Cargo.toml),
with its own package name and any extra dependency. Inside the repository, the folder goes under `mods/<name>/`. An example stays out of the workspace like `hello`
(an empty `[workspace]` table in its manifest, so a build of the repository does not compile it); a mod that ships with the
game, like `nfsmw-2012-content`, is a member of the root `Cargo.toml`. The workspace denies `unsafe_code`, so an item that uses
`unsafe` carries `#[allow(unsafe_code)]`.

Build an example from the repository root with its manifest, for example `cargo build --manifest-path mods/hello/Cargo.toml`
(a mod that is a workspace member builds with `-p <package>`). The library is `mods/hello/target/debug/hello_mod.dll`,
or `libhello_mod.so` on Linux.

Install by copying `mod.toml`, the library (named by `[library]`) and any `files/`, `cars/` and `music/` folders into
`%APPDATA%\nfsmw\config\mods\<id>\`, where `<id>` is the `id` in `[mod]`. The start-up log names each loaded mod, and
each refused mod with its reason. The F12 command `mods` shows the same in the game. Use `--no-mods` when you report a bug.

## 1. Hello hotkey (generic, not built by CI)

Works in any game that offers `bb.commands` and `bb.input`. The hotkey `F5` and the console command `hello` both show a
message and move one blip to the player's car. Based on the APIs in the 0.1 ABI.

Layout: `mods/hotkey/` with `Cargo.toml`, `mod.toml` and `src/lib.rs`.

`mod.toml`:

```toml
[mod]
id = "example.hotkey"
name = "Hello Hotkey"
version = "0.1.0"

[api]
blackbox_modding = "0.1"

[games]
supported = ["*"]

[requires.interfaces]
"bb.commands" = "1.0"
"bb.input" = "1.0"

[optional.interfaces]
"bb.hud" = "1.0"
"bb.map" = "1.0"
"bb.vehicles" = "1.0"

[library]
windows = "hotkey_mod.dll"
linux = "libhotkey_mod.so"
```

`Cargo.toml`: as hello, with `name = "hotkey-mod"`.

`src/lib.rs`:

```rust
use std::cell::Cell;
use std::rc::Rc;

use blackbox_modding::sdk::{BlipId, Ctx, Level, MessageStyle, Mod, ModResult, declare_mod};

/// A blip style the game names. NFS:MW's list is in `crates/nfsmw/src/hud/blip_styles.rs`.
const BLIP_STYLE: &str = "checkpoint";

/// The mod keeps no state of its own. The blip it last placed sits in a cell the hotkey and the command share.
pub struct HelloHotkey;

impl Mod for HelloHotkey {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let blip: Rc<Cell<Option<BlipId>>> = Rc::new(Cell::new(None));

        if let Some(commands) = ctx.commands() {
            let shared = Rc::clone(&blip);
            commands.register("hello", "Show the hello message and move the blip to the car", move |ctx, _args| {
                say_hello(ctx, &shared)
            })?;
        }
        if let Some(input) = ctx.input() {
            let shared = Rc::clone(&blip);
            input.register("say_hello", "Say hello", "key:F5", move |ctx| {
                if let Err(error) = say_hello(ctx, &shared) {
                    ctx.log(Level::Warn, &format!("hello: {error}"));
                }
            })?;
        }
        Ok(HelloHotkey)
    }
}

/// Show the message and put the blip at the player's car. The command and the hotkey both call this.
fn say_hello(ctx: &mut Ctx, blip: &Cell<Option<BlipId>>) -> ModResult<String> {
    if let Some(hud) = ctx.hud() {
        hud.message("Hello from the hotkey mod", MessageStyle::Info, 4.0)?;
    }
    let Some(vehicles) = ctx.vehicles() else {
        return Ok("hello: bb.vehicles absent, no blip".to_owned());
    };
    let Some(map) = ctx.map() else {
        return Ok("hello: bb.map absent, no blip".to_owned());
    };
    let car = vehicles.player()?;
    if let Some(old) = blip.take() {
        // A blip the game already dropped is stale: not an error here.
        let _ = map.remove(old);
    }
    blip.set(Some(map.add(BLIP_STYLE, car.position, true)?));
    Ok(format!("hello: blip at {:?}", car.position))
}

declare_mod!(HelloHotkey);
```

- The binding is the form the `bind` command reads (`key:F5`). A blip style is a name the game registers; NFS:MW's are in
  `crates/nfsmw/src/hud/blip_styles.rs`. `vehicles.player()` answers `NotReady` with no world up.

## 2. Fast clock (generic, not built by CI)

A Mods-page slider sets the real seconds per in-world hour through `bb.time` (10 fast, 600 slow, 60 the default).
The slider is added with `ctx.menus()`, the SDK wrapper of `bb.menus` (see [api-world.md](api-world.md#menus)), so the
mod has no `unsafe`. Based on the 0.1 ABI.

Layout: `mods/fastclock/` with `Cargo.toml`, `mod.toml` and `src/lib.rs`.

`mod.toml`:

```toml
[mod]
id = "example.fastclock"
name = "Fast Clock"
version = "0.1.0"

[api]
blackbox_modding = "0.1"

[games]
supported = ["*"]

[requires.interfaces]
"bb.time" = "1.0"

[optional.interfaces]
"bb.menus" = "1.0"

[library]
windows = "fastclock_mod.dll"
linux = "libfastclock_mod.so"
```

`Cargo.toml`: as hello, with `name = "fast-clock-mod"`.

`src/lib.rs`:

```rust
use std::cell::Cell;
use std::rc::Rc;

use blackbox_modding::abi::BB_EVENT_FRAME_START;
use blackbox_modding::sdk::{Ctx, Event, Level, Mod, ModResult, SliderRange, declare_mod};

pub struct FastClock {
    /// The slider's value, real seconds per in-world hour. The row's closure writes it and the mod reads it on the next
    /// frame. Zero means the player has not moved the slider.
    seconds_per_hour: Rc<Cell<f32>>,
    applied: f32,
}

impl Mod for FastClock {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let seconds_per_hour = Rc::new(Cell::new(0.0));
        register_slider(ctx, Rc::clone(&seconds_per_hour));
        Ok(FastClock { seconds_per_hour, applied: 0.0 })
    }

    fn event(&mut self, ctx: &mut Ctx, event: &Event<'_>) {
        if event.kind() != BB_EVENT_FRAME_START {
            return;
        }
        let wanted = self.seconds_per_hour.get();
        if wanted == 0.0 || wanted == self.applied {
            return;
        }
        let Some(time) = ctx.time() else { return };
        if time.set_day_length(wanted).is_ok() {
            self.applied = wanted;
        }
    }
}

/// Add the slider to the Mods page. Its closure keeps the new value in `seconds_per_hour`.
fn register_slider(ctx: &Ctx, seconds_per_hour: Rc<Cell<f32>>) {
    let Some(menus) = ctx.menus() else {
        ctx.log(Level::Info, "fast clock: bb.menus absent, no slider");
        return;
    };
    let range = SliderRange { min: 10.0, max: 600.0, step: 10.0 };
    let added = menus.slider("seconds_per_hour", "Seconds per hour", range, 60.0, move |_ctx, value| {
        seconds_per_hour.set(value)
    });
    if let Err(error) = added {
        ctx.log(Level::Warn, &format!("fast clock: slider refused: {error}"));
    }
}

declare_mod!(FastClock);
```

- A value saved by an earlier run (under `[mod_values."example.fastclock"]`) is not applied at start-up here. A fuller
  version would read the row with `Menus::get` on the first frame, using the handle `slider` returns.

## 3. Content pack: a car and music (NFS:MW, not built by CI)

A content-only mod has no library and no code: a car of its own, and music in two contexts. The car is one more entry in the
car list, named by its folder; it copies nothing from a stock car, so its manifest says what it is (`[stats]`) and its
folder holds everything it needs. The repository ships no game assets, so the car's files are yours to make in the install's
format ([licensing.md](../licensing.md) applies before you share them).

Layout:

```
mods/example.pack/
  mod.toml
  cars/fastcoupe.toml
  files/CARS/FASTCOUPE/GEOMETRY.BIN     your body and wheel solids, in the install's format
  files/CARS/FASTCOUPE/TEXTURES.BIN     your textures
  files/CARS/FASTCOUPE/BOUNDS.BIN       your collision bounds (a BoundsPack); without it the car cannot drive
  music/track01.ogg
  music/track02.mp3
  music/mix.m3u                         a playlist; its entries are relative to it
```

`mod.toml` (no `[library]`):

```toml
[mod]
id = "example.pack"
name = "Example Pack"
version = "0.1.0"

[api]
blackbox_modding = "0.1"

[games]
supported = ["nfsmw"]

[music]
contexts = ["menu", "free_roam"]    # the default; "race" and "pursuit" wait until the radio has them
always = false                      # true: play even when the custom music setting is off
```

`cars/fastcoupe.toml`:

```toml
role = "both"           # player, cop or both (default: player)
heat = [2, 3, 4]        # cop waves that may use it; allowed only with a cop role; left out means every level
name = "Fast Coupe"     # optional display name (left out: the car id)
manufacturer = "Acme"   # optional brand

[stats]                 # optional; absolute SI values, each checked against a range (car-stats.md)
mass_kg = 1397.0
front_weight_pct = 45.0
cg_height_m = 0.46
wheelbase_m = 2.55
front_track_m = 1.56
rear_track_m = 1.54
front_tire_radius_m = 0.33
rear_tire_radius_m = 0.335
idle_rpm = 900.0
redline_rpm = 7500.0
torque_curve = [[900.0, 330.0], [3000.0, 410.0], [5500.0, 440.0], [7500.0, 380.0]]   # [rpm, N m]
gear_ratios = [3.8, 2.4, 1.7, 1.3, 1.05, 0.85]
reverse_ratio = 3.4
final_drive = 3.6
drive = "rwd"
top_speed_kph = 290.0
front_tire_grip = 1.4
rear_tire_grip = 1.4
front_brake_torque_nm = 1400.0
rear_brake_torque_nm = 1000.0
nitrous = true
nitrous_boost = 0.6

[customization]         # optional; without it the shop offers the car nothing of its own (car-customization.md)
paints = [
  { name = "Red", color = [173, 43, 44] },
  { name = "Silver", color = [200, 204, 197], finish = "metallic" },
]
```

- The car id is the file name in upper case (`FASTCOUPE`; 1 to 32 letters, digits, `_` or `-`), and its `GEOMETRY.BIN` must
  sit in the mod's own `files/CARS/FASTCOUPE/`. There is no key that names a stock car, and an unknown key (an `[override]`
  table, say) refuses the car. Cars are registered at start-up, so a change needs a restart.
- The files this car relies on: `GEOMETRY.BIN` with the body as `FASTCOUPE_KIT00_BODY_A` to `_D` and the wheel as
  `FASTCOUPE_KIT00_FRONT_TIRE_A` to `_D`, `TEXTURES.BIN` with the paint placeholder `FASTCOUPE_SKIN1` on the paint surfaces,
  tail pipe markers on the body if you want nitrous flames, and `BOUNDS.BIN`
  ([car-customization.md](car-customization.md#the-files-of-a-car), [car-collision.md](car-collision.md)). The example has no
  `rims`, `spoilers`, `roof_scoops` or `vinyls`: each needs models or textures you add to the car's own files.
- A key left out of `[stats]` takes the engine's generic car (a 1450 kg rear-drive saloon of about 290 hp). With no
  `[engine_sound]` the car plays the game's built-in engine, and with no `[car_sound]` it has no shift, turbo, backfire or
  pop sounds ([car-sound.md](car-sound.md)).
- Without `always = true`, the tracks play only while the custom music setting is on.

Install: copy the folder to `%APPDATA%\nfsmw\config\mods\example.pack\`. No build step is needed.

## 4. Bounty hunter (NFS:MW, not built by CI)

NFS:MW only. On `F6` it raises the heat of a chase, and it logs each step of the chase and its result. The `nfsmw.*`
events arrive as `Event::Unknown` with their payload bytes, and `MwEvent::decode` types them. Based on the 0.1 ABI.

Layout: `mods/bounty/` with `Cargo.toml`, `mod.toml` and `src/lib.rs`.

`mod.toml`:

```toml
[mod]
id = "example.bounty"
name = "Bounty Hunter"
version = "0.1.0"

[api]
blackbox_modding = "0.1"

[games]
supported = ["nfsmw"]

[requires.interfaces]
"bb.input" = "1.0"
"nfsmw.pursuit" = "0.1"

[library]
windows = "bounty_mod.dll"
linux = "libbounty_mod.so"
```

`Cargo.toml`: as hello, with `name = "bounty-mod"` and this extra dependency:

```toml
nfsmw-modding = { path = "../../../crates/nfsmw-modding", features = ["sdk"] }
```

`src/lib.rs`:

```rust
use blackbox_modding::sdk::{Ctx, Event, Level, Mod, ModResult, declare_mod};
use nfsmw_modding::sdk::{MwEvent, NfsmwExt};

pub struct Bounty;

impl Mod for Bounty {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        if let Some(input) = ctx.input() {
            input.register("raise_heat", "Raise the heat", "key:F6", |ctx| {
                if let Err(error) = raise_heat(ctx) {
                    ctx.log(Level::Warn, &format!("bounty: {error}"));
                }
            })?;
        }
        Ok(Bounty)
    }

    fn event(&mut self, ctx: &mut Ctx, event: &Event<'_>) {
        let Event::Unknown { kind, frame, payload } = *event else { return };
        let line = match MwEvent::decode(kind, frame, payload) {
            MwEvent::PursuitStarted { level, cops, heat, .. } => {
                format!("chase began at level {level}, {cops} cops, heat {heat:.2}")
            }
            MwEvent::HeatChanged { level, old_heat, new_heat, .. } => {
                format!("heat {old_heat:.2} to {new_heat:.2}, level {level}")
            }
            MwEvent::Busted { level, bounty, .. } => format!("busted at level {level}, bounty {bounty}"),
            MwEvent::Escaped { level, duration, .. } => format!("escaped at level {level} after {duration:.1} s"),
            MwEvent::PursuitEnded { reason, bounty, duration, .. } => {
                format!("chase ended ({reason:?}), bounty {bounty}, {duration:.1} s")
            }
            _ => return,
        };
        ctx.log(Level::Info, &format!("bounty: {line}"));
    }
}

/// Raise the heat of the running chase by one level. With no chase running, start one at level 1.
fn raise_heat(ctx: &mut Ctx) -> ModResult<()> {
    let Some(pursuit) = ctx.nfsmw_pursuit() else {
        ctx.log(Level::Info, "bounty: nfsmw.pursuit absent");
        return Ok(());
    };
    if !pursuit.is_pursuing()? {
        return pursuit.start(1, None);
    }
    let heat = pursuit.heat()?;
    pursuit.set_heat(heat + 1.0)
}

declare_mod!(Bounty);
```

- `heat` and `set_heat` answer `NotReady` with no chase, so the key starts one first; the chase appears a frame later.
  `heat` reads the frame before, and `add_heat` is not in 0.1.
- To change the cop wave, use `cop_table` and `set_cop_table` before the first pursuit
  ([api-nfsmw.md](api-nfsmw.md#cop-tables)).

## 5. A nitrous variant (NFS:MW, not built by CI)

A content mod with a few lines of code: it adds the value `rumble` to the game's "Nitrous variant" option and ships a profile.
Nothing runs while the nitrous burns; the game plays the profile ([nitrous-variants.md](nitrous-variants.md)).

```
mods/example.rumble/
  mod.toml
  rumble.dll
  files/NITROUS/RUMBLE/profile.toml
  files/NITROUS/RUMBLE/LOOP.WAV
```

```toml
# mod.toml (extra)
[optional.interfaces]
"bb.settings" = "1.0"
```

```toml
# files/NITROUS/RUMBLE/profile.toml
format = 1
[audio]
loop = "NITROUS/RUMBLE/LOOP.WAV"
loop_fade_in = 0.2
[flame]
tint = [1.0, 0.5, 0.2]
[screen]
vignette = 0.15
fade_out = 0.8
```

```rust
use blackbox_modding::sdk::{Ctx, Mod, ModResult, SettingValue, declare_mod};

struct Rumble;

impl Mod for Rumble {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        // With this value the game's one-value list gets a second one, so the row shows in the gameplay settings.
        if let Some(settings) = ctx.settings() {
            settings.add_option("nitrous_variant", "Nitrous variant", &[SettingValue { id: "rumble", label: "Rumble" }])?;
        }
        Ok(Rumble)
    }
}

declare_mod!(Rumble);
```

Without the mod, the row is not in the menu. With it and "Original" chosen, the game is unchanged.

## Gaps the sketches run into (0.1)

- **No subscription.** A mod receives the events the host sends; `nfsmw.*` ones are decoded with `MwEvent::decode`.
