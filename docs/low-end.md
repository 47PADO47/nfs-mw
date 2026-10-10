# Running on a weak GPU or a low-end laptop

Everything that costs GPU time beyond the plain 1:1 picture is optional and off or cheap by default, and
nothing is built, allocated or run for a setting that is off. This page lists what each setting costs,
a starting configuration for a weak machine, and how to find out what is slowing you down.

## A starting point

The shortest way is one setting. In the config file (`~/.config/nfsmw/config.toml` on Linux; the F12
console and the Video options use the same file):

```toml
graphics_preset = "low"
```

or on the command line: `nfsmw view-world --drive --graphics-preset low`. The same as individual keys, in
case you want to change one:

```toml
car_shading = "simple"      # one light on cars instead of the glossy three-light rig with reflections
post_tonemap = "off"
post_bloom = "off"
post_aa = "off"
render_scale = 75           # percent of the window, per axis
upscaler = "bilinear"       # see "Render scale" below before choosing fsr1
tire_smoke = true
skid_marks = true
smoke_quality = "standard"
collision_sparks = false
speed_trails = false
```

```sh
nfsmw view-world --drive --car-shading simple --render-scale 75 --upscaler bilinear \
    --post-bloom off --post-aa off --show-metrics advanced
```

Also worth doing on a weak machine, and unchanged by this page: a frame limit (`max_fps`) your screen can
use, vsync on, and the window or `resolution` setting at the panel's own size or lower.

## Graphics presets

`graphics_preset` is `custom` (the default, which changes nothing), `low`, `medium`, `high` or `ultra`. A preset
stands for these settings:

| Setting | `low` | `medium` | `high` | `ultra` | default (`custom`) |
|---|---|---|---|---|---|
| `car_shading` | `simple` | `glossy` | `glossy` | `glossy` | `glossy` |
| `post_tonemap` | `off` | `off` | `off` | `off` | `off` |
| `post_bloom` | `off` | `off` | `low` | `low` | `off` |
| `post_aa` | `off` | `fxaa` | `fxaa` | `taa` | `off` |
| `render_scale` | `75` | `100` | `100` | `100` | `100` |
| `upscaler` | `bilinear` | `fsr1` | `fsr1` | `fsr1` | `fsr1` |
| `ray_tracing` | `off` | `off` | `off` | `medium` | `off` |
| `tire_smoke`, `skid_marks` | on | on | on | on | on |
| `smoke_quality` | `standard` | `standard` | `high` | `high` | `standard` |
| `collision_sparks` | off | off | on | on | off |
| `speed_trails` | off | off | off | off | off |

`ultra` is for the optional Bevy renderer. `taa` and ray tracing do not exist on the native renderer, which
runs them as `fxaa` and `off`, so there `ultra` is `high` (with the two downgrades logged); see
[renderers.md](renderers.md).

`upscale_sharpness` and the other settings are not part of a preset. Nothing is detected from your GPU:
the preset is only ever what you pick.

**Precedence.** A preset is the layer below every explicit setting and above the built-in defaults:
command line, then environment, then config file, then the preset, then the defaults, key by key.
So `--graphics-preset low` on a machine whose config file says `render_scale = 90` renders at 90, and
`graphics_preset = "low"` in the file with `NFSMW_CAR_SHADING=glossy` in the environment gives glossy cars. The
preset itself is resolved the same way (command line over environment over file). A value that does not
parse is ignored with a warning, like every setting.

**Reading it back.** `get graphics_preset` and the menu row show the preset only while the settings it
covers still match it; as soon as one differs (an explicit key overrode it, or you changed a row) it reads
`custom`. Picking a preset in the F12 console (`set graphics_preset low`) or the Video menu applies all of
its settings at once, the menu writes them to the config file when you leave the screen, and the console
change lasts for the run.

| Setting | Config key | Environment | Command line |
|---|---|---|---|
| Preset | `graphics_preset` | `NFSMW_GRAPHICS_PRESET` | `--graphics-preset` |
| Car shading | `car_shading` | `NFSMW_CAR_SHADING` | `--car-shading` |

## What each setting costs

Measured on one machine, so read them as orders of magnitude and ratios, not as promises. The machine is an
Intel Iris Xe (Alder Lake GT2) laptop GPU on Mesa Vulkan, a 2560x1440 window, vsync off, `view-world --drive`
at map position 2151,1399 with the car standing still, 900 frames after 300 warm-up frames, three runs each
(the runs agree within about 0.1 ms). "Mean frame" is the average frame time.

| Setting | Mean frame | What it does to the cost |
|---|---|---|
| (default) | 1.78 ms | Scene drawn straight into the window in its 8-bit format. |
| the stack before the direct path, defaults | 2.27 ms | The scene always went through a 16-bit float image and a copy. |
| `car_shading = simple` | 1.63 ms | About 0.15 ms less here, and no glossy shader, environment cube map, bind groups or pipelines exist. Cars lose the sun highlight and reflections. |
| `post_aa = fxaa` | 2.80 ms | About +1.0 ms: one full-screen pass, and the scene goes through an offscreen 8-bit image. |
| `post_tonemap = aces` | 3.30 ms | About +1.5 ms: a full-screen pass and a 16-bit float scene image (twice the memory and bandwidth of 8-bit). |
| `post_bloom = low` / `high` | 4.17 / 4.2 ms | About +2.4 ms: the 16-bit float image plus a prefilter, five down-samples, five up-samples and a composite. The strength steps cost the same. |
| `render_scale = 75`, `upscaler = bilinear` | 1.83 ms | No change here: this scene is limited by the CPU and the driver, not by pixels (see below). |
| `render_scale = 50`, `upscaler = bilinear` | 1.58 ms | About 0.2 ms less. |
| `render_scale = 75`, `upscaler = fsr1` | 4.29 ms | About +2.5 ms: FSR 1's two full-size passes cost more than drawing fewer pixels saves. |
| `render_scale = 50`, `upscaler = fsr1` | 3.8 ms | The same, a little less. |
| `render_scale = 150` / `200` (supersampling) | 3.7 / 6.3 ms | Pixels are the cost here: 14.7 Mpixels at 200 % takes about 4.5 ms more than native. |
| `graphics_preset = low` | 1.66 ms | |
| `graphics_preset = medium` | 2.78 ms | |
| `graphics_preset = high` | 5.22 ms | |
| `low` with `upscaler = fsr1` | 3.95 ms | |

What the numbers say, and what they do not:

- **This scene is CPU- and driver-bound.** Frame time barely moves with the render scale, so the GPU is
  not the limit here, and on this GPU the render scale in the `low` preset buys almost nothing. It is
  there for GPUs and screens where the pixel count is the limit; the 150 % and 200 % rows show that pixels
  do cost when there are enough of them (about 0.3 ms per megapixel here, so a GPU several times slower
  than this one saves several times more).
- **FSR 1 is not in the `low` preset on purpose.** It costs about 2.5 ms per frame at 2560x1440 on this GPU,
  more than the whole frame, and at 75 % the scene it saves is worth well under 1 ms here. It pays off
  only when the scene itself is far more expensive per pixel than here (a much weaker GPU, or a heavier
  scene). If you are pixel-bound and want a sharper picture than bilinear, set `upscaler = "fsr1"` and check the
  frame time with the overlay; a lower render scale with FSR 1 is the usual combination.
- **Not measured:** other GPUs and drivers (Windows, Direct3D 12, OpenGL, NVIDIA, AMD), busy city
  scenes or traffic (there is none yet), tire smoke and sparks in action (the car stood still), memory use
  numbers other than the sizes below, power draw and temperatures. The `low` and `high` presets were
  chosen from costs and looks, not tuned on slower hardware.

### Memory

A scene drawn into an offscreen image needs a colour image the size of the render area: 4 bytes a pixel in
the window's format, 8 in 16-bit float (bloom, tone mapping). At 2560x1440 that is 14 MiB or 28 MiB. Post
effects need up to two more such images; bloom adds about a third of one for its half-size levels. With
every effect off, no such image exists: only the depth buffer (4 bytes a pixel) is allocated next to the
window. Images are freed when the setting that needed them is turned off.

### Start-up and first use

Pipelines are built the first time they are needed, per format: switching a post effect on for the first
time can pause a frame or two while the driver compiles its shader, and so does the first glossy car
(`car_shading = glossy` is built when the first car is loaded, and never with `simple`).

## Reading the performance overlay

`--show-metrics basic` (or `show_metrics`, or the Performance Overlay row of the Video options) draws the
frame rate and frame time. `--show-metrics advanced` adds the 1% low (the frame rate of the slowest 1 % of
the last 240 frames), the worst frame, a frame-time graph with a 60 fps line, the GPU name, the number of
meshes and textures and the scene status. To measure instead of guess:

1. Turn vsync off (`--no-vsync`) and the frame limit to `unlocked`, otherwise the display rate hides the cost.
2. Stand still in the same place and note the average and the 1% low.
3. Change one setting at a time in the F12 console (`set render_scale 50`, `set post_aa off`, `set car_shading
   simple`) and watch the frame time move.

How to read it:

- **Frame time that does not follow the render scale is not pixel-bound.** If `set render_scale 50` with
  `set upscaler bilinear` changes it little, the limit is the CPU, the driver or the streaming of the
  city, and lowering visual settings will not help much; the fixes are a lower frame limit, closing other
  programs, or waiting for the engine to get faster.
- **Frame time that falls with the render scale is pixel-bound.** Keep the scale as high as you like and
  turn off post effects first (they cost per pixel), then lower it.
- **A good average with a bad 1 % low or worst frame** is stutter, not speed: usually the city loading new
  sections as you drive, or a shader being compiled the first time an effect appears.
- Compare the **graph** with the 60 fps line: a flat line above it is headroom, spikes are stutter.

Reproducing the table above takes a window and a small local change that logs the frame times; the
overlay is the supported way and gives the same averages.

## See also

- [post-processing.md](post-processing.md), [upscaling.md](upscaling.md), [tire-effects.md](tire-effects.md)
  and [vehicle-effects.md](vehicle-effects.md) for each group of settings in detail.
- The `blackbox-render` README, "Render pipeline", for how the renderer picks the cheapest path.
