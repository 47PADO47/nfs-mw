# 0005 — Mods are native plugins behind a stable C ABI

- **Status:** proposed (2026-10-16). The owner has fixed four points in advance: mods are native `cdylib`
  plugins (not Lua or WASM); there are two new crates, `libs/blackbox-modding` and `crates/nfsmw-modding`; the
  list of required capabilities below; and the content goals (cars, maps, scenery, music from other Black Box
  games). Everything else here is a proposal for the owner to accept or change.
- **Milestone:** 12 (scripting API for mods). Touches 6 (menus), 7 (career, pursuit) and 13 (telemetry, which
  can be a mod).
- **API catalogue:** [docs/modding/](../modding/README.md) lists every call, event and extension table.

## Question

The game is a Rust program whose world is **not** an ECS: it is one `Box<dyn Scene>` inside the `NonSend`
`Host` resource (`crates/nfsmw/src/app/host.rs`, trait in `crates/nfsmw/src/viewer/mod.rs`). Players expect to
add cars, cop tables, music, maps and scenery, console commands, hotkeys, abilities, map blips and menu rows,
to read what happens in the game and to react to it, and to change the clock and the pursuit. The same engine
code is meant to serve other Black Box games later (NFS Underground 2), so a mod for the engine layer should
run on both. How do we let third parties extend the game in native code without a stable Rust ABI, without
freezing the internals, and without a failing mod taking the game down?

## Facts checked on 2026-10-16

| Fact | Consequence |
|---|---|
| Rust has no stable ABI. Struct layout, `Vec`, `String`, trait objects and the allocator can all differ between two rustc versions or two builds. | Nothing but `#[repr(C)]` plain data and `extern "C"` functions may cross the plugin boundary. |
| Game state is the `Scene` trait object plus `Host`, driven from Bevy systems in `FrameSet::{Prepare, Commands, Frontend, SceneUpdate, Ui, Hud, Draw}` (`crates/nfsmw/src/app/mod.rs`). Nothing is an entity. | A mod API cannot hand out a Bevy `World`. The host side is a **facade** over `Host` and the scene. |
| The console has built-in commands (`devtools/console/parse.rs`, `BUILT_IN`) and scene commands from `Scene::commands()` / `Scene::command()`, both static `&'static [(&str, &str)]` lists. | Mod commands need a dynamic registry consulted before the scene fallback. |
| `input::Action` is a closed enum. Map blips are `MapBlip { style: &'static str, .. }` with styles registered in `hud/blip_style*.rs`. Menus are hard-coded FEng screens (`frontend/{factory,screens,options_rows}.rs`). The day clock is `scenes/world/day.rs`. | Each needs a small open-ended extension point (a dynamic action id, runtime style names, extra option rows, a clock setter). |
| `libs/game-install/src/vfs.rs` indexes the install only, case-insensitively. Cars are folders under `CARS/` (`nfsmw-data/src/car/mod.rs`); cop waves are rows of the `pursuitlevels` / `heattable` collections (`nfsmw-data/src/pursuit.rs`). Entry zones for shops, safe houses and races are in `scenes/world/entry_zones.rs`. | Content mods need a layered file view and registries. Cop tables are data a mod can supply. |
| The career (bounty, rap sheet, blacklist, milestones, shop and safe-house state) has **no runtime state yet**; only the data files are read (`nfsmw-data/src/gameplay/`). | `nfsmw-modding`'s career table is specified now and implemented as the career lands in milestone 7. |
| `libloading` 0.8.9 (licence ISC) and `semver` 1.0.28 (MIT OR Apache-2.0) are already in `Cargo.lock` (libloading through wgpu's `ash`). Both licences are in the `deny.toml` allow list. | No new licence to approve. |
| The workspace sets `unsafe_code = "deny"`. `xtask/src/leak.rs` forbids the extensions `dll`, `so`, `dylib`, `asi`, `zip` in the repo. | The two new crates need a narrow local `unsafe` allowance. Example mods are built into `target/`, never committed as binaries. |

## Decision

**Mods are native dynamic libraries that talk to the game through versioned, size-prefixed `#[repr(C)]`
function tables. A safe Rust SDK in `blackbox-modding` hides the C layer, a host runtime in the game drives it
from fixed points of the frame, and content (files, cars, music, interiors) flows through a layered virtual
file system and registries.**

### 1. The ABI

Only these cross the boundary: integers of fixed width (`u8` to `u64`, `i32`), `f32`, raw pointers,
`#[repr(C)]` structs of those, and `extern "C"` function pointers. No `usize` in a struct that crosses (lengths
are `u64`), no `bool` (use `u8`), no enums with payload (a `u32` kind plus a payload struct), no Rust types, no
unwinding. Windows and Linux 64-bit are the targets; the pointer width is whatever the game is built for.

**Entry symbols.** The mod library exports two functions with fixed names:

```rust
#[unsafe(no_mangle)] pub extern "C" fn bb_mod_info() -> *const BbModInfo;
#[unsafe(no_mangle)] pub extern "C" fn bb_mod_create(host: *const BbHost, out: *mut BbModApi) -> BbStatus;
```

`bb_mod_info` returns a pointer to a static `BbModInfo { size: u32, abi_major: u16, abi_minor: u16,
sdk_version: BbStr }`. The loader reads it **first**, before it builds any host table, so a mod built for
another major version is refused without running any of its code beyond that one getter. `bb_mod_create`
receives the host table and fills the mod table (the mod's side of the handshake).

**Size-prefixed tables.** Every table starts with `size: u32, major: u16, minor: u16`. The builder writes the
number of bytes it filled in. The reader checks `size` against the offset of a field before it reads the
field, and treats a missing field as "not supported". Minor versions only **append** fields; a field is never
moved, retyped or removed (a removed call keeps its slot and returns `BB_ERR_REMOVED`). A new major version is
a new interface id suffix, so two majors can be offered side by side during a deprecation period.

**The host table** is small and generic:

```rust
#[repr(C)] pub struct BbHost {
    pub size: u32, pub abi_major: u16, pub abi_minor: u16,
    pub ctx: *mut c_void,            // opaque; first argument of every host function
    pub game_id: BbStr,              // "nfsmw", later "nfsu2"
    pub game_version: BbStr,
    pub log: unsafe extern "C" fn(ctx: *mut c_void, level: u32, msg: BbStr),
    pub query_interface: unsafe extern "C" fn(ctx: *mut c_void, id: BbStr, major: u16, min_minor: u16)
        -> *const BbIface,           // null when absent or too old
    pub mod_dir: BbStr,              // the mod's own folder (appended later; check `size` before reading)
    pub game_dir: BbStr,             // the game's install folder, empty when the host has none (appended later)
}
```

**Interfaces are the capability unit.** Everything beyond logging comes from `query_interface`, by a string id
such as `bb.commands`, `bb.events`, `bb.input`, `bb.time`, `bb.map`, `bb.menus`, `bb.content`, `bb.vehicles`,
`bb.hud`, `bb.audio`, `bb.camera`, `bb.world`, and, for the game-specific extension, `nfsmw.career`,
`nfsmw.pursuit`, `nfsmw.races`. An interface table is one `BbIface` header plus its function pointers. The ids
of `bb.*` belong to `blackbox-modding` and mean the same in every Black Box game; the ids of `nfsmw.*` belong to
`nfsmw-modding`. A game offers the `bb.*` interfaces it can implement and none it cannot: asking for an absent
one returns null, and the SDK turns that into `None`. This is **capability negotiation** in two layers:

- **Declared** in the manifest (`[requires]` must be present or the mod is not loaded; `[optional]` may be absent).
  The loader checks the declarations against what the host offers and refuses with a one-line reason.
- **Probed** at run time by the SDK for anything optional, so a mod degrades instead of failing.

**The mod table** the mod fills in is `BbModApi { size, state, on_event, destroy }`. `state` is the mod's own
pointer (the SDK boxes the mod struct); `on_event(state, *const BbEvent) -> BbEventResult` receives every event
the mod subscribed to. `BbEvent { size: u32, kind: u32, frame: u64, payload: *const c_void }`; `payload` points
to a per-kind struct that itself starts with `size`, so a payload can grow. Event kinds are `u32` constants that
are never reused. A mod ignores kinds it does not know. Handlers that the mod gives to the host (commands,
hotkeys, menu callbacks) are `extern "C" fn(user: *mut c_void, ..)` pairs where `user` is the mod's pointer.

**Why not `abi_stable` or `stabby`.** Both generate stable layouts for Rust types and check them at load time.
`abi_stable` (MIT OR Apache-2.0) is mature but heavy (proc macros, its own `RString`/`RVec`/trait-object
types), and I believe its upstream has been quiet since 2023. `stabby` (EPL-2.0 OR Apache-2.0; deny accepts the
Apache-2.0 choice) is newer, with niche-preserving layouts, less proven. Both would make the whole API
Rust-only and tie every mod author, and our own releases, to one third-party crate's macro output and
version. A hand-written C ABI costs us a few hundred lines of `#[repr(C)]` and trampolines, which the SDK and
table builders hide; in exchange it is stable by construction, auditable in one place, readable by a C or Zig
compiler, and immune to rustc upgrades. We keep the layout honest with a snapshot test (section 9).

### 2. The safe Rust SDK

`blackbox-modding` has three feature-gated parts: `abi` (the tables, ids and constants; no dependencies),
`sdk` (what a mod links; needs only `abi`) and `host` (the loader and dispatcher; pulls `libloading`,
`semver`, `toml`, `serde`, `sha2`, `log`). A mod depends on `sdk` only, so no loader code and no `libloading`
ends up in a mod.

A mod author writes plain Rust:

```rust
use blackbox_modding::sdk::{declare_mod, Ctx, Event, Mod, ModResult};

struct Hello { seen: u32 }

impl Mod for Hello {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        ctx.commands()?.register("hello", "say hello", |ctx, _args| ctx.reply("hello"))?;
        ctx.events()?.subscribe_all()?;
        Ok(Hello { seen: 0 })
    }

    fn event(&mut self, ctx: &mut Ctx, event: &Event) {
        let Event::PlayerCarChanged(_) = event else { return };
        ctx.hud().map(|hud| hud.message("new ride"));
        self.seen += 1;
    }
}

declare_mod!(Hello);
```

`declare_mod!` generates `bb_mod_info`, `bb_mod_create`, the mod table and every `extern "C"` trampoline. Each
trampoline wraps the user code in `catch_unwind` (section 3), turns a panic or an `Err` into a `BbStatus`, and
never lets an unwind reach the host. `Event` is a `#[non_exhaustive]` enum of the generic events, built from
the `kind` and payload (the NFS:MW events are a second enum, `nfsmw_modding::sdk::MwEvent`, reached with
`Event::Ext`), with borrowed data copied or tied to the callback's lifetime. Handles (`VehicleId`, `BlipId`, ...)
are small `Copy` newtypes. Wrappers return `Result<_, Error>` where `Error` maps `BbStatus`; a missing interface is
`Error::Unsupported`. Extension tables are fetched with `ctx.ext::<nfsmw_modding::sdk::Career>()`, which does
the `query_interface` call with the right id and version.

The SDK is the supported way to write mods. A C, C++ or Zig mod is possible because the ABI is plain C. A C
header will be **generated with `cbindgen`** (MPL-2.0, an allowed licence; a build-time tool that is not
linked into anything) from the `abi` module by a `cargo xtask modding-header` command, but not in the first
delivery: it waits until there is a non-Rust mod to test it with. The header is generated, never hand-edited,
and a CI step checks that it matches the source.

### 3. The host loader

Lives in `blackbox-modding` (`host` feature) as game-independent code; the game supplies the interface
providers (section 8).

**Folder layout.** Mods live under `<config dir>/mods/` (the per-user directory that `game-install` already
uses for `config.toml`: `%APPDATA%\<app>\config\mods\` on Windows, `~/.config/<app>/mods/` on Linux), one
folder per mod. `--mods-dir <path>` adds a second search root. The game install is never written to.

```
mods/
  example.hello/
    mod.toml              the manifest (required)
    hello.dll             the library (optional: a content-only mod has none)
    files/                VFS overlay, mirrors the install layout (CARS/MYCAR/..., GLOBAL/...)
    content/              declarative registrations (cars, music, interiors, blip styles) as TOML
    config.toml           the mod's own settings (written by the host, see bb.config)
```

**Manifest (`mod.toml`).**

```toml
[mod]
id = "example.hello"            # [a-z0-9_.-], unique, lower case
name = "Hello"
version = "0.1.0"               # semver
authors = ["..."]
license = "MIT"
[api]
blackbox_modding = "1.0"        # the ABI major.minor the mod was built against
[games]
supported = ["nfsmw"]           # game ids; "*" for a mod that only uses bb.* interfaces
[library]
windows = "hello.dll"
linux   = "libhello.so"
[requires.interfaces]           # absent => the mod is not loaded
"bb.commands" = "1.0"
[optional.interfaces]           # absent => the mod must cope
"nfsmw.pursuit" = "1.0"
[dependencies]                  # other mods, semver requirements
"example.base" = ">=1.2, <2"
[load]
after  = ["example.other"]      # ordering hints without a hard dependency
before = []
[content]
priority = 0                    # VFS overlay priority (section 5)
```

**Discovery and order.** The loader scans the search roots, parses every `mod.toml`, and builds a graph. A
mod is **rejected** (with a recorded reason, shown by the `mods` console command) when: the manifest does not
parse or fails validation; the game id is not in `supported`; the ABI major is not offered; a required
interface is missing or too old; a dependency is missing or its version does not satisfy the requirement; a
dependency cycle contains it; or its dependency was rejected. Duplicate ids: the higher version wins, the other
is logged and skipped. Load order is a topological sort over `dependencies` and `load.after/before`; ties break
by id, ascending. Event delivery, command registration and VFS tie-breaks follow load order (a later mod wins).

**Failure isolation.** Each stage of each mod is a separate `Result`. A mod that cannot be loaded (missing
file, wrong architecture, missing symbol, failed handshake, `bb_mod_create` returning an error) is marked
`rejected`, logged with its id and the reason, and everything it had registered is removed; the game and all
other mods continue. A mod can never stop start-up. On Windows the library is opened with an altered search path
so its own DLL dependencies in the mod folder resolve.

**Panic containment.** Since Rust 1.81 a panic that unwinds out of an `extern "C"` function aborts the process,
so the containment is on **both** sides:

- Every exported trampoline that the SDK generates is `catch_unwind(AssertUnwindSafe(..))`. A panic is logged
  with the mod id and the callback, the callback returns `BB_ERR_PANIC`, and the mod's *fault counter* goes up.
- Every host function that a mod can call is equally wrapped (the table builders in `blackbox-modding`
  generate the trampolines, so a bug in the game's facade cannot unwind into a mod either).
- After 5 faults the host *quarantines* the mod: no more events, its commands answer "mod disabled", its
  registrations stay visible but inert. `mod enable <id>` retries.
- A mod must be built with `panic = "unwind"` (the default). A mod built with `panic = "abort"` takes the game
  down on its first panic; the manifest field `[build] panic` is informational and the SDK refuses to compile
  with abort.

What cannot be contained: a native crash (access violation, stack overflow, an abort in non-SDK code) in a mod
kills the process, because the mod runs with the game's privileges. The host keeps a breadcrumb (the id of the
mod and callback in progress) in the crash log, and the next start prints "mod X was running when the last
session crashed; run with --no-mods to rule it out". No mod is disabled automatically on that evidence.

**Unload and hot reload.** The policy is two modes:

- **Normal runs:** mods are loaded at start and unloaded only at exit (`destroy` is called, then the library is
  closed). No unload during play: function pointers, threads and statics of a mod could still be referenced.
- **Developer runs** (`--mod-dev`, or `mod reload <id>` while it is set): a mod can be reloaded at a frame
  boundary (the start of `FrameSet::Commands`). The host copies the library to a cache folder under the local
  data dir first and loads the copy, because **Windows locks a loaded DLL** and the build would otherwise fail to
  overwrite it; the copy is cleaned at the next start. The sequence is: deliver `ModUnloading`, call `destroy`,
  remove everything the host recorded as registered by this mod (commands, subscriptions, hotkeys, abilities,
  blips, menu rows, overlay layer), close the library, load the new file, run the normal load path. Limits: the
  mod must stop its own threads in `destroy`; content that is already in the world from a mod's registry
  (a spawned car) makes the host refuse the reload until it is despawned (`mod reload` prints which); the
  content registries that are sealed after start-up (cars, interiors) are reloaded only with the next start;
  the mod's in-memory state is lost (persist it through `bb.config` to survive). If the new library fails to
  load, the old one is already gone: the mod stays rejected until fixed. Hot reload is not a supported
  production feature and a bug found only with it is lower priority.

### 4. Threading, ownership, timing

- **Everything happens on the main thread**, inside a fixed place of the frame. Two new system sets are added to
  `FrameSet`: `ModsPre` (right after `Commands`: frame-start, input actions, console commands from mods, timers)
  and `ModsPost` (after `SceneUpdate`, before `Ui`: the events the scene queued during its update, the tick
  event, menu callbacks). Host calls from a mod are valid only while a mod callback is running; outside it they
  return `BB_ERR_NOT_IN_CALLBACK`. A host call from another thread returns `BB_ERR_WRONG_THREAD` (the host
  records the main thread id).
- **No shared mutable state across the boundary.** The host never gives a mod a pointer into game memory that
  outlives a call. Reads return values (structs copied into caller memory); writes are calls. Objects are
  referred to by **opaque 64-bit handles** with a generation counter (`VehicleId`, `BlipId`, `AbilityId`,
  `MenuId`, `ZoneId`). A handle from a despawned vehicle or a previous world returns `BB_ERR_STALE`, never
  undefined behaviour.
- **Strings** are UTF-8 `BbStr { ptr, len: u64 }`, not NUL-terminated, valid **only for the duration of the
  call** they are passed in. The receiver copies what it keeps. Limits: names 128 bytes, messages 1024 bytes,
  paths 512 bytes; longer is `BB_ERR_ARG`.
- **Buffers and allocation.** Memory is never freed on the other side of the boundary (different allocators).
  Variable-size results are written through a **sink**: the caller passes a function `fn(user, BbStr)`, or a
  `(ptr, capacity)` buffer and the callee returns the needed length. Lists (nearby vehicles) fill a caller
  array and return the total.
- **Time budget.** The host measures each callback. Above `mods.callback_budget_ms` (default 2 ms) it logs a
  warning with the mod id, rate-limited. It cannot interrupt a native call; a mod that blocks stalls the frame.
  Mods that need long work start their own thread, which may not call the host; it hands results over on the next
  callback (the SDK offers a `Mailbox` for it).

### 5. Content mods

Two mechanisms, both owned by the host:

- **VFS overlay.** `game-install`'s `GameDir` becomes a stack of layers: the install (priority lowest) and one
  layer per mod folder `files/` (priority from `[content] priority`, ties by load order, later wins). A lookup
  goes through the layers from the highest priority down, case-insensitively, as now. A mod can **replace** a file
  (same relative path) or **add** one. Layers are read-only directories; archives are a later step. The overlay
  is file-level: two mods cannot merge into the same `.BUN` chunk in the first delivery. Chunk-level patches
  (`patches/`) are listed as a follow-up. Format crates still take `&[u8]` and never see the layers; only
  `game-install` does.
- **Content registries**, declarative in `content/*.toml` and, for code, through `bb.content`: cars (player,
  cop, traffic roles; a folder in the overlay plus attribute rows, optionally "clone attributes from car X and
  override"), cop waves per heat (rows of the cop tables), music tracks, fonts, scenery or map packs (a tile
  set in the overlay plus an index entry), interiors (an entry zone, a blip, and either a menu page or a scene
  package). Registry keys are namespaced `modid:name`; a collision is rejected and logged, never silently
  overwritten. Registries for cars, music, fonts, scenery and interiors **seal** when start-up content is
  built; later registration returns `BB_ERR_PHASE` (except under `--mod-dev`). Code-free mods are normal: a
  manifest with no `[library]` is a content mod.

**Cross-game content.** A mod can carry assets from another Black Box game (an NFS Underground 2 car for MW, an
MW track for UG2) **only in formats the `libs/` readers support**; the registry entry names its `source_game`
and the host rejects it if no reader or layout table for that game is loaded. Converting data between the
games' attribute schemas and file layouts is the mod author's or a tool's job. The engine will not guess.
Because the readers are engine-generic (`libs/`), a conversion tool built on them is a normal part of the
mod toolchain, but it is not part of this ADR.

### 6. Security

Native mods are **fully trusted code with the user's privileges**: they can read files, open sockets and run
anything. There is no sandbox and the project does not claim one. Therefore:

- The game **never downloads a mod, never fetches one from a peer, and never loads one from a network
  location**. A mod is on the machine because the user put it in the mods folder. No in-game mod browser.
- Multiplayer (milestones 10 and 11): mods are allowed only by that same explicit local install. A session
  does not transfer mods. The runtime switches off the gameplay-affecting interfaces (`bb.vehicles` writes,
  `bb.time` setters, `nfsmw.career`, `nfsmw.pursuit` and `nfsmw.races` writes, content registration) in an
  online session unless the host of the session and every client enable it by setting; this is an accident
  guard, not a security boundary. **A server never trusts state reported by a client, mod or not**: it
  validates positions, speeds and results itself, and never accepts "mod-reported" values. Anti-cheat is out
  of scope.
- Per-mod opt-in: the setting `mods = off | allowlist | all` (default `all` for mods in the folder) with
  `mods.allowed = ["id", ...]`, and `mods.disabled = [...]`, in the layered settings (CLI > env > TOML). The
  flag **`--no-mods`** (and env `NFSMW_NO_MODS=1`) loads none, including content-only mods, and is the first
  step for any bug report.
- The startup log lists each loaded mod with id, version and the SHA-256 of its library (the workspace already
  has `sha2`). Content files are parsed by the same readers as install files, so those readers must reject
  malformed input without panicking (already a rule of the format crates); fuzzing them is welcome.

### 7. Versioning and stability

- **ABI version** `major.minor` is separate from the game version. Within one major, a mod built against minor
  N loads on every host with minor N or higher; tables only grow. A major bump may break and is announced one
  release ahead; the previous major is offered beside the new one for at least one release (as a second
  interface id suffix) before removal.
- **While the version is `0.x`** (until the owner declares the API stable, see the owner decisions) nothing
  is promised across minors; the loader refuses a mod whose `0.minor` differs from the host's.
- **Interface versions** are per interface (`bb.map 1.2`). A game version may add or fix behaviour behind an
  unchanged table; behaviour that mods rely on (units, coordinate space, ordering of events) is part of the
  catalogue and changes only with a minor or major bump and a changelog entry.
- **Coordinates and units** are fixed in the catalogue: metres, seconds, radians, render space (x forward, y
  left, z up), as the game uses today. A game port that differs converts at the facade.
- **Deprecation:** a deprecated entry is marked in the catalogue and in the SDK (`#[deprecated]`), keeps working
  for at least two minor releases, logs one warning per mod and entry the first time it is called, and is
  removed only at a major bump (the slot stays and returns `BB_ERR_REMOVED`).
- **Game versions:** the NFS:MW extension (`nfsmw.*`) follows the same rules. Internal types (`Host`, `Scene`,
  `WorldScene`) are **not** part of any promise; the facade absorbs their changes.

### 8. Crate split and dependency direction

| Crate | Where | Contains | Depends on |
|---|---|---|---|
| `blackbox-modding` | `libs/` | `abi` (tables, ids), `sdk` (mod side), `host` (loader, manifest, resolution, dispatcher, table builders over backend traits). No Bevy, no NFS:MW names. | `libloading`, `semver`, `toml`, `serde`, `sha2`, `log` (host feature only) |
| `nfsmw-modding` | `crates/` | `nfsmw.*` tables, ids, events and the SDK wrappers for career, pursuit and races; backend traits for them. | `blackbox-modding`, `nfsmw-data` (id and enum types only) |
| host implementation | `crates/nfsmw/src/modding/` | The facade: implements the backend traits over `Host` and the scene, owns the `ModRuntime` (a `NonSend` resource beside `Host`), registers the two system sets, wires console, input, blips, menus, VFS. | both, plus the existing game modules |

`crates/` depends on `libs/`, never the reverse; `blackbox-modding` mentions no game. The generic interfaces are
written once as **backend traits** in `blackbox-modding` (for example `trait CommandsBackend`); a generic builder
`commands_table::<B>()` emits the `extern "C"` trampolines (argument checks, `catch_unwind`, handle
validation), and the game implements the trait on its facade in safe Rust. The same goes for `nfsmw-modding`.
New game code touching the `Scene` trait is limited to one optional method that returns the scene's bridge
(default `None`), so `ViewCar` and the front end implement nothing.

`unsafe` is confined: `blackbox-modding` and `nfsmw-modding` carry their own `[lints]` table with
`unsafe_code = "deny"` and `#[allow(unsafe_code)]` on the `abi`, `host::loader` and `sdk::ffi` modules only;
every `unsafe` block has a `// SAFETY:` comment. The game crate itself keeps the workspace lint.

**Example mods** are workspace members in `examples/mods/*`, each a `cdylib` (plus `rlib` for tests), listed in
`members` but **not** in `default-members`, so a plain build never compiles them. Planned set: `hello`
(commands, events, runs on any game), `nitro-ability` (hotkey, ability, HUD message), `mw-heat-hud` (uses
`nfsmw.pursuit`, optional), `car-pack` (content only, no library), and two negative fixtures used by tests
(`panics`, `future-abi`). `mod.toml` files are allowed in `.gitignore`; no library is ever committed.

### 9. Testing

- **Unit tests** in `blackbox-modding`: manifest parsing and validation, version resolution (cycles, missing
  dependency, duplicate id, `after/before`), size-prefix reading, string and handle rules.
- **ABI snapshot test:** `size_of`, `align_of` and `offset_of` of every table and payload are asserted against
  numbers in the test; changing one needs a deliberate edit and a version bump.
- **Integration test** (`libs/blackbox-modding/tests/load_example.rs`): builds the `hello` example with
  `cargo build -p hello-mod` (the test runs the `CARGO` binary), loads the produced library with the real
  loader against a **fake host** (a test implementation of the backend traits), and checks: the handshake, a
  registered command runs, a subscribed event arrives, `destroy` runs. Further cases: the `panics` fixture is
  quarantined after 5 faults while the host keeps running; the `future-abi` fixture is rejected with a reason;
  a missing library file is rejected without affecting a second mod; load order follows `load.after`.
- **Game-side tests** in `crates/nfsmw`: the facade against a headless scene, skipped when the install is absent
  like the existing install-dependent tests; a `--screenshot` run with the example mods enabled (offscreen, no
  window) to prove a mod command and a blip appear.

## Alternatives considered

- **Lua (or Rhai) scripting.** Safe-by-construction and hot-reloadable, but the owner chose native; it also
  means a second language and runtime to bind and keep in step, and cannot link to existing Rust crates.
  Rejected by the owner.
- **WASM (wasmtime or wasmi).** A real sandbox, any language, and a stable ABI by design. Rejected by the
  owner for the cost of the host (large dependency, call overhead for per-frame callbacks) and because mods that
  load assets need broad host access anyway. It stays a possible *second* backend behind the same interface
  tables if untrusted mods become a goal: the table design does not preclude it.
- **ASI-style injection** (the community loader, [Ultimate ASI Loader](../mods/ultimate-asi-loader.md)). That
  patches the original `speed.exe`. There is no original binary in this game, and no addresses to patch; the
  reference mods are used as behaviour specs, not as an ABI.
- **`abi_stable` / `stabby`:** see section 1.
- **Exposing the Bevy `World`:** impossible (the world is not ECS) and would freeze Bevy's version into the mod
  ABI.
- **A bigger single table** (no per-interface query): simpler, but every game must fill the whole table, and
  game-specific parts would leak into the generic crate.

## Consequences

- **Costs:** a few hundred lines of ABI and trampoline code per interface, kept honest by builders and tests.
  The facade is a new surface that must track `Scene` changes (that is its job). The scene needs a small
  event queue so events can be dispatched at a fixed point rather than from deep inside its update. `Action`,
  `MapBlip::style` and the console command list each need a dynamic extension point.
- **Risk accepted:** a buggy or hostile native mod can crash the game or run arbitrary code. Mitigations are
  policy and tooling (section 6), not a sandbox.
- **No effect when unused:** with no mods installed the runtime does a directory scan and nothing else.
- **Licences:** no new third-party crate. `cbindgen` (MPL-2.0) is a build-time tool, used only for the optional
  header.
- **Docs:** the reference-mod pages in [docs/mods/](../mods/README.md) become the source of ideas for the
  catalogue; each page already says how a feature could be done in the Rust rewrite.

## Delivery order (proposal)

1. `blackbox-modding` `abi` + `sdk` + `host` loader, manifest, `hello` example, the integration test.
2. Facade: `bb.commands`, `bb.events`, `bb.input` (hotkeys, abilities), `bb.time`, `bb.hud`.
3. VFS overlay in `game-install`; `bb.content` for cars, then cop waves, music, fonts.
4. `bb.map` (blips), `bb.menus`, interiors; `bb.vehicles`, `bb.camera`, `bb.audio`.
5. `nfsmw-modding`: pursuit and races first (they exist), career when its state lands.

## Revisit triggers

- A second Black Box game joins the workspace (check that the `bb.*` split holds, move more into it).
- Users ask for untrusted or downloaded mods: add the WASM backend, or a permission model, before any download.
- A stable Rust ABI appears in the language.
- The scene is moved onto the ECS: the facade changes, the ABI does not.

## Owner decisions this ADR asks for

1. Accept this ADR as written.
2. The mods folder location (`<config dir>/mods/`) and whether the install may have a `mods/` too.
3. Default policy: load every mod in the folder (proposed) or require an explicit allow list.
4. The multiplayer rule of section 6 (gameplay interfaces off online unless every peer enables mods).
5. When the API leaves `0.x` (proposal: after the example set and three real content mods work).
6. Whether to generate and publish the C header in the first delivery or later (proposal: later).
7. Whether v1 interiors may use custom geometry or only reuse the existing shop and safe-house scenes with new
   locations and menus (proposal: reuse first).

## Could not verify

- The maintenance state and exact licences of `abi_stable` and `stabby`: stated from memory, not re-fetched on
  2026-10-16. They do not change the decision; re-check if the ABI choice is reopened.
- Whether a panic in an `extern "C"` function aborts on every supported toolchain (it does since Rust 1.81;
  the workspace needs 1.95).
- That the shipped game is 64-bit on both platforms; the ABI uses fixed-width fields so it would also work
  for 32-bit, but the snapshot test would need a second set of numbers.
- Which attribute rows a new MW car needs beyond the `CARS/` folder (the content registry will clone from an
  existing car until this is specified); see the open items in [api-content.md](../modding/api-content.md).
