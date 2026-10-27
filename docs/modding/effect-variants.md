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
