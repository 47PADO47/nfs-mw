# nfsmw-recomp (veritr1x)

A work-in-progress static recompilation of Need for Speed: Most Wanted (2005, PC Black Edition, `speed.exe`) to native code for macOS, iPadOS, Linux, Windows and the browser. The game code is translated ahead of time to C and compiled natively. The user must supply their own game files. The runtime, translator and hosts come from a `recomp-kit` submodule (`kit/`); this repo holds the game-specific config, tests, analysis tooling and docs.

Evidence tags used below: [source] = seen in a file's content (read through a summarising fetch, not verbatim); [README] = stated only in the README; [inferred] = my reading, not stated.

## 1. Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/veritr1x/nfsmw-recomp |
| Author / owner | veritr1x |
| Type | Native recompilation port (game-specific) built on a kit; not an .asi plugin, not a mod loader |
| Licence | MIT (Copyright 2026 veritr1x), per the `LICENSE` file. The LICENSE text says it covers the native implementation described in `NOTICE` and does not relicense the original game materials. |
| Language | Not shown by the fetched page (the Languages section was empty in the summary). C (native overrides, e.g. `culling.c`), Python (`tools/`, `tests/`), CMake. [inferred from file names] |
| Target game build | `speed.exe` only, pinned by SHA-256 in `game.toml`. Black Edition PC. Direct3D 9, DirectSound, `.abk` sound banks. |
| Latest visible activity | 54 commits shown on the repo page; no commit date visible. The README's "Status as of 2026-09-19" is the platform-status check date, not a commit date. |
| Fetched | 2026-10-14 |

## 2. What it does

User-visible behaviour, as stated by the README unless marked otherwise.

- **Boots and plays through the menus.** [README] "The game boots and reaches track select and races."
- **Races, intermittently.** A scripted race completes in only one or two of five macOS runs; the others hang while loading a race. This is the stated main open blocker. [README]
- **Runs on several platforms.** macOS 14+ is the only platform re-checked against the current kit pin. iPadOS 17+, Linux, Windows and browser (WebGPU) were last verified on an earlier kit pin. [README]
- **Widescreen display fixes.** `game.toml` contains patches that redirect some memory operands and instructions "to support widescreen display", plus a data seed for the widescreen fix and the original six resolutions. [source, summary of `game.toml`]
- **Native overrides of renderer hot routines.** `game.toml` lists native C overrides for renderer hot paths. [source, summary of `game.toml`]
- **Keyboard/pad control preset.** `[controls.mapped]` maps a racing preset onto default PC keys, with the pad mapped to keyboard keys. [source, summary of `game.toml`]
- **Install detection and bundle rules.** `[launcher]`, `[setup]` and `[bundle]` detect the install by name, require the folders CARS, FRONTEND, GLOBAL, SOUND and TRACKS, and exclude some files from the iOS bundle. [source, summary of `game.toml`]
- **Web build.** The web build "produces a servable site". [README]
- **Shutdown quirk.** A guest worker can outlive the run, giving exit code 4. [README, known issues]

## 3. How it does it

- **Not an ASI loader plugin.** Nothing in the fetched material mentions an ASI loader, a DLL proxy, or a runtime hook into the original `.exe`. The approach is ahead-of-time translation of the executable to C, then native build. [inferred from README "translated to C ahead of time and compiled natively" and the `[translate]` section]
- **Static analysis with Ghidra.** Build step: "run Ghidra-based analysis", then `tools/build.py`. The translator consumes Ghidra listings and curated symbols referenced from `[translate]`. [README, source]
- **Pinned input.** `game.toml` pins the executable by SHA-256, image base and entry point, and the expected install path. [source, summary]
- **Function addresses.** `game.toml` holds a long `entry_points` list of addresses the disassembler missed (vtable methods, codec routines, thread start routines). The file itself warns some entries may be false positives and need checking against instruction boundaries. [source, summary]
- **Hooks.** `[hooks]` lists frame-clock, cursor, mouse and camera addresses. Most are unidentified placeholders, and many addresses are "sentinel" values the game never references, so those hooks do not fire yet. [source, summary]
- **Renderer.** Direct3D 9 is the target API for the original game; the native side of the renderer is in `native/core/nfsmw/` with `culling.c` and `overrides.h` visible in the listing. Whether the native build translates D3D9 to Metal, Vulkan or WebGPU is not stated in the fetched material. [README for D3D9; the translation layer is in the `kit/` submodule, not fetched]
- **Sound.** DirectSound and `.abk` banks are named as the original's audio stack. [README]
- **Repo layout.** Top level: `.github/workflows`, `docs`, `kit` (submodule at `eb32cc4`), `mods`, `native`, `smoke`, `tests`, `tools`, plus `AGENTS.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `LICENSE`, `NOTICE`, `README.md`, `game.toml`, `globals.toml`. [source, file listing]
- **`mods/`.** The listing fetched showed a `core/nfsmw` folder and a `CMakeLists.txt`; the listing was incomplete. I did not confirm the contents of `mods/` beyond that. [source, partial listing]

## 4. Configuration

Configuration is in `game.toml` and `globals.toml` (the latter was not fetched). Keys named in the fetched summary of `game.toml`:

| Section | What it holds | Default value |
|---|---|---|
| `[game]` | executable name, SHA-256, image base, entry point, expected install path | pinned to one `speed.exe` image (exact hash not recorded here) |
| `[translate]` | Ghidra listing and symbol paths, native C overrides, operand redirects and widescreen patches | not recorded |
| `data_seeds` | widescreen fix, cinematic quad, six original resolutions | presets, exact values not recorded |
| `entry_points` | function addresses to treat as code entries | list; some unverified |
| `[hooks]` | frame clock, cursor, mouse, camera addresses | mostly placeholders |
| `[controls.mapped]` | key/pad mapping | racing preset over default PC keys |
| `[launcher]`, `[setup]`, `[bundle]` | install detection name, required folders, iOS bundle exclusions | CARS, FRONTEND, GLOBAL, SOUND, TRACKS required |

Exact values were not verified because the fetch returned a summary, not the file. A verbatim read of `game.toml` is needed before relying on any value.

## 5. How we could implement it in nfs-mw

Ground rules for us: this repo is MIT and we do not copy its code. Our project is spec-first and clean-room (see `docs/licensing.md`). We describe behaviour and facts only.

Important: the mechanism here (ahead-of-time translation of the game binary to C, Ghidra symbol files, pinned SHA-256, per-address hooks) is the kit approach. Our rewrite is a different approach. It reads the install directly and re-implements the physics, AI, and formats from specs. So most of this repo's mechanism does not apply to a clean-room rewrite. Loader-specific hacks (sentinel addresses, operand redirects into a translated binary) do not apply at all.

**Behaviours worth specifying for our own work, independent of this repo:**

- Widescreen aspect handling and the original six resolutions as a data table, not hard-coded. Natural home: `libs/` as a generic aspect and resolution helper, with the NFS:MW resolution list in `crates/`.
- Controls: a default racing keymap with remapping, pad-to-keyboard mapping. Home: `libs/` for the input mapping layer; the NFS:MW default preset in `crates/`.
- Install detection by required folder set, and a bundle exclusion list. This maps onto our existing `game-install` crate (the only crate allowed to touch the install). Home: `crates/` or the install module; keep the format parsing out of it.
- Frame timing and camera control. Our rewrite will own these, so no hooks are needed. Spec them as behaviour (fixed step, camera rules) in our own docs.

**Data or formats needed:** the NFS:MW install folder layout (CARS, FRONTEND, GLOBAL, SOUND, TRACKS), and the original resolution list. Both can be checked against the user's own install, not against this repo's files.

**Risks:**
- Copying code or the `game.toml` tables would breach the spec-first rule, even though MIT would allow it. Treat this repo as reference for facts only, and cite it in our notes as a source of observations, not as a source of code.
- Its addresses and offsets are for one pinned `speed.exe` hash. They do not transfer to other builds.
- The race-loading hang it reports is a timing bug in a translated runtime. We have no equivalent path, so the risk does not carry over directly.
- Its status is "work in progress" with a known blocker. Do not treat its status as a feature list for us.

**Does not apply to a clean-room rewrite:** the ahead-of-time binary translation, Ghidra symbol import, `entry_points` and `[hooks]` addresses, sentinel addresses, and the operand/instruction patches.

## 6. Open questions and what could not be verified

- Exact contents of `game.toml` and `globals.toml`. Only summaries were fetched. Verbatim values, hashes and addresses are not recorded here.
- Full contents of `mods/` and `native/`. The directory listings were partial.
- Full contents of `docs/`, `tests/`, `tools/`, `CHANGELOG.md`, `NOTICE` and `CONTRIBUTING.md`. Not fetched.
- The `kit/` submodule and the translation layer it provides (commit `eb32cc4`). Not fetched.
- Language breakdown. The fetched page did not show it.
- Latest commit date. Not visible in the fetch.
- Whether the README's status claims are current. Only macOS was re-checked against the current kit pin, per the README.
- Whether the repo is actively maintained. 54 commits and a 2026-09-19 status date suggest recent work, but this is inferred.
- No code was cloned or run. Nothing was executed, per the task rules.

## 7. Sources

- https://github.com/veritr1x/nfsmw-recomp (repo page, via WebFetch; summarised)
- https://raw.githubusercontent.com/veritr1x/nfsmw-recomp/main/README.md (via WebFetch; summarised, not verbatim)
- https://raw.githubusercontent.com/veritr1x/nfsmw-recomp/main/LICENSE (via WebFetch)
- https://raw.githubusercontent.com/veritr1x/nfsmw-recomp/main/game.toml (via WebFetch; summarised)
- https://github.com/veritr1x/nfsmw-recomp/tree/main/mods (directory listing, partial)
- https://github.com/veritr1x/nfsmw-recomp/tree/main/native (directory listing, partial)
