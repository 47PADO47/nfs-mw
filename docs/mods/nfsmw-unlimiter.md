# NFSMW Unlimiter

Reference notes on [nlgxzef/NFSMWUnlimiter](https://github.com/nlgxzef/NFSMWUnlimiter), a script mod for the
PC game (2005) that fixes problems with added cars and lifts several engine limits. This page covers what
it does, how it is set up, and what its settings file controls. The add-on car parts pack built for it is in
[nfsmw-unlimiter-essentials.md](nfsmw-unlimiter-essentials.md). The copy bundled in the REDUX pack is in
[redux-v3-plugins.md](redux-v3-plugins.md#nfsmwunlimiterasi-and-unlimiterdata).

**Licence: none.** The repository has no LICENSE file (GitHub reports no licence). Its files are all rights
reserved by default ([licensing.md](../licensing.md#repositories-with-no-license)). This page records
**facts** only: feature names, file names, keys, defaults. It copies no source, config file, texture or
string table. Nothing from the repository is committed to this one.

## Snapshot inspected

| Item | Value |
|---|---|
| Repository | `nlgxzef/NFSMWUnlimiter`, default branch `master` |
| Description | "Fixes some issues for added cars." |
| Last push | 2026-10-11 |
| Stars / forks | 68 / 8 at inspection |
| Language | C++ (Visual Studio solution `NFSMWUnlimiter.sln`), with a `.rc` resource file |
| Licence | none |
| Downloads | GitHub Releases, or NFSMods.xyz (`nfsmods.xyz/mod/688`) |

The README and the settings file were read from the repository with `gh`. The source headers were listed,
not compiled. Nothing was run against the game, so the feature list below is the author's claim unless a
row says otherwise.

## What it does

The mod is aimed at car modders. Stock Most Wanted has hard limits and a few bugs that show up only when
new cars are added. Unlimiter fixes those and gives modders hooks to configure the rest from `.ini` files.

### Fixes for added cars

- Visibility, damage and crash problems on added cars.
- Disappearing wheels (open-wheel cars, after focus is lost).
- Secondary logo missing on a car that has no manufacturer logo.
- Flickering cars in crowded areas with rain.
- Character names not shown outside Career events.
- Missing front-end objects, by cloning them (`FNGFix`, configured in `_FNGFixes.ini`).
- Paint menu not scrolling when a category has more than 80 colours.

### Limits lifted

- More than 10 cars in a traffic pattern.
- Expanded memory pools (also fixes the flicker above).
- A custom frame delay, to help frame pacing (`FrameDelay`).
- More skins per car than the stock four (`CarSkinFix`, `CarSkinCount`).

### Hooks for modders

| Hook | What the modder configures |
|---|---|
| Custom manufacturer on the front-end VLT node | Manufacturer name, through an `.ini` file |
| Custom rim brands | Brand list in `_RimBrands.ini`; `NoRimSize` disables the rim-size check for a brand |
| Cop destroyed bounty messages | Text for any cop car, in `_CopDestroyedStrings.ini` |
| AI opponent names | Up to 1000 names (`RACERNAME_000` to `RACERNAME_999`) |
| Engine type per car | Piston, Castrol (the "castrol" cheat code) or rotary |
| Forced LOD A | For a car with no lower-detail models |
| Random AI parts | Any part slot, per car |
| Extra customisation items | New menu entries from car-specific or general `.ini` files; the parts themselves must be added by the modder |
| Bonus cars | Custom bonus cars, marked **experts only**, for new save games only |
| Rims with `LANGUAGEHASH` | Localised rim names through a rim attribute |
| Decal dummies with `NUM_DECALS` | Number of decal slots on a dummy part |

### Custom attributes

These work only when Extra Customization is enabled. Both are floats in metres and go on Wide Body (`BODY`)
parts:

| Attribute | Effect |
|---|---|
| `FRONT_TIRE_OFFSET` | Spacer offset for the front wheels |
| `REAR_TIRE_OFFSET` | Spacer offset for the rear wheels |

### Other

- Backroom (junkman parts) enabled in the My Cars menu.
- Some BETA features enabled if elaymm4's BETA Content Mod is installed.

## Setup the README requires

For the added cars to work fully, the modder must add assets that the stock game does not have. The release
pack ships a binary and NFS-VltEd scripts that do this automatically. The README lists:

1. **Textures** in `FrontEnd\FrontB.lzc` and `FrontEndTextures.tpk`: ten `VISUAL_PART_*` names for the
   customisation icons (attachment, brake, driver, headlights, interior, licence plate, roof, side mirror,
   tail lights, tire).
2. **Strings** in `LANGUAGES\(LanguageName).bin`: the matching `CO_*` labels (`CO_DRIVER`,
   `CO_ATTACHMENTS`, `CO_ATTACHMENT_1` to `CO_ATTACHMENT_10`, `CO_INTERIOR`, `CO_HEADLIGHTS`, `CO_ROOF`,
   `CO_TAILLIGHTS`, `CO_SIDE_MIRROR`, `CO_DECAL_SLOT_7`, `CO_DECAL_SLOT_8`).
3. **Camera angles** through NFS-VltEd, as new `customize_customizeparts_*` entries in the
   `customize_master` screen (20 IDs, from `_263` to `_788`).

The README gives each name with its hash. The hashes are not copied here; take them from the README.

## Settings file: `NFSMWUnlimiterSettings.ini`

The table is the upstream file as it stands on `master`. Every key is read from a `[Section]` block. The
defaults are the values in the file. Comments in the file are the source for the descriptions.

### `[Main]`

| Key | Default | Meaning |
|---|---|---|
| `ReplacementModel` | `1` | Car type used when a requested model is missing or invalid (`1` is CARRERAGT). |
| `EnableManufacturerHook` | `1` | Backports Carbon's manufacturer lookup instead of hard-coded IDs. |
| `MyCarsBackroom` | `1` | Shows the backroom in My Cars. |
| `EnableCopDestroyedStringHook` | `1` | Reads cop-destroyed text from `UnlimiterData\_CopDestroyedStrings.ini`. Turned off automatically if Bartender is installed. |
| `EnableRandomPartsHook` | `1` | Lets AI opponents use random parts from `UnlimiterData\CARNAME.ini`. |
| `EnableBonusCarsHook` | `0` | **Experts only.** Reads bonus cars from `UnlimiterData\_BonusCars.ini`. |
| `EnablePresitter` | `1` | Saves customisation records as presets with the game, so they survive edits to the game. |
| `RandomizeTraffic` | `0` | Random colours for traffic cars. |
| `SeparateRims` | `1` | Separate front and rear rim choice when Extra Customization is on. |

### `[Fixes]`

| Key | Default | Meaning |
|---|---|---|
| `DisappearingWheelsFix` | `1` | Stops wheels vanishing after focus loss. |
| `SecondaryLogoFix` | `1` | Shows the secondary logo when the manufacturer logo is missing. |
| `AddOnCopsDamageFix` | `1` | Makes parts on add-on cop cars damageable. |
| `ChallengeSeriesOpponentNameFix` | `1` | Opponent names used outside Career. |
| `LightMaterialCrashFix` | `0` | Tries to stop a crash when a missing model changes its light material. |
| `RideHeightFix` | `1` | Always adds both ride-height values (ecar and chassis), not only when close. |
| `FNGFix` | `0` | Clones missing front-end objects, per `UnlimiterData\_FNGFixes.ini`. |
| `CarSkinFix` | `1` | Lets more cars be skinned than the stock four. Forced on when `RandomizeTraffic` is on. Off when Zolika1351's Limit Adjuster is present. |
| `ScalingLightFix` | `0` | Fixes wheel scaling when they are changed by custom attributes. |
| `CopBrightLightsOrderFix` | `1` | Order and node info of the flashing cop lights. |
| `CopSpoilerDamageFix` | `1` | Makes the cop spoiler damageable like other parts. |

### `[Misc]`

| Key | Default | Meaning |
|---|---|---|
| `ExpandMemoryPools` | `1` | Larger memory areas; fixes flicker. Forced on by `CarSkinFix` or `RandomizeTraffic`. Off when Zolika1351's Limit Adjuster is present. |
| `FrameDelay` | `-1` | Custom frame delay in milliseconds. `-1` is off; `0` and up set it. |
| `ForceStockPartsOnAddOnOpponents` | `0` | Stock parts on add-on opponents, to fix missing parts. Needs `EnableRandomPartsHook`. |
| `ForceLightsOnInFE` | `0` | Car lights on in menus. |
| `LightTextureSwapInFE` | `1` | Swaps car light textures in menus. |
| `CarSkinCount` | `20` | Skins used by `CarSkinFix`. The game needs matching `DUMMY_SKIN`, `DUMMY_WHEEL` and `DUMMY_SPINNER` textures. |
| `DisableCameraAutoCenter` | `0` | Stops the camera re-centring without input. |

### `[Debug]`

| Key | Default | Meaning |
|---|---|---|
| `DisableTextureReplacement` | `0` | Turns off texture replacement from Extra Customization. |
| `DisableNeon` | `0` | Turns off the neon feature. |
| `DisableLightFlareColors` | `0` | Turns off custom light-flare colours. |
| `DisableKitWheelModifications` | `0` | Turns off kit wheel changes. |
| `ExitWorkaround` | `1` | **Experimental.** Works around the game exiting after the first loading screen. |
| `TestCareerCustomization` | `0` | Shows part prices and heat changes without entering Career. Customisation quits to free roam when on. |
| `ForceSignalsOn` | `0` | Forces the turn-signal position markers on. |

## Per-car files: `UnlimiterData/`

The folder holds one optional `.ini` per car, named after the car, and a set of shared files whose names start
with `_`.

| File | Role |
|---|---|
| `_General.ini` | Defaults for any car without its own file. |
| `_BonusCars.ini` | Bonus car definitions (experts only). |
| `_CopDestroyedStrings.ini` | Cop destroyed bounty messages. |
| `_FNGFixes.ini` | Front-end objects to clone (for `FNGFix`). |
| `_RimBrands.ini` | Custom rim brands. |
| `_PaintGroups.ini`, `_VinylGroups.ini` | Paint and vinyl groups. |
| `TRAF*.ini` | Traffic car files (ten in the upstream folder listing). |
| `BonusCars/`, `PaintGroups/`, `RimBrands/`, `VinylGroups/` | Sub-folders with the per-item data. |

The per-car file's keys are not in the README. The Essentials README gives the ones it needs: a `[Parts]`
section with `Brakes`, and a `[Visual]` section with `Tires`, `LicensePlate` and `Neon`, each `1` to show a
menu or `0` to hide it. See [nfsmw-unlimiter-essentials.md](nfsmw-unlimiter-essentials.md#known-issues-and-fixes).

## Known issues (from the README)

- Pressing Esc to leave the Backroom stops you moving between its options. Press Enter or Esc to go back in
  and enter the menu again.
- Bonus car definitions are for experts only and apply only to newly created save games.
- BETA features depend on elaymm4's BETA Content Mod.
- Custom attributes do nothing unless Extra Customization is enabled.

The upstream repository lists 11 open issues and 1 pull request at inspection. The issue list was not read.

## Relevance to this port

- **Limits, not features.** Most of the list above works around engine limits (traffic counts, memory pools,
  skin counts, rim checks, part slots). Those limits may matter to the car-data and traffic work if the port
  ever loads added cars. This is a checklist to verify, not something checked yet.
- **Car data is read by the port.** The settings and per-car `.ini` files change the parts a car has. The
  car tables the port reads are in [formats/cardata.md](../formats/cardata.md).
- **Two builds in circulation.** The REDUX pack's `NFSMWUnlimiter.asi` names `_CarManufacturers.ini` and a
  `PaintMenuFix` key. Neither appears in the upstream `UnlimiterData/` listing or settings file above. The
  REDUX copy is a different build; do not assume one build's keys apply to the other. Its keys are in
  [redux-v3-plugins.md](redux-v3-plugins.md#nfsmwunlimiterasi-and-unlimiterdata).

## Sources

- Repository README (`README.md` in the `nlgxzef/NFSMWUnlimiter` source tree).
- `NFSMWUnlimiterSettings.ini` and the `UnlimiterData/` folder listing, same tree.
- Repository metadata (`gh api repos/nlgxzef/NFSMWUnlimiter`).
