# Effect variants

An **effect variant** is a look (and for the nitrous a sound) that a mod gives to one of the game's effects. Every kind of effect
that can have variants has one option in the gameplay settings, with one value of the game's own, **Original**: the effect as
the 2005 game plays it. The option is **not shown** while it has that one value, so the menu is as it was without a mod. A mod
adds a value with [`bb.settings`](api-settings.md); the row then shows, the player steps through the values, and the choice is
saved with the other settings (`[mod_values.nfsmw] option_<kind>_variant = "<value id>"`).

| Kind | Option id | Folder in the mod's `files/` | Profile | Page |
|---|---|---|---|---|
| Nitrous | `nitrous_variant` | `NITROUS/<ID>/` | `blackbox_modding::content::nitrous_fx` | [nitrous-variants.md](nitrous-variants.md) |
| Tire smoke | `tire_smoke_variant` | `TIRESMOKE/<ID>/` | `blackbox_modding::content::tire_smoke` | below |
| Collision sparks (Original PC spark style) | `collision_spark_variant` | `SPARKS/<ID>/` | `blackbox_modding::content::spark_variant` | [effect-variants-sparks-xenon.md](effect-variants-sparks-xenon.md) |
| Restored sparks (Restored spark style; sparks only, no contrail or streaks) | `xenon_spark_variant` | `XENON/<ID>/` | `blackbox_modding::content::xenon_variant` | [effect-variants-sparks-xenon.md](effect-variants-sparks-xenon.md) |
| Wind (looping recordings played with the car's speed) | `wind_variant` | `WIND/<ID>/` | `blackbox_modding::content::wind_fx` | below |
| Trees (models and textures of the world's trees, swapped in as sections load; now one kind of the [replacement sets](api-replace.md)) | `trees_variant` | `TREES/<ID>/` or `replace/<set>.toml` | `blackbox_modding::content::model_swap` (old form), `replace` (new) | [below](#profile-format-of-the-trees) |

## How a variant works

1. The mod adds a value to the kind's option, in `Mod::load`: `settings.add_option("tire_smoke_variant", "Tire smoke variant",
   &[SettingValue { id: "mymod", label: "My mod's smoke" }])`. The id is `a-z`, `0-9`, `_`, `-`.
2. The mod puts `profile.toml` and the files it names (PNGs, WAVs) in `files/<FOLDER>/<ID>/` of its folder (the id in upper
   case: `TIRESMOKE/MYMOD/profile.toml`). No code runs while the effect plays.
3. When the player has chosen a value that a mod provides, the game reads `<FOLDER>/<ID>/profile.toml` **through the install's
   layers**, once for each chosen value (a profile that is missing is looked for again every few seconds, so a mod may still be
   writing it), checks it with the kind's parser and hands it to the scene. A profile that is missing or refused is logged and
   the game's own effect plays. A mod that is there with the value not chosen costs nothing: nothing is read.

Every profile starts with `format = 1` and has keys the kind's schema knows; unknown keys and numbers out of range are refused.
Paths are relative, of letters, digits and `_ - . /`, and name files in the mod's `files/` folder.

## Profile format of the tire smoke

```toml
format = 1

[[layer]]                         # up to 8; every layer is drawn at every wheel that smokes
texture = "TIRESMOKE/MW2012/PUFF.PNG"   # an RGBA PNG in the mod's files
blend = "alpha"                   # or "additive" (the default)
grid = 2                          # a sheet of 2 to 16 cells per side; 0 for the whole picture
random_start = true               # a random cell for each particle ...
fps = 0                           # ... which then keeps it (fps = 0), or plays the sheet at this many frames a second
rate = 4.5                        # particles a second at full smoke ...
rate_per_speed = 0.5              # ... plus this many for each m/s the car goes
life = 1.0                        # seconds at most; life_variance (0 to 1) is how much of it a particle may fall short of
speed = 0.3                       # m/s along the road's normal at birth; speed_variance, spread (cone, degrees) too
lift = 0.25                       # m/s2 the smoke rises; drag, damping (the share of its velocity it loses every 60th of a second) too
inherit = 1.0                     # the share of the car's velocity a particle is born with; inherit_variance too
keys = [0.0, 0.15, 1.0]           # where along the life (0 to 1) size and colour are given: 2 to 16 keys, straight lines between
size = [0.5, 1.7, 2.5]            # full width in metres at each key
colors = [[200, 205, 220, 0], [200, 205, 220, 80], [150, 155, 170, 0]]   # red, green, blue, alpha at each key
angle_range = 360.0               # degrees of starting rotation; spin = degrees a second a particle turns at most
```

The keys of a layer are those of a nitrous `[[effect]]` ([nitrous-variants.md](nitrous-variants.md)) except `on`, `duration`
and the flipbook's `fps` default. The rate is **at full smoke**: the game multiplies it by the wheel's own smoke, the slip
intensity of the vehicle library (0 to 1), so the physics still decides how much there is. A layer's emitter sits on the road
under the wheel, its axis along the road's normal; the wheels' particles are drawn far to near within a layer. The game's own
colour mode (`smoke_color_mode`, [tire-effects.md](../tire-effects.md#colourful-smoke)) recolours the sprites of a variant too:
it replaces red, green and blue and keeps the alpha.

The 2012 mod's value, **NFS Most Wanted 2012**, is converted from the player's own 2012 install: what it holds and how it was
read is in [chameleon-effects.md](../formats/chameleon-effects.md#wheel-effects-tyre-smoke).

## Profile format of the wind

The wind is a sound, not a picture: the game's own is the `FX_WIND` module of the 2005 sound banks, which plays for every car. A
wind variant replaces it with looping recordings that play all the time the car is driven.

```toml
format = 1
volume = 1.0                       # every layer is multiplied by it, 0 to 8 (default 1)

[[layer]]                          # one to sixteen
file = "WIND/MW2012/W182159.WAV"   # a 16-bit PCM WAV, mono or stereo, looped over its whole length
volume = 0.2197                    # this layer's level, 0 to 8 (default 1)
rate = 1.0                         # playback rate, 0.25 to 4 (default 1)
gain = [[0.0, 0.0], [45.0, 0.0], [50.0, 0.25], [90.0, 0.0]]   # speed in mph and level, 0 to 8
```

`gain` is piecewise linear and flat outside its points (two to 64 points, the speeds rising). The car's speed is its speed in
miles an hour whichever way it goes. All layers start with the car and play on; a layer whose file cannot be read gives up the
whole variant and the game's wind plays (a warning says why). While the loops play, the game's wind module plays no left, right
or rumble sound (the weather wind, the `FX_WIND_Weather` module, is a separate sound and keeps playing). The car's sound plays
them (`audio/car/wind_fx.rs`) for the car being driven, in the Sfx group, and an original car hears them as well once the
variant is chosen: the setting is the player's, not the car's.

The 2012 mod's value, **NFS Most Wanted 2012**, is the wind sequence of `SOUND/GLOBAL.BNDL`: eleven loops with the level of each at
every speed from 0 to 180 mph, the windows, fades, curves and channel gains of the data multiplied; what the data holds and
the loops left out (two that swell in the air, two muted) are in
[chameleon-sound-graph.md](../formats/chameleon-sound-graph.md#the-sound-of-speed). Loudness against the game's own wind was not
heard, so `volume` is the data's alone. In a co-op session each car's voice plays its own copy of the loops.

## Profile format of the trees

The trees are not an effect but a set of models of the world, so this kind has no per-frame look: its profile names two packs
the mod ships and says which solid of the game each model replaces. The game applies it to every section of the world its loader
threads parse while the variant is chosen.

```toml
format = 1
geometry = "TREES/MW2012/GEOMETRY.BIN"   # the models: solids in the install's format (a GeometryPack, as a car's GEOMETRY.BIN)
textures = "TREES/MW2012/TEXTURES.BIN"   # the textures they use (a texture pack, as a car's TEXTURES.BIN)

[fit]                                    # optional, every key optional: the limits a replacement keeps (below)
spread = 1.25                            # at most this much wider than the footprint it replaces (1 to 4)
squeeze = 0.5                            # squeezed sideways to no less than this share of its height scale (0.1 to 1)
min_height = 0.7                         # not used if it would be shorter than this share of the replaced height (0 to 1)

[[swap]]                                 # one per replaced solid, 1 to 4096
solid = "XT_CEDARM_1B_JG_00"             # the game's solid, by name as the game spells it (the hash is case-sensitive)
model = "MW12_CONIFER_TALL_L0"           # the solid of `geometry` that takes its place
fit = "height"                           # "height" (default) or "row"
```

- **The pack files** are the formats of the cars' files ([models.md](../formats/models.md), [textures.md](../formats/textures.md)): write
  them with `blackbox_solid::write_geometry_pack` and `blackbox_tpk::write_tpk`. A model's vertices are 36 bytes (position, normal,
  colour, uv); its groups name their effect: **effect 6** (the tree shader) for leaf cards, **effect 0** (the world shader) for bark.
  The texture of a leaf card is a punch-through texture (`AlphaUsageType` 1, blend 0): the engine draws it cut out and from both sides.
- **A model's origin is its base**: z is up, the foot of the trunk is at the origin, and the model's height is how far it reaches above it
  (roots may reach below). **Every level of a model must stand on the ground** (its lowest point near the origin): a level that is a crown
  alone hovers over the ground wherever the game draws it, so a mod leaves such levels out (the 2012 mod does, see
  [chameleon-vegetation.md](../formats/chameleon-vegetation.md#the-levels-that-stand)). Texture names are the mod's own (the 2012 mod's are
  `MW12_T<id>`), so they cannot clash with the game's.
- **A replacement keeps the identity of the solid it replaces**: its name (so the hash the scenery refers to it by), its flags (the
  sway bit among them), its pivot matrix and the polygon count and density its header carries (the scenery's choice of level of detail
  reads them). Only the geometry changes.
- **The rule: a replacement never grows beyond the footprint it replaces** (`model_swap/place.rs`, the limits are the `[fit]` table).
  The game measures the replaced solid from its own vertices, so the mod never reads the 2005 install:
  - *Height of a copy.* It is scaled evenly to the height of the geometry it replaces (a quarter to four times for a `height` swap, up to
    one and a half times for a `row`), its foot on that geometry's **floor** (the lowest vertex of its stretch). A solid modelled around its
    own origin (the scenery places it with a matrix) stands its tree on the origin, which is the ground, even when the old solid is only a
    crown that begins above it (a far level): the tree reaches the old top from the ground. For a world-space copy the floor is the
    only ground there is: a 2005 tree line is a crown without trunks, and its crowns' floor is where its trees stand (measured at a wall
    top, where the old crowns began exactly at the wall's top, so the trunks of a replacement start there and do not pierce the wall).
  - *Width of a copy.* It may be `spread` times as wide as the geometry it replaces (the wider side of the box that holds its
    vertices in x and y: the whole solid for a single tree, the cell for the copies of a row). A model that is wider than that for its height (a broadleaf crown on a thin cypress) is **squeezed sideways**, down
    to `squeeze` of its height scale; if even that is too wide, the copy is made smaller until its squeezed width fits. A copy that would
    end up shorter than `min_height` of what the height asked for is **not used**: a solid with no copy keeps the game's own model, and
    so does a solid when more than a third of its stretches cannot be fitted. The refusal is a rule, not an error: nothing is logged.
  - *One copy for each stretch of the old geometry.* **`fit = "height"`** on a solid modelled around its own origin (the scenery places it with a
    matrix) is one copy at that origin. Every other solid (**`fit = "row"`** of either kind, and every world-space copy of either fit) is
    cut into the cells of a grid as wide as one copy (the model's width at the scale it needs, a fifth less for a row so that crowns overlap;
    a solid no longer than one and a half cells is one cell); each cell with three vertices or more gets a copy, **at the middle of its
    vertices, on the floor of its vertices**, as thick as they are. At most 32 copies and 60,000 vertices to a solid (the cells grow until
    they fit; a solid that cannot be brought under the limit keeps its own model). Each copy is turned by the golden angle more than the
    one before. So a line of trees that climbs a slope or bends round a road keeps its shape and every copy stands on its own floor: it
    neither floats over a lower floor nor sinks into a higher one (the older rule put at most eight copies on a straight line on the
    lowest floor of the solid, which hung the crowns of a row that climbed 50 metres in the air and carried them past the old footprint by up to 45 m).
- **What is swapped.** A section is rewritten right after it is parsed (`Streamer::set_rewrite`), before its textures are decoded: each
  solid whose name hash a swap lists is replaced (a swap whose model or texture is missing from the packs is left out, with a warning). The
  shared sets (`X0`: the library of the scenery, which the scenery places with a matrix) are rewritten like the tiles. Solids the profile does
  not list keep the game's look.
- **World-space copies.** The game's scenery has no instance of most of its trees: it holds a copy of the solid for each place, with the
  instance's matrix baked into its vertices and the name `<NAME>_DEINST<n>_<tile>` (910 of the 1,004 tree solids,
  [chameleon-vegetation.md](../formats/chameleon-vegetation.md#how-the-2005-game-draws-its-trees-verified)). A swap listed under `<NAME>`
  therefore reaches every copy of it too, and a copy may be a whole hillside of trees (a far level merges many), so it is fitted cell by cell as above. The copy keeps its
  name, flags, pivot, polygon count and density; if the instance it was made from is a mirror image (`docs/specs/backface-culling.md`) the
  replacement is wound the same way. A copy is fitted where it loads (the fit of a solid with a matrix is cached by its name and bounds, a copy's is not kept).
- **Textures once.** The textures of the models are **not** copied into the sections that are rewritten. The world uploads them once, into
  the shared sets (`residency/trees.rs`: they join the global textures that are uploaded before the shared models), and the solids of every
  tile find them there as they find the shared library's textures. On disk they are in `TEXTURES.BIN` once; in memory the pack holds the
  compressed pictures once and the GPU one copy of each.
- **When.** The pack is read when the choice, or its profile, changes (`Bridge::publish_trees`, once a frame at most). A choice made before a
  drive is in every section. A choice made **during** a drive takes effect at once: the residency notices that the pack in effect is another,
  uploads its textures into the shared sets and asks the streamer for every resident map tile again; each comes back rewritten with the new
  models, replaces the tile it was made from (the old tile's GPU resources are released, its collision and props are as they were) and shows
  within a few seconds, as the zone's tiles load. Not refreshed: the shared library `X0` (84 trees, loaded once for the drive: they follow the
  choice at the next drive), the map packs of mods, and everything in a multiplayer session (lockstep), where a tile that is uploaded again
  is not the same moment on every machine. A pack that is missing or does not parse leaves the game's trees (a warning says why).

The 2012 mod's value, **NFS Most Wanted 2012**, is the 2012 game's trees converted from the player's own install: eleven species, up to four
levels of detail each (the ones that stand on the ground), 81 swaps (they reach the world-space copies too). What the conversion reads, writes and takes:
[chameleon-vegetation.md](../formats/chameleon-vegetation.md#what-the-conversion-costs). What the 2012 files hold, the species, and which
solids they replace: [chameleon-vegetation.md](../formats/chameleon-vegetation.md). The mod's setting `trees = false` leaves them out.

### The bushes set

The 2012 mod declares a second set through the replacement API ([api-replace.md](api-replace.md)): `replace/bushes-mw2012.toml`, kind
`bushes`, so the game offers a second option, **Bushes** (`bushes_variant`, Original / NFS Most Wanted 2012), next to Trees. It
names the **same two pack files** as the trees set (`TREES/MW2012/GEOMETRY.BIN` and `TEXTURES.BIN`: the bush models are solids of the
trees pack and their textures are in the trees' texture pack, so nothing is converted or stored twice, and the world adds a texture
of the same name once even when both options are chosen), and lists the 2005 bush patches, hedges and grasses (58 solid names, 832 solids
of the world, `fit = "row"` for the patches, hedges and grasses and `height` for the single bushes) with its own `[fit]` limits: spread 1.5, squeeze 0.5 and min_height 0.5, looser than
the trees' because a bush is small and a patch is dense. What the families and models are, and why: [chameleon-vegetation.md](../formats/chameleon-vegetation.md#the-bushes).
The mod's setting `bushes` (default on, needs `trees`) turns the set off; the three round-bush families the Trees value always had
then stay in the Trees set as before, and with it on they belong to the Bushes set (the two sets never list one solid twice). The
labels are message ids of the mod's catalogue (`nfsmw2012-bushes` = "Bushes"), the value label is the proper name of the game. The older
`TREES/MW2012/profile.toml` is still written for the trees set only.

## The spark kinds

The collision sparks (`SPARKS/<ID>/`) and the restored sparks (`XENON/<ID>/`) are two kinds with one page,
[effect-variants-sparks-xenon.md](effect-variants-sparks-xenon.md): the profile of each (impact and scrape channels with a strength
gate, a list of layers or sparks), what the game keeps of its own, and how each is drawn. The restored variant is **the sparks
only**: the wind contrail and the speed streaks of the Restored style have no 2012 equivalent and no variant. A spark variant
acts in its own spark style (Options > Video, Collision Sparks): the collision sparks in Original PC, the restored sparks in
Restored (Experimental).

## In the game (for the next kind)

| Piece | Where |
|---|---|
| The kinds, their option ids, folders and texts | `blackbox_modding::content::variant` (`VariantKind`, `NITROUS`, `TIRE_SMOKE`, `COLLISION_SPARKS`, `XENON_SPARKS`), `crates/nfsmw/src/modding/backends/variants.rs` (`EffectKind`, `KINDS`) |
| The options (one hidden list each, "Original" only) | `settings::builtin`, which makes one from every entry of `KINDS`; the catalogue texts are `options-<kind>-variant` and `options-<kind>-variant-original` in `crates/nfsmw/i18n/en/options.ftl` |
| Which value is chosen, and which mod provides it | `variants::choice(bridge, &KIND)` (`VariantChoice { value, owner }`) |
| Reading the profile through the install's layers, once per value | `Bridge::variant_profile(&KIND, &cache, parse)`; the kind's own getter is `Bridge::tire_smoke_profile()` |
| Handing the profiles to the scene | `crate::effect_variants::EffectVariants` (one field per kind, the nitrous's too), made by `Bridge::effect_variants()` and given every frame to `Scene::set_effect_variants` (one trait method for every kind: a new kind adds no method to `Scene`, `Pausable` or the viewer) |
| Drawing | the scene: for the tire smoke `WorldScene::scene_set_effect_variants` (`scene_effects.rs`) gives the profile to `TireEffects` (`scenes/world/effects/profile.rs`), which loads its pictures from `maintain_tire_smoke` and draws them with the game's particle system (`blackbox-particles`; `exhaust::variant::{emitter_spec, spawn_intensity, upload_png}` are shared by the effects that draw particles) |
| Drawing the spark kinds | `WorldScene::scene_set_effect_variants` gives the collision sparks to `ImpactEffects::set_variant` (`scenes/world/impact_effects/variant*.rs`: its own small particle system with a streak quad and an overbright colour, loaded from `ImpactEffects::maintain`, played for vehicle contacts the install accepts) and the restored sparks to `VehicleEffects::set_spark_variant` (`scenes/world/vehicle_effects/sparks/variant.rs`: the restoration's line sparks with the profile's numbers). Each acts in its own spark style; the contrail and the speed streaks have no variant |
| The trees | not an effect: `Bridge::trees_profile` (the same `variant_profile`), `Bridge::publish_trees` (`backends/trees.rs`) reads the pack the profile names (`content::model_swap::Pack`) and makes it the pack in effect; the world's loader threads call `model_swap::rewrite` on every section (`residency/start.rs` sets it with `Streamer::set_rewrite` before the first request) |
| The mod's side | `mods/nfsmw-2012-content/src/effect_variants.rs` (`Kind`, `ALL`, `add_all`) adds the values; `tire_smoke.rs` and `spark_assets*.rs` convert the files |

### Adding a kind: checklist

1. **Library** (`libs/blackbox-modding/src/content/`): a `VariantKind` constant in `variant.rs` (option id, folder); a module
   with the profile schema and `parse(text) -> Result<Profile, ContentError>` that starts with `variant::check_format`
   (reuse `nitrous_effects::check_layers` for particle layers); a path helper; exports in `content/mod.rs`; tests next to it.
2. **Host settings** (`crates/nfsmw/src/modding/backends/`): an `EffectKind` constant and its entry in `KINDS`
   (`variants.rs`); the two catalogue texts; a `CacheCell<Profile>` in `Bridge` (`bridge.rs`) with its default in
   `with_store`; a getter `Bridge::<kind>_profile()` of one line; a field in `EffectVariants` and its line in
   `Bridge::effect_variants`. The option id constant of `settings.rs` is only for tests.
3. **Scene**: read the field in `WorldScene::scene_set_effect_variants` (it runs every frame; keep the profile in a field of
   the scene when a car spawned later has to get it, hand it to the effect, which compares it with what it has loaded). An effect that needs pictures loads them in a
   `maintain_*` call of `effects/world.rs` (`upload_effects`) with a renderer and frees them on change and in `Drive::release`.
   Nothing is allocated per particle or per frame once loaded.
4. **Mod** (`mods/nfsmw-2012-content/src/`): a `Kind` constant and its entry in `effect_variants::ALL`; a module that writes
   `files/<FOLDER>/MW2012/` (`profile.toml` and the pictures) and a one-line call after `write_nitrous` in `import.rs`. The
   supervisor bumps `CONVERSION` in `stamp.rs`.
5. **Docs**: a row in the table above, the profile format of the kind here (or its own page), and the decoded 2012 numbers in
   `docs/formats/chameleon-*.md` with evidence tags.
6. **Tests**: the parser (range checks, defaults, refusals), the profile writer of the mod (accepted by the host's parser), the
   scene effect with a profile loaded without a renderer (`profile_tests.rs` shows how), and the choice and the cache
   (`variants_tests.rs`).
