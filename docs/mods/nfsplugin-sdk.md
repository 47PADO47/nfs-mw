# NFSPluginSDK (berkayylmao)

## Summary

NFSPluginSDK is a header-only C++20 library of reverse-engineered types from the BlackBox-era Need for Speed games (Underground 2, Most Wanted 2005, Carbon, ProStreet). Its purpose is to let plugin and script-mod authors reference the games' internal classes, fields and functions from their own code. The repository contains no executable, no loader and no injected code of its own. Two third-party mods named in the README (NFS Chat Chaos Mod and B.A.M.) are said to depend on it. Whether the SDK is used on its own or as part of a mod that is loaded into the game process is not stated in the repository.

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/berkayylmao/NFSPluginSDK |
| Author | Berkay Yigit (berkayylmao), per the LICENSE copyright line. Credits to @nlgxzef and @heyitsleo in the README. |
| Type | Header-only SDK (type definitions for plugins and script mods). Not an .asi plugin, not a tool, not a research write-up. |
| Licence | BSD 3-Clause License (LICENSE file, Copyright (c) 2022, Berkay Yigit). [README] and [source] agree. |
| Language | C++ (C++20 or later required, [README]). GitHub language breakdown was not visible to the fetch. |
| Target game build | Not pinned to a version or executable hash. The README gives completeness per game: Underground 2 (not in table), Most Wanted 2005 about 60%, Carbon about 40%, ProStreet about 75%. |
| Latest visible activity | Not visible. The repository page shows 386 commits and 50 stars, but the fetch did not return the date or message of the latest commit. |
| Visibility | Public. Fetched without authentication. |

## What it does

Each feature below is a capability the SDK provides to a consumer.

1. Game type identification. `enum class Game { NotSupported, U2, MW05, Carbon, ProStreet }` and `GetCurrentGame()` return which supported title is running. [source: NFSPluginSDK/NFSPluginSDK.h]
2. Window, D3D and DirectInput handles. `GetInformationOfCurrentGame()` returns a `GameInformation` struct holding the game enum, the window title string, and pointers to the main window handle, the Direct3D device and the DirectInput8 device. [source: NFSPluginSDK/NFSPluginSDK.h]
3. Game-specific class and function types. Namespaces such as `MW05::` provide types for attributes, the player vehicle, AI vehicles, and pursuit. The Game.MW05 folder is named in the README. [README] The README table marks MW05 as having Attrib, PVehicle, AIVehicle and Pursuit coverage. [README]
4. Example call. The README shows `MW05::Game::BlowEngine(MW05::PlayerEx::GetPlayerInstance());`, which calls a game function on the player object. [README] This example was not verified against the headers, because the MW05 subfolder was not readable (see section 6).

## How it does it

Mechanism claims are tagged as follows: [source] verified in the code, [README] stated only in the README, [inferred] my reading.

- Header-only. There is no build step; consumers include the header for their game. [README] [source: NFSPluginSDK.h has no build or link dependency beyond the standard library]
- Standard library only in the top-level header. `<cstdint>` for `std::uintptr_t` and `<cstring>` for `std::strstr`. [source: NFSPluginSDK.h]
- Game detection by string search at fixed addresses. `GetCurrentGame()` runs `std::strstr` against the title string at four hardcoded addresses (0x789694, 0x8AF684, 0x9E9E94, 0x98E9A0). The header does not state which address belongs to which game. [source] [inferred] This only makes sense inside the game's own process address space. A consumer must therefore be running in-process, so the SDK itself implies a code-injection or in-process plugin setup, but the repository does not show how that process is entered.
- Fixed addresses for handles. `GetInformationOfCurrentGame()` uses hardcoded addresses per game for the window name, the window handle pointer, the D3D device pointer and the DI8 device pointer. For U2 the D3D and DI8 addresses are 0. [source]
- Pointers are returned, not dereferenced. The header stores the addresses and does not read them. The caller dereferences. [source]
- No patches, no hooks. The header contains no detours, no memory writes and no function hooks. [source] Hook or patch code, if any, would live in the consumer mods, which are not in this repository.
- No loader. The repository does not contain an .asi loader, an injector or a launcher. [source: repository file listing] The README does not mention an ASI loader. [README] If the consumer mods use an ASI loader, that is [inferred], not stated.
- Game function calls. The README example calls a game function through the SDK's class types. How the call is resolved (direct address, vtable, or a symbol table) is not visible in the header I read. The MW05 subfolder was not readable, so this remains unverified.

## Configuration

None. The README and header show no config file, no environment variables, and no runtime keys. [README] [source]

The only configurable input is the choice of game header and the pointer values passed into `GameInformation`. Both are compile-time or caller-supplied, not read from a file.

## How we could implement it in nfs-mw

Ground rules for this page:

- The repository is licensed BSD 3-Clause, so its code could in principle be copied with attribution. nfs-mw's own rule is spec-first clean-room: the behaviour and facts below are what we record, and we do not copy the header. Any port of the type layout must come from our own reading of the install (per `docs/licensing.md` and the spec-first process in `CONTRIBUTING.md`), not from this repository.
- The nfs-mw project does not run in-process mods. It reads the D:\ install and has its own engine. Most of this SDK is therefore not applicable to a clean-room rewrite.

What applies:

- Behaviours to specify (if we need them at all):
  - Identify which supported title is installed from data files on disk, not from memory addresses. Our existing install discovery in `libs/` already covers this kind of check [inferred from `docs/architecture.md`, not verified here].
  - Expose the attribute, vehicle and pursuit data that the SDK lists as coverage, but as data read from the install, not as in-process type layouts.
- Data or formats needed: the NFS:MW data files for attributes and vehicles. These are in the gameplay reader already described in the repo's `docs/formats` material [inferred from commit `84dab2db docs(formats): record the gameplay race and zone reading`]. The SDK's own address tables are not needed.
- Natural home:
  - Anything that reads or describes the NFS:MW attribute and vehicle data goes in `crates/` as NFS:MW-specific code, since it names NFS:MW concepts.
  - Any generic data-reading helpers go in `libs/`, with no NFS:MW names, per the code-organisation rule in `AGENTS.md`.
  - Crates depend on libs, never the reverse.
- Not applicable to a clean-room rewrite:
  - The fixed-address scanning in `GetCurrentGame()`.
  - The handle lookup for window, D3D and DI8 devices. Our rewrite owns its own window and graphics backends (see the backend list in `docs/architecture.md`), so there are no game-owned handles to borrow.
  - The loader-specific assumptions. Any ASI-loader or injection approach is out of scope. The nfs-mw rule is to read the install and never patch the game process.
- Risks:
  - Addresses are tied to specific executable builds. Copying them would mean matching a build we have not verified.
  - Coverage claims (for example MW05 about 60%) are the author's and cannot be checked from this repository.
  - Licence: BSD 3-Clause is permissive, but attribution would be required if any code were copied. Our spec-first rule avoids that question by not copying.
  - Using the SDK as a dependency would import a C++ header library into a Rust project, which is not a fit.

Verdict: this SDK is a reference for what the Blackbox-era types look like. It is not something nfs-mw would adopt. The useful output for us is an independent spec of attribute and vehicle data, derived from the install.

## Open questions and what could not be verified

- The MW05 subfolder (`NFSPluginSDK/Game.MW05/`) was not readable through the fetch. The GitHub tree URL returned HTTP 404 (the default branch may not be `main`). The per-game type layouts, the BlowEngine signature, and the "about 60%" coverage claim are therefore unverified.
- The mapping of the four hardcoded addresses in `GetCurrentGame()` to specific games was not stated in the fetched output. Not verified.
- The build or version of the MW05 executable the addresses target is not stated. Not verified.
- The latest commit date and message were not visible. The 386-commit count and the 50 stars come from the repository page as fetched. Activity level is therefore unknown.
- Whether the two mods named in the README (NFS Chat Chaos Mod and B.A.M.) are public, and how they load into the game, was not checked. Not verified.
- Whether the consumer mods use an ASI loader was not checked. [inferred] only.
- The GitHub language breakdown was not visible. C++ comes from the README.
- Whether the repository has open issues, releases or CI workflows was not checked. The `.github/` folder exists; its contents were not read.

## Sources

- https://raw.githubusercontent.com/berkayylmao/NFSPluginSDK/HEAD/README.md
- https://github.com/berkayylmao/NFSPluginSDK
- https://raw.githubusercontent.com/berkayylmao/NFSPluginSDK/HEAD/LICENSE
- https://raw.githubusercontent.com/berkayylmao/NFSPluginSDK/HEAD/NFSPluginSDK.h
- https://github.com/berkayylmao/NFSPluginSDK/tree/main/NFSPluginSDK (HTTP 404, not read)
