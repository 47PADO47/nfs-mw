# Chameleon police vehicles

Which of the 2012 game's vehicle bundles are police cars, and what the install holds about where they appear. Used by
[`mods/nfsmw-2012-content`](../../mods/nfsmw-2012-content) (`src/police.rs`), which makes them cop cars of the 2005 game
([cop-cars.md](../modding/cop-cars.md)). Container and resources: [chameleon-bundle.md](chameleon-bundle.md),
[chameleon-vehicle.md](chameleon-vehicle.md).

Tags as in the [docs README](../README.md#evidence-tags): **[verified]** is read from the 2012 install (v1.4.0.0) with the
readers or a script on all 93 `VEH_<n>_HI.BNDL`; **[exe]** is from `NFS13.exe` (Ghidra); **[unconfirmed]** is a reading.

## The result [verified]

Five of the 93 bundles are police vehicles. None of the five is in a list of cars the player can be given, and the 2012 game's
vehicle list has no role flag, so the role is read from the bundle:

| Bundle | Vehicle | Marks in the bundle |
|---|---|---|
| `VEH_122713_HI` | Ford Crown Victoria (cop) | siren corona object `Ford_Crown_Victoria_COP_Coronas`, a `LightBarHinge` part, `Ford CrownVictoria COP DamageBehaviour` |
| `VEH_122706_HI` | Dodge Charger SRT8 (cop) | `Dodge_Charger_SRT8_Cop_Coronas`, `LightBarHinge` |
| `VEH_510072_HI` | Ford Explorer Interceptor 2011 | `Ford_ExplorerInterceptor_Cop_Coronas`, `LightBarHinge` |
| `VEH_122699_HI` | Chevrolet Corvette Z06 2006 | `LightBarHinge` (its lamp objects are named like a player car's) |
| `VEH_1467242_HI` | NFS SWAT van 2012 | objects named `NFS_SwatVan_2012_Coronas` (two of them); no collision mesh, no skeleton |

The two cops of the first rows carry no model year in their behaviour objects, so the reader that names cars
(`vehicle_name`) finds no name for them; the siren corona object names them (`PoliceMarks::siren_name`).

**The rule** (`chameleon_vehicle::police_marks`): a bundle is a police vehicle when one of its Genesys objects has a name that
ends in `_Coronas` and has the token `Cop` (any case), or names a `LightBarHinge` part, or has a `_Coronas` name starting with
`Swat`. The rule marks exactly these five bundles of the 93 [verified], and `MW2012_GAME_DIR=<install> cargo test -p
nfsmw-2012-content --lib police -- --include-ignored` checks it.

## The second corona object [verified]

A police vehicle has two `VehicleVfxBehaviour` objects: its ordinary lamp records (state bits 0 to 2, as every car:
[chameleon-lamps.md](chameleon-lamps.md)) and the siren records, `<Car>_Cop_Coronas`. The Charger's holds 8 coronas, each on
its own locator group, all on **state bit 3** (the vehicle's second switch, bit 3 of the light-state word), mode 0, colour
factor (1, 1, 1, 1); the SWAT van's second object holds 8 more on bit 3. The colour of a light is in the referenced `Corona`
object, not in the record. The siren lamps are therefore the bit-3 coronas of the second object **[verified]**; each also names a
flash pattern, and the objects' glow colours say which lamps are red and which blue (the Charger: three blue, three red and two
small strobes of both colours; counts per car, the patterns and the colours are in [chameleon-corona.md](chameleon-corona.md#flash-patterns-verified)).
The converter turns them into the `[[light_bar]]` of a converted police car ([cop-cars.md](../modding/cop-cars.md#the-light-bar)).

## The role table of the game's logic [verified, meaning unconfirmed]

`GAMELOGIC/GAMEPLAY.BNDL` holds a Genesys object (`0x46194`, 18,160 bytes) with 118 records of 116 bytes: a vehicle id at +88
(the ids of `VEHICLELIST.BNDL`) and a reference to another object in the last four bytes. The records of the other vehicles reference other objects (`0x1948xx` and `0x19d933` for most, `0x1f3axx` and `0x21xxxx` for the later ones); **exactly the police vehicles' records reference `0x108d4f`
(Crown Victoria twice, Charger twice, Corvette), `0x108dd5` (Explorer Interceptor, SWAT van) and `0x108d63` (the Corvette's
second record)**, and carry a pair of 16-bit numbers (2, 3 or 4) before it. The meaning of the numbers is not decoded
(a tier or a priority; the Crown Victoria and the Charger have 4, the Explorer and the SWAT van 3, the Corvette 2 and 4), so
the converter does not use them. What the table does show is that the same five vehicles are the ones the game sets apart,
which matches the bundle marks.

## What is not in the install in a form that was decoded

- **The unit tables of the heat levels.** `Genesys.Gen.HeatLevel` (40 objects in one list, `0x5f51c`) has a
  `PursuitBehaviour.CopBehaviour` record per level whose fields (cop type keys such as `0x194509`, counts, timers) were
  not decoded. Its cop type list (`Genesys.Gen.Gameplay.CopType`, 25 records, object `0x101dac`) pairs those keys with a
  feedback group of the dispatch speech (`EN_US/FEEDBACKGROUPS/<n>.BNDL`) and four flags; it names no vehicle. The car
  select screen of the 2012 game has pictures per cop tier and per racer tier (`UI/IMAGES/CARSELECT/COPTIER<n>.BNDL` and
  `RACERTIER<n>`, from `FUN_006579a0`) **[exe]**.
- **Which levels a vehicle appears at.** Nothing decoded ties a police vehicle to a heat level, so the mod's waves are a
  design ([mods/nfsmw-2012-content/README.md](../../mods/nfsmw-2012-content/README.md#police-cars)).
- **The car-select cop mode.** `CarSelectViewModel.IsCopCarSelect` and the online `VEHICLE_TYPE_COP` exist **[exe]**; the
  police vehicles are probably what the player drives in its Interceptor game mode **[unconfirmed]**. The 2005 game has no such mode, so the
  converted police cars are `role = "cop"` by default and `role = "both"` is a setting.
