# Customizing mod cars: the shop, paints and parts

Part of the [modding catalogue](README.md). A car a mod adds ([api-content.md](api-content.md#cars)) is one more entry in the
car list, named by its folder, with files of its own. Nothing about it comes from a car of the 2005 game: not its
parts, not its physics, not its sound. This page says what the auto shop (the garage, the career shop and the F6 garage)
offers such a car, how a mod lists the paints and parts, and what the car's files must hold for the shop to work on it. The
other halves of a car are on their own pages:

| Page | Covers |
|---|---|
| [car-stats.md](car-stats.md) | `[stats]` (mass, engine, gearbox, tyres, brakes in SI units), the upgrade levels, the shop's bars, nitrous |
| [car-sound.md](car-sound.md) | `[engine_sound]`, `[car_sound]`, the built-in engine, the 2012 recordings |
| [car-collision.md](car-collision.md) | `BOUNDS.BIN`: the collision shape, the damage zones |
| [car-damage-parts.md](car-damage-parts.md) | `[[damage_parts]]`: bonnet, bumpers, mirrors and doors that turn or fall off as the car is damaged |
| [car-lamps.md](car-lamps.md) | the lamp markers and lamp textures: flares and lit surfaces of the head, brake and reverse lamps |

The [2012 cars mod](../../mods/nfsmw-2012-content/README.md) is the worked example, and its settings are on its own page.

## The files of a car

A car is the folder `files/CARS/<ID>/` of a mod plus the manifest `cars/<id>.toml` ([api-content.md](api-content.md#cars)):

| File | Needed | Holds |
|---|---|---|
| `GEOMETRY.BIN` | yes | the body and wheel solids, in the install's format, **under the names below** |
| `TEXTURES.BIN` | yes | their textures |
| `BOUNDS.BIN` | yes, to drive | the car's own collision bounds ([car-collision.md](car-collision.md)); the garage leaves out a car whose physics cannot be built, and the game refuses it with the reason in the log |
| `VINYLS.BIN` | only for `vinyls` | the vinyl pictures, a texture pack (`PREVINYL.BIN` is read too) |
| `SOUND/...` WAV files | only for `[engine_sound]`, `[car_sound]` | [car-sound.md](car-sound.md) |

The game builds the car from its own solids, named after its folder (`<ID>` is the folder, in upper case, as in the
manifest's file name). A solid it does not find is not drawn, and a car with neither a body nor a wheel solid does not show:

| Solid | Is |
|---|---|
| `<ID>_KIT00_BODY_A` to `_D` | the body, with the glass, lamps and interior, the tail pipe markers ([Tail pipes](#tail-pipes)) and the lamp markers ([car-lamps.md](car-lamps.md)) |
| `<ID>_KIT00_FRONT_TIRE_A` to `_D` | one wheel solid: tyre, rim, brake disc and caliper, axle along y, rim face at y = 0, as the game's wheels are modelled; the car draws it at all four corners and the game turns it for the left side |
| `<ID>_KIT00_DRIVER_A` to `_D` | optional: the driver, drawn in the cockpit while the car is driven (never in the garage or the shop), modelled in car space like the body, seated where the car's seat is (the 2012 content mod puts the game's own seated character there, [mods/nfsmw-2012-content/README.md](../../mods/nfsmw-2012-content/README.md#the-driver)); without it the car has no driver |
| `<ID>_KIT00_DRIVER_S0_A` / `_B`, `S1`, `S2` ... and `<ID>_KIT00_DRIVER_BACK_A` / `_B` | optional: the driver posed for the steering and for the reverse gear ([The driver's poses](#the-drivers-poses)) |

The letters are the levels of detail, `A` the finest; `E` is not looked for, and a level a car lacks draws nothing at that
distance, so A and B are the finest the player sees. The paint placeholder texture is `<ID>_SKIN1`, on the paint surfaces.
Solids are in car space (+x forward, +y left, +z up), centred on the wheelbase: the wheels sit at plus and minus half the
`wheelbase_m` of `[stats]` ([car-stats.md](car-stats.md)).

The game finds these through three parts it adds to its parts database for the car (`car/mods/parts.rs`): the body, the wheel
and the driver, each of the car's own type, with these solids as their models (a part whose solid the car lacks draws nothing). Nothing else is added: the car has no brake part (the wheel
solid holds the discs), no hood, spoiler, roof, damage overlay, decal or window tint part, and its paint, rims and tints are
the choices of its `[customization]` table. The 2005 parts database allows 256 car and part-group names, about 106 of which
the game uses, so a game that publishes more than about 150 mod cars leaves the later ones without a body part (the log says
so). Hand-made cars must use the names above: the 2012 mod writes exactly these, and its real-install test loads them.

## What the shop offers a mod car

The auto shop offers a mod car **what the car's own `[customization]` table lists** and nothing from a stock car's parts
table. The table and the files in the car's folder are the whole source:

| Auto shop item | Mod car | Where it comes from |
|---|---|---|
| Paint finish, colours, custom RGB body colour | yes | `paints`: each has a name, a colour and a finish; one is the car's paint until the player picks another |
| Rim paint | yes | `rim_paints` |
| Window tint | yes | `tints`: a colour and an opacity |
| Rims | yes | `rims`: wheel models in the car's own `GEOMETRY.BIN` |
| Spoilers, roof scoops | yes | `spoilers`, `roof_scoops`: models in the car's own `GEOMETRY.BIN` |
| **Vinyls** and their colour channels | yes | `vinyls`: pictures in the car's own `VINYLS.BIN`; `vinyl_colors` is the palette |
| **Liveries** | yes, listed with the vinyls | `liveries`: whole-body pictures in the car's own `TEXTURES.BIN` ([Liveries](#liveries)) |
| Performance upgrades (engine, transmission, tires, brakes, suspension, induction, nitrous) | yes, three levels each | steps of the car's own `[stats]` ([car-stats.md](car-stats.md#upgrades)) |
| Nitrous | yes: from the factory (`nitrous = true`), or bought as a kit | [car-stats.md](car-stats.md#nitrous) |
| License plate text | yes, free, not in the cart | `[customization.plate]`: a glyph sheet and a plate mesh; the text is kept beside the save ([License plates](#license-plates)) |
| License plate design | yes, free, not in the cart | `designs` of `[customization.plate]`: a list of backgrounds shared by the cars of a mod ([Plate designs](#plate-designs)) |
| Body kits, hoods, manufacturer decals, door numbers | no | a mod car has no such parts; a mod that wants a body variant adds another car |

A car that gives no `[customization]` table offers the shop nothing of its own: its paint is what its textures hold and the
shop lists no choice for it.

The garage lists the car under the display name and brand its manifest gives (`name`, `manufacturer`; without a name it shows
the car id, without a brand none). `nfsmw_data::car::mods::titled` is the one place that decides the name and brand a screen
shows for a car, and the screens built on the `Car_Select` package (the garage's Add New Car page, the car lot, the custom race's
car select, Quick Play, the Showcase) name a mod car with two pictures drawn from that brand and name: the game names a stock car
with its maker's logo and a picture of its model name, and the package's own sample pictures read `tp` and `temp350`
(`frontend/car_title.rs`, `ui/name_picture.rs`). The maker's slot shows the brand's logo when there is one, and the brand written
out when there is none (clear for a car with none); the model's slot shows the name less the brand in front of it. A brand has a
logo when the game already has one of that name (compared ignoring case, spaces and punctuation: a mod car of brand `subaru`
shows Subaru's logo) or when a mod declares one ([Manufacturers](api-content.md#manufacturers), which can also replace the
game's logo of a brand).

**The developer tools.** The F12 `parts` command and the F6 garage panel list and apply a mod car's own choices (paints, rim
paints, tints, rims, spoilers, scoops, vinyls, and the liveries among the vinyls) from the same catalogue as the auto shop
(`Inventory::catalog`), for the car being driven (not for the car the panel has selected for browsing), and read the inventory
again when a mod car was published after it was loaded. In a world with a converted Nissan Skyline R34 GT-R, `parts vinyl 3`
and `view-car NISSKYLINE99 --part vinyl=3` put its third livery on the car.

To look at a car's paint without starting the world:

```
nfsmw view-car POR911CARR12 --part paint=3 --screenshot out.png --muted
```

The number after `=` counts from 1 in the order the `parts` console command lists (F12, then `parts` in a world with the
car; `parts paint 3` applies it there; both come from `garage::listing::arrange`, which puts each category in name order and
numbers choices that share a name, so a number means the same part in the viewer, the console and the F6 panel). The categories are `body`, `spoiler`, `hood`, `roof`, `rims`, `paint`,
`tint`, `rimpaint` and `vinyl`.

## Customization: `[customization]` in the car manifest

A mod car carries its own customization. Nothing in it names another car. The table is checked by
`libs/blackbox-modding/src/content/customization.rs` and read by `crates/nfsmw-data/src/car/mods/custom/`.

```toml
# cars/fastcoupe.toml
[customization]
default_paint = 0                   # position in paints (default 0)
paints = [
  { name = "Red", color = [173, 43, 44] },                          # finish: gloss (default), metallic or chrome
  { name = "Silver", color = [200, 204, 197], finish = "metallic" },
]
rim_paints = [{ name = "Black", color = [10, 10, 10] }]
tints = [{ name = "Smoke", color = [20, 30, 40], opacity = 0.7 }]   # opacity 0 (clear) to 1
rims = [{ name = "Five spoke", model = "FASTCOUPE_RIM1" }]
spoilers = [{ name = "Wing", model = "FASTCOUPE_WING" }]
roof_scoops = [{ name = "Scoop", model = "FASTCOUPE_SCOOP" }]
vinyls = [{ name = "Stripe", bitmap = "FASTCOUPE_STRIPE", mask = "FASTCOUPE_STRIPE_MASK", channels = 2 }]
vinyl_colors = [{ name = "White", color = [255, 255, 255] }]        # left out: a built-in palette of 12 colours
liveries = [{ name = "Livery 1", texture = "FASTCOUPE_LIV1" }]       # a picture in the car's TEXTURES.BIN
```

| Key | Meaning |
|---|---|
| `paints` | At least one, at most 64. `color` is sRGB (three numbers from 0 to 255). `finish` picks the shading: `gloss` is the plain paint material, `metallic` the metallic one, `chrome` the mirror one (the shared light materials `REGPAINTWHITE`, `METPAINTSILVER`, `CHROME`). |
| `default_paint` | The paint the car has until the player changes it; it is the car's stock paint, so it is not listed among the choices (picking nothing is picking it). |
| `rim_paints` | Colours for the metal of the rims. They work on the wheels the car ships with and on `rims`. |
| `tints` | A copy of the shared windshield material with the colour applied and the alpha set to `opacity`. |
| `rims` | `model` is a wheel solid of the car's own `GEOMETRY.BIN`, **without the level-of-detail letter**: the game looks for `<model>_A` to `<model>_E` and uses the nearest level the pack has. The solid replaces the whole wheel (tyre and rim) and is modelled as the car's own wheel solid is: axle along y, rim face at y = 0. |
| `spoilers`, `roof_scoops` | `model` as for rims. The solid is in car space (+x forward, +y left, +z up), already at its place on the body: it is drawn as it is, with no mount marker. |
| `vinyls` | `bitmap` and `mask` are texture names in the car's own `VINYLS.BIN` (a texture pack; `PREVINYL.BIN` is read too). The mask's red channel is the opacity. `channels` (0 to 3) says how many of the bitmap's red, green and blue weights are colour channels the player can recolour; 0 keeps the bitmap's own colours. |
| `vinyl_colors` | The colours the channels can take, for all three channels. |
| `liveries` | `texture` is a texture name in the car's own `TEXTURES.BIN`, in the layout of the car's paint surfaces. See [Liveries](#liveries). |
| `default_livery` | A position in `liveries`: the livery the car wears until the player picks a vinyl or a livery. The shop lists a `No livery` choice with the liveries, which takes it off and leaves the plain paint (and the paints and finishes the player picks). Left out, the car starts in its plain paint. |

- A name is 1 to 32 characters, with no control character, and unique within its list. Models and textures are letters,
  digits, `_` and `-`, up to 56 bytes. A list has at most 64 entries. A wrong value, or a key the table does not know,
  refuses the car and the log names the key.
- A choice is told apart by a hash of the car's folder, the kind and the name, so a saved car keeps its choice when the mod
  reorders its lists, and loses it when the mod renames it.
- A model that is missing from `GEOMETRY.BIN` (the `_A` level), or a vinyl whose pictures are missing, differ in size or
  cannot be decoded, is **not offered** and the log says why. The paint placeholder is the texture `<ID>_SKIN1` (the car's
  own folder) on the car's paint surfaces, with the light material `CARSKIN`; the game swaps it for the colour, and for a
  vinyl laid over it.
- The shop prices every choice of a mod car at the first tier of its category (the game's price table has no entry for the
  mod's parts).
- Decals, door numbers and the HUD gauge skins are not part of the table; the gauge skins of the install work as for any car.

### Vinyls

A vinyl is a picture authored in **the car's own paint texture layout**: the picture is drawn over the paint surfaces through
their texture coordinates, so the mod authors the picture for its own coordinates. The bitmap and the mask are texture-pack
entries of `VINYLS.BIN`; the shop composes the layer (`vinyl::compose`) and draws it over the paint colour. The 2012 cars give
none (see below).

### Liveries

A livery is a whole-body picture, authored for the car's own paint texture coordinates, laid over the body paint **by its
alpha**: where the picture is opaque it replaces the paint, where it is transparent the paint shows (the rule of the 2012
game's livery shader, [chameleon-customization.md](../formats/chameleon-customization.md#liveries)). It differs from a vinyl
in three ways: the picture carries its own colours (there are no recolourable channels), its alpha is its mask (there is no
mask texture), and it lives in `TEXTURES.BIN`, not `VINYLS.BIN`. The picture may be DXT1 with one-bit alpha, DXT3, DXT5 or
a 32-bit texture; DXT1 is read with its one-bit alpha.

The shop lists the liveries among the **vinyls** (the vinyl category: a livery and a vinyl are both a picture over the paint),
after the vinyls, in the order of the list; choosing none keeps the plain paint. A livery is not offered, and the log says
why, when its texture is missing from the car's `TEXTURES.BIN` or cannot be decoded. A livery may not take the name of a
vinyl or of another livery. The choice is told apart like the others, by a hash of the folder, the kind (`LIVERY`) and the
name. A livery is drawn on every paint surface of the car (they all share one texture layout); the paint the player picked
shows only where the picture is transparent.

A car that gives `default_livery` wears that livery from the start, in the garage, the shop and the world, wherever the car is
built without a vinyl choice (a cop car that spawns in a pursuit, a car of the traffic). Picking `No livery` in the shop is a
choice too, so the car then has the plain paint. A livery whose picture cannot be read
leaves the plain paint and the log says so. The car's vinyl and livery list (the garage's *Vinyls & Liveries* page) is the
way to take another look.

### The 2012 cars

The [2012 mod](../../mods/nfsmw-2012-content/README.md) fills `paints`, with the colour records of each car's own bundle (1 to 15 per
car, 396 in the 93 bundles), and `liveries`: that is all the 2012 game offers for a car's looks (no vinyls, rims, spoilers,
roof scoops or tints to choose; [chameleon-customization.md](../formats/chameleon-customization.md)). The paint texture
coordinates of a converted car are the 2012 car's own. The 2012 game ships five vehicles (the BMW M3 GTR, Lamborghini Diablo SV,
Porsche 911 GT2, Nissan 350Z and Nissan Skyline R34) in five liveries each, a bundle per livery; the mod makes one car of each
and lists the five pictures under the names of the 2012 game (`Hero`, `Race`, `Street`, `Abstract`, `Urban`; 67 cars from 93
bundles, the 2012 list having six cars under two vehicle ids). A car whose own bundle draws one livery
picture over most of its paint, with no other bundle to choose from (the Porsche 918 Spyder, the Bentley, the Lancia Delta, the
Bugatti Veyron Grand Sport Vitesse, and the police cars: the Dodge Charger and the Crown Victoria, the Ford Explorer
Interceptor and the Corvette), lists that picture as `Livery 1`; every car with a livery wears its first one from the start
(`default_livery = 0`), and the player can pick `No livery` to use the paints and finishes instead.

### Tail pipes

The game starts the nitrous flames at the markers `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST` of the car's body solid
(the marker's z axis points backwards), so a mod's own car has to carry them. A car without pipe markers boosts, but shows no
flame. The flame emitters themselves are the game's default `ecar` effect group. The 2012 files have tail pipe locators,
and the 2012 mod writes one marker for each of a car's pipes from them ([chameleon-exhaust.md](../formats/chameleon-exhaust.md)),
none for a car without a pipe (an electric car boosts with no flame). Only a car whose bundle has no effect records gets
pipes derived from its body's shape.

### License plates

A car can carry a licence plate whose text the player types. The plate is a **glyph sheet** (a texture of the car's own
`TEXTURES.BIN`, `columns` by `rows` equal cells, one glyph each) and a mesh of `slots` quads, side by side, in the body solid:
quad `n` is mapped to cell `n` of the sheet, so the sheet as authored shows its first glyphs. To show a text the game draws a
copy of the sheet whose first `slots` cells hold the glyphs of the text, centred, with the blank glyph (the space of the
sheet) in the slots the text leaves empty. The table is `[customization.plate]`:

```toml
[customization.plate]
text = "NEED4SPD"                                   # what the car wears until the player types another (optional)
picture = "FASTCOUPE_PLATE"                         # the glyph sheet, in TEXTURES.BIN
glyphs = "ABCDEFGHIJKLMNOPQRSTUVWXYZ 123456789"     # the glyph of each cell, row by row; the space is the blank
columns = 9
rows = 4
slots = 8                                           # characters the plate shows
aliases = { "0" = "O" }                             # typed characters drawn as another glyph (optional)
designs = "PLATES/SET/DESIGNS.TOML"                 # the plate designs the car offers (optional, [below](#plate-designs))
default_design = "1863272"                          # the design it wears until the player picks one (optional)
```

| Key | Meaning |
|---|---|
| `picture` | A texture name of the car's own `TEXTURES.BIN`; DXT1, DXT3, DXT5 or a 32-bit texture. A sheet that is missing or cannot be decoded leaves the plate as authored, and the log says so. |
| `glyphs` | `columns * rows` printable ASCII characters (at most 256 cells), each once, **including the space**, which is the blank glyph. Upper case only: a typed letter is made upper case first. |
| `columns`, `rows`, `slots` | The cells of the sheet and the quads of the plate. Cell `n` is column `n % columns`, row `n / columns`; the first `slots` cells are the ones the plate quads show. `slots` is 1 to 16. |
| `aliases` | A typed character and the glyph it is drawn as, one character each (`"0" = "O"`); the typed character is not a glyph itself. |
| `text` | The car's default text, at most `slots` characters. Left out, the sheet as authored is the default. |
| `designs` | A `.toml` path under `files/` (letters, digits, `_ - . /`, no `..`): the index of the plate designs ([below](#plate-designs)). Left out, the plate has none. |
| `default_design` | The id of a design of the index that the car wears until the player picks another; needs `designs`. Left out, the plain sheet. |

- A typed character that is neither a glyph nor an alias is drawn as a space. A text longer than `slots` is cut.
- The **player's text** is kept per car and per profile, beside the save, not in it ([below](#where-the-text-and-the-design-are-kept)), and is
  typed in the garage: Visual, **License Plate** (a keyboard prompt like the alias name; the letters and digits the sheet has,
  at most `slots`; an empty text goes back to `text`). Typing costs nothing and is not part of the cart.
- `nfsmw view-car FASTCOUPE --part plate=VELOCITY --screenshot out.png --muted` shows a car with a text on its plate.

### Plate designs

A plate can offer **designs**: pictures the player picks to put behind the glyphs, as the 2012 game's Autolog has them
([chameleon-plate-designs.md](../formats/chameleon-plate-designs.md)). The designs are not part of a car. The plate names an
**index**, a TOML file under the mod's `files/` that every car of the mod can name, so the pictures are written once, not once
per car (the 2012 mod's 469 designs are about 4 MB for all its cars):

```toml
# files/PLATES/SET/DESIGNS.TOML
pack = "DESIGNS.BIN"            # a texture pack in the folder of the index (any TPK, DXT1, DXT3, DXT5 or 32-bit textures)

[[design]]
id = "1863272"                  # 1 to 64 letters, digits, _ and -: what plates.toml keeps; each id once
name = "START ME UP"            # shown in the garage, 1 to 64 characters
picture = "PD1863272"           # a texture of the pack: the ground behind the glyph strip
group = "Base game"             # optional category label
glyphs = "PG1863272"            # optional: a glyph sheet of the pack in the car's layout and size, instead of the car's own
ink = [20, 20, 24]              # optional: the glyph colour; left out, dark on a light picture and light on a dark one
```

- **How it is drawn.** The picture is stretched over the `slots` cells the plate shows (so author it in the shape of the
  plate's glyph strip: about 4.4 times wider than high for the 2012 cars), and the glyphs of the sheet are drawn over it: the
  sheet's ground becomes the picture, its dark strokes keep their shape in the ink, and the picture is eased a little toward the
  opposite of the ink so the glyphs stay readable. A design's own `glyphs` sheet is used for the glyph shapes instead of the
  car's; a sheet that is not the size of the car's is refused (the car's is drawn instead, and the log says so). A sheet with no
  contrast takes no design.
- **Offered.** A design whose picture is not in the pack is left out; an index that does not read, or a pack that is missing,
  offers none and the log says so (the car still has its text). A design that is malformed or repeats an id is left out of the
  index with a warning.
- **The garage.** Visual, **Plate Design** (next to License Plate): the plain plate first, then the designs in the order of the
  index. Left and Right step through them (L and R ten at a time, the mouse wheel too), the car shows each, Accept keeps one,
  Back puts back the one it wore. In a theme with vertical menus the dialog lists the names around the one picked. It costs
  nothing and is not part of the cart.
- `nfsmw view-car FASTCOUPE --part plate=VELOCITY --part plate-design=1863272 --screenshot out.png --muted` shows a design;
  `--part plate-design=none` is the plain plate.

#### Where the text and the design are kept

The original game's save has no place for a plate text, and a mod car's record in it already depends on the mod's car
folder, so the text is **not** written into the save: it is in the profile's own data folder, `plates.toml` next to
`my-cars.toml` (a guest has `plates-guest.toml` in the settings folder):

```toml
[plates]
POR911CARR12 = "MY PLATE"

[plate_designs]
POR911CARR12 = "1863272"    # the empty id is the plain plate, picked on purpose
```

The key is the car folder in upper case: one text and one design for each car model of the profile, not for each copy of it in the
garage. A design the player never picked is not in the file, and the car wears the manifest's `default_design`.
A profile without the file, and every save of the original game, wear the manifest's texts; the file never changes the save,
so saves stay compatible both ways (a file written before designs existed has no `[plate_designs]` table and reads as before).
`garage/plates.rs` reads and writes it; `custom::plate::set_source` and `set_design_source` are how the car builder asks for the
text and the design.

## The driver's poses

A car may ship its driver in several poses, so it moves with the steering. Besides `<ID>_KIT00_DRIVER_A..D` (the driver as
modelled) it names pose solids in the same file, in car space and seated like the driver:

| Solid | Is |
|---|---|
| `<ID>_KIT00_DRIVER_S<k>_A` to `_D` | steering pose k = 0, 1, 2 ... (no gaps, at most 31): k = 0 is the driver with the wheel full left, the last one full right, the middle one straight (an odd count puts the straight one in the middle) |
| `<ID>_KIT00_DRIVER_BACK_A` to `_D` | the driver looking back, drawn while the gear is reverse |

The game draws, each frame, the one pose whose steering is nearest to the player's steering input (-1 full left, 1 full right;
the input, not the wheels' angle, which the speed limits), or the look-back pose in reverse. It draws no more than one pose
and does nothing else per frame: no vertex is moved and nothing is allocated. At a level of detail (`A` to `D`) where the car
has any pose solid the plain driver is not drawn; at a level with none (say a car poses only `A` and `B`) the plain driver is, so
distant levels may be left out. The poses are separate geometry (the driver's whole mesh each), so a few are enough: they step,
they are not blended. A car with no pose solid is drawn exactly as before, and so is every stock car (`car/mods/driver_poses.rs`).

The 2012 content mod bakes 7 steering poses and the look back from the 2012 game's own animation
([chameleon-driver-animation.md](../formats/chameleon-driver-animation.md)). To see one without a window: `nfsmw view-world --drive
<ID> --drive-script "1:throttle=1;1:throttle=1,steer=1" --camera-mode hood --screenshot out.png --muted` (the last second at full
right; `steer=-1` for left); the look back shows while the gear is reverse.

## Using it

1. Add the car (a mod with a `cars/<id>.toml`, or the [2012 mod](../../mods/nfsmw-2012-content/README.md)).
2. Start the game, open the garage or the shop (a safe house or an auto shop in the world, My Cars in the main menu) and
   pick the car. The paints and parts its `[customization]` table lists, and the performance upgrades, are on the usual screens.
3. To change how the car drives, edit its `[stats]` table ([car-stats.md](car-stats.md); for the 2012 cars, the
   multipliers of `config.toml`) and restart.

## Limits

- **Body kits and hoods** are not offered: a mod car has none (a body variant is another car).
- **Saved careers.** My Cars slots remember the car's folder. The career's own garage record names a car by its car type,
  which for a mod car is the hash of its folder name; the 2005 database has no vehicle record of that name. A career save
  with a mod car has not been tested.
- **Part fit.** A rim, spoiler or roof scoop model is drawn as it is, at the place the mod modelled it. The game does not
  move it to fit the body.
- **Livery blending.** A livery's picture is flattened over the paint into one texture per choice, without mip levels, as the
  vinyls are; the flat paint of a car with no livery is a 4 x 4 texture. Two liveries cost one texture of the picture's size
  each in memory while chosen. A livery changes only the paint surfaces' texture: lamps, glass and the rest of the car are
  unchanged.
- **Coverage.** The paints of the 2012 cars are checked against the real installs (the conversion of all 93 bundles, and the
  garage catalogue and paint of one). The other parts of the table (rim paints, tints, rims, spoilers, roof scoops, vinyls)
  are covered by unit tests only, because no 2012 car uses them; a mod that does has to check them on its car.
- **Not played.** Nobody has driven the converted cars or used their shop in a window: the checks are the conversion of
  all 93 cars and the tests named on [mods/nfsmw-2012-content/README.md](../../mods/nfsmw-2012-content/README.md#tests).
