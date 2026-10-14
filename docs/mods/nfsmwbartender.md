# NFSMWBartender (Bartender)

Repository: https://github.com/rng-guy/NFSMWBartender

Bartender is an `.asi` mod for *Need for Speed: Most Wanted* (2005, PC). It exposes police pursuit behaviour as configurable data, split into a "Basic" and an "Advanced" feature set. Defaults match the vanilla game apart from a set of bug fixes that are always applied, plus optional fixes. The README recommends using VltEd alongside it. For nfs-mw this is a useful reference for the pursuit (Heat) tuning surface and for the list of known original-game bugs, but its code cannot be copied (see section 5).

Evidence tags used below: [source] = seen in code fetched from the repo; [README] = stated only in the README; [inferred] = our reading, not stated.

Caveat on method: source content was read through WebFetch, which returns a model-generated summary rather than raw bytes. Treat [source] items as "checked against a summary of the file", and confirm exact addresses and key names from a local clone before relying on them (cloning was out of scope for this pass).

## 1. Metadata

| Field | Value |
|---|---|
| Repo | rng-guy/NFSMWBartender (public; 12 stars, 1 fork, 2 watchers at time of reading) |
| Author | rng-guy (GitHub account; real name not stated in what was read) |
| Type | `.asi` plugin (ASI loader mod) with a C++ source tree |
| Licence | CC0 1.0 Universal. Stated in the sidebar and the README and in `LICENSE.txt` [source: LICENSE.txt read via WebFetch] |
| Language | C++ (MSVC, C++20, 32-bit Windows). Visual Studio solution `NFSMWBartender.sln` [source: dllmain.cpp build checks] |
| Target game build | NFS:MW PC, `speed.exe` of 6,029,312 bytes (5.75 MB), unpacked/unmodified copy [README] |
| Version | v4.00.00 (code banner string); latest release archive named `NfSMW_Bartender_v4.00.0.7z` [README, source] |
| Latest visible activity | Not visible in what was read. The release section was empty in the captured page and no commit date was shown. Date of last activity: unknown |

## 2. What it does

User-visible features, per the README unless marked otherwise.

- **Basic feature set.** Per-Heat-level pursuit parameters. The README lists: cop behaviour in non-player pursuits; bounty and combo multipliers; BUSTED and EVADE bar behaviour; NOS and Speedbreaker charge gained from tagging, assaulting or destroying cops; roadblock and ramming behaviour (HeavyStrategy 3/4, LeaderStrategy 5/7); helicopter-related settings. [README]
- **Basic bug fixes.** Always applies 14 bug fixes; 3 more are optional. [README]
- **Advanced feature set.** Cop spawning behaviour and tables, "without limits" per the README; roadblock setups; chasing-cop spawn rules; helicopter spawning and respawning; Heat gain and loss; Strategy requests. [README]
- **Advanced bug fixes.** Always applies 18 bug fixes; 6 more are optional. [README]
- **Log file.** When logging is enabled it writes `BartenderLog.txt` with a session banner and the version, and notes which other known `.asi` mods are loaded (for example NFSMWUnlimiter, XNFSMusicPlayer, NFSMWExtraOptions, NFSMWLimitAdjuster, WidescreenFix). [source: dllmain.cpp]
- **Compatibility reporting.** Logs the presence of about a dozen named `.asi` mods so that conflicts can be diagnosed. [source: dllmain.cpp]
- **Sharing.** The author allows bundling Bartender with another pursuit mod without credit and suggests linking to the repo. [README]

## 3. How it does it

Mechanism, from the source and README.

- **Loader.** Standard ASI loader: the built DLL is loaded by an `.asi` loader (the README names ThirteenAG's WidescreenFix mod as an example loader). [README]
- **Entry point check.** `DllMain` acts only on `DLL_PROCESS_ATTACH`. It reads `MemoryTools::GetEntryPoint()` and compares it with `0x3C4040`. On mismatch it shows a message box ("This .exe isn't compatible with Bartender.") and returns FALSE. This is a build-identity check. [source: dllmain.cpp]
- **Hook.** A patch hook is installed at `0x6665B4` for a function the source comments as `InitializeEverything (0x665FC0)`. The patch address and the comment address differ, so one of them is probably stale or refers to a call site. Verify before reuse. The hooked `Initialise` calls the original (`CALL_HOOK_ORIGINAL`) first and then does its own setup. [source: dllmain.cpp]
- **Startup work after the original init.** Opens the log if enabled; checks for other mods with `MemoryTools::IsModuleLoaded`; builds a `ConfigParser::Parser` sized for 6 files, 30 sections per file and 25 key/value pairs per section; calls `FeatureSets::Initialise(parser)`; logs whether Basic and Advanced are enabled. [source: dllmain.cpp]
- **Debug aid.** In `_DEBUG` builds it spins until a debugger attaches (`IsDebuggerPresent`). [source: dllmain.cpp]
- **Configuration input.** INI files in a `BartenderSettings` folder, edited by the user. The loader code is in `ConfigParser` and `FeatureSets`, which were not read in this pass. [README, inferred for the file-to-code mapping]
- **Game data.** The mod runs as code inside the game process and patches game functions at fixed addresses for this exe build. It does not appear to edit game data files, but this was not verified. [inferred]
- **Repo layout.** Top level: `Binary`, `Config` (with `Basic` and `Advanced` subfolders), `Docs`, `Editor`, `Source` (with `Headers`). Also `NFSMWBartender.vcxproj`, `README.md`, `README.txt`, `LICENSE.txt`, `Thumbnail.jpg`. [source: directory listings]
- **Files not read.** Only `Source/dllmain.cpp` was read. The `Source/Headers` tree, `FeatureSets`, `ConfigParser`, `MemoryTools`, and the per-Heat tables were not read. Hook sites for the bug fixes and pursuit logic are therefore unknown.

## 4. Configuration

Config is in INI files under `BartenderSettings` (install path from the README). Config folders in the repo are `Basic` and `Advanced`. [README, source listing]

Keys and defaults were not captured. The README points to its own docs and the `READMORE.md` troubleshooting file for names and values. Known constraints from the README:

- Defaults match vanilla apart from the always-on bug fixes.
- More than three cop vehicle types per Heat level, or cop spawn limits above 8, need a car-loader replacement mod (NFSMW LimitAdjuster or OpenLimitAdjuster). [README]
- Updating means reinstalling. Upgrading from versions before v4.00.00 needs all old configuration files replaced. [README]
- Installs must match the 6,029,312-byte `speed.exe`. [README]

## 5. How we could implement it in nfs-mw

Licence: CC0 1.0 is a permissive public-domain dedication, so copying is legally allowed. Project policy (spec-first, clean-room) still applies: we describe behaviour and facts from the repo and implement from our own specification. Do not copy source text or constants unless the spec is written from observed behaviour and verified against the game.

Relevance to our rewrite: the Bartender approach (patch the original game's pursuit code via an ASI hook) does not apply to a clean-room rewrite. Our Rust code does not load into the game process, so the loader, `DllMain`, `MemoryTools`, patch addresses and hook-in-place logic have no equivalent. What does apply is the data model and the list of behaviours.

Behaviours to specify (from section 2, to be re-checked against the game):

- Per-Heat-level pursuit parameter table: cop behaviour, bounty and combo multipliers, BUSTED and EVADE bar rules, NOS and Speedbreaker gain rules, roadblock and ramming strategy selection, helicopter rules.
- Cop spawn tables and limits per Heat level, including the advanced case of more vehicle types and spawn caps above the vanilla limit.
- Heat gain and loss rules.
- A list of original-game pursuit bugs and their fixes, split into "always" and "optional".

Data and formats needed:

- A pursuit tuning schema in a plain-text config format (INI or TOML/RON), read by a loader that takes `&[u8]` or a parsed string. Format crates take bytes and do not open files; only the install-discovery layer touches the install.
- Per-version layout tables for the game build, so the same tuning applies to other builds if ever needed. Do not hard-code `speed.exe` sizes or addresses in `libs/`.
- Our own reference data for pursuit parameters, derived from the game's own data files through our format readers, not from the mod.

Natural home:

- `libs/`: generic config parsing, INI or TOML reader, validation and schema types. Engine-generic, no NFS:MW names.
- `crates/`: NFS:MW pursuit rules (Heat tables, cop spawn logic, bug-fix list), using the `libs/` config code. `crates/` depends on `libs/`, never the reverse.

Risks:

- Fixed-address patches are build-specific. Any spec we write must be based on observed behaviour, not on the mod's addresses.
- Bug-fix list may encode game-specific quirks. Each fix needs a repro in our test corpus before it is adopted.
- Config keys and default values are unknown until the INI files are read. Do not invent defaults; pin them from the vanilla data once our reader is ready.
- Compatibility with other mods is a concern only if we support the same mod ecosystem. Out of scope for the rewrite by default.

## 6. Open questions and what could not be verified

- Exact config key names and defaults: not read. The `Config/Basic` and `Config/Advanced` INI contents were not fetched.
- Hook address mismatch in `dllmain.cpp` (`0x6665B4` patch vs `0x665FC0` comment). Unresolved.
- Source of `FeatureSets`, `ConfigParser`, `MemoryTools`, and the pursuit-logic hook sites: not read.
- Full README text: the README was read as a model-written summary, not verbatim. Exact wording of feature lists and install steps should be checked from the file.
- Default branch: `main` returned 404; `master` worked. Current head commit and date: not confirmed.
- Latest activity and release date: not visible in what was read.
- Real identity of the author beyond the GitHub handle: not stated.
- Whether a git clone or the release archive matches the README claims: not tested. No download was made, per the brief.

## 7. Sources

- https://github.com/rng-guy/NFSMWBartender (repo page, WebFetch)
- https://raw.githubusercontent.com/rng-guy/NFSMWBartender/main/README.md (404)
- https://raw.githubusercontent.com/rng-guy/NFSMWBartender/master/README.md (README, WebFetch summary)
- https://github.com/rng-guy/NFSMWBartender/tree/main/Source (404)
- https://github.com/rng-guy/NFSMWBartender/tree/master/Source (directory listing, WebFetch)
- https://raw.githubusercontent.com/rng-guy/NFSMWBartender/main/LICENSE.txt (404)
- https://raw.githubusercontent.com/rng-guy/NFSMWBartender/master/LICENSE.txt (licence, WebFetch)
- https://raw.githubusercontent.com/rng-guy/NFSMWBartender/master/Source/dllmain.cpp (source, WebFetch summary)
- https://github.com/rng-guy/NFSMWBartender/tree/master/Config (directory listing, WebFetch)
