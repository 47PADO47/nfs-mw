# NFSMW Unlimiter Essentials

Reference notes on [nlgxzef/NFSMWUnlimiterEssentials](https://github.com/nlgxzef/NFSMWUnlimiterEssentials),
the official add-on car parts pack for [NFSMW Unlimiter](nfsmw-unlimiter.md). It adds paints, neon colours,
rims, licence plates, brakes and tyres to the PC game (2005). This page covers what each pack adds, how it
installs, and the known issues. Unlimiter itself is in [nfsmw-unlimiter.md](nfsmw-unlimiter.md).

**Licence: none.** The repository has no LICENSE file (GitHub reports no licence), so the files are all
rights reserved by default ([licensing.md](../licensing.md#repositories-with-no-license)). This page records
**facts** only. It copies no part data, texture, binary or string. Nothing from the pack is committed here.

## Snapshot inspected

| Item | Value |
|---|---|
| Repository | `nlgxzef/NFSMWUnlimiterEssentials`, default branch `main` |
| Description | "Official car parts pack for NFS Most Wanted Unlimiter." |
| Last push | 2026-10-11 |
| Stars | 1 at inspection |
| Version (`Read Me.txt`, `Changelog.txt`) | 1.1.0.1337 |
| Requires | NFSMW Unlimiter v4.1 or later; Binarius v2.10.4 or later |
| Licence | none |

The contents were read from the repository with `gh`: `Read Me.txt`, `Changelog.txt`, and the `Binarius/`
folder listing. The installer was not run. The pack is data only (an `.endlauncher` file that Binarius reads,
with an `Install`, `Uninstall` and `_Options` script set), so nothing here was executed.

## What it adds

The pack is split into optional sub-packs. Installing one does not require the others.

| Pack | Adds | Notes |
|---|---|---|
| **Paint** | Paints from NFSMW Beta and from Underground 2, with their light materials. Body paints go into 3 new categories and rim paints into 3 new categories. Vinyl paints go into the existing vinyl category. | Light materials come from the Beta Content Mod data. Rim paints 207 to 210 had a light-material fix. |
| **Neon** | Neon colours from NFS Underground 2. | Core component of Unlimiter v4, so it cannot be uninstalled. |
| **Wheels** | Rims from NFS Carbon, plus Foose Speedster, Splitz and Nitrous Silver (cut from Carbon), Work Equip 03 (ported from NFS World) and BBS RE595 (18-inch copied from the BMW M3 GT E46 ALMS, other inner radius made for it). | Configures tyres on all models to use Carbon-style textures so they can be tuned. Optional change of the maximum wheel inner size to 21 inches. |
| **License Plates** | Various licence plates in a lore-friendly style. | Optional change of the default plates on preset (Challenge Series, Bonus, Blacklist), traffic and police cars. Partly needs `StockPartsHook` from Unlimiter. A changed version of the Better License Plates mod, which is now deprecated. |
| **Brakes** | Three brake styles left in the game files from NFS Underground 2. | Model positions adjusted to fit Most Wanted wheels. Taken from the Leftover Brakes mod, now deprecated. |
| **Tires** | Four tyre styles: Stock, Retro, Carbon and Carbon Retro. | Tyre packs are a core component of Unlimiter v4, so they cannot be uninstalled. |

The `Read Me.txt` also notes that the Paint and Wheels packs were recycled from an earlier, cancelled mod
project, and that the pack will keep getting parts.

The `Unlimiter Logo.png` in the repository is for mod previews. It shows that a mod uses the Extra
Customization features in Unlimiter.

## Install

The README gives this order. The installer is Binarius, a separate tool.

1. Install NFSMW Unlimiter v4.1 or later from its Releases page.
2. Run the game and save each profile at least once. This creates Presitter data, which stops customisation
   records being corrupted when parts are added. Restart the game between profiles, because loading several
   profiles in one session can break progression. Skip this if there are no profiles, or if the Presitter
   data already exists. If profiles will not load, NFSMW Extra Options can turn off the checksum check.
3. Extract the release archive anywhere.
4. Install the `_NFSMW-UnlimiterEssentials-Binarius.endlauncher` script with Binarius v2.10.4 or later:
   run Binarius as administrator, choose User Mode, open the `.endlauncher` file, and point it at the game
   folder.
5. At the Binarius prompt, pick one of:
   - **Install / Upgrade**: installs the pack. Binarius asks about each sub-pack; answer "No" to skip one you
     already have from another pack.
   - **Uninstall**: removes the files the pack added. It leaves textures and strings, which other mods may
     use. Common, Neon and Tires cannot be uninstalled.
   - **Change Maximum Wheel Inner Size**: sets the largest rim to 20 or 21 inches for all cars.
6. Save the files when Binarius asks (the first dialog must be "Yes"), then launch the game if prompted.
   Errors in the log are usually safe to ignore.

The README warns that the pack does not check compatibility with other part packs. Overlaps are the user's
problem to resolve.

## Known issues and fixes

- **21-inch wheels missing on add-on cars installed after the pack.** Rerun the installer through Binarius
  and choose *Change Maximum Wheel Inner Size*, then select 21.
- **Brakes, Neon, Tires or License Plates missing for one car.** Each car's config file is
  `scripts\UnlimiterData\CARNAME.ini`. Set the matching sections to `1`:

  ```ini
  [Parts]
  Brakes = 1

  [Visual]
  Tires = 1
  LicensePlate = 1
  Neon = 1
  ```

  A car with no file of its own takes the same keys from `_General.ini`, or a new `CARNAME.ini` can be
  created with them. The key set is described in [nfsmw-unlimiter.md](nfsmw-unlimiter.md#per-car-files-unlimiterdata).

## Changelog

| Version | Change |
|---|---|
| 1.1.0.1337 | Requires Unlimiter v4.1 or later. Names for the new paint groups. New paint groups for unused, pearl and beta rim paints. |
| 1.0.0.1337 | Initial release. |

## Credits (from `Read Me.txt`)

- **Paint:** elaymm4, Red_Fire and trackmaniamatt (light materials and paint data from the PS2 demo and
  Underground 2, via the Beta Content Mod); nlgxzef (new paint parts, Unlimiter v4 support, rim paint light
  material fix).
- **Wheels:** 379Felipe (Foose Speedster, Splitz, Nitrous Silver, Work Equip 03); nlgxzef (Carbon and BBS RE595
  models, Unlimiter v4 support).
- **Neon, Brakes, License Plates:** nlgxzef.
- **Tools:** Binarius by MaxHwoy, nlgxzef, rx, avail, Sh2dow, NI240SX and 379Felipe.

## Relevance to this port

- **Plain game data.** The packs change car parts, paints, rims, tyres and brakes through the same tables the
  stock game uses. A port that reads those tables ([formats/cardata.md](../formats/cardata.md)) sees the
  parts as more entries, not as code. Testing a port against this pack would exercise the part lookup on
  cars the stock game does not have.
- **Depends on Unlimiter's hooks.** The README says the pack is built for Unlimiter, and the new menu
  entries need its extra slots and attributes. Whether a port would need the same hooks has not been checked.

## Sources

- Repository `Read Me.txt` and `Changelog.txt` (`nlgxzef/NFSMWUnlimiterEssentials`).
- `Binarius/` folder listing in the same repository.
- Repository metadata (`gh api repos/nlgxzef/NFSMWUnlimiterEssentials`).
