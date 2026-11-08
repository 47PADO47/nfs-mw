# Sky variants

A **sky variant** gives the game's sky the clouds and the moon of another game: pictures of the cloud cover for the hours of the day
and for fair, cloudy and overcast weather, and a picture of the moon. It is the `sky` kind of the one replacement API
([api-replace.md](api-replace.md)) with a profile of its own, in the manner of the [effect variants](effect-variants.md). Part of the
[modding catalogue](README.md). The 2012 mod's value, **NFS Most Wanted 2012**, is converted from the player's own install
([chameleon-environment.md](../formats/chameleon-environment.md)).

## What the game does

The game's sky has two renderings ([enhanced-sky.md](../enhanced-sky.md)). **Enhanced** (the default) computes the atmosphere per
pixel and marches a procedural cloud volume into a shared 1536 x 384 map of the upper hemisphere; **Original PC** draws three layers
of the install's textures with the original shader. The original shader can only scroll tiled strips of cloud, not show a panorama,
so **a sky variant is a feature of the Enhanced sky**: while a variant is chosen the game draws the sky with the procedural
atmosphere and puts the variant's domes into the cloud map where its own march would go. Everything that reads the map keeps working:
the main view, the mirrors, the car reflections, the wet road, the sun glare's cloud occlusion. Choosing a variant while the Original PC
sky is on switches that sky to the procedural atmosphere (the other sky settings keep their meaning); the sky model of the install is
needed (Original PC or Enhanced, not the legacy backend). The procedural cloud shadows on the city are off while a variant is
chosen: they would not match the pictures.

Each frame the game takes the profile's weather for the cloud cover (the game's own weather: the clear, cloudy and rainy sky), picks the
two keys around the hour of its day clock, loads the two domes the first time they are needed, and draws their blend: the clouds'
light is `colour x tint x exposure`, their cover the alpha times the weather's gain. A dome with a painted sun (`glow`) is turned
so the painting is under the sun the game draws; the other domes drift with the game's wind. At night the moon picture takes the place of
the procedural disc where the full day/night cycle shows the moon (Options > Video, Day/night cycle); the stars stay the game's own.
The atmosphere, sun disc, sun glare and fog remain the game's, drawn behind the clouds.

## The option and the set

A mod declares the value with a replacement set, no code:

```toml
# replace/sky-mymod.toml
format = 1
[set]
kind = "sky"
kind_label = "mymod-sky"          # what the option is called in the Gameplay options (a message id of the mod's catalogue, or text)
value = "mymod"
value_label = "mymod-sky-value"

[[texture]]                       # a set needs a model, a texture or an add: add the moon's picture (any picture of the mod)
name = "MYMOD_SKY_MOON"
file = "SKY/MYMOD/MOON.DDS"
```

The option is `sky_variant`, a list with the game's own value `original` and the mod's; it shows only while a mod offers a value, and
the choice is saved as `[mod_values.nfsmw] option_sky_variant = "mymod"`. The profile is `SKY/MYMOD/profile.toml` (the value id in
upper case) in the mod's `files/`, read through the install's layers once for each chosen value. A profile that is missing or refused is
logged (`the sky variant "mymod" has no profile ...`) and the game's own sky is drawn.

## Profile format

```toml
format = 1
exposure = 0.55                  # a multiplier of the clouds' light, 0.05 to 4 (default 1)
wind = 0.01                      # turns of the sky that one unit of the game's cloud scroll moves a dome without a glow, 0 to 1 (default 0.01)

[moon]                           # optional
file = "SKY/MW2012/MOON.DDS"     # a DDS (BC1, BC3 or uncompressed, mips kept) or a PNG; the disc fills the middle 3/4 of the width
size = 0.0173                    # half the picture's width as an angle in radians, 0.002 to 0.1 (default 0.0173)
brightness = 1.0                 # 0 to 4

[[dome]]                         # one to 16
name = "morning"                 # a-z 0-9 _ -, used once
file = "SKY/MW2012/MORNING.DDS"  # equirectangular, upper hemisphere: the width is 360 degrees, the top row the zenith, the bottom the horizon
glow = 0.5                       # optional: where along the picture (0 to 1) the sun is painted

[[weather]]                      # one to eight, tried in the order of the file
max_cover = 0.3                  # 0 to 1: used while the cloud cover is at most this; the last one takes the rest
gain = 1.0                       # a multiplier of the domes' cloud cover, 0.1 to 4 (default 1)

[[weather.key]]                  # one to 48, the hours rising, 0 up to 24; the day wraps: the last key blends into the first
hour = 5.0
dome = "morning"
tint = [1.0, 1.0, 1.0]           # what the dome's linear colour is multiplied by, 0 to 8 each
```

A dome is a picture the pixel colour of which is the cloud's light and the alpha its cover; pixels of alpha 0 show the atmosphere. The
colour is not premultiplied. A dome may have any size and any aspect (the 2012 ones are 2048 x 512 and 2048 x 256); block-compressed
BC1/BC3 pictures go to the GPU as they are, mips included. `blackbox_modding::content::parse_sky_profile` checks the schema:
unknown keys, numbers out of range, a key that names no dome, hours that do not rise and paths outside the mod's `files/` are refused.

Between two keys the dome changes by a cross-fade; at a key it is that key's. A change of weather (the cloud cover crossing a
`max_cover`) switches at once.

## In the game (for the next change)

| Piece | Where |
|---|---|
| The profile and the choice of weather and keys (`SkyProfile::pose`) | `libs/blackbox-modding/src/content/sky_variant.rs` |
| The option, the profile read once per chosen value | `Bridge::sky_profile` (`modding/backends/bridge.rs`), `SKY_KIND` (`variants.rs`, not in `KINDS`: the sets create the option), `EffectVariants::sky` |
| The scene: loading the domes, the turns, the tints | `crates/nfsmw/src/scenes/world/sky_panorama.rs`, called from `WorldScene::update_sky` |
| The renderer: the pictures and the state | `RenderBackend::set_sky_panorama`, `SkyState::panorama` (`libs/blackbox-gfx/src/sky.rs`); `libs/blackbox-render/src/gpu/sky/` (the material's two spare texture bindings carry the domes, binding 8 the moon; the cache pass samples them: `panorama_clouds` in `sky_enhanced.wgsl`) |
| The 2012 mod | `mods/nfsmw-2012-content/src/sky.rs` and `sky/` |

## Cost

The 2012 mod writes 5.5 MB (`sky_size = 1024`: 1.4 MB). The game reads two domes (1 MB each) and the moon when the
variant is first drawn, uploads them compressed, and then draws the clouds with the cost of two texture samples per map texel; see the
measurements of the 2012 value in [the mod's README](../../mods/nfsmw-2012-content/README.md#sky).
