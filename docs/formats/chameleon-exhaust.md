# Chameleon car tail pipes

Where a 2012 car keeps its tail pipes and what the 2012 game plays at them. The locator groups and the
`VehicleVfxBehaviour` object are described in [chameleon-lamps.md](chameleon-lamps.md); this page is the pipe half.
Reader: `libs/chameleon-vehicle/src/lamps.rs` (`SpotEffect`); the mod's use is `mods/nfsmw-2012-content/src/pipes.rs`.
Tags as on [chameleon-lamps.md](chameleon-lamps.md): **[verified]** on the 93 `VEH_<n>_HI.BNDL` of the install, **[exe]** from
`NFS13.exe`, **[unconfirmed]** a reading of the data.

## Where the pipes are

A pipe has no record of its own. It is a **locator group** (numbered placements of the graphics spec) that the car's
**spot effects** point at. The `VehicleVfxBehaviour` object of a car holds up to eight `SpotEffect` records of 0x20
bytes:

| Offset | Type | Field |
|---|---|---|
| +0x08 | u32 | hash of the record type (`0xF2A65681`) |
| +0x0C | 8 bytes | the effect to play: an id in the import table, a `BearEffect` (type 0x301) of `GLOBALEFFECTS.BNDL` |
| +0x14 | u32 | the locator group the effect plays at |
| +0x18 | u16 | the state bit that starts it |
| +0x1A | u16 | 0 |
| +0x1C, +0x1D | u8, u8 | 0; **the mirror flag**: the effect also plays at the mirror image of each locator [verified pattern] |

The effects are shared by all cars (the same ids in every bundle), and each has one state bit:

| Effect id | Bit | Size | Reading |
|---|---|---|---|
| `0x24F34` | 8 | 4.3 KB | nitrous pressed **[exe]**: bit 8 of the state word is set while the nitrous input (the float at +0x1944 of the vehicle's input block, written next to throttle and brake) is above zero |
| `0x16205B` | 9 | 9.1 KB | nitrous active: bit 9 is set while the byte at +0x195B is 1 **[exe]**; this is the large flame effect |
| `0x24F2D` | 14 | 6.4 KB | after the nitrous: bit 14 is set while a timer runs out when neither bit 8 nor 9 is set **[exe]**, **[unconfirmed]** afterburn |
| `0x24F2F` | 19 | 1.9 KB | bit 19, set when `FUN_00572C20` is true, **[unconfirmed]** meaning |
| `0x22030B`, `0x220308` | 20 | 1.7, 1.6 KB | bit 20, `+0x195B == 3`, **[unconfirmed]** (a second state of the nitrous byte); `0x220308` is placed at a rear centre group, `0x22030B` at the pipes |
| `0x24F5A`, `0x14023F` | 12 | 7.7, 10.9 KB | bit 12, at the shared default group (below) |

The state word is the one `FUN_0057A5E0` builds for the lamps (see chameleon-lamps.md, "The light-state word"). So **a car's
pipes are the locators of the groups its bit 8 and bit 9 spot effects name**, and every effect of the pipe group shoots out
of the same places.

The group `0x1401FC` (62 of the 93 cars) is the **default group**: one locator at (0, 0.35, 0), the middle of the car. The
effects that have no place on a car (bit 12 for all cars) point there. A nitrous effect at the default group is no pipe.

## Reading the group

- Every locator of the group is one pipe. A locator's +z axis is the way the flame shoots: back and a little up or down for
  most cars (within 30 degrees of straight back), and out
  to the side for side exits (the Viper, the Cobra, the 918 concept study, the Caterham, the E46 GTR).
- With the mirror flag on the flame record (bit 9; bit 8 for a car with no bit 9 record), each locator is also played at its
  mirror image across the centre plane. The 918 Spyder has **one** locator, (-0.32 right, 0.96 up, -1.06 back), and the flag:
  its two pipes sit on top of the rear deck, behind the cabin, as in the 2012 game. A car with both sides spelled out
  (the 911: four locators at +-0.41 and +-0.51) has no flag.
- A car with one locator and no flag has one pipe: the Caterham, the Atom, the BAC Mono, the Skyline R34, the Lancia and the
  911 Turbo (1982) have one, and some are not centred (the Skyline, the Lancia and the 911 Turbo on one side).
- Several effect records can share a group; the mirror flag is read from the flame record only. The Audi RS3 has the flag on
  its bit 9 record alone and four pipes (two at 0.35 and 0.45 and their mirror images).

## Cars without pipes **[verified]**

| Car | Why |
|---|---|
| Tesla Roadster 2.5 S (1085764) | electric: its four spot effects are at the default group and none is a nitrous record |
| SWAT van (1467242) | the same: four spot effects, none nitrous |

Both get no exhaust marker, so the game plays no flame and no pop at the tail. One more bundle (122706, no name) has no
effect object at all; the mod gives it the markers derived from its body shape (rear face, low).

## Per car

Positions are the pipe markers the mod writes, in the game's car space after the car is centred on its wheelbase (the 2012
vehicle space turned: x forward, y left, z up). The check that every pipe lies on the car's body (inside its extents with a
margin of 0.15 m) passes for every car except the BMW 1M, whose bundle holds the pipes of the car with the body of the *Cars*
tie-in model (Lightning McQueen), 0.3 m beyond it.

| Car (bundle) | Pipes | Positions (x forward, y left, z up, metres) |
|---|---|---|
| AlfaRomeo_4C_2012 (1085698) | 2 | (-1.99, 0.59, 0.34); (-1.99, -0.59, 0.34) |
| AlfaRomeo_MitoQV_2012 (2196263) | 2 | (-1.95, 0.30, 0.23); (-1.95, 0.37, 0.23) |
| Ariel_Atom500V8_2010 (1160082) | 1 | (-1.41, 0.00, 0.57) |
| AstonMartin_DB5Vantage_1964 (2196529) | 2 | (-2.30, 0.38, 0.33); (-2.30, 0.48, 0.33) |
| AstonMartin_DBS_2008 (2196595) | 2 | (-2.25, 0.46, 0.37); (-2.25, -0.46, 0.37) |
| Audi_A1ClubsportQuattroConcept_2011 (1160139) | 1 | (-0.67, 0.85, 0.23) |
| Audi_R8GTSpyder_2012 (122675) | 2 | (-2.01, 0.67, 0.31); (-2.01, -0.67, 0.31) |
| BAC_Mono_2012 (1160025) | 1 | (-1.79, 0.00, 0.65) |
| Bentley_ContinentalGTSuperSportsISRConvertible_2011 (866774) | 4 | (-2.33, 0.50, 0.37); (-2.28, 0.61, 0.37); (-2.33, -0.50, 0.37); (-2.28, -0.61, 0.37) |
| BMW_1M_2012 (2196065) | 4 | (-2.08, 0.36, 0.29); (-2.05, 0.44, 0.29); (-2.08, -0.36, 0.29); (-2.05, -0.44, 0.29) |
| BMW_M3E46GTR_2001 (393276) | 2 | (-0.24, -0.87, 0.19); (-0.34, -0.87, 0.19) |
| BMW_M3E46GTR_2001 (2277210) | 2 | (-0.24, -0.87, 0.19); (-0.34, -0.87, 0.19) |
| BMW_M3E46GTR_2001 (2277224) | 2 | (-0.24, -0.87, 0.19); (-0.34, -0.87, 0.19) |
| BMW_M3E46GTR_2001 (2277238) | 2 | (-0.24, -0.87, 0.19); (-0.34, -0.87, 0.19) |
| BMW_M3E46GTR_2001 (2277252) | 2 | (-0.24, -0.87, 0.19); (-0.34, -0.87, 0.19) |
| BMW_M3E92_2009 (122682) | 4 | (-2.26, 0.24, 0.30); (-2.25, 0.33, 0.30); (-2.26, -0.24, 0.30); (-2.25, -0.33, 0.30) |
| Bugatti_Veyron164SuperSport_2010 (122692) | 4 | (-2.15, 0.07, 0.31); (-2.15, 0.16, 0.31); (-2.15, -0.07, 0.31); (-2.15, -0.16, 0.31) |
| Bugatti_VeyronGrandSportVitesse_2012 (2076005) | 4 | (-2.15, 0.07, 0.31); (-2.15, 0.16, 0.31); (-2.15, -0.07, 0.31); (-2.15, -0.16, 0.31) |
| Caterham_SuperlightR500_2008 (393001) | 1 | (-0.71, -0.80, 0.24) |
| Caterham_SuperlightR500_2008 (1467156) | 1 | (-0.71, -0.80, 0.24) |
| Chevrolet_CamaroZL1_2011 (621053) | 4 | (-2.37, 0.50, 0.33); (-2.34, 0.59, 0.33); (-2.37, -0.50, 0.33); (-2.34, -0.59, 0.33) |
| Chevrolet_CorvetteZ06_2006 (122699) | 4 | (-2.08, 0.12, 0.28); (-2.08, 0.24, 0.28); (-2.08, -0.12, 0.28); (-2.08, -0.23, 0.28) |
| Chevrolet_CorvetteZR1_2009 (122701) | 4 | (-2.08, 0.12, 0.29); (-2.08, 0.23, 0.29); (-2.08, -0.12, 0.29); (-2.08, -0.23, 0.29) |
| Dodge_ChallengerSRT8392_2011 (122704) | 4 | (-2.45, 0.48, 0.34); (-2.44, 0.58, 0.34); (-2.45, -0.48, 0.34); (-2.44, -0.58, 0.34) |
| Dodge_ChargerRT_1970 (2196466) | 2 | (-2.74, 0.42, 0.40); (-2.74, -0.42, 0.40) |
| Dodge_ChargerSRT8_2012 (1399322) | 2 | (-2.35, 0.53, 0.31); (-2.35, -0.53, 0.31) |
| Ford_ExplorerInterceptor_2011 (510072) | 2 | (-2.35, 0.54, 0.34); (-2.35, -0.54, 0.34) |
| Ford_F150SVTRaptor_2011 (300097) | 2 | (-2.57, -0.91, 0.33); (-2.45, -0.92, 0.34) |
| Ford_F150SVTRaptor_2011 (1467164) | 2 | (-2.57, -0.91, 0.33); (-2.45, -0.92, 0.34) |
| Ford_FiestaST_2013 (2196197) | 2 | (-1.89, -0.30, 0.28); (-1.88, -0.39, 0.28) |
| Ford_FocusRS500_2010 (1160480) | 2 | (-2.09, 0.47, 0.30); (-2.10, -0.46, 0.30) |
| Ford_FocusST_2013 (1399539) | 2 | (-2.13, 0.07, 0.45); (-2.13, -0.07, 0.45) |
| Ford_GT_2006 (122714) | 2 | (-2.28, 0.08, 0.40); (-2.28, -0.08, 0.40) |
| Ford_MustangBoss302_2012 (122716) | 2 | (-2.26, 0.58, 0.32); (-2.26, -0.58, 0.32) |
| Hennessey_VenomGTSpyder_2012 (2076137) | 2 | (-2.24, 0.05, 0.72); (-2.24, -0.05, 0.72) |
| Jaguar_XKR_2010 (1399602) | 4 | (-2.30, 0.34, 0.29); (-2.28, 0.43, 0.29); (-2.30, -0.34, 0.29); (-2.28, -0.43, 0.29) |
| Koenigsegg_AgeraR_2011 (589261) | 2 | (-2.13, 0.04, 0.31); (-2.13, -0.04, 0.31) |
| Lamborghini_AventadorJ_2012 (2076203) | 4 | (-2.35, 0.08, 0.34); (-2.34, 0.19, 0.34); (-2.35, -0.08, 0.34); (-2.34, -0.19, 0.34) |
| Lamborghini_AventadorLP7004_2011 (392074) | 4 | (-2.31, 0.04, 0.31); (-2.31, 0.12, 0.31); (-2.31, -0.04, 0.31); (-2.31, -0.12, 0.31) |
| Lamborghini_Countach5000QV_1985 (1085091) | 4 | (-2.05, 0.26, 0.35); (-2.05, 0.34, 0.35); (-2.05, -0.26, 0.35); (-2.05, -0.34, 0.35) |
| Lamborghini_DiabloSV_1995 (2076339) | 4 | (-2.10, 0.25, 0.29); (-2.10, 0.33, 0.29); (-2.10, -0.25, 0.29); (-2.10, -0.34, 0.29) |
| Lamborghini_DiabloSV_1995 (2277456) | 4 | (-2.10, 0.25, 0.29); (-2.10, 0.33, 0.29); (-2.10, -0.25, 0.29); (-2.10, -0.34, 0.29) |
| Lamborghini_DiabloSV_1995 (2277467) | 4 | (-2.10, 0.25, 0.29); (-2.10, 0.33, 0.29); (-2.10, -0.25, 0.29); (-2.10, -0.34, 0.29) |
| Lamborghini_DiabloSV_1995 (2277478) | 4 | (-2.10, 0.25, 0.29); (-2.10, 0.33, 0.29); (-2.10, -0.25, 0.29); (-2.10, -0.34, 0.29) |
| Lamborghini_DiabloSV_1995 (2277489) | 4 | (-2.10, 0.25, 0.29); (-2.10, 0.33, 0.29); (-2.10, -0.25, 0.29); (-2.10, -0.34, 0.29) |
| Lamborghini_GallardoLP5704SpyderPerformante_2011 (1085633) | 4 | (-2.10, 0.55, 0.40); (-2.08, 0.66, 0.39); (-2.10, -0.55, 0.40); (-2.08, -0.66, 0.39) |
| Lancia_DeltaHFIntegraleEvoluzione_1993 (1097100) | 1 | (-1.83, 0.62, 0.31) |
| LandRover_RangeRoverEvoque_2011 (1085958) | 2 | (-1.73, 0.52, 0.33); (-1.73, -0.52, 0.33) |
| Lexus_LFA_2010 (1085511) | 3 | (-2.19, -0.00, 0.30); (-2.21, 0.07, 0.38); (-2.21, -0.07, 0.38) |
| Marussia_B2_2011 (1551590) | 2 | (-2.13, 0.07, 0.78); (-2.13, -0.07, 0.78) |
| Maserati_GranTurismoMCStradale_2011 (122757) | 2 | (-2.38, 0.26, 0.34); (-2.38, -0.26, 0.34) |
| Maserati_GranTurismoMCStradale_2011 (1467172) | 2 | (-2.38, 0.26, 0.34); (-2.38, -0.26, 0.34) |
| McLaren_F1LM_1996 (2076070) | 4 | (-2.12, 0.05, 0.38); (-2.12, 0.14, 0.38); (-2.12, -0.05, 0.38); (-2.12, -0.14, 0.38) |
| McLaren_MP412C_2012 (122765) | 2 | (-2.22, 0.23, 0.71); (-2.22, -0.23, 0.71) |
| MercedesBenz_SL65AMGBlackSeries_2009 (122769) | 4 | (-2.19, 0.48, 0.29); (-2.17, 0.57, 0.28); (-2.19, -0.48, 0.29); (-2.17, -0.57, 0.28) |
| MercedesBenz_SL65AMGBlackSeries_2009 (1467179) | 4 | (-2.19, 0.48, 0.29); (-2.17, 0.57, 0.28); (-2.19, -0.48, 0.29); (-2.17, -0.57, 0.28) |
| MercedesBenz_SLSAMG_2010 (122773) | 2 | (-2.28, 0.54, 0.35); (-2.28, -0.54, 0.35) |
| Mitsubishi_LancerEvolutionX_2008 (866832) | 2 | (-2.16, 0.42, 0.30); (-2.16, -0.42, 0.30) |
| NFS_SwatVan_2012 (1467242) | 0 | none (no nitrous flame record: no exhaust marker) |
| Nissan_350Z_2006 (2076462) | 2 | (-2.12, 0.39, 0.22); (-2.12, -0.39, 0.22) |
| Nissan_350Z_2006 (2255945) | 2 | (-2.12, 0.39, 0.22); (-2.12, -0.39, 0.22) |
| Nissan_350Z_2006 (2255956) | 2 | (-2.12, 0.39, 0.22); (-2.12, -0.39, 0.22) |
| Nissan_350Z_2006 (2255967) | 2 | (-2.12, 0.39, 0.22); (-2.12, -0.39, 0.22) |
| Nissan_350Z_2006 (2255978) | 2 | (-2.12, 0.39, 0.22); (-2.12, -0.39, 0.22) |
| Nissan_GTR_SpecV (122672) | 2 | (-2.09, 0.61, 0.32); (-2.09, -0.61, 0.32) |
| Nissan_GTREgoist_2011 (435169) | 4 | (-2.22, 0.57, 0.32); (-2.18, 0.71, 0.33); (-2.22, -0.57, 0.32); (-2.18, -0.71, 0.33) |
| Nissan_SkylineR34GTR_1999 (2076524) | 1 | (-2.29, 0.40, 0.36) |
| Nissan_SkylineR34GTR_1999 (2196131) | 4 | (-2.05, 0.35, 0.25); (-2.04, 0.45, 0.25); (-2.05, -0.35, 0.25); (-2.04, -0.45, 0.25) |
| Nissan_SkylineR34GTR_1999 (2255901) | 1 | (-2.29, 0.40, 0.36) |
| Nissan_SkylineR34GTR_1999 (2255912) | 1 | (-2.29, 0.40, 0.36) |
| Nissan_SkylineR34GTR_1999 (2255923) | 1 | (-2.29, 0.40, 0.36) |
| Nissan_SkylineR34GTR_1999 (2255934) | 1 | (-2.29, 0.40, 0.36) |
| Pagani_Huayra_2011 (535435) | 4 | (-2.25, 0.05, 0.66); (-2.23, 0.05, 0.75); (-2.25, -0.05, 0.66); (-2.23, -0.05, 0.75) |
| Pagani_ZondaR_2009 (2076266) | 4 | (-2.32, 0.04, 0.50); (-2.29, 0.04, 0.57); (-2.29, -0.04, 0.57); (-2.32, -0.04, 0.50) |
| Pontiac_FirebirdTransAmSpecialEdition_1977 (2196399) | 4 | (-2.09, 0.62, 0.25); (-1.99, 0.63, 0.23); (-2.09, -0.62, 0.25); (-1.99, -0.63, 0.23) |
| Porsche_911CarreraS_2012 (1085007) | 4 | (-2.19, 0.41, 0.27); (-2.16, 0.51, 0.26); (-2.19, -0.41, 0.27); (-2.16, -0.51, 0.26) |
| Porsche_911CarreraS_2012 (1467186) | 4 | (-2.19, 0.41, 0.27); (-2.16, 0.51, 0.26); (-2.19, -0.41, 0.27); (-2.16, -0.51, 0.26) |
| Porsche_911GT2_2008 (2076399) | 2 | (-2.17, 0.49, 0.29); (-2.17, -0.49, 0.29) |
| Porsche_911GT2_2008 (2277332) | 2 | (-2.17, 0.49, 0.29); (-2.17, -0.49, 0.29) |
| Porsche_911GT2_2008 (2277344) | 2 | (-2.17, 0.49, 0.29); (-2.17, -0.49, 0.29) |
| Porsche_911GT2_2008 (2277356) | 2 | (-2.17, 0.49, 0.29); (-2.17, -0.49, 0.29) |
| Porsche_911GT2_2008 (2277368) | 2 | (-2.17, 0.49, 0.29); (-2.17, -0.49, 0.29) |
| Porsche_911Turbo930_1982 (1085186) | 1 | (-1.95, 0.64, 0.31) |
| Porsche_918Spyder_2013 (2196000) | 2 | (-1.06, 0.32, 0.96); (-1.06, -0.32, 0.96) |
| Porsche_918SpyderConceptStudy_2010 (1085830) | 4 | (-0.80, 0.92, 0.30); (-0.92, 0.93, 0.30); (-0.80, -0.92, 0.30); (-0.92, -0.93, 0.30) |
| Porsche_PanameraTurboS_2011 (535621) | 4 | (-2.44, 0.57, 0.30); (-2.42, 0.67, 0.30); (-2.44, -0.57, 0.30); (-2.42, -0.67, 0.30) |
| Shelby_Cobra427_1965 (1085576) | 2 | (-0.62, 0.83, 0.22); (-0.62, -0.84, 0.23) |
| Shelby_MustangGT500_1967 (2196334) | 2 | (-0.81, 0.87, 0.25); (-0.81, -0.87, 0.25) |
| SRT_Viper_2013 (1160350) | 2 | (-0.64, 0.91, 0.22); (-0.64, -0.91, 0.22) |
| Subaru_CosworthImprezaSTICS400_2010 (122814) | 4 | (-2.12, 0.40, 0.35); (-2.11, 0.49, 0.35); (-2.12, -0.40, 0.35); (-2.11, -0.49, 0.35) |
| Tesla_Roadster25S_2011 (1085764) | 0 | none (no nitrous flame record: no exhaust marker) |
| VEH_122706 (122706) | 2 | no effect records: body-shape fallback |
| VEH_122713 (122713) | 2 | (-2.70, 0.67, 0.33); (-2.70, -0.67, 0.33) |

## Tests

`MW2012_GAME_DIR=<2012 install> cargo test -p nfsmw-2012-content --lib every_car_has_pipes -- --ignored --nocapture` prints
this table and checks the pipes are on the bodies and the Tesla has none. The pure parts (group of both sides, mirror flag,
centre pipe, no pipe, the unplaced group, the fallback) are unit tests of `pipes.rs`.
