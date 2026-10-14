# Most-Wanted-Vehicles-Decomp

## Summary

Most-Wanted-Vehicles-Decomp is a research project by Brawltendo that works toward a matching decompilation of the vehicle physics code in Need for Speed: Most Wanted (2005, PC). It is not a mod, plugin, or game patch. It is a set of C++ source files, PowerShell and Python helper scripts, and a workflow for compiling those sources and diffing the output against the original `speed.exe` disassembly. The README says the code is "NOT fully usable or pretty" and is intended "purely for research purposes." Only the README was read in detail; the source files could not be fetched (see section 6).

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/Brawltendo/Most-Wanted-Vehicles-Decomp |
| Author | Brawltendo (GitHub account) |
| Type | Research / decompilation project (not an .asi plugin, tool, or mod) |
| Licence | None stated. No LICENSE file was found, and the sidebar shows no licence. The README says the original code "belongs to Electronic Arts." |
| Language | Not shown by GitHub. Files referenced: C++ (`.cpp`), PowerShell (`.ps1`), Python (`.py`). |
| Target build | Need for Speed: Most Wanted (2005, PC), `speed.exe`, built with MSVC 7.1 |
| Default branch | `master` |
| Commits | 49 (per the GitHub page) |
| Latest activity | Not visible in the pages read. No commit date was shown. |

## 1. What it does

The project gives a researcher a way to rebuild selected Most Wanted vehicle physics functions from C++ and check that the compiled assembly matches the original. The user-visible result is not a feature in the game; it is a development workflow.

- Compiles a single `.cpp` file from the `../speed/indep/src` tree, such as `physics/behaviors/chassis.cpp`, and writes cleaned-up assembly for diffing. [README]
- Disassembles the original `speed.exe` with `dumpbin /disasm:NOBYTES`. [README]
- Extracts one function's assembly from that disassembly, given start and end addresses, for comparison. [README]
- Optional VS Code `tasks.json` that binds the compile script to Ctrl+Shift+B. [README]

## 2. How it does it

Mechanism: this is not an ASI loader plugin. Nothing in the README runs inside the game process. There are no hooks, patches, DirectX hooks, memory writes, or config files for the game. The work is entirely offline.

Build and compare workflow, from the README:

1. `compile_and_gen_asm.ps1 <relative .cpp path>` compiles the file with `cl.exe` (MSVC 7.1 / Visual Studio .NET 2003) and writes formatted assembly. [README]
2. `dumpbin /disasm:NOBYTES` produces a disassembly of `speed.exe`. [README]
3. `asm_splitter.py <assembly file> <start addr> <end addr> <output>` isolates one function. [README]
4. The two assembly outputs are compared with a diff tool to check for an exact match. [inferred: the README says the tool produces "assembly for diffing" and "matching assembly"; the diff tool is not named.]

Code quirks the README calls out on purpose, to make the output match: misuse of `volatile`, temporary variables added to steer register allocation, and unusual code structure. [README]

Not verified: the contents and exact logic of `compile_and_gen_asm.ps1`, `devenv_setup.ps1`, and `asm_splitter.py`. The source files were not read (section 6). Game function names, addresses, and the physics model are not verified.

## 3. Configuration

No runtime configuration exists for the game. The only setting named in the README is the Visual Studio .NET 2003 install path, which is set in `devenv_setup.ps1` when VS2003 is installed outside the default location. [README] The key names and default values inside that script were not verified.

## 4. Requirements (from the README)

- Visual Studio .NET 2003 (MSVC 7.1, the compiler used for the original game). [README]
- Python 3. [README]
- Windows and PowerShell, since the helper script is `.ps1`. [inferred]

## 5. How we could implement it in nfs-mw

Important: this repository has no licence. Under the rules in `docs/licensing.md` and `CONTRIBUTING.md`, its code must not be copied, ported, or adapted. Everything below is behaviour and facts only, written from the README, to be specified and then built clean-room.

Does the feature apply to a clean-room rewrite? Partly. The "match the original binary" workflow applies, but it is a verification method, not a feature to copy.

- The MSVC 7.1 compile and `dumpbin` steps are specific to the original toolchain. Our Rust rewrite has no need to compile MSVC C++ to match `speed.exe`. These steps do not carry over.
- The useful idea is a verification harness: check the rewritten physics against recorded reference behaviour from the original game. This is spec-first work. It belongs in the spec and test plan, not in a code port.
- Loader-specific hacks: not applicable. Nothing in this repository is an ASI loader patch, so none needs to be replicated.

Behaviours to specify (from the physics area, to be confirmed against our own specs and the game data, not against this repo's code):

- Reference traces: for a given vehicle and input sequence, record the state (position, velocity, forces) the original game produces, and compare our output frame by frame within a tolerance.
- Function-level boundaries: a way to name a physics function and select its range for comparison. This is a test-tooling concern, not an engine feature.

Data or formats needed:

- Original game data: the vehicle and physics tuning data already read by `game-install` and the format crates. Do not read or ship game assets in the repo. Use the install.
- Reference traces: a format for recorded traces, stored outside the repo (the repo is a whitelist-only `.gitignore`, and game data must not be committed).

Natural home:

- Engine-generic code (a trace comparison library, numeric tolerance checks, a frame-diff reporter): `libs/`. It must stay Bevy-free and must not name NFS:MW.
- NFS:MW-specific code (the vehicle physics model, the reference trace definitions, the install-specific paths): `crates/`. `crates/` depends on `libs/`, never the reverse.
- Any game-specific layout data goes into per-version layout tables, per `AGENTS.md`.

Risks:

- Licence: copying any code or structure from this repo breaks the project's licence rules. Keep it as a reference for facts only, and record that in the spec.
- Scope: a matching decompilation is a large effort. It is not needed for a clean-room rewrite; we only need physics that matches observed behaviour.
- Legal and data: the original game is copyrighted, and the README says the original code belongs to Electronic Arts. We must not publish or import original code or assets.
- Accuracy: the README states the code is not fully usable, so it must not be treated as a source of truth for the physics. Any value taken from it must be re-verified against the game.

## 6. Open questions and what could not be verified

- The source tree was not read. The raw GitHub URLs for `compile_and_gen_asm.ps1` and `asm_splitter.py` returned HTTP 404 on the `master` branch. The files probably sit in a subfolder (for example `Speed/` or `packages/`), which was not confirmed. The `Speed/` folder listing also returned 404 on the path tried.
- The licence: no LICENSE file was found on the root listing or the sidebar. The root listing ends with "View all files," so a licence file could be in a subfolder. Not verified.
- Latest commit date and activity: not shown on the pages read.
- The GitHub language breakdown was not shown.
- The contents of `packages/snd/9/`, `.gitattributes`, and `.gitignore` were not read.
- Game addresses, function names, and the vehicle physics model: not verified. Nothing in the repo was checked against the game.
- Whether `dumpbin` output was compared with a particular diff tool: not stated.

## 7. Sources

Read or attempted, with the result:

- https://github.com/Brawltendo/Most-Wanted-Vehicles-Decomp (repository page, read via WebFetch; gave the README summary, file list, default branch `master`, 49 commits, no licence shown)
- https://raw.githubusercontent.com/Brawltendo/Most-Wanted-Vehicles-Decomp/master/README.md (README, read via WebFetch; main source of this page)
- https://github.com/Brawltendo/Most-Wanted-Vehicles-Decomp/tree/main/Speed (HTTP 404)
- https://raw.githubusercontent.com/Brawltendo/Most-Wanted-Vehicles-Decomp/main/README.md (HTTP 404; branch is `master`)
- https://raw.githubusercontent.com/Brawltendo/Most-Wanted-Vehicles-Decomp/main/LICENSE (HTTP 404)
- https://raw.githubusercontent.com/Brawltendo/Most-Wanted-Vehicles-Decomp/master/compile_and_gen_asm.ps1 (HTTP 404)
- https://raw.githubusercontent.com/Brawltendo/Most-Wanted-Vehicles-Decomp/master/asm_splitter.py (HTTP 404)
