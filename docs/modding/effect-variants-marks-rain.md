# Effect variants: skid marks, rain splash, rain drops and the wet road

Part of [effect-variants.md](effect-variants.md) (read its "How a variant works" first: the option, the folder, the profile read through
the install's layers). This page is the profile of four more kinds. The 2012 mod's values for them, how they were found and what the
2012 install does and does not hold: [chameleon-tyre-marks.md](../formats/chameleon-tyre-marks.md) and
[chameleon-rain-water.md](../formats/chameleon-rain-water.md).

| Kind | Option id | Folder in the mod's `files/` | Profile | Draws |
|---|---|---|---|---|
| Skid marks | `skid_marks_variant` | `SKIDMARKS/<ID>/` | `blackbox_modding::content::skid_marks` | pictures on the strips the tyres leave (`scenes/world/effects/marks_variant.rs`) |
| Rain splash | `rain_splash_variant` | `RAINSPLASH/<ID>/` | `blackbox_modding::content::rain_splash` | splashes where the rain meets the road (`scenes/world/effects/rain_splash.rs`) |
| Wet road | `wet_road_variant` | `WETROAD/<ID>/` | `blackbox_modding::content::wet_road` | nothing of its own: it scales the road's wetness (`wet_reflections/state.rs`) |
| Rain drops | `rain_drops_variant` | a [replacement set](api-replace.md) of kind `rain_drops` | none | the drops on the lens, by replacing the 2005 picture `RAINDROPPC` |

The first three are the effect-variant machinery of [effect-variants.md](effect-variants.md#in-the-game-for-the-next-kind): an option
with the value `original` (hidden until a mod adds a value), a profile read once for each chosen value, handed to the scene in
`EffectVariants`. Each has an option in **Options > Gameplay** (the variants group, after the game's own rows), labelled in the
catalogue (`options-skid-marks-variant`, `options-rain-splash-variant`, `options-wet-road-variant` in `crates/nfsmw/i18n/en/options.ftl`;
a language pack translates them like any other message, [translations.md](translations.md)). The rain drops are *only* a replacement
set: nothing in the game's code knows them, the host adds the option `rain_drops_variant` from the set
([api-replace.md](api-replace.md)).

## Profile format of the skid marks

```toml
format = 1

[[set]]                                 # one to eight; a road kind and a weather appear once
surface = "paved"                       # "paved" or "loose" (grass, golf turf, sand, dirt, mud, gravel)
wet = false                             # true: the set of a wet road; a surface with no wet set uses its dry one
slip = "SKIDMARKS/MW2012/PAVED_SLIP.PNG"   # the mark of a tyre that slides sideways (RGBA PNG)
spin = "SKIDMARKS/MW2012/PAVED_SPIN.PNG"   # the mark of a tyre that spins or locks along the road
slip_tile = 4.0                         # metres of mark one repeat of the picture covers, 0.25 to 64 (default 4)
spin_tile = 2.0                         # (default 2)
width = 1.0                             # times the width of the tyre's tread, 0.25 to 4
opacity = 1.0                           # times the picture's alpha, 0 to 1
life = 45.0                             # seconds a mark stays, 5 to 300; the last 5 of them it fades
```

A set needs at least one of `slip` and `spin` (the other reads it). The picture's **columns run along the mark** and its rows across
the tyre; its alpha is the mark's strength (a harder skid draws it darker) and its colour is the mark's colour.

What the game keeps of its own: **where a mark begins and ends, how long a stamp for a wheel spinning on the spot is, the tread's
width, and whether a bike leaves one at all** (`bike_slide.rs`, the physics' `skid` intensity) are unchanged; a profile only changes
what a mark looks like, how wide a tread's mark is by a factor, and how long it stays. With no variant, or for a road the profile has
no set for, the strips are the game's own dark untextured ones. The marks of a variant are cut at every repeat of the picture, so a
quad's texture coordinates stay in 0 to 1 (the wrapping of the sampler does not matter), and are lifted 3 cm off the road (the
textured pass has no depth bias).

Which picture a mark gets: the road under the tyre (`Contact::loose`, from the `simsurface` hash of the wheel against the loose
surfaces `grass, golf_fairway, golf_rough, golf_green, sand, dirt, mud, gravel`), whether the road is wet (the game's own `wet`, the
rain or the enhanced wet surfaces' dampness, under cover excepted) and whether the tyre slid sideways more than it spun or locked
(`Contact::slide`, the lateral slip speed over the sum of it and the longitudinal one, 0.4 and over is `slip`). The mark keeps that
choice for its life.

## Profile format of the rain splash

The game's own rain makes no splash on the road, so "Original" draws none.

```toml
format = 1
radius = 10.0            # metres around the player in which splashes are made, 2 to 40 (default 10)
density = 1.0            # a level every layer's rate is multiplied by, 0 to 4

[[layer]]                # one to four
texture = "RAINSPLASH/MW2012/RING.PNG"   # an RGBA PNG; a sheet when grid is above 1
kind = "ring"            # "ring": lies on the road and grows; "burst": stands up from the road facing the camera
blend = "alpha"          # or "additive"
grid = 1                 # cells per side of the sheet, 1 to 8: a splash takes a random cell
rate = 140.0             # splashes a second at full rain in the whole area, 0 to 2000
life = 0.55              # seconds, 0.05 to 5
life_variance = 0.35     # how much of the life a splash may fall short of, 0 to 1
size = [0.06, 0.55]      # full width in metres at birth and at the end of the life, 0.01 to 20
opacity = 0.55           # the greatest alpha, 0 to 1: it rises over the first fifth of the life and falls to 0
color = [215, 225, 240]  # red, green and blue the picture is multiplied by
```

While it rains (the visible rain intensity, which includes the Rain amount setting) and the player is not under cover (the same
exposure test as the tyre spray and the lens drops), `RainSplash::step` makes splashes: each layer is owed `rate * density * rain`
a second, at random places of a disc of `radius` metres around the player's car, on the road below (the ground query of the
collision world, `space::ground_below`, 3 m above to 6 m below the car), at most 8 a frame and 384 alive. The pictures are loaded
once for each profile and freed when the choice or the scene goes; nothing is allocated per splash. A ring is drawn flat 3 cm above the
road and turns by a random angle; a burst stands on the road and faces the camera. The variant adds splashes to the game's rain; the
falling streaks, the lens drops and the tyre spray are the game's own (or the rain drops' set).

## Profile format of the wet road

```toml
format = 1
film = 1.25       # times the dampness of the road: how dark and glossy a wet road is, 0 to 2 (default 1)
pooling = 1.3     # times the pooling: how much standing water there is, 0 to 3 (default 1)
soak = 1.5        # times the rate at which the road gets wet, 0.25 to 4 (default 1)
dry = 0.7         # times the rate at which it dries and the pools drain, 0.25 to 4 (default 1)
```

The wet road is `WetRoad` (`wet_reflections/state.rs`): a dampness that rises while it rains (the original's 0.005 a render call
recurrence) and falls afterwards, and a pooling that builds after saturation. `soak` and `dry` scale the two rates; `film` and `pooling`
multiply the two levels the shading reads (`SurfaceWater`, kept to 0 to 1), which the darkening, the reflections and the enhanced
wet surfaces' puddles follow. When it rains and how hard is the game's own, the Environment override (F8) still sets both levels for
comparisons, and with "Original" every factor is 1, exactly the game's road. Note the road reflections themselves (the projected
capture, the ray traced ones) are the game's: a variant changes how wet the road is, not how it reflects.

## The rain drops: a replacement set

The drops on the lens are the 2005 game's `RAINDROPPC` (a displacement picture, [rain-pc.md](../specs/rain-pc.md)); a mod replaces
that texture by a set of kind `rain_drops`:

```toml
format = 1

[set]
kind = "rain_drops"
kind_label = "Rain drops"
value = "mw2012"
value_label = "NFS Most Wanted 2012"

[[texture]]
target = "RAINDROPPC"
file = "REPLACE/RAIN_DROPS/RAINDROPPC.png"
format = "rgba8"        # the picture is a displacement map: not block compressed
```

in `replace/rain-drops-mw2012.toml` of the mod. The picture's green and blue are where a pixel of the drop reads the scene (0.5 is the
neutral centre), its alpha is the drop's body; the game keeps the sizes, positions, lifetimes and the Lens droplet size setting. A
choice made during a drive follows at the next drive (a global texture, [api-replace.md](api-replace.md#textures)).

## The 2012 mod's values

All four are **NFS Most Wanted 2012**, written by the mod from your own install (a few KiB read by ranges, about 20 KiB written, no
car conversion needed), each with a setting in the mod's `config.toml`: `skid_marks`, `rain_splash`, `rain_drops`, `wet_road` (all
`true`; see the [mod README](../../mods/nfsmw-2012-content/README.md#skid-marks-and-rain)).

- **Skid marks**: the slip and spin diffuse pictures of the 2012 tyre mark materials, three sets (dry paved, wet paved, loose), found
  through the `SurfaceType` and `SurfaceVfx` objects of `SURFACELIST.BNDL` ([chameleon-tyre-marks.md](../formats/chameleon-tyre-marks.md)).
  `slip_tile` 4 m and `spin_tile` 2 m are **chosen**; `width` and `opacity` are 1.
- **Rain splash** and **rain drops**: the 2012 game has **no rain**, so these are made from the 2012 water pictures (a splash ring and
  a spray column; the round dots of a lamp glow turned into one drop on the lens) and are labelled so in the format page; every number
  is **chosen**.
- **Wet road**: chosen factors (a road that soaks sooner, shows a little more film and pools more, dries slower).

## Cost

Measured in `docs/formats/chameleon-tyre-marks.md#cost`. With no variant chosen nothing is loaded and nothing is drawn: the marks
cost one test per mark per frame (`covers`) and the splash one early return; with a variant, the marks cost one textured batch per
picture in use and the splash at most 8 ground queries a frame.
