# NFS:MW REDUX V3: plugin and script reference

Per-file reference for the scripts and configs in the REDUX V3 pack. The pack and its install are in
[redux-v3.md](redux-v3.md). Every config key below is listed with its shipped default and a short
description written for this repository. The INI files and the binaries are not copied here
([licensing.md](../licensing.md#repositories-with-no-license)).

Evidence tags, as in [docs/README.md](../README.md#evidence-tags):

- **[verified]**: checked in the pack's files (INI contents, plugin strings, the batch file).
- **[inferred]**: from file names and strings only; not run or tested.
- **[unverified]**: not checked.

Defaults are the values in the shipped INI. "Not shipped" means the plugin reads a file the pack does not
contain, so it runs with its built-in defaults.

## Loader and wrapper

### `dinput8.dll`: ASI loader

**[verified]** The strings are those of ThirteenAG's Ultimate ASI Loader, which the binary's build path
also names. At start-up it loads every `.asi` in the game's `scripts/` folder. Its strings also include
`.\modloader\modloader.asi` and `modupdater.asi`. Nothing in the pack places those two files, so they
are unused here **[verified]**.

### `d3d9.dll`: d3d9-wrapper

**[verified]** The binary names ThirteenAG's d3d9-wrapper and reads `d3d9.ini`. The pack does not ship
that file, so the wrapper runs with its defaults.

### `FirewallInstallHelper.dll`, `GameuxInstallHelper.dll`

**[verified]** These export Windows installer helper functions. Firewall: `CanLaunchMultiplayerGameA/W`,
`SetMSIFirewallProperties`. Game Explorer: `AddToGameExplorer*`, `RemoveFromGameExplorer`,
`SetMSIGameExplorerProperties`, and media-centre registration. They are Microsoft helpers for MSI
installers, not mod code. Their use by `Redux_Setup.exe` is **[unverified]**.

---

## `NFSMostWanted.WidescreenFix.asi`

**[verified]** Widescreen and HUD correction, FOV correction, gamepad icon replacement, shadow and
motion-blur options, windowed modes, and frame pacing. Reads `NFSMostWanted.WidescreenFix.ini`. Its
strings also name `buttons-xbox.tpk` and `buttons-playstation.tpk` (the two icon packs it swaps in), a
`Settings.ini` in the game's profile folder, and internal video and texture-filter settings.

HUD and FOV behaviour is also described from the port's side in
[specs/hud-viewport.md](../specs/hud-viewport.md).

| Section | Key | Default | Meaning |
|---|---|---|---|
| `[MAIN]` | `ResX`, `ResY` | `0`, `0` | Render resolution override; `0` leaves the game's own choice. |
| `[MAIN]` | `FixHUD` | `1` | Corrects the HUD for the window's aspect ratio. |
| `[MAIN]` | `FixFOV` | `1` | Corrects the field of view for the aspect ratio. |
| `[MAIN]` | `Scaling` | `1` | FOV scaling when `FixFOV` is on: `0` original, `1` Xbox 360 behaviour, `2` proportionally correct. |
| `[MAIN]` | `HUDWidescreenMode` | `1` | Moves HUD elements to the screen edges at 16:9. |
| `[MAIN]` | `FMVWidescreenMode` | `2` | Movie aspect: `1` cropped, `2` stretched to fill. |
| `[MISC]` | `SkipIntro` | `0` | Skips the movies at launch. |
| `[MISC]` | `WindowedMode` | `0` | Window mode: `0` fullscreen, `1` borderless, `2` bordered, `3` bordered with resizing. |
| `[MISC]` | `ShadowsRes` | `1024` | Dynamic shadow map size; `2048` matches the Xbox 360. Also enables shadows on Intel GPUs. |
| `[MISC]` | `AutoScaleShadowsRes` | `0` | Scales `ShadowsRes` with the aspect ratio. Can cost performance. |
| `[MISC]` | `ShadowsFix` | `1` | Keeps dynamic shadows in tunnels and under bridges. |
| `[MISC]` | `ImproveShadowLOD` | `0` | Higher level of detail for shadows; can cost performance. |
| `[MISC]` | `ForceEnableMirror` | `1` | Rear-view mirror shown in every camera view. |
| `[MISC]` | `CustomUserFilesDirectoryInGameDir` | `SAVE` | Folder inside the game folder for user files; `0` disables. |
| `[MISC]` | `WriteSettingsToFile` | `1` | Stores registry settings in `settings.ini` in the profile folder. |
| `[MISC]` | `ImproveGamepadSupport` | `1` | Gamepad icons: `1` Xbox, `2` PlayStation, `3` none. |
| `[MISC]` | `LeftStickDeadzone` | `10.0` | Left-stick dead zone. |
| `[MISC]` | `ForceHighSpecAudio` | `1` | Forces 44.1 kHz audio. |
| `[MISC]` | `SimRate` | `-1` | Simulation rate: `-1` monitor refresh, `-2` twice the refresh, `0` disables the frame-rate patches. |
| `[MISC]` | `DisableMotionBlur` | `0` | Turns motion blur off without changing the registry. |

---

## `NFSMWUnlimiter.asi` and `UnlimiterData/`

**[verified]** Lifts content limits and adds customisation. The plugin reads
`NFSMWUnlimiterSettings.ini` and the folder `UnlimiterData/`. Its strings name the files it reads
(`_General.ini`, `_RimBrands.ini`, `_CopDestroyedStrings.ini`, `_FNGFixes.ini`, `_BonusCars.ini`,
`_CarManufacturers.ini`, and one `CARNAME.ini` per car), and the texture and paint names it swaps.

The upstream project and its car parts pack are documented separately in
[nfsmw-unlimiter.md](nfsmw-unlimiter.md) and [nfsmw-unlimiter-essentials.md](nfsmw-unlimiter-essentials.md).
The keys below are this pack's build, which differs from the current upstream settings file.

### `NFSMWUnlimiterSettings.ini`

| Section | Key | Default | Meaning |
|---|---|---|---|
| `[Main]` | `ReplacementModel` | `1` | Car type ID used when a requested model is missing or invalid (`1` is CARRERAGT). |
| `[Main]` | `TrafficCarCount` | `10` | Traffic cars per pattern (maximum 127). |
| `[Main]` | `EnableManufacturerHook` | `1` | Reads manufacturer names from `_CarManufacturers.ini`. |
| `[Main]` | `EnableExtraCustomization` | `1` | Enables the extra options in `_General.ini`. |
| `[Main]` | `MyCarsBackroom` | `1` | Shows the junkman parts backroom in My Cars. |
| `[Main]` | `EnableCopDestroyedStringHook` | `1` | Reads cop-destroyed text from `_CopDestroyedStrings.ini`. |
| `[Main]` | `EnableRandomPartsHook` | `1` | Lets AI opponents use random parts from each car's file. |
| `[Main]` | `EnableBonusCarsHook` | `1` | Reads bonus cars from `_BonusCars.ini` (listed as expert only). |
| `[Fixes]` | `DisappearingWheelsFix` | `1` | Stops wheels vanishing after focus loss (open-wheel cars). |
| `[Fixes]` | `SecondaryLogoFix` | `1` | Shows the secondary logo when the manufacturer logo is missing. |
| `[Fixes]` | `AddOnCopsDamageFix` | `1` | Damageable parts on add-on cop cars. |
| `[Fixes]` | `ChallengeSeriesOpponentNameFix` | `1` | Opponent names outside Career. |
| `[Fixes]` | `PaintMenuFix` | `1` | Paint menu scrolls correctly with more than 80 colours. |
| `[Fixes]` | `FNGFix` | `1` | Clones missing front-end objects, per `_FNGFixes.ini`. |
| `[Misc]` | `ExpandMemoryPools` | `1` | Larger memory pools; fixes flicker. |
| `[Misc]` | `ForceStockPartsOnAddOnOpponents` | `0` | Stock parts on add-on opponents (needs `EnableRandomPartsHook`). |

The plugin's strings also include `CarSkinFix` and `DisableTextureReplacement`. Neither is in the shipped
INI, so they keep the plugin's built-in value **[verified: strings only]**.

### `UnlimiterData/`

63 per-car files named by the car's folder name (for example `BMWM3.ini`, `911GT2.ini`, `964.INI`) and
six shared files starting with `_`. Each car file is optional: a missing one falls back to `_General.ini`.
**[verified]**

**`_General.ini`** has four sections:

- `[Main]`: `ForceLODA` (`0`: use the highest level of detail only if the car has no lower LODs) and
  `EngineType` (`-1` not set, `0` piston, `1` Castrol, `2` rotary; picks the upgrade names shown).
- `[Parts]`: which part families appear in the menu, each `1` shown or `0` hidden: `BodyKits`,
  `Spoilers`, `Rims`, `Hoods`, `RoofScoops`, `Interior`, `Roof`, `Brakes`, `Headlights`, `Taillights`,
  `Mirrors`, and `Attachments` (a count, 0 to 10).
- `[Visual]`: which paint and decal options appear: `Paint`, `Vinyls`, `RimPaint`, `WindowTint`,
  `Decals` and the per-side decal keys, `Numbers`, `CustomGauges`, `Driver`, `LicensePlate`, `Tires`.
- `[RandomParts]`: the parts AI opponents may get. Adds `ForceCustomizationLevel`, `CustomRandomParts`,
  and `Attachment0` to `Attachment7`.

**Per-car files** repeat the `[Main]` and `[Parts]` sections with the same keys, and their comments say
the part names follow the pattern `CARNAME_(KITxx_)BODY`, `CARNAME_(STYLExx_)SPOILER`, and so on. The
shipped copies are **[verified]** and not reproduced here.

**Shared files:** `_BonusCars.ini` (sections `[BonusCars]`, `[BonusCar-1]` to `[BonusCar29]`),
`_CarManufacturers.ini` (`[Manufacturers]`), `_RimBrands.ini` (`[RimBrands]`, `[Brand0]` to `[Brand30]`),
`_CopDestroyedStrings.ini` (`[CopDestroyedStrings]`, `[String1]` to `[String11]`), and `_FNGFixes.ini`
(`[FNGFixes]`, `[FNG1]` to `[FNG14]`). Their key layout is in the files; the contents are not copied here.

---

## `NFSMWExtraOptions.asi`

**[verified]** Exposes hidden options, debug tools and cheat hotkeys, and adds the `[Fixes]` and
`[Pursuit]` sections. Reads `NFSMWExtraOptionsSettings.ini`. Its strings name the `[Fixes]` keys it
recognises, and the `PursuitActionMode` and `EnableHiddenCameraModes` keys.

**The format and the applied keys are documented in [specs/extra-options.md](../specs/extra-options.md).**
That spec covers the file reader, precedence, the keys the port applies, and the keys it does not yet
apply, with reasons. This page does not repeat that table.

The shipped file has these sections (key counts are from the shipped file **[verified]**):

| Section | Keys | Spec coverage |
|---|---|---|
| `[Hotkeys]` | 10 | Headlights applied; the rest not applied (features missing in the port). |
| `[LapControllers]`, `[OpponentControllers]`, `[TrafficControllers]` | 4, 6, 3 | Not applied. |
| `[Menu]` | 22 | Not applied. |
| `[Gameplay]` | 29 | Five applied (`GameSpeed`, `WorldAnimationSpeed`, `CarScale`, `NoRevLimiter`, `ShowHUDOnLookBack`); the rest not applied. |
| `[Pursuit]` | 11 | Not applied (pursuit systems are under construction). |
| `[VisualTreatment]` | 7 | Not covered by the spec. |
| `[Weather]` | 8 | `AlwaysRain` and `GeneralRainAmount` applied. |
| `[Fixes]` | 8 | Not applied. The spec's rule: fixes repair bugs of the original, not reproduced in the port. |
| `[Misc]` | 11 | `SkipMovies`, `EnableSound`, `EnableMusic` applied. |

The `[Fixes]` keys present in the shipped file are `DisappearingWheelsFix`, `ExperimentalSplitScreenFix`
(off), `HelicopterBountyFix`, `ZeroBountyFix`, `CarSkinFix`, `ImmobileCollisionsFix`, `DDayFix`,
`BustedNISFix`. Their meaning is not documented by the pack, so this repository does not describe them
beyond their names.

---

## `NFSMWRaceFixes.asi`

**[verified]** Adds game modes to the Quick Race menu, makes track options configurable, and applies
two fixes. Reads `NFSMWRaceFixesSettings.ini`. Its strings name `OutroMovieFix` (not in the shipped
INI, so it keeps the plugin's default) and the mode icons it uses.

| Section | Key | Default | Meaning |
|---|---|---|---|
| `[Main]` | `QuickRaceModeSelectMenuHook` | `1` | Makes the Quick Race mode list configurable. |
| `[Main]` | `QuickRaceTrackOptionsHook` | `1` | Makes the race options configurable. |
| `[Main]` | `ChallengeSeriesFixes` | `1` | Unhardcodes the toll-booth icon in Challenge Series; fixes pursuit HUDs. |
| `[Main]` | `LANFixes` | `1` | Fixes for LAN races. |
| `[StartingPursuitCondition]` | `Minimum`, `Maximum` | `1`, `5` | Range for the starting pursuit option. |

Each mode section has the keys below. Defaults differ by mode.

| Mode section | `ShowInMenu` | Other keys (default) |
|---|---|---|
| `[Sprint]` | `1` | `CopDensity` `1`, `StartingPursuitCondition` `0` |
| `[Circuit]` | `1` | `CopDensity` `1`, `StartingPursuitCondition` `0` |
| `[Drag]` | `1` | `TrafficLevel` `0` (visual only), `CopDensity` `1`, `StartingPursuitCondition` `0` |
| `[LapKnockout]` | `1` | `CopDensity` `1`, `StartingPursuitCondition` `0` |
| `[Tollbooth]` | `1` | `NumOpponents` `1`, `CopDensity` `1`, `StartingPursuitCondition` `0` |
| `[Speedtrap]` | `1` | `CopDensity` `1`, `StartingPursuitCondition` `0` |
| `[TokenPickup]` | `0` | `CopDensity` `1`, `StartingPursuitCondition` `0` |
| `[Cashgrab]` | `0` | `ShowCashbagMessage` `1`, `CopDensity` `1`, `StartingPursuitCondition` `0` |
| `[Challenge]` | `0` | `StartingPursuitCondition` `0` |

`CopDensity` adds a heat-level option to the track options, and `StartingPursuitCondition` adds a
starting pursuit option. Both are shown only in the modes that have them. **[verified]** from the INI
comments' meaning, restated here.

---

## Graphics plugins

These plugins change rendering. Settings are in their own INI files.

### `NFSMWGraphics.asi` and `NFSMWGraphicsSettings.ini`

**[verified]** Graphics and visual-treatment overrides. Its strings name `NFSMWGraphicsSettings.ini`, the
`LODLights.asi` plugin (not shipped, so any dependency on it is missing) and the keys below.

| Section | Key | Default | Values |
|---|---|---|---|
| `[Main]` | `OverrideGameSettings` | `0` | `1` applies the `[Graphics]` block. |
| `[Main]` | `OverrideVisualTreatment` | `1` | `1` applies `[VisualTreatment]`. |
| `[Main]` | `OverrideOther` | `1` | `1` applies `[Other]`. |
| `[Hotkeys]` | `ToggleVisualTreatment` | `86` | Virtual-key code (86 is V). |
| `[Hotkeys]` | `ReloadConfig` | `135` | Virtual-key code. The shipped comment says the default is 112 (F1); the shipped value is 135, so the two disagree. |
| `[Graphics]` | `FullScreenAntiAliasing` | `2` | `0` to `2`. |
| `[Graphics]` | `TextureFiltering` | `2` | `0` to `2`. |
| `[Graphics]` | `VSync` | `0` | `0` or `1`. |
| `[Graphics]` | `WorldLevelOfDetail` | `3` | `0` to `3`. |
| `[Graphics]` | `RoadReflectionDetail` | `3` | `0` to `3`. |
| `[Graphics]` | `ShadowDetail` | `2` | `0` to `2`. |
| `[Graphics]` | `CarGeometryDetail` | `1` | `0` or `1`. |
| `[Graphics]` | `CarReflectionDetail` | `1` | `0` or `1`. |
| `[Graphics]` | `CarReflectionUpdateRate` | `3` | `0` to `3`. |
| `[Graphics]` | `RainEffect` | `1` | `0` or `1`. |
| `[Graphics]` | `OverBright` | `0` | `0` or `1`. |
| `[Graphics]` | `VisualTreatment` | `1` | Turns the treatment on. |
| `[VisualTreatment]` | `Red`, `Green`, `Blue` | `0.6`, `0.7`, `1` | Tint of the treatment. |
| `[VisualTreatment]` | `ColorBloomIntensity` | `0.6` | Colour bloom density. |
| `[VisualTreatment]` | `Saturation` | `1` | Saturation of the treatment's colours. |
| `[VisualTreatment]` | `BlackBloomIntensity` | `0.7` | Black bloom density. |
| `[Other]` | `UnderCarShadowDarkness` | `90` | Darkness of the shadow under each car. |

The port's visual treatment is documented in [visual-treatment.md](../visual-treatment.md). The
ExtraOptions file has its own `[VisualTreatment]` section with different defaults; the two files do not
share settings.

### `NFSMWFEShadows.asi` and `NFSMWFEShadows.ini`

**[verified]** Shadows in the front end. Its author is named in the plugin's start-up string as Aero_.

| Section | Key | Default | Meaning |
|---|---|---|---|
| `[MAIN]` | `RenderWorldShadows` | `1` | Environment shadows in the menus. |
| `[MAIN]` | `RenderCarShadows` | `1` | Vehicle shadows in the menus. |
| `[MAIN]` | `DisableShadowsOnCars` | `1` | Shadows are not drawn on the car itself in the menus (matches the Xbox 360). |
| `[MAIN]` | `RestoreShadows` | `1` | Experimental. Restores missing menu shadows on the original platform models. |

### `NFSMWHDReflections.asi` and `NFSMWHDReflections.ini`

**[verified]** Higher-resolution reflections for cars, the road and the rear-view mirror. Its author is
named as Aero_ in its own strings. It targets "v1.3 SPEED.exe from RELOADED" (6,029,312 bytes), a
different build label from the other plugins; the size matches the retail build.

| Section | Key | Default | Meaning |
|---|---|---|---|
| `[RESOLUTION]` | `HDReflections` | `1` | Raises reflection resolution to the desktop's resolution. |
| `[RESOLUTION]` | `OldGPUCompatibility` | `0` | Rounds reflection sizes to a power of two. |
| `[RESOLUTION]` | `VehicleScale`, `RoadScale`, `MirrorScale` | `1.0` each | Resolution multipliers. |
| `[GENERAL]` | `ImproveReflectionLOD` | `1` | More detail in reflections. |
| `[GENERAL]` | `RestoreVisualTreatment` | `1` | Visual treatment on the mirror. |
| `[GENERAL]` | `RestoreShaders` | `1` | Shader effects on reflections; barrier animation. |
| `[GENERAL]` | `RestoreShadows` | `0` | Real-time shadows in the mirror (needs `RestoreShaders`). |
| `[GENERAL]` | `RestoreDetails` | `1` | Lights and car shadows in the mirror; `2` adds particles. |
| `[GENERAL]` | `RestoreBackfaceCulling` | `1` | Fixes the upside-down car in the menus. |
| `[GENERAL]` | `OptimizeRenderDistance` | `1` | Limits reflection render distance. |
| `[GENERAL]` | `VehicleReflectionBrightness` | `0.8` | Intensity of vehicle reflections. |
| `[EXTRA]` | `ExpandMemoryPool` | `1` | Fixes objects vanishing when there are too many polygons. |

### `NFSMWMotionBlurController.asi` and its INI

**[verified]** Motion-blur strength and the speed threshold for it.

| Section | Key | Default | Meaning |
|---|---|---|---|
| `[Main]` | `MotionBlurAmount` | `0.25` | Blur strength; `1.0` is the original game's amount. |
| `[Main]` | `MotionBlurMinEffectiveSpeed` | `1.0` | Minimum speed in m/s before blur appears; the original uses 25.0. |

### `NFSMWHDContentSupport.asi`

**[verified]** Named "NFS Most Wanted HD Content Support" by Aero_ in its strings. It reads
`NFSMWHDContentSupport.ini`, which the pack does not ship, so the defaults apply. Its purpose beyond
its name is **[unverified]**.

### `BloomResIncreaser.asi`

**[inferred]** No INI shipped. The name suggests a higher-resolution bloom buffer. The binary carries no
configuration strings, so its behaviour is **[unverified]**.

### `SpeedReflect.asi`

**[verified]** The strings are game file paths for car parts: wheels, spoilers, exhaust, brakes, roof
scoops, and the `TEXTURES.BIN`, `VINYLS.BIN`, `LOGOS.BIN` and `GLOBALB.LZC` data they load. It
therefore touches car part models and textures. Its purpose (reflection on car parts) is **[inferred]**
from the name and the paths. The port's own treatment of car parts is in
[specs/car-assembly.md](../specs/car-assembly.md).

### `XNFS-ShaderLoader-MW.asi`

**[inferred]** A shader loader by name. It is a small binary (12.8 KB) with no configuration strings,
and it mentions `LODLights.asi`, which is not shipped. Which shaders it loads, and from where, is
**[unverified]**.

### `MW360Tweaks.asi`

**[verified]** Its strings name the Xbox 360 bloom and motion-blur parameters (`BloomPower`, `BloomColor`,
`MotionBlurVector`, `MotionBlurViewMat`, `MotionBlurViewProjMat`, `MotionBlurMask`, `BlurMask.png`), the
D3DX texture loader, a "Shader error!" message, and a `LuaDebugPrintout.log` file. So it overrides the
bloom and motion-blur shaders and writes Lua debug output. It is installed as `scripts/MW360Tweaks.asi`
by `X360STUFF.bat`.

---

## `NextGenGraphics.MostWanted.asi`

**[verified]** The next-generation graphics layer. Its feature list, from its log lines and strings:

- **Custom Texture Loader**: loads DX9 texture packs from `NextGenGraphics/TexturePacks/<pack>/`. Each
  pack needs a `TexturePackInfo.json` with `texturePath`, `textureMappings`, `NormalMapTexture`, and
  `SPECULARMAPTEXTURE`. A pack without the file is skipped (log line text in the plugin).
- **Next Gen Shader Values**: shader parameters, including `TEXTUREANIMOFFSET`.
- **Next Gen Motion Blur**: a motion-blur fix.
- **Front End Shadows** (`EnableFeShadows`), **Front End Safe Zone** (`FrontEndSafeZone`), and
  **Front End Vehicle Flares** (`FrontEndVehicleFlares`).
- **Time of Day Slider** (`TimeOfDaySlider`).
- It reads generic textures from `NextGenGraphics/GenericTextures/`, for example `SFX_WAVES_NORM.dds`
  and `2DNOISE.png`.

It writes `NextGenGraphics.MostWanted.log`. The log shipped in the pack is from an earlier run on
2022-02-15 and shows the "HRT v7" pack loading with 387 entries and many texture-load failures. The
failures carry two different HRESULT codes; this page does not interpret them.

The pack ships **no** texture packs and no `NextGenGraphics/` folder **[verified]**.

### `fmt.dll`

**[verified]** A shared library (`fmt` formatting, linked against the MSVC 2015 runtime). The NextGenGraphics
plugin imports it, and its log format strings use the `{}` placeholders that `fmt` expects
**[inferred]** that this is the dependency.

---

## `X360STUFF.bat` and `IDI_CAR_FX`

**[verified]** An interactive installer for a separate "NFS MW (2005) – X360 stuff" pack, included in
the repo but not part of the REDUX installer's described steps. Its language-selection prompt offers
English and Russian. What it does, from its commands:

1. Checks that it is in the game folder and that its required files exist, including
   `ResourceHacker.exe` and `X360DATA\EXE\IDI_CAR_FX.bin`, `IDI_SKYBOX_FX.bin` and
   `IDI_SCREENFILTER_FX.bin`.
2. Asks for a backup of the affected files, then copies `MW360Tweaks.asi` into `scripts/`.
3. Uses ResourceHacker to replace the `RT_RCDATA` resources `IDI_CAR_FX`, `IDI_SKYBOX_FX` and
   `IDI_SCREENFILTER_FX` inside `speed.exe`.
4. Copies the Xbox 360 icon textures into `FRONTEND\FrontB\unk_*.bin`, and patches `GLOBAL\GlobalB.lzc`
   and `FRONTEND\FrontB.lzc`.
5. Can remove everything it installed.

**Step 3 is the same resource-replacement mechanism that [formats/shaders.md](../formats/shaders.md)
documents**: `speed.exe` stores 31 compiled `fx_2_0` effects as `RT_RCDATA` resources named `IDI_*_FX`.
The three the batch replaces are among them. The repo's loose `IDI_CAR_FX` file (17,580 bytes) is the
compiled car effect the batch installs.

The batch does not work from this repository alone: it needs `ResourceHacker.exe`, a `.CT` cheat table,
two `.tpf` texture packs and the `X360DATA/` folder. None are in the pack. It writes to the game folder
and patches `speed.exe`, so back up first, and do not run it as part of REDUX unless you know its
steps.

`IDI_CAR_FX` is a shader binary from the game's own resources, so this repository does not commit it
([licensing.md](../licensing.md) rule 1). The loose copy in the pack was not compared against the
game's resource.

---

## Icon textures

### `buttons-xbox.tpk` and `buttons-playstation.tpk`

**[verified]** Gamepad button icon sets, each 666,384 bytes, that WidescreenFix swaps in for keyboard
prompts. They are game-style texture packs (TPK), so the port treats them as assets and does not
commit them. Format: [formats/textures.md](../formats/textures.md).

---

## What is not documented

| Item | Status |
|---|---|
| Behaviour of `NFSMWHDContentSupport.asi`, `BloomResIncreaser.asi`, `XNFS-ShaderLoader-MW.asi` | Inferred from names only; no config. |
| `Redux_Setup.exe`'s payload and changes | Not extracted. |
| The meanings of the `[Fixes]` keys in ExtraOptions | Names only. |
| The upstream sources of each plugin, and their licences | Not found. |
| `d3d9.ini`, `NFSMWHDContentSupport.ini`, `LODLights.asi`, `NFSMWBeta.asi` | Not shipped. |
