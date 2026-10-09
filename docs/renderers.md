# Renderers and graphics settings

Two things decide how the game is drawn, and they are separate settings:

- **`renderer`** is *who draws*: the native **Black Box** renderer (`blackbox`, the default) or the
  optional **Bevy** renderer (`bevy`).
- **`backend`** is *which graphics API* the renderer uses: `auto`, `vulkan`, `dx12` or `gl`.

The Bevy renderer is **not built yet**. The settings, menu rows and checks described here are in
place so that configuration files, the console and the menus already work the same way for either
renderer; today every build runs `blackbox`. The design is in
[the plan](plans/gfx-renderers/README.md) and [ADR 0004](decisions/0004-swappable-renderers.md).

Everything defaults to what the game did before these settings existed: `blackbox`, every effect
off, ray tracing off, native resolution. Nothing extra is built or allocated for a feature that is
off, so the default stays lean for low-end PCs ([low-end.md](low-end.md)).

## Choosing a renderer

```toml
renderer = "blackbox"   # or "bevy"
```

or `--renderer bevy`, `NFSMW_RENDERER=bevy`, `set renderer bevy` in the F12 console, or the Renderer
row at the bottom of the Video options (in the main menu and the pause menu). The choice **applies
after a restart**; the console and the menu say so.

If you ask for `bevy` and this build does not have it (it needs the `renderer-bevy` cargo feature, which
no build has yet), the game logs why and starts `blackbox` instead. It never fails to start because of
the setting. The same goes for a PC the Bevy renderer cannot run on, once it exists.

## What each renderer can do

| Feature | `blackbox` | `bevy` (planned) |
|---|---|---|
| Upscalers `off`, `bilinear`, `fsr1` | yes | yes |
| Anti-aliasing `fxaa` | yes | yes |
| Bloom, tone mapping `aces` | yes | yes |
| Anti-aliasing `smaa`, `taa` | no | yes (TAA on Vulkan and DX12) |
| Upscaler `fsr3` (temporal, any GPU) | no | yes (Vulkan, DX12) |
| Upscaler `fsr4` (experimental) | no | DX12, Radeon RX 7000 and 9000 only |
| Upscaler `dlss` | no | Vulkan, NVIDIA RTX only |
| `ray_tracing` | no | GPUs with ray queries; noisy without DLSS Ray Reconstruction |
| Graphics APIs | Vulkan, DX12, OpenGL | Vulkan, DX12 |

`blackbox` is the renderer for OpenGL and for low-end PCs and its feature set is frozen.
`dlss` needs the NVIDIA DLSS SDK at build time, so it is a separate build option. `fsr4` has no
tester yet. Frame generation and DLSS 5 are not planned.

## The settings

Settings resolve like every other setting: **command line, then environment, then the config file,
then the graphics preset, then the defaults**, key by key. A value that does not parse is ignored with
a warning and the next layer decides.

| Setting | Config key | Environment | Command line | Values | Default |
|---|---|---|---|---|---|
| Renderer | `renderer` | `NFSMW_RENDERER` | `--renderer` | `blackbox`, `bevy` | `blackbox` |
| Graphics API | `backend` | `NFSMW_BACKEND` | `--backend` | `auto`, `vulkan`, `dx12`, `gl` | `auto` |
| Anti-aliasing | `post_aa` | `NFSMW_POST_AA` | `--post-aa` | `off`, `fxaa`, `smaa`, `taa` | `off` |
| Upscaler | `upscaler` | `NFSMW_UPSCALER` | `--upscaler` | `off`, `bilinear`, `fsr1`, `fsr3`, `fsr4`, `dlss` | `fsr1` |
| Upscale quality | `upscale_quality` | `NFSMW_UPSCALE_QUALITY` | `--upscale-quality` | `auto`, `native`, `quality`, `balanced`, `performance`, `ultra_performance` | `quality` |
| Render scale | `render_scale` | `NFSMW_RENDER_SCALE` | `--render-scale` | `50` to `200` (percent) | `100` |
| Upscale sharpness | `upscale_sharpness` | `NFSMW_UPSCALE_SHARPNESS` | `--upscale-sharpness` | `0` to `100` | `80` |
| Tone mapping | `post_tonemap` | `NFSMW_POST_TONEMAP` | `--post-tonemap` | `off`, `aces` | `off` |
| Bloom | `post_bloom` | `NFSMW_POST_BLOOM` | `--post-bloom` | `off`, `low`, `medium`, `high` | `off` |
| Ray tracing | `ray_tracing` | `NFSMW_RAY_TRACING` | `--ray-tracing` | `off`, `low`, `medium`, `high` | `off` |
| Preset | `graphics_preset` | `NFSMW_GRAPHICS_PRESET` | `--graphics-preset` | `custom`, `low`, `medium`, `high`, `ultra` | `custom` |

- `upscaler` is the default `fsr1`, which does nothing at a render scale of 100.
- `upscale_quality` sets the render size of the **temporal** upscalers (`fsr3`, `fsr4`, `dlss`):
  `native` 100 %, `quality` 67 %, `balanced` 59 %, `performance` 50 %, `ultra_performance` 33 %
  (`auto` is `quality`). They take their size from it and ignore `render_scale`. `render_scale` and
  `upscale_sharpness` belong to `bilinear` and `fsr1`. See [upscaling.md](upscaling.md).
- A temporal upscaler does its own anti-aliasing, so it replaces `post_aa`.
- `smaa` and `taa` are anti-aliasing methods of the Bevy renderer only.

### What needs a restart

- **`renderer`** always.
- **`ray_tracing`** switching between `off` and any level, on a renderer that has it. Changing the level
  while it is on does not.
- Everything else applies at once, in the menus and in the console.

### Presets

`graphics_preset` stands for a set of the settings above (and the cost settings of
[low-end.md](low-end.md)); picking one applies all of its keys. `ultra` is new:

| Setting | `low` | `medium` | `high` | `ultra` |
|---|---|---|---|---|
| `post_aa` | `off` | `fxaa` | `fxaa` | `taa` |
| `post_bloom` | `off` | `off` | `low` | `low` |
| `ray_tracing` | `off` | `off` | `off` | `medium` |
| `render_scale`, `upscaler` | `75`, `bilinear` | `100`, `fsr1` | `100`, `fsr1` | `100`, `fsr1` |

A preset is a set of *requests*. What a renderer runs is decided afterwards from what it can do, so
**`ultra` on `blackbox` is `high`**: the `taa` and `medium` ray tracing it asks for are downgraded (to
`fxaa` and `off`) with a log line. `ultra` asks for no temporal upscaler on purpose: `dlss` versus `fsr3`
depends on your GPU, and asking for either would make the native renderer draw at two thirds
resolution. Choose a temporal upscaler yourself.

## When a renderer cannot do what you asked

The settings are what you *request*; the renderer maps them onto what it can run:

- A missing upscaler falls back along `fsr4 > fsr3 > fsr1 > bilinear > off` and `dlss > fsr3 > fsr1 >
  bilinear > off`. A temporal upscaler requested on `blackbox` therefore runs as `fsr1` at the render
  size of its quality mode (`dlss` at `quality` is `fsr1` at 67 %).
- A temporal upscaler turns `post_aa` off (it does its own anti-aliasing). A missing `taa` or `smaa`
  becomes `fxaa`, then `off`.
- Ray tracing the renderer lacks is `off`; without a denoiser it is allowed but noisy.

The game **logs every downgrade once, when it changes** (`graphics: upscaler: dlss -> fsr1 (not
supported by the blackbox renderer on vulkan)`), and the F12 console shows the whole picture:

```text
> gfx
renderer:     blackbox
graphics api: vulkan (requested backend: auto)
adapter:      <your GPU>
capabilities:
  anti-aliasing: off, fxaa
  upscalers:     off, bilinear, fsr1
  tone mapping:  off, aces
  bloom:         yes
  ray tracing:   no
  render scale:  0.25 to 2.00
  BC textures:   yes
  needs restart: nothing
settings (requested -> effective):
  antialiasing    taa       -> fxaa      not supported by the blackbox renderer on vulkan
  upscaler        dlss      -> fsr1      not supported by the blackbox renderer on vulkan
  upscale quality quality   -> quality
  render scale    1.00      -> 0.67      set by the upscale quality
  ...
```

**The console refuses a value the running renderer cannot do** and leaves the setting alone:

```text
> set upscaler dlss
dlss is not available with the blackbox renderer (available: off, bilinear, fsr1)
```

**The menus only offer what works.** A menu row has no greyed-out state, so rows the renderer cannot do
(Ray Tracing, and the Upscale Quality row, which only temporal upscalers use) are *left out*, and the
toggles skip values it cannot run: the Anti-Aliasing row of `blackbox` cycles `off` and `fxaa`.

**Stored values are kept.** A value in the config file for something the running renderer cannot do
(`ray_tracing = "high"`, `post_aa = "taa"`) stays there untouched, so switching renderer later brings it
back; only the *effective* value changes, and the rows show that (a stored `taa` reads as FXAA on
`blackbox`). Changing a hidden row's value is not possible; picking a preset does rewrite the keys it sets.

## Checking it

```sh
cargo test -p nfsmw                       # settings, console, menus and fallbacks (no GPU needed)
cargo test -p blackbox-gfx                # the capability resolver
nfsmw view-world --exec "gfx" --exec "set upscaler dlss" --screenshot gfx.png
nfsmw view-world --upscaler dlss --render-scale 100 --screenshot dlss-as-fsr1.png   # logs the downgrade
```

## For developers

`libs/blackbox-gfx` holds the types and the pure `resolve(requested, capabilities)`;
`crates/nfsmw/src/settings/graphics.rs` builds the request from the settings,
`app/graphics.rs` sends it to the renderer (once per change) and logs what comes back,
`settings/availability.rs` answers "does the renderer offer this value?" for the console and the menus,
and `app/render/select.rs` decides which renderer starts.
