# NFSMWBeta (nlgxzef/NFSMWBeta)

Summary: NFSMWBeta is a small C++ DLL that patches the original Need for Speed: Most Wanted (2005, PC) executable in memory. Its README describes it as the companion script for the NFSMW Beta Content Mod by elaymm4, bundled with that mod so users do not compile or download it separately. Per the code, it adds or re-enables a few menu options (Gauges in Parts, Challenge Series in Quick Race), disables some options, and changes a few gameplay and UI constants. The repository has no licence, a 2021 commit history, and is read here only through web fetches.

## 1. Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/nlgxzef/NFSMWBeta |
| Author | nlgxzef (GitHub account; no other identity shown) |
| Type | Game patch DLL (native C++ module that patches game memory at load time) |
| Licence | None stated. No LICENSE file in the top-level listing; the README has no licence statement. |
| Language | C++ (Visual Studio solution and project files; file extensions only, the page shows no language bar) [inferred] |
| Target game build | The DllMain entry-point check compares against 0x7C4040, and patch addresses are absolute. Exact build/version not stated [inferred: one specific NFS:MW PC executable] |
| Latest visible activity | 3 commits on `main`, all dated 2021-11-18: "Initial commit", "v1.1.2", "Update README.md" [source: commits page] |
| Stars / forks | 2 stars, 1 fork, 1 watcher (GitHub page) |
| Companion mod | NFSMW Beta Content Mod by elaymm4, https://nfsmods.xyz/mod/1711 [README] |

## 2. What it does

Each feature below is taken from the code summary of dllmain.cpp. Treat them as the intended behaviour of the code, not as tested behaviour.

- Gauges option in the Parts menu. A code cave is hooked into the Parts customisation screen so a Gauges entry appears there. [source: dllmain.cpp, PartsCustomGaugesCodeCave]
- Challenge Series option in Quick Race. A new menu option is added to Quick Race that opens ChallengeSeries.fng. [source: dllmain.cpp, ChallengeSeriesCodeCave and ChallengeSeriesCodeCave2]
- Visual and Backroom customisation options disabled. Two ranged NOPs remove the Visual option and the Backroom option from the customise category screen. [source: dllmain.cpp, Init()]
- Main menu option disabled. Two jumps hide an option in the Main menu. [source: dllmain.cpp, Init()] The option is not named in the summary.
- Bounty interval changed from 10 seconds to 5 seconds. Two float writes set a value of 0.2f, which the code comment describes as a change from 10 to 5 seconds. [source: dllmain.cpp, WriteMemory of ZeroPointTwo at 0x444513 and 0x444524]
- Autosave fix. Two byte writes change a value at 0x5A7C72 and 0x5A8260 to 2. The code comment says "autosave fix". [source: dllmain.cpp, Init()]
- Pause menu options. Two dword writes change PauseMenu::SetupOptions call sites at 0x52B7A0 and 0x52C18B. Purpose not stated in the summary. [source: dllmain.cpp, Init()]
- "Light Streaks??" flag. A byte at 0x9017E0 is set to 1. The code comment is a question mark, so the purpose is unknown. [source: dllmain.cpp, Init()]

Not applied in the code: an AutoDrive dialog hook (AutoDriveDialogCodeCave) is defined but its MakeJMP call is commented out. [source: dllmain.cpp]

## 3. How it does it

Mechanism, as read from the source summary:

- Entry: DllMain on DLL_PROCESS_ATTACH checks that the host process entry point equals 0x7C4040. If it matches, it calls Init(). If not, it returns FALSE. [source: dllmain.cpp]
- Patch helpers: MakeJMP, MakeRangedNOP and WriteMemory (templated for BYTE and DWORD) come from an `injector` header library. The summary says the file includes stdafx.h and an injector header. The injector header itself is not in the top-level file listing and was not fetched. [source for calls; the header is unverified]
- Code caves: naked functions (PartsCustomGaugesCodeCave, ChallengeSeriesCodeCave, ChallengeSeriesCodeCave2, ChallengeSeries_React) are jumped into from patched game code. They call game functions directly. [source]
- Game functions called by absolute address: CustomizeIsInBackRoom at 0x575B00, CustomizeCategoryScreen_AddCustomOption at 0x007BB560, ShowThreeButtons at 0x598FA0, j_malloc at 0x652AD0, IconOption_Create at 0x586FA0, IconScrollerMenu_AddOption at 0x573960, cFEng_QueuePackageSwitch at 0x525940. FEngSetLanguageHash at 0x525220 is declared but not used in the code shown. [source]
- Game data: cFEng::mInstance at 0x91CADC; a read at 0x905EC4; a write of 3 at 0x9B9E68; strings at 0x89CE1C (ChallengeSeries.fng) and 0x89736C (InGameDialog.fng, inactive path). [source]
- Vtable replacement: a vtable slot at 0x901424 is set to ChallengeSeries_React, and the vtable pointer at 0x901420 is set to 0x7B4950. [source]
- Hash keys: menu option identifiers are compared by 32-bit hash values (for example 0x307, 0x78980A6B, 0x28F88BC for Gauges; 0xCC8CB746, 0x9A962438 for Challenge Series). The hash function is not in the summary. [source]
- Config files: none read. No DirectX hooks. No ASI loader calls are visible in the summary. [source] Whether it is used through an ASI loader in practice is not stated. [inferred: the README says it ships inside the mod, which suggests a loader-based drop-in, but this is not shown]

All addresses are fixed to one executable build. Any other build would need different addresses.

## 4. Configuration

No configuration file, command-line switch or INI key was found in the source summary. All behaviour is compiled in. Values that can be changed are constants in the source:

| Setting | Value in code | Meaning |
|---|---|---|
| Bounty interval | 0.2f written at two sites | Changed from 10 to 5 seconds per the comment |
| Autosave byte | 2 | Autosave fix |
| Light Streaks flag | 1 | Unknown purpose |

## 5. How we could implement it in nfs-mw

Applicability: most of this does not apply to a clean-room rewrite. The repo is a binary patcher for the original executable. It writes to fixed memory addresses, hooks game functions, and uses naked assembly-style code caves. nfs-mw is a Rust rewrite that does not run the original executable, so there is nothing to patch and no loader to target. Address values, hash keys and game-function pointers are facts about one original binary and should not be used as the basis of rewrite code.

What still applies, as behaviour to specify:

- Menu entries. Gauges in the Parts screen, Challenge Series in Quick Race, and the disabled Visual, Backroom and Main-menu options. Specify each option's visibility and destination screen, then implement in the menu layer. Keep the option set data-driven so that options can be enabled or hidden per game version.
- Bounty interval. Specify the bounty timer as a gameplay parameter with a default value and a unit (seconds), and a source for it. Implementation: a config value in the gameplay crate, not a memory write.
- Autosave behaviour. The source states only a value change. Specify the intended autosave behaviour from the original game and from user expectations before implementing. Do not infer it from the byte change.
- Pause menu options. Specify what the pause menu should show. The summary does not say what the two changed call sites do.

Natural homes:

- Menu-option layout, screen flow and the option table: crates/ (NFS:MW-specific screens and options). The FEng-like screen framework, if generic, belongs in libs/.
- Bounty and other tunable gameplay values: a data or config format read by a crate; the format reader takes &[u8] under the project rule and does not open files. Game-install discovery stays in game-install.
- Anything engine-generic (a config loader, a settings-file parser) belongs in libs/, with no NFS:MW-specific names in it.
- Dependency rule: crates/ may depend on libs/, never the reverse.

Clean-room rule: the repository has no licence, so its code must not be copied or adapted. Use only the behaviour listed above as the spec, write it in docs/specs/, and implement from that spec.

Risks:

- The source is a summary from a model-based fetch, not a verbatim read. Details such as the exact values, the hash function, and the purpose of each patch need confirmation from a direct read of the file before any spec relies on them.
- Hash keys and addresses are tied to one build. Reusing them in a rewrite would couple the rewrite to that build.
- Behaviour changes (bounty interval, Challenge Series) may conflict with the original game's data, which the rewrite must match first.
- Option visibility may depend on the mod's content; that content is not in this repository.

Loader-specific hacks (memory patching, code caves, hooks into a running process) do not apply to a clean-room rewrite and should not be ported.

## 6. Open questions and what could not be verified

- Licence: none stated. Under our rules, no code may be copied. This is a no-licence repository.
- The README was fetched as rendered text. It contains only a title and one description paragraph. Installation steps, the feature list and any usage notes are not present in what was fetched.
- The source files were read through a summarising web fetch, not verbatim. Exact values, hash constants and the full patch list should be re-checked against the raw file if a spec depends on them.
- Not read: stdafx.h, stdafx.cpp, targetver.h, NFSMWBeta.vcxproj, NFSMWBeta.sln, the injector header. Language and build settings are inferred from file extensions.
- The game build the addresses target is not stated. The entry-point check (0x7C4040) suggests a specific build but was not matched to a known version.
- The purpose of the Pause menu patch, the Main menu option hidden, and the Light Streaks flag are not explained by the code or README.
- The loader used to inject the DLL is not stated.
- Commit history after 2021-11-18: none visible. The repository may be inactive since then.
- The GitHub page footer showed "You can't perform that action at this time." The repository content still loaded.
- The companion mod page (nfsmods.xyz/mod/1711) was not fetched.

## 7. Sources

- https://github.com/nlgxzef/NFSMWBeta (repository page; metadata, file listing, stars and forks)
- https://raw.githubusercontent.com/nlgxzef/NFSMWBeta/main/README.md (README; the fetch returned a title and description only)
- https://raw.githubusercontent.com/nlgxzef/NFSMWBeta/main/dllmain.cpp (source; read as a model-summarised fetch, not verbatim)
- https://github.com/nlgxzef/NFSMWBeta/commits/main (commit list and dates)
- Linked from README, not fetched: https://nfsmods.xyz/mod/1711
