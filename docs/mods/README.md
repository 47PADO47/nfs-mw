# Community mods and tools: reference index

One page per repository, written from its README, file listing and main source files. Each page covers what the
project does, how it does it (hooks, addresses, config), how we could implement the same behaviour in the Rust
rewrite, and what could not be verified.

**Licence rule.** Only permissive licences (MIT, BSD, CC0, Unlicense) allow copying code. Our project is
MIT OR Apache-2.0, so GPL and AGPL pages are spec-only. For "none stated" or "conflicting" licences, treat the code
as all rights reserved. Behaviour and facts can still be described and re-implemented from a spec.

Facts are tagged `[source]` (checked in code), `[README]` (stated only in docs) or `[inferred]`. Several pages were
read through summarising fetches rather than raw files; those pages say so, and their addresses need re-checking
before use.

## Loaders, SDKs and tools

| Page | Repository | Licence | What it is |
|---|---|---|---|
| [ultimate-asi-loader.md](ultimate-asi-loader.md) | ThirteenAG/Ultimate-ASI-Loader | MIT | The `.asi` loader the DLL mods depend on |
| [nfsplugin-sdk.md](nfsplugin-sdk.md) | berkayylmao/NFSPluginSDK | BSD-3-Clause | C++ SDK for writing NFS plugins |
| [mirrorhook.md](mirrorhook.md) | berkay2578/MirrorHook | MIT | Hook library used for reflection/mirror work |
| [opennfstools.md](opennfstools.md) | MWisBest/OpenNFSTools | none stated | Tools for NFS data files (e.g. VLT) |
| [nfsmw-save-editor.md](nfsmw-save-editor.md) | x07x08/nfsmw-save-editor | Unlicense | Save editor |
| [ealayer3-nfsw.md](ealayer3-nfsw.md) | driftyz700/ealayer3-nfsw | BSD-3-Clause | EALayer3 audio encoder/decoder CLI (not a plugin) |
| [most-wanted-vehicles-decomp.md](most-wanted-vehicles-decomp.md) | Brawltendo/Most-Wanted-Vehicles-Decomp | none stated | Vehicle decompilation (README only; EA-owned code) |
| [nfsmw-2005-re.md](nfsmw-2005-re.md) | s-b-repo/nfsmw-2005-re | BSD-3-Clause | Reverse-engineering trainer and CLI |
| [nfsmw-recomp.md](nfsmw-recomp.md) | veritr1x/nfsmw-recomp | MIT | Native recompilation project (Xbox 360 build) |
| [nfsmw-recompiled.md](nfsmw-recompiled.md) | madelrandel-blip/NFSMW-Recompiled | GPL-3.0 | Xbox 360 recompilation on the ReXGlue SDK |
| [nfs-online-server.md](nfs-online-server.md) | moio9/NFS-Online-Server | AGPL-3.0-or-later | Unofficial emulator of EA's online services (spec only) |

## Widescreen, reflections, shadows and effects

| Page | Repository | Licence | What it is |
|---|---|---|---|
| [widescreenfixespack.md](widescreenfixespack.md) | ThirteenAG/WidescreenFixesPack | MIT | Widescreen and aspect-ratio fixes |
| [xboxraindroplets.md](xboxraindroplets.md) | ThirteenAG/XboxRainDroplets | MIT | Rain droplets on the windscreen, Xbox-style |
| [sweetfx.md](sweetfx.md) | CeeJayDK/SweetFX | MIT | Post-processing shader framework (LumaSharpen etc.) |
| [nfsmwhdreflections.md](nfsmwhdreflections.md) | sniten/NFSMWHDReflections | GPL-3.0 | Higher-resolution road, car and mirror reflections |
| [nfshdreflections.md](nfshdreflections.md) | AeroWidescreen/NFSHDReflections | GPL-3.0 | HD reflections across NFS titles |
| [nfshdcontentsupport.md](nfshdcontentsupport.md) | AeroWidescreen/NFSHDContentSupport | GPL-3.0 | Content support for the HD reflections mod |
| [nfsmwfeshadows.md](nfsmwfeshadows.md) | AeroWidescreen/NFSMWFEShadows | GPL-3.0 | Dynamic shadows in front-end menus |
| [nfsmwmotionblurcontroller.md](nfsmwmotionblurcontroller.md) | nlgxzef/NFSMWMotionBlurController | none stated | Motion blur path and scale controls |
| [nfsmwhudadapter.md](nfsmwhudadapter.md) | nlgxzef/NFSMWHUDAdapter | none stated | Shifts HUD objects for the player's aspect ratio (WIP) |
| [nfsmw_xenoneffects.md](nfsmw_xenoneffects.md) | xan1242/NFSMW_XenonEffects | MIT | Xenon-style effects from the console build |
| [nfsmw_enistimeofday.md](nfsmw_enistimeofday.md) | xan1242/NFSMW_ENISTimeOfDay | MIT | Fix for the `ENISTimeOfDay` handler so cutscenes show the right time |
| [nfsmw_sunset.md](nfsmw_sunset.md) | ArchieGoldmill/NFSMW_SunSet | conflicting | Sunset graphics mod (redistribution limits; not reusable) |
| [nfsmw_photomode.md](nfsmw_photomode.md) | Hydra455/NFSMW_PhotoMode | conflicting | Photo mode ("like Carbon"); not reusable |
| [nfsmwantagonist.md](nfsmwantagonist.md) | rng-guy/NFSMWAntagoNISt | CC0 | Replaces background traffic and police in selected NIS cutscenes |
| [nfs_customfont.md](nfs_customfont.md) | xan1242/NFS_CustomFont | MIT | Custom font replacement |

## Gameplay, police and fixes

| Page | Repository | Licence | What it is |
|---|---|---|---|
| [mwfixes.md](mwfixes.md) | GrimMaple/mwfixes | GPL-3.0 | Fixes to the game's behaviour (spec only) |
| [nfsmw-redux-fixes.md](nfsmw-redux-fixes.md) | 1Filipek2/nfsmw-redux-fixes | none stated | Fix scripts for the REDUX pack |
| [nfsmwracefixes.md](nfsmwracefixes.md) | ExOptsTeam/NFSMWRaceFixes | none stated | Fixes and extra Quick Race modes; last commit Nov 2023 |
| [nfsmwspeedfixer.md](nfsmwspeedfixer.md) | rng-guy/NFSMWSpeedFixer | CC0 | Makes the Speedbreaker values configurable |
| [nfsmwdaylightsavingtime.md](nfsmwdaylightsavingtime.md) | rng-guy/NFSMWDaylightSavingTime | CC0 | Stops in-game time advancing while paused, scales it with play |
| [nfsmwbartender.md](nfsmwbartender.md) | rng-guy/NFSMWBartender | CC0 | Pursuit behaviour exposed as configurable data (Basic and Advanced sets) |
| [nfsmwfairbusts.md](nfsmwfairbusts.md) | eatincrispies/NFSMWFairBusts | MIT | Removes the instant bust on car reset during pursuit |
| [nfsmwhelicopteroptions.md](nfsmwhelicopteroptions.md) | eatincrispies/NFSMWHelicopterOptions | MIT | Per-heat-level police helicopter tuning |
| [nfsmwrevisedairsupport.md](nfsmwrevisedairsupport.md) | eatincrispies/NFSMWRevisedAirSupport | MIT | Restores unused helicopter radio speech and fixes police radio |
| [nfsmwslingshotfeature.md](nfsmwslingshotfeature.md) | Kevin4e/NFSMWSlingshotFeature | all rights reserved | Drafting boost meter that pushes the car forward |
| [nfsmwhpc.md](nfsmwhpc.md) | nlgxzef/NFSMWHPC | none stated | Companion DLL for the Hot Pursuit Challenges mod |
| [mwhealthbars.md](mwhealthbars.md) | trelbutate/MWHealthbars | none stated | Health bar above each police car |
| [nfsmwbeta.md](nfsmwbeta.md) | nlgxzef/NFSMWBeta | none stated | Companion memory-patch DLL for the Beta Content Mod |
| [nfsmw_feloader.md](nfsmw_feloader.md) | bluesky-dev12/NFSMW_FELOADER | none stated | Replaces the front-end garage platform (port of NFSC_FELoader) |
| [xnfsmusicplayer.md](xnfsmusicplayer.md) | xan1242/XNFSMusicPlayer | MIT | Custom music player (bundles BASS, separately licensed) |
| [nfs-xtendedinput.md](nfs-xtendedinput.md) | xan1242/NFS-XtendedInput | MIT | Extended input support |

## Earlier pages (not from this batch)

- [redux-v3.md](redux-v3.md), [redux-v3-plugins.md](redux-v3-plugins.md): the REDUX V3 pack and its plugins.
- [nfsmw-extra-options.md](nfsmw-extra-options.md), [nfsmw-unlimiter.md](nfsmw-unlimiter.md),
  [nfsmw-unlimiter-essentials.md](nfsmw-unlimiter-essentials.md): earlier mod references.
