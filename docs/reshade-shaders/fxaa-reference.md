# FXAA reference notes (NVIDIA FXAA 3.11 header)

Part of [the ReShade shader pack survey](README.md). Read-only review of `FXAA.fxh` (2047 lines, mostly console
paths we don't need). No code is copied. Our own FXAA is described in [post-processing.md](../post-processing.md).

## Licence

Header (lines 1-19): "NVIDIA FXAA 3.11 by TIMOTHY LOTTES", "COPYRIGHT (C) 2010, 2011 NVIDIA CORPORATION.
ALL RIGHTS RESERVED", followed by an all-caps "*AS IS*" warranty disclaimer and limitation of liability.

Type: a proprietary vendor copyright notice with a disclaimer. It is not an open-source licence (no MIT,
Apache, BSD or GPL terms). The header grants no explicit right to copy, modify or redistribute the code.
Treat the file as all rights reserved by NVIDIA. Do not port its code into the Rust rewrite. Learning
from the published algorithm (the FXAA idea and the publicly described steps) is a separate matter and
is what we are doing.

## Purpose

FXAA is a single-pass, post-process anti-aliasing filter. It works on the final LDR image (after tone
mapping), with no geometry or depth data. It finds high-contrast edges from luma, estimates edge
orientation (horizontal or vertical), walks along the edge to find its ends, and blends the pixel toward
the edge neighbour that gives the shorter span. The file bundles a PC high-quality path (FXAA_PC), a
PC console-style path (FXAA_PC_CONSOLE), and console paths for 360 and PS3 that we do not need.

## Input contract (what the engine must supply)

- Input texture: RGB plus luma. Luma is stored in alpha (default) or read from green (`FXAA_GREEN_AS_LUMA`).
- Luma must be perceptual. The header's recipe: tone map, `sqrt` for gamma 2.0, then
  `alpha = dot(rgb, (0.299, 0.587, 0.114))`. For linear RGB output the luma is `sqrt(dot(...))`.
- Run after tone mapping, on LDR data. Do it in gamma space, not linear, because dithering gets
  more visible in linear space (header lines 151-154).
- Bilinear sampler on the input.
- Uniforms: `fxaaQualityRcpFrame` (1/width, 1/height), `fxaaQualitySubpix`, `fxaaQualityEdgeThreshold`,
  `fxaaQualityEdgeThresholdMin`. The console path also takes `fxaaConsole*` values.
- Vertex stage: full-screen triangle that outputs `pos` (pixel centre) and `fxaaConsolePosPos`
  (corner offsets). Use noperspective interpolation.

## Public macros (configuration)

| Macro | Default | Meaning |
|---|---|---|
| `FXAA_PC` | 0 | Select the high-quality PC path (we want this). |
| `FXAA_PC_CONSOLE` | 0 | Select the cheap console-style PC path. |
| `FXAA_PS3`, `FXAA_360`, `FXAA_360_OPT` | 0 | Console-only paths. Not needed. |
| `FXAA_HLSL_3/4/5`, `FXAA_GLSL_120/130` | 0 | API selection (sampler and texture macros). |
| `FXAA_QUALITY__PRESET` | 12 | Quality preset, see below. Compiled in, not a uniform. |
| `FXAA_GREEN_AS_LUMA` | 0 | Use green as luma. Needs no luma in alpha, but dark non-green colours lose AA. |
| `FXAA_EARLY_EXIT` | 1 | Return early on pixels without an edge. |
| `FXAA_DISCARD` | 0 | Discard instead of returning the unchanged colour (GL only). |
| `FXAA_GATHER4_ALPHA` | auto | Use gather for the 4 luma taps. Enabled automatically on HLSL 5. |
| `FXAA_FAST_PIXEL_OFFSET` | auto | GLSL-only offset optimisation. |
| `FXAA_CONSOLE__PS3_EDGE_SHARPNESS` / `..._THRESHOLD` | 8.0 / 0.125 | PS3-only tuning. Not needed. |

Runtime tuning inputs (quality path, from the function signature):

- `fxaaQualitySubpix`: 1.00 soft, 0.75 default, 0.50 sharper, 0.25 almost off, 0 off.
- `fxaaQualityEdgeThreshold`: 0.333 fast, 0.250 low, 0.166 default, 0.125 high, 0.063 overkill.
- `fxaaQualityEdgeThresholdMin`: 0.0833 upper limit (default), 0.0625 faster, 0.0312 slower and dark-edge-aware.
  With `FXAA_GREEN_AS_LUMA`, set to 0 first.

## Quality presets (`FXAA_QUALITY__PRESET`)

Digit meaning (header lines 390-398): the lowest digit controls speed, the highest controls style.
- 10-15: medium dither. 10 is fastest, 15 highest quality. 12 is the default.
- 20-29: low dither, more expensive. 20 fastest, 29 highest. 23 matches FXAA 3.9 most closely.
- 39: no dither, very expensive.

Edge-search step tables: each preset is a list of step multipliers `P0..Pn` (the walk distance in texels
per step) and a count `PS` (number of steps). The walk uses an increasing step size, and the last
step is the largest.

| Preset | Steps (PS) | Step multipliers P0..P(PS-1) |
|---|---|---|
| 10 | 3 | 1.5, 3, 12 |
| 11 | 4 | 1, 1.5, 3, 12 |
| 12 (default) | 5 | 1, 1.5, 2, 4, 12 |
| 13 | 6 | 1, 1.5, 2, 2, 4, 12 |
| 14 | 7 | 1, 1.5, 2, 2, 2, 4, 12 |
| 15 | 8 | 1, 1.5, 2, 2, 2, 2, 4, 12 |
| 20 | 3 | 1.5, 2, 8 |
| 21 | 4 | 1, 1.5, 2, 8 |
| 22 | 5 | 1, 1.5, 2, 2, 8 |
| 23 | 6 | 1, 1.5, 2, 2, 2, 8 |
| 24 | 7 | 1, 1.5, 2, 2, 2, 3, 8 |
| 25 | 8 | 1, 1.5, 2, 2, 2, 2, 4, 8 |
| 26 | 9 | 1, 1.5, 2 x5, 4, 8 |
| 27 | 10 | 1, 1.5, 2 x6, 4, 8 |
| 28 | 11 | 1, 1.5, 2 x7, 4, 8 |
| 29 | 12 | 1, 1.5, 2 x8, 4, 8 |
| 39 | 12 | 1, 1, 1, 1, 1, 1.5, 2, 2, 2, 2, 4, 8 |

Note on preset 39: the table has 12 entries, but P0 to P4 are all 1.0. That makes the first five steps
uniform and dense, which fits "no dither, very expensive".

The walk in the quality path has a maximum of 12 steps. Each step samples one texel per side and
continues until the luma difference at that end passes the gradient threshold (see stages below).

## Algorithm stages (quality path, PC)

Written in my own words. Based on the full read of `FxaaPixelShader` (lines 721-1238).

1. Sample the centre pixel's luma, plus the S, E, N, W neighbours (4 taps, or one gather on HLSL 5).
2. Compute the local luma range: `rangeMax - rangeMin` over the 5-tap cross.
3. Early exit. If the range is below `max(EdgeThresholdMin, EdgeThreshold * rangeMax)`, return the centre
   pixel unchanged. This is the fast path for flat areas.
4. Sample the four diagonal corners (NW, SE, NE, SW) for the full 3x3 neighbourhood.
5. Compute edge-direction measures. Horizontal `edgeHorz` and vertical `edgeVert` are second-derivative
   style sums over the 3x3 neighbourhood. The larger one decides `horzSpan`.
6. Compute the sub-pixel aliasing term. Average the 3x3 luma and compare to the centre. `subpixC` is
   clamped to 0..1. It scales a blend factor later (`subpixF`, `subpixH`).
7. Choose the edge side. Use the larger of the two gradients across the edge (`pairN`). Flip the step
   sign so the walk goes toward the side with the bigger contrast. Shift the start position half a
   texel across the edge.
8. Walk along the edge in both directions from the midpoint, using the preset's step table. At each step
   sample the luma at the walk end, subtract the average of the two sides, and stop when the difference
   reaches `gradientScaled` (gradient / 4). Up to 12 steps, per preset.
9. Span ends. Compute the distance to each end (`dstN`, `dstP`). Mark an end as "good" if the luma
   crosses the centre's sign. The shorter span wins; its `pixelOffset = 0.5 - dst/spanLength`.
10. Blend. Take `max(pixelOffset if good span, subpixH)`, shift the sample position by that offset along
    the edge normal, and return `texture(pos)` with the original luma kept in alpha.

Console path (FXAA_PC_CONSOLE, lines 1256-1333): cheaper. Four corner taps, a single normalised edge
direction, two extra taps along the direction scaled by `fxaaConsoleEdgeSharpness`, and a 2-tap fallback
if the result leaves the min/max range. Edge threshold is `fxaaConsoleEdgeThreshold` (0.125 default) and
the darks cut-off is `fxaaConsoleEdgeThresholdMin` (0.05 default, header line 862).

Console 360 and PS3 variants (lines 1351-2047): fully separate, console-only. Not useful for us.

## Parameters and macro list

Edge-search multipliers: `FXAA_QUALITY__P0` to `P11`, and `FXAA_QUALITY__PS` (step count). Set by the
preset. Nothing to tune by hand except the preset itself.

API portability macros (lines 614-699): `FxaaFloat`, `FxaaFloat2..4`, `FxaaTexTop`, `FxaaTexOff`,
`FxaaTexAlpha4`, `FxaaTexGreen4`, `FxaaSat`. These map one shader body onto HLSL, GLSL and console APIs.
We will not reuse these; the Rust rewrite targets WGSL or similar.

## Usefulness verdict

Verdict: low usefulness for the rewrite as code, moderate as a reference. The file is NVIDIA-copyright
and cannot be copied. The useful part is the algorithm description, which is already public. The
preset table and tuning ranges are facts about the algorithm, and they are usable as tuning values.

Gaps versus `docs/post-processing.md`:

- That doc (lines 38-41) says only that the FXAA pass is "written from the published description of
  version 3.11", smooths edges in one pass, and runs before upscaling. It does not state the luma
  requirement (luma in alpha, or green as luma), the perceptual-space luma recipe, or the rule to run
  after tone mapping on LDR data. These are the first things to check in our implementation.
- The doc does not mention the edge threshold and darks-threshold ranges, the subpix knob, or the
  preset ladder (quality versus speed trade-off). If our shader has fixed values, check them against
  the ranges above (threshold 0.166 default, threshold min 0.0833, subpix 0.75).
- The doc does not mention the early-exit path or the "shorter span wins" blend rule. Both are in
  the stages above and affect speed and look.
- The doc does not discuss the linear-versus-gamma colour issue (dither visibility). This is worth
  noting in our notes.

Nothing here is missing from our repo's design that a rewrite must add. The one piece of value is the
preset table as a tuning reference, which costs nothing to write from our own derivation.
