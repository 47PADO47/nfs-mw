# nfsmw-2005-re (s-b-repo)

Community reverse-engineering project for Need for Speed: Most Wanted (2005, PC retail `speed.exe`). It publishes RE documentation (architecture notes, attribute hash tables, SDK address/enum/struct JSON, renamed-function list), a pure-Python command-line tool, and a small proof-of-concept ASI trainer that makes NOS and the GameBreaker (race breaker) effectively infinite. It ships no game executables or assets. A companion header-only mod SDK lives in a separate repository (see Open questions).

## 1. Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/s-b-repo/nfsmw-2005-re |
| Author | s-b-repo (GitHub account). Copyright line: "Copyright (c) 2026 NFSMW reverse-engineering contributors" [README/LICENSE] |
| Type | Research (RE documentation) plus tool (Python CLI) plus mod (ASI trainer) |
| Licence | BSD 3-Clause License (LICENSE file present and read) [source] |
| Language | Python (CLI, `tools/nfsmw-tool`, requires Python 3.10+) [README]; C (trainer, `mods/infinite_trainer/nfsmw_trainer.c`) [source listing]; shell scripts and Makefiles |
| Target build | NFS:MW 2005 PC retail `speed.exe` [README] |
| Visibility | Public [github page] |
| Latest visible activity | README status line dated 2026-05-15; 18 commits on `main`; no commit date shown on the page [github page] |

## 2. What it does

User-visible or user-facing features, each with evidence.

- Infinite NOS: NOS never depletes while the trainer is active. Evidence: README (trainer section) names `Tweak_InfiniteNOS` [README].
- Infinite GameBreaker (race breaker / SpeedBreaker): the breaker never depletes. Evidence: README names `Tweak_InfiniteRaceBreaker` [README].
- Attribute lookup and name hashing from the CLI: `hash`, `lookup` subcommands map attribute names to hashes and back. Evidence: CLI subcommand listing [github tree] and README summary [README].
- JDLZ bundle decompression: `jdlz` subcommand. Evidence: CLI subcommand listing [github tree]; README says the JDLZ v0x02 algorithm has been reverse-engineered [README].
- Attribute listing: `attrs` subcommand [github tree].
- Save file verification: `verify-save` subcommand. README says save files use an MD5 trailer, not CRC [README].
- SDK data queries: `sdk` subcommand, reading the address/enum/struct JSON [github tree].
- Project coverage stats: `stats` subcommand; README says it prints coverage numbers [README].
- Reference documentation under `docs/`: architecture reference, anti-RE techniques, progress log, hash tables, Ghidra tooling notes, debugger plan [README].
- Findings the docs publish (stated, not verified here): bChunk hash is a Bob Jenkins mix3 variant with a specific seed; script VM is vanilla Lua 5.0.2; cutscenes use On2 VP6 video with MAD audio, not Bink; the AI does not use nitrous; the physics integrator runs on the main thread [README].

Status claims from the README (as of 2026-05-15): 294 of 345 attribute names cracked (85.2%); 6,664 functions named in Ghidra; HUD subsystem reported fully reverse-engineered; 27 subsystems mapped [README/github summary].

## 3. How it does it

### 3.1 Trainer (ASI mod)

- Packaging: a 32-bit PE DLL renamed to `.asi` for an ASI loader (the README names Ultimate-ASI-Loader as the loader used by the companion SDK; the trainer's own loader requirement is not stated in the fetched text) [README, inferred for loader use].
- Source file: `mods/infinite_trainer/nfsmw_trainer.c` [github tree].
- Mechanism as stated: on game load it patches two globals so they stay set. `Tweak_InfiniteNOS` at `0x937804` (NOS never depletes); `Tweak_InfiniteRaceBreaker` at `0x988E1C` (GameBreaker never depletes) [README]. The addresses are stated in the README; I did not read the C source, so they are not verified in code.
- Guard: a `validate_speed_exe()` function is said to stop the mod from applying to a non-NFS:MW process [README]. Not verified in code.
- Write helpers: `write_byte()` and `write_float()` are named as the way to add features [README]. Not verified in code.
- Hooks, DirectX hooks, function patches or code caves: none stated [README]. Not found in the fetched text. Treat as absent until the source is read.
- Build: `make mingw` (MinGW-w64, preferred) or `make` (winegcc) in `mods/infinite_trainer`; output `nfsmw_trainer.asi` [README]. The Makefile was not readable in the fetched output.

Confidence on the trainer: everything above is [README] except the file names [github tree]. No claim is [source].

### 3.2 CLI

- Language and layout: pure Python. Files: `tools/nfsmw-tool/nfsmw_tool.py`, `nfsmw-tool` (entry script), `generate_trainer.py`, `Makefile`, `README.md` [github tree].
- The CLI is a pure-Python single tool. The README says install via the folder's Makefile [README].
- The fetched output did not include source of `nfsmw_tool.py`, so the algorithms (hash, JDLZ decode, MD5 trailer check) are [README] only.

### 3.3 Launch helpers

- `launch-debug.sh` and `launch-then-attach.sh` at the repo root (names only) [github page]. Purpose not read; likely debugger launch, which is [inferred] from the names and the README's debugger plan.

### 3.4 Repo data

- `speed_renames.txt` at root: renamed-function list [github page; contents not read].
- `docs/`: hash tables, SDK JSON [README].

## 4. Configuration

- No configuration file is documented.
- Trainer tunables: `Tweak_InfiniteNOS` and `Tweak_InfiniteRaceBreaker` are the globals the trainer sets [README]. Defaults and whether they are user-editable at runtime are not stated.
- CLI: no config keys documented in the fetched text.

Not verified: default values, any `.ini` or `.cfg` keys. The README did not list them.

## 5. How we could implement it in nfs-mw

Licence: BSD 3-Clause. Code can be reused under the licence terms with attribution, but this project's rule is spec-first clean-room for physics/AI and no decomp or asset code (per project memory). Plan below uses facts and behaviour only; nothing is copied.

### 5.1 Behaviours to specify

- Infinite NOS and GameBreaker as a feature toggle: when enabled, the resource counters do not decrease. Spec the counter, the reset/refill logic and the input path, not the patch.
- Attribute name hashing and reverse lookup: name to hash, hash to name, over a documented hash function. Spec: the exact hash definition (seed, mix steps) and test vectors, derived from our own observations.
- JDLZ decompression: spec the container header, block framing, and output size rules for v0x02 from our own reading of bytes.
- Save verification: spec the trailer format (MD5 per README) and what "valid" means.
- Coverage stats and SDK queries as a CLI report.

### 5.2 Data and formats needed

- Attribute hash test vectors and our own list of attribute names (from our own traces).
- Save file samples from the user's own install (never shipped).
- JDLZ sample bundles from the user's own install (never shipped).
- Address tables are game-specific. They must not enter `libs/`.

### 5.3 Natural home

- Hash function, JDLZ decoder, save-trailer verifier: engine-generic format code goes in `libs/` (format crates take `&[u8]` and never open files, per AGENTS.md). Keep NFS:MW names and paths out of `libs/`.
- NFS:MW-specific bindings (attribute tables, NOS/GameBreaker globals, `speed.exe` offsets): `crates/`. `crates/` depends on `libs/`, never the reverse.
- Trainer-style runtime patching: not applicable. See 5.4.
- Install discovery: `game-install` (per AGENTS.md, only that crate touches the install).

### 5.4 What does not apply to a clean-room rewrite

- ASI loader packaging and in-process memory patching of `speed.exe`: loader-specific hacks. Our rewrite reads the game data from the install and runs its own engine, so there are no game globals to flip at runtime. The equivalent is a config/gameplay setting in our engine (for example, a resource-cost parameter set to zero), specified as a rule, not a patch.
- Hard-coded addresses (`0x937804`, `0x988E1C`): do not carry over. They belong to the original binary only.
- Lua 5.0.2 script VM and the Bink/VP6 video findings: facts about the original, usable only as spec notes after independent verification. Do not reuse the code.

### 5.5 Risks

- Using the repo's addresses or code would breach the clean-room rule for physics/AI and would import a BSD obligation (attribution) into the project. Keep the spec independent.
- Hash definition and JDLZ format are stated only in the README. Verify independently against the user's own install before relying on them.
- Trainer and CLI were not run or tested here. Their claims are stated, not checked.
- Adding a "cheat" feature is a product decision, not a technical one. Flag before scoping.
- Save format: the MD5 trailer claim is README-only. Verify on real save files before writing a verifier.

## 6. Open questions and what could not be verified

- Source code of `nfsmw_trainer.c`, `nfsmw_tool.py` and the Makefiles was not read. The fetched pages showed file lists and the README summary only. All trainer addresses and CLI algorithms are [README].
- The README fetch returned a summary, not verbatim text. Exact wording of the setup steps and any caveats was not captured.
- Last-commit date is not shown. The README status line is 2026-05-15.
- Whether the trainer needs a specific loader version: not confirmed. The README names Ultimate-ASI-Loader only for the companion SDK.
- Companion SDK repo (header-only SDK, "NFSPluginSDK" credited to berkayylmao, built on this project's data, works with Ultimate-ASI-Loader and BepInEx 6): not read. Its URL was not fetched.
- The README says 51 hashes remain uncracked. Count vs 345 total is consistent with 294 cracked, but not independently checked.
- A Monero donation section exists in the README (collapsed). Noted; not acted on.
- Acknowledged prior projects: NFSTools/Attribulator, MWisBest/OpenNFSTools, yugecin/nfsu2-re. Not read.

## 7. Sources

- https://github.com/s-b-repo/nfsmw-2005-re (repository page)
- https://raw.githubusercontent.com/s-b-repo/nfsmw-2005-re/main/README.md (README, read as a summary)
- https://raw.githubusercontent.com/s-b-repo/nfsmw-2005-re/main/LICENSE (licence)
- https://github.com/s-b-repo/nfsmw-2005-re/tree/main/mods/infinite_trainer (file listing and trainer section)
- https://github.com/s-b-repo/nfsmw-2005-re/tree/main/tools/nfsmw-tool (file listing and subcommands)
