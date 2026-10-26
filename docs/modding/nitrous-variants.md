# Nitrous variants

The gameplay settings have a **Nitrous variant** option. Its only value in the game is **Original**: the nitrous of the 2005
game, exactly as it plays. A mod can add variants to it with [`bb.settings`](api-settings.md); the one in the repository,
[`nfsmw-2012-content`](nfsmw-2012-content.md), adds **NFS Most Wanted 2012**. While no mod adds a value, the option has one value
and is not shown, so the menu is as it was. With a variant added, the row shows and the player steps through the values; the
choice is saved with the other settings (`[mod_values.nfsmw] option_nitrous_variant`).

## What a variant is

The value's id (`mw2012`) names a folder in the mod's `files/` overlay: `NITROUS/MW2012/`, and a file in it, `profile.toml`.
When the player has chosen a value that a mod provides, the game reads `NITROUS/<ID>/profile.toml` through the install's layers
(once per chosen value), and plays what it describes. The mod may be there with the variant not chosen: then nothing is read
and nothing changes. A profile that is missing or refused is logged, and the game's own nitrous plays.

```toml
format = 1                    # required; the only format

[audio]                       # sounds; at least one of start, loop, end
start = "NITROUS/MW2012/START.WAV"   # once, when the nitrous lights
loop = "NITROUS/MW2012/LOOP.WAV"     # over the whole burn, looping
end = "NITROUS/MW2012/END.WAV"       # once, when it stops
start_gain = 1.0              # 0 to 4, default 1
loop_gain = 1.0
end_gain = 1.0
loop_fade_in = 0.1            # seconds, 0 to 10, default 0.1
loop_fade_out = 0.25          # default 0.25
replace_original = true       # default true: the game's own nitrous sound is silent while these play

[flame]                       # the tail pipe flames of the burning nitrous
tint = [0.4, 0.6, 1.0]        # each 0 to 4: multiplies red, green and blue of the game's blue flame
gain = 1.0                    # multiplies its alpha

[screen]                      # effects that follow the burn
vignette = 0.2                # 0 to 1 at full effect (the renderer's additive vignette)
radial_blur = 0.3             # 0 to 1 at full effect (the high-speed radial blur)
fade_in = 0.25                # seconds to full effect, default 0.25
fade_out = 0.5                # seconds to leave it, default 0.5
```

Every section is optional, and every key in a section. Unknown keys and out of range numbers are refused. Sound files are 16-bit
PCM WAV, mono or stereo, relative paths of letters, digits, `_ - . /`, in the mod's `files/` folder (`SOUND/` is where the car
sounds are; `NITROUS/` is for these). The format is `blackbox_modding::content::nitrous_fx` (`parse_nitrous_profile`).

### What the game does with it

| Section | Where | Notes |
|---|---|---|
| `[audio]` | `audio/car/nitrous_fx.rs` | The start and end play on the edges of the burn (`CarInput::nos_active`), the loop is faded in and out. With `replace_original` and the files read, `PowerPlayback::set_silence_nitrous` keeps the game's own nitrous sound from starting. If a file cannot be read, the game's sound plays |
| `[flame]` | `scenes/world/exhaust` | Applied to the blue nitrous lane of the exhaust flames, so only with the `exhaust_flames` setting on. Without `[flame]` the game's blue flame is drawn |
| `[screen]` | `scenes/world/focus_feed.rs` | `blackbox_gfx::FocusFilter`'s `vignette` and `radial_blur` are raised to `strength x level`, where the level follows the burn (`BurnFollower`) with the fades. Renderers without the blur ignore it |

What a profile does not change: the game's nitrous physics, its speed trails, its camera, its HUD bar. A variant is a look and
a sound, not a rule.

## Adding a variant from a mod

```rust
// in Mod::load, when the variant's files exist (or will, once converted):
if let Some(settings) = ctx.settings() {
    settings.add_option("nitrous_variant", "Nitrous variant",
        &[SettingValue { id: "mymod", label: "My mod's nitrous" }])?;
}
```

Put `profile.toml` and the sounds in `files/NITROUS/MYMOD/` of the mod. That is all: no code runs while the nitrous burns. The
variant row shows only while a value besides "Original" is registered, and a value goes when its mod does.

## The 2012 variant

`nfsmw-2012-content` converts the nitrous sounds of the player's own 2012 install into its folder (`files/NITROUS/MW2012/`,
with the conversion of the cars, and stamped with it) and adds the value `mw2012`, "NFS Most Wanted 2012", when a 2012 install
is set in its `config.toml`. What it finds, and what it does not, is in
[chameleon-nitrous.md](../formats/chameleon-nitrous.md):

| 2012 effect | In the variant |
|---|---|
| burn loop, end sound, start sound | played (`[audio]`), at the level they have in the install |
| post effects (vignette, bloom tint, colour grade) | **not played**: the values are known, the renderer has no darkening vignette, bloom tint or grade control |
| tail pipe flames | not changed: the 2012 effect is not found in the data; the game's blue flame stays |
| camera, speed lines, motion blur | not changed: nothing in the 2012 data names the nitrous |
| HUD bar and flash | not changed: the 2012 HUD movies are not read |

The fades of the sound (`loop_fade_in`, `loop_fade_out`) are the profile's defaults, not read from the 2012 timeline, which is
not decoded. Nothing from the 2012 game is in the repository: the files are made on the player's machine from the player's own
install.
