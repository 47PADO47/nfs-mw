# Colour grading with 3D look-up tables

Status: built; the CPU reference tests, the shader and GPU tests against the reference on Vulkan passed (the gfx, gpu-passes, render, modding, postfx and mod tests), dry screenshots were taken offscreen on the native renderer; the pass cost, the wet pictures and the last small changes
(section 8) were not run. The Bevy renderer does not draw it.

A scene can hand the renderer a **colour grade**: one or two 3D look-up tables (cubes) and a strength. The renderer looks every
pixel of the finished frame up in the cube (one filtered 3D texture sample) and moves the colour the strength's share of the way to
the result. Nothing in the host has a look of its own: the grade comes from a mod's **grading variant**. The 2012 content mod offers
one, the cubes of NFS: Most Wanted (2012) by hour of the day and rain ([chameleon-environment-grading.md](../formats/chameleon-environment-grading.md)).

| Layer | Where |
|---|---|
| Types and the reference arithmetic | `libs/blackbox-gfx/src/grading.rs`: `ColourCube`, `ColourGrade`, `RenderBackend::set_colour_grade`, `Capabilities::colour_cube` |
| The pass | `libs/blackbox-gpu-passes/src/colour_cube.rs` and `shaders/colour_cube.wgsl` (`ColourCubePass`) |
| The pass in the native renderer's chain | `libs/blackbox-render/src/gpu/post/colour_cube.rs` |
| The variant kind, its profile and cube files | `libs/blackbox-modding/src/content/grading.rs`, `grading_cube.rs` (`GRADING` in `variant.rs`) |
| Picking the grade each frame | `crates/nfsmw/src/grading/mod.rs` (`GradingLook`, `Selector`), `scenes/world/grading_feed.rs` |
| The settings | `grading_strength` (`settings/grading.rs`), the `grading_variant` option (`modding/backends/variants.rs`), the Video rows (`frontend/mod_options.rs`) |
| The 2012 values | `libs/chameleon-postfx/src/environment.rs`, `mods/nfsmw-2012-content/src/grading.rs` |

## 1. The cube and the lookup

A `ColourCube` is `size`^3 entries of 8-bit RGB, **red varying fastest, then green, then blue** (the layout of the Adobe `.cube`
format and of a 3D texture whose x is red). The edge is 2 to 64; 16 and 32 are the usual ones. The lookup of an input `c` (each
channel clamped to 0 to 1) is the **trilinear** interpolation of the 8 entries around `c * (size - 1)`, so the first entry is
hit exactly at 0 and the last at 1 (the texture coordinate is `(c * (size - 1) + 0.5) / size`). The identity cube (every entry at
its own position, `ColourCube::identity`) leaves a colour unchanged to a 255th: the entries are bytes, so for edge 16 the
entries are exact (multiples of 17), for edge 32 they are within half a byte and the lookup stays within one byte
(tests below). A hardware filter on an 8-bit texture has about 8 bits of weight, which stays under a byte for cubes this size.

A tetrahedral lookup is not used: with the cubes of this game (smooth, 16 entries) the difference from trilinear is far below a
byte, and the trilinear filter is the hardware's.

## 2. The grade

`ColourGrade { from, to, mix, strength }`: the lookup in `from`, blended with the lookup in `to` by `mix` (0 to 1, only when `to`
is set), then the colour moved `strength` (0 to 1) of the way from the original to the result. A grade with no strength is neutral
and draws nothing. **Values above 1** (a scene that is not tone mapped holds bright glows) keep their excess: only the part inside
0 to 1 is graded and the rest is added back, so a bright sky is not clipped. `ColourGrade::apply` is the arithmetic on the CPU, and
the reference the GPU tests compare with.

`ColourCube::blend(parts)` makes one cube of the weighted mean of several (looked up at the first one's entries). The game uses it
when the weather is between dry and wet (the pass blends two cubes, the weather needs four).

## 3. The pass

`ColourCubePass` (blackbox-gpu-passes) is one fullscreen draw: it reads the input image with `textureLoad`, takes one
`textureSampleLevel` of the cube (two with a second cube, one per cube, bound as 3D textures with a clamping linear sampler) and
writes the output. The parameter block is one `vec4` (strength, mix, whether a second cube is there). The cubes are uploaded as
`Rgba8Unorm` 3D textures once, by `ColourCube::id` (a hash of the size and the entries), kept up to 16 and dropped least recently
used first; a grade that arrives again every frame uploads nothing. Pipelines are built per output format on the first draw and use
the renderer's pipeline cache.

### Where it sits

In the native renderer's chain (`docs/post-processing.md`) the pass comes **right after the post effects** (bloom, tone mapping,
FXAA, vignette) and before the scene's own screen effects (the radial blur, the nitrous screen grade) and the upscalers. So the cubes see the display-range
picture they were authored for, and a nitrous or pursuit effect adds to the graded frame. It is in the chain from the first grade
on and **inactive** while none is set: it is then left out of every frame's plan, adds no pass and no scratch image, and the scene is
drawn as it was (with all other effects off, straight into the output). With a grade the scene is drawn into the HDR scene image
first, like any post effect. It runs for the main view and the rear-view mirror alike; reflection captures and the HUD are not graded.
The HDR output modes are not special-cased: the lookup is on the scene-referred value clamped to 0 to 1 and the excess is added
back (section 2).

## 4. Renderers

- **Native (`blackbox-render`)**: the pass above, `Capabilities::colour_cube = true` (also the mock capabilities the tests use).
- **Bevy (`blackbox-bevy-render`)**: **not implemented**. `set_colour_grade` is the trait's default (ignored) and the capability is
  false. A Bevy implementation would add one fullscreen node with the same shader and a 3D `Image`; the WGSL has no dependency on
  the native renderer (its bindings are a 2D texture, two 3D textures, a sampler and a uniform), so it can be reused as it is
  ([ADR 0004](../decisions/0004-swappable-renderers.md)). The Video rows are not listed while the capability is false.

## 5. The variant kind

`grading_variant` is an effect variant kind like the nitrous and the trees ([effect-variants.md](../modding/effect-variants.md),
"Profile format of the grading"): a list option with the game's own value `original`, shown as **Off**, and one value for each
look a mod adds. The profile is `GRADING/<VALUE>/profile.toml` with `[[cube]]` files (`.LUT` or `.cube`) and `[[set]]`s of hourly
keys for the weathers `clear` and `rain`. The choice is the player's: **it is off by default**, as every variant is, and the 2012
content mod only adds the value (the same way it adds its trees and nitrous). A mod that is installed but not chosen costs nothing:
no file is read.

The cube files: `.LUT`, the compact form (8-byte header, `size`^3 RGB entries; 12,296 bytes for a cube of 16), and the Adobe `.cube` text any
grading tool writes (`LUT_3D_SIZE`, red varies fastest, domain 0 to 1). A file that is missing or refuses decodes to no look at all
(a warning once; the picture stays as it is).

## 6. Selecting the grade

Every frame the world scene (`grading_feed.rs`) gives the selector the clock's hour (`DayClock::hour`), how wet it is, and the
strength setting:

- **Hour.** The set's keys are `(hour, cube)`. The cube in force is that of the last key at or before the hour and the next key's
  cube is blended in by the share of the time between the two; the day wraps from the last key to the first. Two neighbours with the
  same cube blend to that cube. This is the GPU's two-cube blend: no upload, no CPU work.
- **Weather.** `wet = max(rain intensity, road wetness)` (`weather::rain::Rain::intensity`, `WetRoad::amount`), 0 to 1: the rain
  that falls and the wet the road keeps after it, so the look fades out as the road dries. Cloud alone does not make it wet. With
  both sets, `wet` below 1/128 is the clear set, above 127/128 the rain set, and in between the four cubes (the two neighbours in each set) are blended into one cube on the CPU
  (`ColourCube::blend`, 4096 entries of four lookups, a few hundred microseconds) whenever the quantised hour share (1/32) or `wet` (1/64) changes. A profile with
  one set uses it in every weather.
- **Strength.** `grading_strength` (0 to 1), passed through.

The mapping from the rewrite's clock and weather to the 2012 timeline is the rewrite's own (the 2012 keys are hours of its day;
the rewrite's day is the custom clock, sunrise 6 and sunset 18 by default, `docs/specs/time-of-day.md`).

## 7. Settings and rows

| Setting | Key | Values | Default |
|---|---|---|---|
| Colour grading (which look) | `[mod_values.nfsmw] option_grading_variant` | `original` (Off) or a mod's value (`mw2012`) | Off |
| Grading strength | `grading_strength`, env `NFSMW_GRADING_STRENGTH`, flag `--grading-strength`, console `set grading_strength 0.5` | 0 to 1 | 1 |

Two rows on the **Video** page, listed only while the renderer has the pass and a mod has added a look (so the page is as it was
without one): **Color Grading** (Off and the looks, left and right step; the same option every variant kind is, shown here
instead of on Gameplay) and **Color Grading Strength** (0 to 100 percent in steps of 10). The texts are in
`crates/nfsmw/i18n/en/options.ftl` and `options-graphics.ftl` (`options-grading-variant`, `options-grading-variant-original`,
`options-grading-strength`); English is the only catalogue the game has.

Why the 2012 grading is off by default: every effect variant is (the choice is the player's, the option's own value is the game's
look), and the trees, the nitrous and the wind of the 2012 mod are the same. The mod's setting list has no switch for it: the value
is always added when the mod converts the cubes, and costs nothing until chosen.

## 8. Tests, cost and what was not checked

- CPU: `blackbox-gfx` `grading::tests` (identity to 1/255 for edge 16 and 32 over a 21^3 grid of colours, trilinear on a known
  cube, strength, mix, the excess above 1, sanitising, ids, the CPU blend); `blackbox-modding` `grading_tests`, `grading_cube`
  (profile schema and its refusals, the hour blend across midnight, both cube file forms); `nfsmw` `grading::tests` (the selector,
  the bake cache, one-set profiles), `backends/grading_tests` (the choice to the look), `settings::grading`, `frontend::post_options`;
  the mod's `grading::tests` and `chameleon-postfx` `environment_tests`.
- GPU (`--ignored`, Vulkan): `blackbox-gpu-passes` `colour_cube::tests` (identity leaves 16x16 charts unchanged to 1/255 at edges
  16 and 32; a bent cube equals `ColourGrade::apply` within 2/255 at edge 16, 32 and with strength 0.5; two cubes; the cube cache is bounded) and
  `blackbox-render` `colour_cube_vulkan` (through `PostChain`: the pass joins on the first grade, is the last of the effects after tone mapping,
  and leaves the chain's plan when the grade goes).
- Screenshots and cost: see [../provenance/colour-grading-2012.md](../provenance/colour-grading-2012.md) for the scene, the commands,
  the dry pairs and what was not run (the wet pairs, the cost).

**Not checked**: another GPU than the author's, DX12 and GL, an HDR display, the DLSS, FSR 3 and frame generation paths (the pass
is a plain render-size pass before them), the rear-view mirror's picture, and a game window on screen.
