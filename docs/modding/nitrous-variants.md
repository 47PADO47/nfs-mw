# Nitrous variants

The nitrous is one *kind* of [effect variant](effect-variants.md) (the mechanism is shared with the tire smoke, and with the next
kinds). The gameplay settings have a **Nitrous variant** option. Its only value in the game is **Original**: the nitrous of the 2005
game, exactly as it plays. A mod can add variants to it with [`bb.settings`](api-settings.md); the one in the repository,
[`nfsmw-2012-content`](../../mods/nfsmw-2012-content/README.md), adds **NFS Most Wanted 2012**. While no mod adds a value, the option has one value
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

[flame]
replace_burn = true           # the game's blue nitrous flame is not drawn while a burn effect plays
replace_pop = true            # the game's blow-off and backfire flame is not drawn while a pop effect plays

[[effect]]                    # a particle emitter at every tail pipe, in the pipe's frame; up to 16
on = "start"                  # "start" (for `duration` seconds after the nitrous lights), "burn" or "pop"
texture = "NITROUS/MW2012/FLAME.PNG"   # an RGBA PNG in the mod's files
blend = "additive"            # or "alpha"
grid = 5                      # a flipbook of 2 to 16 cells per side; 0 for the whole picture
fps = 71                      # flipbook frames a second (the sheet's cells over a particle's life)
rate = 25.0                   # particles a second
life = 0.35                   # seconds; life_variance, speed (m/s along the pipe axis), speed_variance,
spread = 18.0                 #   spread (cone, degrees), lift, drag and inherit (share of the car's speed) too
keys = [0.0, 0.1, 0.5, 1.0]   # where along the life (0 to 1) the next two lists have their values: 2 to 16 keys, straight lines between
size = [0.25, 0.45, 0.55, 0.6]        # full width in metres at each key
colors = [[255, 255, 255, 0], [255, 255, 255, 235], [255, 255, 255, 200], [255, 255, 255, 0]]
spin = 0.0                    # degrees a second a particle turns at most, either way
duration = 0.8                # a start or pop effect spawns this long (0: as long as it lasts); a burn lasts as long as the nitrous
damping = 0.0                 # the share of its velocity a particle loses every 60th of a second (0.75: all but a quarter)

[screen]                      # screen and camera effects that follow the burn; every value is the one at full effect
vignette = 0.2                # 0 to 1 (the renderer's additive vignette, the speedbreaker's)
radial_blur = 0.3             # 0 to 1 (the high-speed radial blur; the Motion Blur settings scale it, off draws none)
edge = [0.04, 0.11, 0.09]     # a darkening vignette: what the frame is multiplied by at a corner, each 0 to 4 (default 1: none)
edge_inner = 0.5              # where it starts, 0 (centre) to 1 (corner), default 0.5
tint = [1.14, 1.09, 0.89]     # what every colour is multiplied by, each 0 to 4 (default 1)
contrast = 1.05               # about mid grey, 0 to 4 (default 1)
fov = 4.0                     # degrees added to the chase camera's field of view, -30 to 60 (default 0)
pull_back = 0.35              # metres the chase camera moves back, -5 to 10 (default 0)
shake = 0.006                 # camera shake size in metres, 0 to 0.5 (default 0)
shake_rate = 22.0             # shakes a second, 0.1 to 60, default 18
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
| `[flame]` | `scenes/world/exhaust` | `tint` and `gain` are applied to the game's blue nitrous flame. `replace_burn` and `replace_pop` stop the game's own flames from spawning while the profile has a burn or a pop effect. Only with the `exhaust_flames` setting on; without `[flame]` the game's blue flame is drawn |
| `[[effect]]` | `scenes/world/exhaust/variant.rs` | One emitter of the game's particle system (`blackbox-particles`) per pipe for each effect, in the pipe's frame (the pipes the car's `EXHAUST` markers give: a car with none shows none), with the PNG as its texture. `start` plays for `duration` after the nitrous lights (the instant it burns, so never with an empty tank), `burn` only while it burns, `pop` from the moment the game's own blow-off or backfire flame lights, for `duration` (it goes on after the game's own short flame is out; 0: only as long as that flame lasts), and only with the pedal up and the nitrous off, as the 2012 state bits do; a new flame starts a new burst. A particle takes `inherit` of the car's velocity at birth and loses `damping` of it every 60th of a second: `inherit = 1` with no damping rides with the car (a flame), a high damping leaves the particle where it was born (a trail). The PNGs are read through the install's layers and uploaded when the variant is chosen |
| `[screen]` | `scenes/world/nitrous_screen.rs`, `focus_feed.rs`, `camera_choice.rs` | One struct (`NitrousScreen`) is fed every frame with the burn: its level rises over `fade_in` while the nitrous burns and falls over `fade_out` after (`BurnFollower`), and every value is mixed from its neutral to its full value by that level. **Screen:** `blackbox_gfx::FocusFilter`'s `vignette` and `radial_blur` are raised to `strength x level` (the blur through the Motion Blur setting), and its `grade` (`ScreenGrade`: `edge` and `edge_inner`, `tint`, `contrast`) is the mix; the native renderer draws it as one pass after the radial blur (`post/grade.rs`, `shaders/grade.wgsl`) that is in the chain only while the grade is not neutral. Renderers without the grade or the blur ignore them (the Bevy backend does). **Camera:** the gameplay lens of the lead car gets the mix as a nudge: `fov` degrees on top of the game's own nitrous kick, the shake as an offset of the eye in the camera's frame (three sines per axis that never line up, the forward axis moving least), and the pull-back along the view on the chase camera only (a mounted camera keeps its place). The nudge is applied to the picture, not to the lens the rules read, and the view keeps its roll |

What a profile does not change: the game's nitrous physics, its speed trails, its HUD bar, and the camera and screen when it has no
`[screen]` (the game's own nitrous kick, blur and streaks stay in every case). A variant is a look and a sound, not a rule.
The profile cannot express: bloom (the player's own post setting), depth of field, the bar and flash of a HUD, a speed-line
image and a per-effect curve other than the straight fades.

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
| post effects (vignette, colour grade, fades) | **played** (`[screen]`, `nitrous_screen.rs`): the dark teal edge (outer colour 0.037, 0.114, 0.089, starting 0.49 of the way out), the warm grade of the nitrous colour cube (tint 1.14, 1.09, 0.89 and contrast 1.06, a straight-line fit of its grey diagonal), faded in and out over the state's 2 s blend time ([chameleon-nitrous.md](../formats/chameleon-nitrous.md), `libs/chameleon-postfx`). The bloom tint is **not played**: the bloom is the player's own post setting. The 2012 vignette darkens, so the additive `vignette` key is left at 0 |
| the start burst, the blue glow, the pop-off flame | **played** (`[[effect]]`), with the numbers the effect files hold (spawn rate and window, life, radius over the life, colour and alpha over the life, lift, the per-step drag as `damping`, inheritance, whether the particles ride with the emitter), decoded in [chameleon-effects.md](../formats/chameleon-effects.md): the flame sheet (5 x 5), the glow and the dust (the smoke puffs are scaled by crash inputs and are left out); they replace the game's blue flame and its blow-off flame. A flame runs 0.5 s at most and then only the glow is left; with an empty tank nothing plays. The flames and the glow leave the pipe along its axis (so a side exit flames sideways), the glow is kept to 0.7 m wide (the width the file gives it at the peak of its alpha), streams out at 1 m/s and has a small bright core, and it spawns steadily at rest: these four are the rewrite's own, chosen against the screenshots (`vfx_profile.rs`, the constants `FLAME_JET`, `GLOW_PLUME`, `GLOW_MAX_WIDTH`, `GLOW_MIN_RATE` and `CORE_*`). Not decoded: the parameter array that scales rates (taken as 1), the sprite draw scale, the rate per speed and whether the glow trails: those four are calibrated against two screenshots of the 2012 game (formats/chameleon-effects.md, "Calibration"). Not carried over: the flipbook cell rule, the blends (chosen from the brightness and the role), wind, turbulence, spin. The light emitter and the heat haze effect `0x24F2F` are **not played** (the particle system has no light and no refraction sprite). **Bloom** is the player's own post setting and applies to the additive sprites as to everything bright; no per-effect bloom, **lens flare** or **rainbow flare** is drawn: the renderer's flares are the lamps' (`car_light_flares.rs`, from lamp profiles) and the 2012 flare data is not decoded |
| camera (field of view, pull-back, shake), motion blur | **played, with the rewrite's own numbers**: the 2012 data holds nothing for them (the camera is the executable's, see the format doc), so `src/nitrous_screen.rs` writes 4 degrees, 0.35 m, a 0.006 m shake at 22 a second and a 0.25 radial blur, small because the 2005 nitrous already widens the view (up to 8 degrees) and blurs. They are constants (`FOV`, `PULL_BACK`, `SHAKE`, `SHAKE_RATE`, `RADIAL_BLUR`) to change by eye |
| speed lines | not changed: nothing in the 2012 data names them; the game's speed streaks stay as they are |
| HUD bar and flash | not changed: the 2012 HUD movies are not read (the HUD is a widget group of the UI movies, `Run Nitrous HUD`) |

The fades of the sound (`loop_fade_in`, `loop_fade_out`) are the profile's defaults, not read from the 2012 timeline, which is
not decoded. Nothing from the 2012 game is in the repository: the files are made on the player's machine from the player's own
install.
