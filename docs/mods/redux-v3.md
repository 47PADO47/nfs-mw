# NFS:MW REDUX V3 mod pack

Reference notes on [LYNXFAME/NFS-Most-Wanted-REDUX-V3](https://github.com/LYNXFAME/NFS-Most-Wanted-REDUX-V3),
a community pack of prebuilt plugins for the PC game (2005). This page covers what the pack contains, how
it installs, and how it relates to this port. The per-plugin key reference is in
[redux-v3-plugins.md](redux-v3-plugins.md).

**Licence: none.** The repository has no LICENSE file, so its files are all rights reserved by default
([licensing.md](../licensing.md#repositories-with-no-license)). This page records **facts** only: file
names, config keys, defaults, behaviour. It does not copy any config file, comment, binary, texture or
icon. Nothing from the pack is committed to this repository, and nothing from it is linked into the port.

## Snapshot inspected

| Item | Value |
|---|---|
| Repository | `LYNXFAME/NFS-Most-Wanted-REDUX-V3` (not a GitHub fork) |
| Commit | `7f0817e` (2026-09-28), 103 tracked files |
| Language | Batchfile on GitHub; the content is prebuilt Windows binaries and INI files |
| Licence | none |
| README | Says the pack is "my personal vision of this timeless game"; no credit list; the screenshot links point to `Ele3wwn/NFS-Most-Wanted-REDUX-V3`, which `gh` could not resolve (so the upstream was not checked) |

The inspection read the repository's files from a local clone. Nothing was run. The installer was read
from git's object store, not executed.

## Contents

| Group | Files | Plugin reference |
|---|---|---|
| Loader and wrapper | `dinput8.dll` (Ultimate ASI Loader), `d3d9.dll` (d3d9-wrapper) | [Loader](redux-v3-plugins.md#loader-and-wrapper) |
| Widescreen and HUD | `NFSMostWanted.WidescreenFix.asi`, `buttons-xbox.tpk`, `buttons-playstation.tpk` | [WidescreenFix](redux-v3-plugins.md#nfsmostwantedwidescreenfixasi) |
| Cars and Quick Race | `NFSMWUnlimiter.asi`, `UnlimiterData/` (63 per-car files and 6 shared files) | [Unlimiter](redux-v3-plugins.md#nfsmwunlimiterasi-and-unlimiterdata) |
| Options and cheats | `NFSMWExtraOptions.asi` | [ExtraOptions](redux-v3-plugins.md#nfsmwextraoptionsasi) (spec: [specs/extra-options.md](../specs/extra-options.md)) |
| Race modes | `NFSMWRaceFixes.asi` | [RaceFixes](redux-v3-plugins.md#nfsmwracefixesasi) |
| Graphics | `NFSMWGraphics.asi`, `NFSMWFEShadows.asi`, `NFSMWHDReflections.asi`, `NFSMWHDContentSupport.asi`, `NFSMWMotionBlurController.asi`, `BloomResIncreaser.asi`, `SpeedReflect.asi`, `XNFS-ShaderLoader-MW.asi`, `MW360Tweaks.asi` | [Graphics plugins](redux-v3-plugins.md#graphics-plugins) |
| Next-gen graphics | `NextGenGraphics.MostWanted.asi`, `NextGenGraphics.MostWanted.log`, `fmt.dll` | [NextGenGraphics](redux-v3-plugins.md#nextgengraphicsmostwantedasi) |
| Xbox 360 conversion | `X360STUFF.bat`, `IDI_CAR_FX` | [X360STUFF.bat](redux-v3-plugins.md#x360stuffbat-and-idi_car_fx) |
| Installer | `Redux_Setup.exe`, `FirewallInstallHelper.dll`, `GameuxInstallHelper.dll` | [Installer](#installation) |

There are 14 `.asi` plugins and 11 shipped INI files. Plugins without an INI take their defaults.

## Installation

### As the author describes it

The README gives four steps: extract all files, run the setup, wait 5 to 10 minutes, then play.

### What is verified about the installer

- `Redux_Setup.exe` is an **Inno Setup** package (`Inno Setup` header at offset 87848 in a 1,397,760 byte
  file; the file is tracked at `HEAD`). SHA-256 `d4d72ae8b72ec6219c6f463076ea77daf8ba64dfa1c3b667d03e8ae3672a44f5`.
- Its embedded strings include a .NET attribute and an "installer files are corrupted" message. Its
  payload was **not extracted**, so the files it writes, the registry entries it sets and whether it
  patches `speed.exe` are **[unverified]**.
- In the working clone used for this review the file disappeared mid-session (git reports it deleted),
  likely removed by a security scanner. It was not run.

### Manual layout (what the pack needs at run time)

The files that matter are the two proxy DLLs and the `scripts/` folder, copied into the game folder that
holds `speed.exe`:

```
<game folder>/
├── speed.exe              (v1.3 English build; see Target build below)
├── dinput8.dll            ASI loader; loads scripts/*.asi on start
├── d3d9.dll               d3d9-wrapper; reads d3d9.ini if present (none shipped)
└── scripts/               all .asi plugins, every .ini, UnlimiterData/, fmt.dll, buttons-*.tpk, NextGenGraphics/
```

This matches the layout the port already documents for the user's own install
([install-layout.md](../install-layout.md)). The loader part is the same mechanism.

### Target build

Every plugin's start-up message names the same build: **"Please use v1.3 English speed.exe (5,75 MB
(6.029.312 bytes))"**, and `NFSMWHDReflections.asi` names "v1.3 SPEED.exe from RELOADED" (6,029,312 bytes).
The port's retail check uses the same size and the SHA-256 `80774c2e…1d253c`
([research.md](../research.md#language-and-toolchain-verified)). So the plugins target the retail 1.3
executable, not a repacked one. **[verified]** from the plugin strings.

## Relation to the port

The pack is an external mod, not something the port depends on. Its value to this project is as a
reference for behaviour that the port has or plans to have:

| Plugin behaviour | Where the port covers it |
|---|---|
| ExtraOptions keys (`GameSpeed`, `NoRevLimiter`, `Headlights`, rain, sound and music mute, and others) | [specs/extra-options.md](../specs/extra-options.md): the same file format, applied keys and the not-applied list |
| WidescreenFix HUD and FOV correction | [specs/hud-viewport.md](../specs/hud-viewport.md), [provenance/hud-viewport.md](../provenance/hud-viewport.md) |
| SpeedReflect and car part texture replacement | [specs/car-assembly.md](../specs/car-assembly.md) |
| Replacing `RT_RCDATA` shader resources in `speed.exe` (the mechanism `X360STUFF.bat` uses for `IDI_CAR_FX`, `IDI_SKYBOX_FX` and `IDI_SCREENFILTER_FX`) | [formats/shaders.md](../formats/shaders.md) |
| Quick Race modes, Unlimiter car and traffic limits, front-end shadows, HD reflections, motion blur controller, next-gen texture loading, Xbox 360 bloom and motion-blur shaders | Not documented elsewhere in the repo; the per-plugin reference is the only description |

Where a plugin's behaviour is needed for the port, cite the plugin's reference section and the
port's own measurement, not the plugin's code.

## Licensing and provenance rules applied here

- **Facts only.** Key names, default values, file layouts and observed behaviour are recorded. Config
  text, comments and shader or texture contents are not copied.
- **No binaries committed.** The `.asi`, `.dll`, `.exe`, `.tpk`, `.bat` and `.bin` files stay out of the
  repository. `.gitignore` already blocks them, and `cargo xtask leak-check` checks it.
- **Per-plugin attribution.** Author names come from the plugins' own start-up strings (for example
  "by Aero_"). The plugin reference names an author only where a string does.
- **Unlicensed upstreams.** The pack bundles community plugins whose original repositories (where they
  exist) may carry their own licences. Those were not checked. Treat every plugin's licence as
  unknown until its upstream is found.

## Known gaps in the pack

| Gap | Effect |
|---|---|
| `NextGenGraphics/TexturePacks/`, including the "HRT v7" pack named in the bundled log, is not shipped | The next-gen texture loader starts with no packs. The bundled log is from an earlier run on 2022-02-15. |
| `LODLights.asi`, referenced by `NFSMWGraphics.asi` and `XNFS-ShaderLoader-MW.asi`, is not shipped | Any behaviour it adds is missing; the plugins' own reference to it is unverified. |
| `NFSMWBeta.asi`, named as a string in `NFSMWUnlimiter.asi`, is not shipped | Unknown; no effect that can be seen from the repo. |
| `d3d9.ini` is not shipped | The d3d9-wrapper runs with defaults. |
| `NFSMWHDContentSupport.ini` is not shipped | That plugin runs with defaults; its purpose is only inferred. |
| `X360STUFF.bat` needs `ResourceHacker.exe`, a `.CT` cheat table, `.tpf` texture packs and the `X360DATA/` folder, none of which are in the repo | The batch file stops at its first missing-file check. |
| The installer's contents are unknown (see above) | The manual layout in this page is the only verified install path. |

## Using the pack safely

- Use a legally owned copy of the game and back up `speed.exe` and `GLOBAL/` before any install.
- `X360STUFF.bat` writes to the game folder and patches `speed.exe`. Do not run it as part of this pack.
- ExtraOptions contains cheats and debug tools (unlock all, auto-drive, freeze car, heat-level hack).
  Keep them out of a career save you care about.
- Verify binaries against an upstream release before use. This review could not do that.

## See also

- [redux-v3-plugins.md](redux-v3-plugins.md): the per-plugin reference and every shipped config key.
- [install-layout.md](../install-layout.md): what the port documents about the user's own mod install.
- [licensing.md](../licensing.md): the rules for unlicensed sources.
