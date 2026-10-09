# Post-processing

Three optional effects run on the finished 3D scene: tone mapping, bloom and anti-aliasing (FXAA).
They are separate switches, **all off by default**, so the default image is exactly the one the game
drew before they existed. The HUD and the menus are drawn afterwards and are never affected.
The design is in [the spec](specs/post-processing.md).

| Effect | Config key | Environment | Command line | Values (default first) |
|---|---|---|---|---|
| Tone mapping | `post_tonemap` | `NFSMW_POST_TONEMAP` | `--post-tonemap` | `off`, `aces` |
| Bloom | `post_bloom` | `NFSMW_POST_BLOOM` | `--post-bloom` | `off`, `low`, `medium`, `high` |
| Anti-aliasing | `post_aa` | `NFSMW_POST_AA` | `--post-aa` | `off`, `fxaa` |

Settings resolve like every other setting: command line, then environment, then the per-user config
file, then the default. A value that does not parse is ignored (with a warning) and the next layer
decides. Values are lower case; `ACES`, `on` or `ultra` are rejected.

Video options in the main menu and the pause menu end with Tone Mapping, Bloom and Anti-Aliasing rows
(left and right cycle the values). The change shows at once and is saved when you leave the settings
screen. In the F12 console, `get post_bloom`, `set post_bloom high` and the shorthand `post_bloom high`
(likewise `post_tonemap`, `post_aa`) read and change the same settings for the run; they are not saved.

## What each effect does

- **Tone mapping (`aces`).** Applies a filmic ACES curve. The game's colours were authored for a plain
  clamp, so the curve changes the look on purpose: it adds contrast, desaturates the brightest colours
  and maps white (1.0) to about 80% grey, so the picture is darker and flatter in the highlights. That
  is why it is off by default. Exposure is fixed at 1.0 in the game (the renderer API takes any value
  from 0.1 to 8).
- **Bloom (`low`, `medium`, `high`).** Pixels brighter than 80% (soft knee) are blurred over several
  half-size images and added back, so bright sky, windows, lights and additive effects glow. The steps
  scale the strength 0.5, 1.0 and 1.8. Bloom adds light, so a cloudy sky gets brighter and softer.
  It also works without tone mapping.
- **Anti-aliasing (`fxaa`).** Smooths jagged edges and shimmering thin lines of the 3D scene with a
  single cheap pass (FXAA, written from the published description of version 3.11). It blurs textures
  very slightly. With render scale below 1.0 it runs before the image is scaled up, which is what
  upscalers want as input.

The effects run in a fixed order: bloom, tone mapping, anti-aliasing. Anti-aliasing comes last so it
sees the final colours. Each effect that is off costs nothing: no pass is created for it. With all
three off the scene is drawn straight into the window; bloom and tone mapping draw it in a 16-bit float
image (twice the memory and bandwidth), FXAA alone in the window's own format.

## Checking them

Screenshots go through the same chain (`--screenshot`), so effects can be compared without a window:

```sh
nfsmw view-world --drive --drive-script "2" --screenshot base.png
nfsmw view-world --drive --drive-script "2" --post-tonemap aces --screenshot aces.png
nfsmw view-world --drive --drive-script "2" --post-bloom medium --screenshot bloom.png
nfsmw view-world --drive --drive-script "2" --post-aa fxaa --screenshot fxaa.png
nfsmw view-world --drive --drive-script "2" --exec "set post_bloom high" --screenshot console.png
```

The renderer's own checks are `cargo test -p blackbox-render` (shader validation, parameter layouts,
chain planning) and, on a machine with a GPU,
`cargo test -p blackbox-render -- --ignored post_effects` (flat colours through the curve, light
spilling from a bright square, a slanted edge before and after FXAA, the order of the chain).

## Licences and credits

The tone curve is Krzysztof Narkowicz's published rational fit of the ACES curve. The bloom filters are
the 13-tap downsample and 3x3 tent upsample described in public conference talks. FXAA is an independent
implementation of Timothy Lottes' published algorithm; no code of the NVIDIA reference header was
copied. All shader code is the project's own (MIT OR Apache-2.0).
