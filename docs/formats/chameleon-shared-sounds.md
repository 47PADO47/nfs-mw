# Chameleon shared sounds: `METALCAR.BNDL`, `WAVES.BNDL` and the graphs that name them

The 2012 game keeps the recordings that every car shares in two bundles of the install and names them in sound graphs
(kind `0x15`). The sample layout (kind `0x81`, an `SPS` stream behind a 128-byte header) and the graph imports are in
[chameleon-sound.md](chameleon-sound.md). Everything here was measured on the 2012 install (v1.4.0.0) with the reader of
`mods/nfsmw-2012-content` (`src/shared/`). For the evidence tags see [README.md](../README.md#evidence-tags). The
conversion that uses it is in [mods/nfsmw-2012-content/README.md](../../mods/nfsmw-2012-content/README.md), the manifest that carries the
result in [car-sound.md](../modding/car-sound.md#the-shared-sounds-car_soundshared).

## What the two bundles hold **[measured]**

| Bundle | Resources | Samples |
|---|---|---|
| `SOUND/METALCAR.BNDL` | 197: 119 samples, 38 graphs, 40 node types | 119 one-shots of 0.4 to 3.2 s, none looping, all mono |
| `VEHICLES/WAVES.BNDL` | 172: 126 samples, 8 graphs, 26 node types | 126 samples: pops, the turbo's tone and blow-off, horn loops, wind-like loops, drive-by whooshes, traffic engines |

Neither bundle carries a name table (`debug_xml` is absent). The graphs that play the samples sit partly in other
bundles: `SOUND/GLOBAL.BNDL` (the shared engine graphs), `HAWAII/SURFACESOUNDS.BNDL` and `HAWAII/SURFACELIST.BNDL` (the
sounds of surfaces), `VEHICLES/TRAFFICATTRIBS.BNDL`. Resource ids are global across bundles, so a graph of one bundle
imports a sample of another by id; the reader finds them by walking the imports.

## Named graphs **[verified: the text is in the graph's main block]**

A sound designer's name for a graph is stored as text in its main block. Searching every graph of the install for text
that is a name (capital letter, a space) gives these that concern a car's sound:

| Name | In | Plays |
|---|---|---|
| `Player Vehicle Collision Small`, `Medium`, `Big` | `METALCAR` (three graphs of each) | METALCAR samples, below |
| `Competitor Vehicle Collision Small`, `Medium`, `Big` | `METALCAR` (three of each) | the same samples and a few more: the crashes of the other cars |
| `Player Vehicle Scraping` | `SURFACESOUNDS` | three METALCAR samples (`b38f2`, `b38f3`, `b38f4`) |
| `Player Vehicle Prop Collision` | `GLOBAL` | samples of `GLOBAL.BNDL`, not shared bundles |
| `Competitor World Collision`, `Landings Large` | `SURFACESOUNDS` | a sample of `SURFACESOUNDS` |
| `Player Vehicle Horn` | `GLOBAL` | two looping horn samples of `WAVES` (`1d1c70`, `1d1c71`) |
| `Racer And Cop Passbys`, `Wrecked Passbys` | each car bundle | WAVES whooshes (`1b0cc0` to `1b0ccd`, `11f49a` to `11f4a6`, `159f4e`): another car passing the player, not the player's own sound |

The collision graphs also read `RaceCars.Entities[#].GetMassForLastCrash` and `...MostWantedCarInLastCrash` (the graph
has copies for a crash with a Most Wanted car and without), so the three sizes follow the mass of the other body in the
last crash and the three graphs of a size are alternatives. The scraping graph reads `LocalPlayer.GetPlayerSpeed` and
`Signals.GetRandomNumber[2]` twice: a random one of its three samples.

## The body hits **[measured; the mixing is not decoded]**

A collision graph imports, in order, a selector node and the samples it picks one of, then another selector and its
samples. The first selector of every `Player Vehicle Collision <size>` graph is the **body crunch**: samples of 0.4 to
2.6 s with the loudest level (rms 0.28 to 0.60 against 0.07 to 0.36 for the others) and an energy centre around 3 to 5 kHz.
Its samples, by size (resource id low digits):

| Size | First selector of its three graphs | Count | Length |
|---|---|---|---|
| Small | `b38f5`-`b38f8`, `b38f9`-`b38fc`, `b38fd`-`b38ff` | 11 | 0.4 to 1.0 s |
| Medium | `b390a`-`b390d`, `b3903`-`b3906`, `b3907`-`b3909` | 11 | 0.5 to 1.8 s |
| Big | `b392a`-`b392d`, `b3922`-`b3925`, `b3926`-`b3929` | 12 | 0.7 to 2.6 s |

The other selectors of the same graphs are layers on top of the crunch, each behind a small gain graph (`753b`, `750c`,
`87c81`, `1b0b8a`): a bright, quiet layer (energy centre 6 to 8.5 kHz, rms 0.12 to 0.36), a thud layer (`87c7a`-`87c7f`,
which `GLOBAL.BNDL` also imports under `LocalPlayer.GetPlayerSuspensionLoad`, so it is part of landings too) and two
2.85 s tails (`1b0b85`, `1b0b86`). How loud each layer is at what mass is in the graph's nodes, which are not decoded, so
**only the first selector is used**: it is a complete crash sound and the one thing every graph of a size shares in
position.

**Not identified.** The METALCAR samples that no collision graph imports (`22662`, `87cd1`/`2`, `87d02`/`3`/`5`, `87dbc`,
`87de2`, `95ada`-`95ae0`, `b3912`-`b3921`, `b4304`/`5`, `14e015`-`14e018`) are listed by the bundle's top graph and
nothing else; the surface graphs of `SURFACELIST` use `87f2e`-`87f35`, `87f3e`-`87f42` and `d4c60`-`d4c63` for surfaces
whose names are in no data (a surface is chosen in code). **No graph names a material** (glass, concrete, a barrier), a
skid, a glass break or a tyre burst in the shared bundles: skid graphs read `SpinBrakingIntensityForSkidSound` and import
`SURFACESOUNDS` samples, six of them, with nothing to say which surface each is. Those stay out.

## The pops **[measured]**

90 of the 93 car bundles have a graph that reads `GetPopProbability` (with `PopGain` and `GurgleGain`), and **only 18 of
them hold pop samples of their own**: the others import them from `WAVES.BNDL`, five or ten of the 40 pop samples
(0.04 to 0.20 s: `157365` to `17f8a6`, `1d1a91` to `1d1aa0`), always the same few combinations of the four shared pop sets
of `GLOBAL.BNDL` (graphs `1d1c4a`, `1d1c4d`, `1d1c53`, `1d1c5a`). The mod names, for each car, the shared samples that
its own pop graph imports. The 92 cars with a `RevLimitGain` graph import two or three more (`157365`, `157373`,
`17f7f8`, 0.04 to 0.10 s): the rev limiter's chop, which no host effect plays yet.

## Other shared samples that were looked at

`d6787` and `d4de7` (the turbo's whistle tone and blow-off) are in [chameleon-sound.md](chameleon-sound.md). `1c0613`,
`1c062e`, `1c062f` are looping 0.9 to 1.05 s samples under `GetAudibleRPM` and `EngineOn`; `182183` and `182186` are
2.9 and 4.0 s loops under `GetPlayerSpeed`, `TimeInAir`, `EngineOn`; `182279` to `18227c` sit under
`WallTracker.Reflection[n]`, `IsInCanyon` and `DamageCritical` (reflections of the engine off walls). None was
identified well enough to play.
