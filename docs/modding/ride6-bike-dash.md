# RIDE 6 bikes: the instrument cluster's screen, live

The clusters of the imported bikes were pictures of a bike at rest ([formats/ride6-materials.md](../formats/ride6-materials.md#instrument-clusters): "the car's
textures do not change while it is drawn"). This page is what changed: a small dynamic-texture call in the renderer, a `[bike.dash]` table the importer writes for
a cluster that has a display, and a painter that fills the display with the bike's speed, gear and revs a few times a second. Tags: **[M]** measured, **[C]** a
choice of this project, **[R]** read in this repository's code.

## What exists, in layers

| Layer | What | Where |
|---|---|---|
| Renderer (generic) | `RenderBackend::update_texture(handle, [x, y, w, h], rgba) -> bool`: overwrite a region of the base level of a 32-bit texture; `false` (the default) where the backend cannot | `libs/blackbox-gfx/src/backend.rs`; implemented by the native renderer (`libs/blackbox-render/src/gpu/textures.rs`, one `write_texture` of the region's bytes, no allocation, the bind group kept); the Bevy renderer and the null backend keep the default |
| Manifest (generic) | `[bike.dash]`: `texture`, `window = [x, y, w, h]`, `[bike.dash.colors]` | `libs/blackbox-modding/src/content/bike_dash.rs`, registered per car by `crates/nfsmw/src/modding/content/bike_dash.rs` and `nfsmw_data::car::mods::bike_dash` |
| Importer | a cluster whose static picture has a black display window is stored as **32-bit colour at 512 px** (not block-compressed: the host writes into it) and the table is written | `mods/ride6-bikes/src/{dash,bake,build,manifest,summary}.rs` |
| Host | the painter and the feed: at most 15 repaints a second, none while the whole km/h, the gear and the lit segments are unchanged, region upload only | `crates/nfsmw/src/scenes/world/drive/rig/dash.rs`, `drive/bike_view.rs` (`Drive::feed_dash`), `scenes/world/dash_feed.rs` |
| Setting | `live_dash` (default on; `config.toml`, `NFSMW_LIVE_DASH=off`, the console `set live_dash off`): off paints the bike at rest once and stops | `scenes/world/bike_assist.rs` |

**A deviation from the brief, and why [C].** The brief had the mod draw the values. That needs a new `bb.*` interface with a per-frame call across the FFI boundary
(ABI table, SDK wrapper, host table, backend, a version), for a picture that changes at most 15 times a second. The importer (the mod's code) decides *which texture is
the screen and where the display is*; the host, which already has the bike's telemetry every frame, paints it. The generic part, the dynamic-texture call, is in the renderer
trait as asked; a mod that wants to draw its own picture would call the same function behind a `bb.textures` interface (not written).

## The importer [R]

`bake::Surfaces::bake` finds the display in the static `gauge<chassis>_d` picture (`dash::find_window`, the existing search for the largest black rectangle that is a display).
The picture stays the exporter's composition of the widgets at rest (shrunk to `dash::LIVE_SIZE` = 512 and stored as `Argb8888` with one level; the window is scaled to the stored
size). The **places of the values come from RIDE 6's own widgets**: `tools/ride6-exporter/dash.py` now writes `<picture>_dash.json` beside `_dash.png`, the rectangles (1024 canvas)
of the text boxes named `speedtext`/`speed` and `gear`/`gear_` (with their colour, justification and size) and of the rev bar's drawn part (with the number of its segments; a dial
whose arc is not a bar is left out). The importer scales them to the stored picture, lifts their colours like the picture and takes as each field's `background` the median colour of the
picture under it (the few glyph texels voted out) and writes them as `[[bike.dash.field]]`. A cluster with a display and no layout (a bike exported before this change; run
`python -I tools\ride6-exporter\dash.py` again) keeps the bezel with the idle panel in the window and gets the generic panel below.

```toml
[bike.dash]
texture = "DUCATI_PANIGALE-V4-S-2025_GAUGE162_D"
window = [..]            # texels of the stored 512 px texture

[bike.dash.colors]       # the generic panel's colours
background = [..]  lit = [..]  dim = [..]  text = [..]  label = [..]  gear = [..]

[[bike.dash.field]]      # one per place the cluster shows a value
channel = "speed"        # speed | gear | rpm_bar
rect = [x, y, w, h]      # texels of the stored texture, erased with `background` and redrawn
color = [..]  background = [..]  align = "right"  segments = 1
```

## The painter [M]

**With fields** the host erases each rectangle with its background and draws the value in it, scaled to the rectangle (the 5 by 7 dot font, the field's colour and justification): the
speed in whole km/h, the gear (`N` when the bike stands at idle: a bike's gearbox has no neutral, so standing at idle is shown as `N`; `R`; else its number), the revs as the
lit part of the bar from the left (in its segments). Only those rectangles are uploaded. The glyphs of RIDE 6's fonts are not reproduced (the dot font is the host's), the position,
size and colour are the cluster's. **Without fields** it is the generic panel below.

The generic panel is the idle panel of `mods/ride6-bikes/src/dash.rs` with live values: a 24 segment rev bar along the top (the share of the engine's `MAX_RPM`), the gear on the
left (`R`, `N`, `1`..), the speed on the right in km/h with `KM/H` under it, in a 5 by 7 dot font scaled to the display. A bike with the table gets this panel whatever the
cluster looked like (the TFT clusters of the Ducatis, BMW, KTM, Yamaha and Aprilia that have a black window); the bezel, the lamps and the logos around the display stay the baked
picture.

## Cost [M]

Painting a display: **35 microseconds** for a 384 by 192 one on the CPU (`cargo test --release -p nfsmw painting_a_display -- --nocapture`, 200 paints; release build). The
Panigale V4 S's display is 376 by 136 texels, so about 25 microseconds and **204 KB** of upload per repaint, at most 15 a second (**3 MB/s**), through one `queue.write_texture`
of that region (no allocation, no new bind group); a bike that stands still repaints nothing (the key is the whole km/h, the gear and the lit segment count: a cruising bike
changes it a few times a second). The texture is 512 by 512 by 4 bytes = **1 MB** a bike with a live screen, about the size of the 1024 px block-compressed picture it replaces (0.7 MB with its levels; arithmetic, not measured). The GPU side of the write was not timed on its own: the frame times in the world screenshots (17 to 24 ms in the overlay
with the offscreen window, with the live screen on and off) do not tell the two apart.

Checked in the world (`NFSMW_EASY_WHEELIE=off`, `view-world --drive <ID> --at 985,4430 --camera-mode hood --drive-script "9:throttle=0.7"`; `C:\r6work\bikegaps-shots\dash\`): the Panigale
V4 S shows `1` and `143 KM/H` with the rev bar where the baked picture (`NFSMW_LIVE_DASH=off`, `off.png`) shows the idle composition (`N`, the lap time), and the KTM RC 390 shows `1` and `37 KM/H`
(`rc390.png`); the HUD's own speedometer agrees to a km/h. In this build the Panigale's and the Italjet's and the RC 390's clusters have a window (3 of the 5 imported for the check); the BMW S 1000 RR 2019's
and the KTM 450's pictures have none that `find_window` accepts, so they keep the baked picture.

## Not done

- **RIDE 6's own layouts** (the digit fonts, the needles, the bar graphs, the lamps and the riding aids of each of the ~205 widget classes): the exporter composes them at rest
  (`tools/ride6-exporter/dash*.py`); a live version needs the widget tree of each cluster as data the host can draw (kinds, rectangles, fonts, value channels and the needle
  mappings, all read already by `dash_widgets.py`) and a font drawer. Only the generic panel is live.
- Analogue clusters (no display window) stay the picture at rest; the Bevy renderer and any backend without `update_texture` keep the baked picture (the feed gives up on the first refusal).
- The odometer, the clock, the units setting (the speed is km/h) and a night colour.
- A row in the gameplay options: `live_dash` is a config key and a console setting.
- The garage and the car viewer (`view-car`) show the baked picture; the feed runs in the world scene for the lead player's bike.
