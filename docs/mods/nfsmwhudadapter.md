# NFSMWHUDAdapter

NFSMWHUDAdapter is a small Visual C++ DLL that, according to its repository description, auto-adjusts HUD objects for the player's aspect ratio. The repository is marked "(WIP)". The code found so far shifts two groups of HUD objects (left and right) horizontally by an offset derived from the screen resolution, and it also patches minimap-related values. It is a plugin for Need for Speed: Most Wanted (2005, PC). It is not a loader, a tool, or a research repository. The repository has no README and no licence file that could be read.

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/nlgxzef/NFSMWHUDAdapter |
| Author | nlgxzef (repository owner; no other author named) |
| Type | Plugin (native DLL, `DllMain` entry point). Not confirmed to be `.asi` by name. |
| Licence | None stated. No LICENSE file was found in the root listing, and no licence text was seen. Treat as "none stated": no reuse rights. |
| Language | C++ (Visual Studio project: `.sln`, `.vcxproj`) |
| Target game build | Need for Speed: Most Wanted (PC). Exact executable version not stated. The code uses fixed addresses that only fit one build (see section 3). |
| Latest visible activity | Not visible. The repo page showed "2 Commits" and no commit date or message. |
| Repo stats (page view) | About 33 stars, 3 forks, 2 open issues, 5 watching (as reported by the repository page summary; not re-verified) |
| Status | Description says "(WIP)" |

## 1. What it does

Features as stated or visible in the code.

- Aspect-ratio HUD adjustment. The repository description says it "Auto-adjusts HUD objects according to your aspect ratio." [README: repository description on the GitHub repo page]. Basis: the hook computes an offset from the resolution (see section 3).
- Left HUD group shifted left and right HUD group shifted right, by the same amount, so the HUD spreads out on wider screens. [source, via fetch summary of `dllmain.cpp`]
- Minimap adjustment. The hook writes a minimap offset value and a pointer to it. [source, via fetch summary of `dllmain.cpp`]
- Behaviour falls back to a 4:3 offset of 0 when the resolution is zero or unknown. The code comment describes this as 4:3. [source, via fetch summary of `dllmain.cpp`]
- Not active (commented out in the code): EA Trax chyron object, wrong-way sign, and some other call sites that the author marked as "not working properly". [source, via fetch summary of `dllmain.cpp`]

No other user-visible features were found. There is no README to list more.

## 2. Repository contents

Root (from the GitHub repo page):

- `includes/` (folder) containing `injector/` with `CPatch.h`, `IniReader.h`, `ini_parser.hpp`, `stdafx.h` [source, listing of `includes/injector`]
- `.gitattributes`, `.gitignore`
- `NFSMWHUDAdapter.sln`, `NFSMWHUDAdapter.vcxproj`
- `dllmain.cpp`, `stdafx.cpp`, `stdafx.h`, `targetver.h`

Not present in the root listing: `README.md`, any `LICENSE` file, any `.ini` config file.

Note that `includes/injector/` contains a header-only patching helper (the header `injector.hpp` is referenced by the code; it was not in the listing, so it may be a file the listing did not show, or a missing dependency. Not verified).

## 3. How it does it

### Mechanism

The code is a DLL. `DllMain` runs `Init()` on `DLL_PROCESS_ATTACH`. [source, via fetch summary of `dllmain.cpp`]

It does not use DirectX hooks. [source: no DirectX references found in the code summary]

It does not name an ASI loader, and it does not read a config file. [source: `IniReader.h` is included but unused]

Init-time patches use the injector helper library, `injector::MakeCALL`, to redirect two call sites to a hook function. [source, via fetch summary]

The hook is `cFEng_QueuePackageMessage_Hook`. It is `__stdcall` and returns `1`. It runs when the game queues a front-end (FEng) package message, which is how the HUD gets rebuilt. [inferred from the hook name and the call sites]

### Game functions bound by address

These addresses come from the code as summarised by the fetch tool. They are specific to one game build.

| Name | Address | Declared signature |
|---|---|---|
| `FE_Object_SetCenter` | `0x525050` | `void __cdecl (DWORD*, float, float)` |
| `FE_Object_GetCenter` | `0x524EE0` | `void __cdecl (DWORD*, float*, float*)` |
| `FEObject_FindObject` | `0x524850` | `void* __cdecl (const char*, unsigned int)` |
| `FEngGetScript` | `0x5249D0` | `DWORD* __cdecl (char const*, unsigned int, unsigned int)` |

### Game data read

| Address | Meaning |
|---|---|
| `0x93CAD8` | Screen width (read by the hook) |
| `0x93CADC` | Screen height (read by the hook) |

### Memory writes and call redirects

| Address | Action | Purpose (from code) |
|---|---|---|
| `0x5696F1` | `MakeCALL` redirect to hook | Active |
| `0x569712` | `MakeCALL` redirect to hook | Active |
| `0x5678AA` | Pointer write of `&DifferenceMin` | Minimap |
| `0x5678B4` | Float write of `DifferenceMin` | Minimap |
| `0x5678C6` | Pointer write of `&Difference` | Minimap |
| `0x58D8C3`, `0x58D86F`, `0x58D7F7` | Call sites | Commented out; "not working properly" |

### Hook logic

1. Read width and height from `0x93CAD8` and `0x93CADC`.
2. Compute `Difference = (ResX / ResY) * 240 - 320`. If either value is zero, `Difference = 0`. [source, via fetch summary; the summariser did not confirm whether the division is integer or float, so this is open]
3. Set `DifferenceMin = -Difference`.
4. Write the minimap values listed above.
5. Find object hash `0x1603009E` in the front-end package and move its centre X by `-Difference` (left group).
6. Find object hash `0x5D0101F1` and move its centre X by `+Difference` (right group).
7. Return `1`.

Object lookup is by hash, not by name. The hash-to-object mapping was not decoded in this review.

### Confidence

- Everything under "Game functions" and "Hook logic" is [source, via fetch summary]. The fetch tool would not return the file verbatim, so these are summaries of the code, not exact copies. Re-check against the raw file before relying on any address or sign.
- Repository description: [README: GitHub repo page description].
- "No DirectX, no config file, no named ASI loader" [source, via fetch summary].

## 4. Configuration

None found. The code does not read a config file. `IniReader.h` is included but not used. There are no keys or defaults to document.

## 5. How we could implement it in nfs-mw

Clean-room rule: the repository has no licence, so no code may be copied. The notes below describe behaviour and facts only. The spec must be written from the game's own data and our own observation, not from this source.

### Applies to a clean-room rewrite?

Partly. The behaviour (keep HUD elements anchored to the screen edges on wider aspect ratios) applies directly to our Rust rewrite, which already has a FEng/HUD model. The memory patching, call redirects, DLL entry point and fixed-address hooks do not apply: our rewrite reads the install and renders itself, so there is no game process to patch. Loader-specific hacks are not needed.

### Behaviours to specify

1. HUD layout offset: given a viewport aspect ratio and a reference 4:3 layout, compute a horizontal offset for each anchor group (left and right). Specify the reference resolution and the exact formula, including rounding and the zero-size case.
2. Anchor groups: which HUD elements belong to which group, and whether a group should move or scale. Decide per element, not from the hash table.
3. Minimap: the offset that moves the minimap, and whether it should follow the same rule.
4. Disabled elements (EA Trax chyron, wrong-way sign): decide whether they are in scope. The source comments them out, so the default is out of scope.
5. Fallback: behaviour when the resolution is unknown.

### Data or formats needed

- The FEng package layout for the HUD screens, and the object identifiers for each HUD element. These must come from our own parse of the game data (see the `feng-viewer` CLI commit in the repo history, which browses FEng screens). Do not use the hashes from this repo as a source.
- Screen resolution from our own window or render target, not from a fixed memory address.

### Natural home

- Engine-generic layout maths (anchor offsets, aspect-ratio scaling): `libs/`. It must not contain NFS:MW names, paths or defaults.
- NFS:MW-specific HUD group membership and reference values: `crates/`. `crates/` depends on `libs/`, never the reverse.
- Per-version tables for layout values, if they differ by game build, go in per-version layout tables (see AGENTS.md).

### Risks

- The formula is simple but the HUD element set is not. Getting the group membership wrong moves the wrong items.
- The source author marked some call sites as not working. The reason is not recorded.
- The game addresses are build-specific. Any spec derived from them would be tied to one build.
- Integer versus float division in the offset formula changes results at some resolutions. Must be settled in the spec.
- A no-licence repository cannot be a source of code or text. Keep the spec independent.

## 6. Open questions and what could not be verified

- The README does not exist in the repository root, and the `master` branch raw URL returned 404 for `README.md`. No other README name was tried. The repo description says "(WIP)", so the author may not have written one yet.
- No licence file was found in the root listing. A LICENSE in another branch was not checked.
- Only two commits are visible. The commit dates and messages were not shown on the page. Latest activity is unknown.
- The source file could not be read verbatim. All code claims come from a model summary of the file. Re-check `dllmain.cpp` directly before using any address.
- The target executable version is not stated. The addresses suggest one specific NFS:MW build, but which one is unknown.
- `injector.hpp` is included by the code but is not in the `includes/injector/` listing. The dependency may be missing or may be inside a sub-folder that was not expanded.
- The repo's `.vcxproj` was not read, so the target platform, configuration and output name are unknown.
- Whether the DLL is loaded through an ASI loader, and which one, is not stated.
- Whether the hook actually works on current game builds, or on any build, was not tested. We did not run or build anything.

## 7. Sources

- https://github.com/nlgxzef/NFSMWHUDAdapter (repo root, read via WebFetch; description, file list, stats)
- https://github.com/nlgxzef/NFSMWHUDAdapter/tree/master/includes (folder listing, read via WebFetch)
- https://raw.githubusercontent.com/nlgxzef/NFSMWHUDAdapter/master/dllmain.cpp (read via WebFetch; returned a summary, not verbatim)
- https://raw.githubusercontent.com/nlgxzef/NFSMWHUDAdapter/master/README.md (HTTP 404)
- https://github.com/nlgxzef/NFSMWHUDAdapter/blob/master/README.md (HTTP 404)
- https://github.com/nlgxzef/NFSMWHUDAdapter/blob/master/LICENSE (HTTP 404)
