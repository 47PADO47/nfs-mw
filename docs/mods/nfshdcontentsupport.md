# NFS HD Content Support

Repository: https://github.com/AeroWidescreen/NFSHDContentSupport

NFS HD Content Support is a community plugin for several old Need for Speed titles, released as an ASI script loaded by Ultimate-ASI-Loader. It corrects how HD-resolution fonts, the mouse cursor and full-motion video (FMV) are scaled, so that high-resolution assets display at the correct size. It does not ship textures or videos. The repository holds C++ source for one DLL per supported game, a `scripts` release folder for each game, and a GPL-3.0 licence. The owner archived the repository on 1 November 2024, so it is read-only and no further changes are expected.

## Metadata

| Field | Value |
|---|---|
| Repository | AeroWidescreen/NFSHDContentSupport |
| Author | Aero_ (AeroWidescreen). Credits also name the ExOpts Team, whose Extra Options project this one is based on [README] |
| Type | ASI plugin (DLL loaded through Ultimate-ASI-Loader) with C++ source |
| Licence | GNU GPL v3.0 (LICENSE file, dated 29 June 2007) [source] |
| Language | C++ (Visual Studio solution, `.sln` and `.vcxproj`) [source] |
| Target game builds | Need for Speed: Most Wanted v1.3 English `speed.exe` (6,029,312 bytes) is the only build whose checks were read in code [source]. ProStreet, Carbon, Underground and Underground 2 are listed in the README [README] |
| Latest visible activity | Last commit 1 November 2024, "updated readme". Repository archived the same day [source] |
| Stars / forks | 30 / 5 at time of reading [source] |
| Download | Hosted on nfsmods.xyz (mod 2151) [README]. Not fetched; see Sources |

## 1. What it does

Each feature is listed with its evidence. "Game" refers to the titles in the README.

- **HD Font support.** Scales specific in-game fonts so that high-resolution font textures render at the intended size. Applies to Most Wanted, Carbon, ProStreet, Underground and Underground 2 [README]. Font identity and scale rules are in the Most Wanted DLL [source].
- **HD Cursor support.** Corrects the mouse cursor's X and Y offset when the cursor is drawn at HD size. Applies to Most Wanted, Carbon, ProStreet, Underground and Underground 2 [README]. Most Wanted implementation is in `SOURCE/NFSMW/dllmain.cpp` [source].
- **HD FMV support.** Forces the FMV (video) scale to a fixed size so that HD videos fill the screen. Applies to Most Wanted and Underground 2 [README]. Most Wanted implementation is in `dllmain.cpp` [source].
- **Global Neon and Global Shadow.** Loads one neon or shadow texture, from `GlobalB.lzc`, for all cars. Applies to Underground and Underground 2 [README]. Not present in the Most Wanted DLL; the `settings.h` flags `GlobalNeon`, `GlobalShadowIG` and `GlobalShadowFE` exist but are not implemented in code that was read [source, partial].
- **Widescreen splash flag.** `settings.h` declares a `WidescreenSplash` flag and an `HDTextureSupport` flag. Neither is described in the README, and no code using them was read [source, unverified use].

## 2. How it does it

Mechanism as read from the Most Wanted source. The other titles' source folders were listed but not read.

**Loading.** The plugin is an ASI script. The README says to place `dinput8.dll` (Ultimate-ASI-Loader) in the game folder and to place the `scripts` folder beside it [README]. The DLL's `DllMain` runs on `DLL_PROCESS_ATTACH`, checks the host executable, then calls `Init()` [source].

**Version gate.** `DllMain` computes the executable's entry point and compares it with the fixed address `0x7C4040` (based on the image base `0x400000`). If the check fails, it shows a message box saying the `.exe` is not supported and that v1.3 English `speed.exe` is required, then does not load [source]. This ties the plugin to one binary.

**Hooks.** Hooks are installed with `injector::MakeJMP` (a 5-byte `JMP rel32` patch) only when the matching feature is enabled [source]:

| Feature | Hook address(es) | Code cave |
|---|---|---|
| Font | `0x585D70` | `FontScaleCodeCave1` |
| Font | `0x6D14F6` | `FontScaleCodeCave2` |
| Cursor | `0x57045C`, `0x570481`, `0x570496`, `0x5704BF` | `CursorScaleCodeCave1` to `CursorScaleCodeCave4` (X/Y offsets of 16) |
| FMV | `0x599F16` | `FMVScaleCodeCave` |

[source, from a summarised read of `dllmain.cpp`; addresses should be re-checked against the binary before reuse]

**Font logic.** Each font cave reads a hashed font-name value at offset `+0x24` of the font object and compares it with known hashes [source]:

| Font name (as labelled in code) | Hash | Scale |
|---|---|---|
| FONT_ARIAL | `0xA87927BE` | 256 (0x100) |
| FONT_ARIAL12 | `0xAB6215C1` | 256 (0x100) |
| FONT_MW_BODY | `0x545570C6` | 256 (0x100) |
| FONT_MW_TITLE | `0xE0463AFA` | 512 (0x200) |
| FONT_MW_CUSTOM_NUMBERS | `0xC7BD87CE` | 128 (0x80) on one path, 64 (0x40) on the other |
| TAC_NUMBERS | `0x01424892` | 128 (0x80) on one path, 64 (0x40) on the other |

Fonts that do not match keep their original scale, read from offset `+0x44` [source]. Cave 1 and cave 2 put the 64 value in different registers (`esi` and `eax`) [source].

**FMV logic.** The FMV cave forces the scale to 1024 (X) and 512 (Y) [source].

**Cursor logic.** Four hooks each shift the cursor by a 16-pixel X or Y offset [source].

**Memory helper.** A `CPatch` class wraps `VirtualProtect` and `memcpy`. It provides `Patch`, `Nop`, `RedirectCall` (`0xE8`), `RedirectJump` (`0xE9`), typed setters, and `AdjustPointer`, which rebases pointers that fall inside a range [source, `SOURCE/includes/CPatch.h`]. `Unprotect` does not restore page protection, and the int-based helpers assume 32-bit addresses [source].

**Config.** `CIniReader` reads `NFSMWHDContentSupport.ini` next to the `.exe` by default. Integers starting with `0x` are parsed as hex. Booleans accept `0` or "false" (case-insensitive) as false [source, `SOURCE/includes/IniReader.h`].

## 3. Configuration

Most Wanted DLL, file `NFSMWHDContentSupport.ini`, section `[GENERAL]` [source]:

| Key | Default | Effect |
|---|---|---|
| `HDFontSupport` | 1 | Enables the font hooks |
| `HDCursorSupport` | 1 | Enables the cursor hooks |
| `HDFMVSupport` | 1 | Enables the FMV hook |

The other titles' configuration was not read. The README says Global Neon and Global Shadow are set in an INI file and that the INI file's instructions should be read before enabling them, but it names no keys [README].

## 4. How we could implement it in nfs-mw

The plugin is a patch set for one executable build. A clean-room Rust rewrite does not need the ASI loader, the DLL proxy, the version gate or the byte-level hooks. What carries over is the set of behaviours and the data they depend on.

**Clean-room rule.** This repository is GPL-3.0, and the rewrite is MIT OR Apache-2.0. Do not copy or port code from it. Write the behaviour spec below from observed game behaviour and the facts on this page, then implement from the spec. Keep this page as a fact record, not a code reference. Per `docs/licensing.md`, the spec-first process applies.

**Behaviours to specify.**
1. Font scaling: for each font the game draws, a scale factor chosen by font identity, with a fallback to the font's own scale. The spec must say how font identity is determined without relying on a memory address.
2. Cursor offset: a fixed pixel offset applied when the HD cursor is drawn.
3. FMV scale: the display size used for FMV playback, as a fixed output resolution.
4. Enable flags: per-feature on/off, with defaults on.
5. Global Neon and Global Shadow (Underground titles): one shared texture per effect, applied to all cars. Out of scope for Most Wanted; the spec must cover it only if those titles are targeted.

**Data and formats needed.**
- Font records and their identity keys in the Most Wanted asset files (the hashed names). These should be read from the install by the reader in `game-install` or a format crate, not hard-coded. The fact that the hash is a 32-bit value of the font name must be confirmed against the data before use.
- Cursor and FMV sizes from the game's own resources or the video format reader.
- A settings file format (the rewrite's own config, not the `.ini` keys unless compatibility is needed).

**Natural home.**
- Generic pieces go in `libs/`: a scaling policy type (a table from resource key to scale, with a default), a config-flag reader, and the overlay or cursor offset helper. No NFS:MW names in `libs/`.
- Game-specific pieces go in `crates/`: the Most Wanted font table, cursor offsets, FMV resolution and the mapping from font identity to scale. Per-version tables are passed in by the caller, as `AGENTS.md` requires.
- `crates/` depends on `libs/`, never the reverse.
- Rendering is in a renderer backend crate, not in the core. Check `docs/decisions/0004-swappable-renderers.md` before adding a Bevy-based backend.

**Does not apply to a clean-room rewrite.**
- ASI loader, `dinput8.dll` proxy, `DllMain` setup and `scripts` folder layout.
- The `0x7C4040` entry-point gate. The rewrite reads the install through `game-install`, so it can identify the build from data.
- `CPatch`, code caves and `JMP` hooks. The rewrite has no need to patch the running game's code.

**Risks.**
- Font identity keyed on hashes or memory offsets is fragile across builds. The rewrite should key on resource names or IDs read from data.
- Scale values were chosen by the original author by eye. Validate against screenshots before treating them as correct.
- FMV and cursor fixes interact with the widescreen fix. The README says the widescreen fix handles aspect ratio, so the rewrite's aspect logic must not be duplicated or fought over.
- The Underground-era Global Neon/Shadow feature depends on a shared texture file whose format was not read here.

## 5. Open questions and what could not be verified

- **Other titles.** The `SOURCE/NFSC`, `NFSPS`, `NFSU` and `NFSU2` folders were listed but not read. Feature and hook claims for those titles rest on the README only [README].
- **Release folder.** Only two folders appeared in the partial `SOURCE/Release` listing: `NFSU2HDContentSupport/scripts` and `NFSUHDContentSupport/scripts`. The rest was not listed. The NFS:MW scripts folder was not seen [source, partial].
- **Hook addresses and hashes.** Taken from a summarised fetch of `dllmain.cpp`, not a byte-for-byte read. Re-check them before relying on them.
- **Full source.** The fetch tool would not return full file contents, so the page relies on summaries for `dllmain.cpp`, `CPatch.h` and `IniReader.h`. Line-level claims are not verified.
- **`settings.h` flags.** `HDTextureSupport` and `WidescreenSplash` are declared but not described in the README. Their use was not found.
- **Download.** The release binaries were not downloaded (per the task rules). The `scripts` contents, `.asi` names and any bundled `.ini` were not inspected.
- **Commit count.** The repository page reports 20 commits; the commit history page showed 19 in view. Both are noted; the count does not affect any claim here.
- **Access.** One WebFetch returned a "You can't perform that action at this time" notice. The repository content still loaded.

## 6. Sources

- https://github.com/AeroWidescreen/NFSHDContentSupport (repository page)
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDContentSupport/HEAD/README.md (README, read as a summary)
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDContentSupport/HEAD/LICENSE (licence, read as a summary)
- https://github.com/AeroWidescreen/NFSHDContentSupport/tree/main/SOURCE (folder listing)
- https://github.com/AeroWidescreen/NFSHDContentSupport/tree/main/SOURCE/NFSMW (folder listing)
- https://github.com/AeroWidescreen/NFSHDContentSupport/tree/main/SOURCE/includes (folder listing)
- https://github.com/AeroWidescreen/NFSHDContentSupport/tree/main/SOURCE/Release (partial folder listing)
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDContentSupport/HEAD/SOURCE/settings.h (read as text)
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDContentSupport/HEAD/SOURCE/NFSMW/dllmain.cpp (read as a summary, several queries)
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDContentSupport/HEAD/SOURCE/includes/CPatch.h (read as a summary)
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDContentSupport/HEAD/SOURCE/includes/IniReader.h (read as a summary)
- https://github.com/AeroWidescreen/NFSHDContentSupport/commits/main (commit history)
- https://nfsmods.xyz/mod/2151 (download page cited in the README; not fetched)
