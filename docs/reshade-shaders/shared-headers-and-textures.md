# ReShade pack: shared headers, PD80 helpers and textures

Part of [the ReShade shader pack survey](README.md). Read-only review of the shared headers, the PD80 grading
helpers, the include graph and the texture folder (dimensions only). Nothing was copied.

Global caveat: none of these headers carries an explicit license file or licence line. Under our project rules, anything without a stated license is "uncertain" and must not be copied. Ideas and published formulas can still be re-implemented from the cited spec.

---

## PD80_00_Base_Effects.fxh

1. Licence/credit: header line 2: "Collection of simple base effects often used across shaders by prod80". Licence: none stated (author credit only).
2. Purpose: simple per-pixel colour adjustments (exposure, contrast, brightness, saturation, vibrance) shared by prod80 effects.
3. Provides:
   - `sl(c, b)`: soft-light blend of colour c with blend b (same formula as the Blend_Modes SoftLight).
   - `getLum(x)`: luminance, dot with (0.212656, 0.715158, 0.072186). Note these are not the Rec.709 coefficients (0.2126, 0.7152, 0.0722 are close, the values here are the Rec.709 rounding variant).
   - `exposure(res, x)`: multiplicative lift `res * (x*(1-res) + 1)`, clamped; negative x is scaled by 0.333 first.
   - `con(res, x)`: contrast as lerp from the colour to its soft-light self, x in [-1, 1.5]; negative x halved.
   - `bri(res, x)`: brightness as lerp toward the screen-blend of the colour with itself; negative x halved.
   - `sat(res, x)`: saturation as lerp from luminance (grey) to colour with factor x+1; x=-1 gives greyscale.
   - `vib(res, x)`: vibrance; the lerp factor is 1 + x*(1 - (max-min)), so already-saturated pixels move less.
   - Uniform ranges are given in the header comment (exposure [-4,4], contrast/brightness [-1,1.5], saturation/vibrance [-1,1]).
4. Usefulness: worth porting as an idea (reference). Reason: the vibrance weighting by (max-min) and the asymmetric negative-range scaling are small, practical ideas; the maths is generic, so write our own WGSL from a spec.

## PD80_00_Blend_Modes.fxh

1. Licence/credit: header line 2: "Blend functions across shaders by prod80". Licence: none stated. Comments cite the Khronos NV_blend_equation_advanced spec and nathanm.com Photoshop blend math.
2. Purpose: 21 Photoshop-style layer blend modes operating on LDR colour, with an opacity lerp, selected by one int.
3. Provides:
   - `getAvgColor`: unweighted average of RGB (1/3 each).
   - `ClipColor`: Khronos clip step that keeps non-linear blend results inside [0,1] by pulling colours toward their average luminance.
   - `blendLuma(base, blend)`: shifts base by the difference of average luminance, then ClipColor. Used for Luminosity and Color.
   - `blendColor(base, blend, lum)`: rescales saturation of base to match blend, then blendLuma. Used for Hue and Saturation.
   - Per-mode functions: darken (min), multiply, linearburn (c+b-1 clamped at 0), colorburn, lighten (max), screen (1-(1-c)(1-b)), colordodge, lineardodge (min(c+b,1)), overlay, softlight, vividlight, linearlight, pinlight, hardmix, reflect (c*c/(1-b)), glow (reflect with args swapped), blendhue, blendsaturation, blendcolor, blendluminosity.
   - `blendmode(c, b, mode, o)`: switch over modes 0..20 (0 = Default, returns b) and returns `lerp(c, result, o)`, saturated.
   - Note: mode numbering in the UI string is the contract for callers. Mode 0 is "Default" (returns blend colour).
4. Usefulness: worth porting as an idea. Reason: a compact, self-contained set of blend formulas we could re-derive from the Khronos/Photoshop maths for a compositing or grade step; the ClipColor logic is the one non-obvious part worth reading.

## PD80_00_Color_Spaces.fxh

1. Licence/credit: header line 2: "Color spaces shared across shaders by prod80". Licence: none stated. Inline credits: RGBToHCV "Based on work by Sam Hocevar and Emil Persson"; RGBToHSV "Collected from lolengine.net"; Lab/XYZ from brucelindbloom.com (Lindbloom formulas); sRGB matrices from Lindbloom.
2. Purpose: colour-space conversions used by PD80 grading (HSL/HSV, sRGB transfer, Kelvin temperature, CIE Lab).
3. Provides:
   - `HUEToRGB(H)`: hue (0..1) to fully saturated RGB by abs-based piecewise ramps.
   - `RGBToHCV`, `RGBToHSL`, `HSLToRGB`: chroma/hue/value helper and HSL pair; min/max branch trick.
   - `RGBToHSV`, `HSVToRGB`: the standard lolengine-style pair, using frac/abs and a lerp with saturate.
   - `KelvinToRGB(k)`: colour temperature in 1000..40000 K, log/power-law curve fit with the piecewise split at 66 (0-based kelvin/100). Curve-fit origin is not documented in the file.
   - `LinearTosRGB` and `SRGBToLinear`: IEC 61966-2-1 sRGB transfer. Encode: 12.92*c below 0.0031308, else 1.055*c^(1/2.4)-0.055. Decode: c/12.92 below 0.04045, else ((c+0.055)/1.055)^2.4. Note the two files use different thresholds (0.0031308 for encode, 0.04045 for decode); both are the standard values for their direction.
   - `reference_white` (D65 = 0.95047, 1.0, 1.08883), `K_val` (24389/27), `E_val` (216/24389): Lindbloom constants.
   - `pd80_xyz_to_lab`, `pd80_lab_to_xyz`: CIE Lab with the standard cube-root branch and linear toe.
   - `pd80_srgb_to_xyz`, `pd80_xyz_to_srgb`: 3x3 D65 sRGB matrices (Lindbloom). Inverse is used on output with a clamp to [0, reference white].
   - `pd80_srgb_to_lab`, `pd80_lab_to_srgb`: wrappers that scale Lab to 0..1 (L/100, a/108, b/108) and back. Note the header comment says max blue-channel b is about 107.86, so 108 is the normaliser.
4. Usefulness: worth porting as an idea (and these formulas are standard, so write WGSL from the Lindbloom/IEC spec). Reason: the sRGB transfer and Lab round trip are needed anyway for a correct grade pipeline in linear light; the Kelvin fit is a curve fit with unknown provenance, so treat as reference only.

## PD80_00_Noise_Samplers.fxh

1. Licence/credit: header line 2: "Shared textures across shaders by prod80". Licence: none stated. Textures are binary assets; do not copy them.
2. Purpose: declares the shared blue-noise and Gaussian-noise textures and samplers, plus a dither helper.
3. Provides:
   - Textures (all 512x512 RGBA8, referenced by file name): `pd80_bluenoise.png`, `pd80_bluenoise_rgba.png`, `pd80_gaussnoise.png`.
   - Point-filtered, wrapping samplers for each: `samplerNoise`, `samplerRGBNoise`, `samplerGaussNoise`.
   - `pp`: ReShade "pingpong" uniform (0..128 step 1), used as a frame counter for animated noise.
   - `dither_uv`: scale that maps screen UV to noise UV (buffer size / 512).
   - `dither(tex, coords, var, enabler, str, motion, swing)`: samples the noise, optionally offsets it by a golden-ratio (0.618) multiple of the frame counter with frac, remaps to [-1,1] scaled by `swing`, and divides by 255 times `str`. Returns zero if disabled.
4. Usefulness: reference. Reason: the animated golden-ratio offset is a useful idea for temporal dither; the textures themselves have unknown licence, so do not ship them. Generate our own noise in WGSL from a hash.

## PD80_02_Bonus_LUT_pack.fx

1. Licence/credit: none in file (it only sets macros and includes PD80_LUT_v2.fxh). Licence: none stated.
2. Purpose: a configuration wrapper that picks one 3D LUT atlas (`pd80_example-lut.png`, 4096x3200 per the PNG header) and offers 50 "PD80 Cinematic" presets from a dropdown.
3. Provides: macro settings only: tile size 64, tile amount 64 (so 64 slices), 50 LUTs stacked vertically, and the dropdown string. Final pass is delegated to PD80_LUT_v2.fxh.
4. Usefulness: none for the code itself. The LUT atlas is an image with unknown licence and is a grade "look" asset; do not copy it. Useful only as an example of how a LUT atlas is laid out.

## PD80_LUT_v2.fxh

1. Licence/credit: header lines 2-7: "Original shader by Ganossa", "ReShade Community edits by:", "MartyMcFly, Otis_Inf, prod80". Licence: none stated.
2. Purpose: a multi-LUT colour-grading pass that samples a tiled 3D LUT atlas, with levels, dithering, and Lab-space chroma/luma mixing.
3. Provides:
   - Token-paste macros to build per-config texture and sampler names (`PD80_TexName`, `PD80_Sampler`, `PD80_NameSpace`).
   - UI: dither toggle and strength, LUT selector, chroma and luma mix sliders, intensity, and per-channel black/white in/out levels plus gamma (`ig`, 0.05..10).
   - `levels(color, blackin, whitein, gamma, outblack, outwhite)`: standard levels: clamp-and-normalise to the in range, power by gamma, then map to the out range.
   - `PS_MultiLUT_v2`: the pixel shader. Steps:
     a. Read back buffer colour.
     b. Compute a 3D lookup by treating the blue channel as slice index inside the tiled atlas, and linearly interpolate between two adjacent slices (`lerpfact = frac(blue slice)`).
     c. Apply levels with a small per-channel noise term from `dither`.
     d. Convert LUT result and source to Lab (`pd80_srgb_to_lab`), blend L with `PD80_MixLuma` and a,b with `PD80_MixChroma`, convert back.
     e. Lerp the source toward the result by `PD80_Intensity`.
   - Single technique with one pass using `PostProcessVS`.
   - Note: the code uses a `texelsize` of 1/tile size, and the per-channel noise uses swizzles (`dnoise.yzx`, `dnoise.wxz`) so each channel gets a different noise sample.
4. Usefulness: worth porting as an idea. Reason: the tiled-atlas 3D LUT lookup and the luma-from-LUT, chroma-from-LUT split in Lab is a clear, reusable grading design. Write our own WGSL from the description; do not copy the code.

## ACES.fxh

1. Licence/credit: header lines 2-4 (Baking Lab): "by MJP and David Neubelt" and "All code licensed under the MIT license". Also: "originally written by Stephen Hill (@self_shadow)". Licence: MIT (permissive, with notice).
2. Purpose: a fitted ACES filmic tone curve (RRT and ODT approximation) with input and output colour matrices.
3. Provides:
   - `ACESInputMat`: 3x3 matrix sRGB to AP1-like space (described as sRGB to XYZ, D65 to D60, AP1, RRT_SAT).
   - `ACESOutputMat`: inverse 3x3, ODT_SAT to sRGB.
   - `RRTAndODTFit(v)`: per-channel rational curve, (v*(v+0.0245786) - 0.000090537) / (v*(0.983729v + 0.4329510) + 0.238081). This is the well-known Narkowicz-style ACES fit.
   - `ACESFitted(color)`: input matrix, then fit, then output matrix, then saturate.
4. Usefulness: worth porting as an idea. Reason: a short, well-documented filmic curve that we can re-implement in WGSL from the published fit; MIT would allow copying with notice, but the project rule favours writing our own from the spec, so cite the fit, not the file.

## Dithering.fxh

1. Licence/credit: none in header. Namespace `FXShaders::Dithering`. Licence: none stated.
2. Purpose: ordered-dither helper (4x4 table) to break banding before quantising to 8 bits.
3. Provides:
   - `Ordered16::Pattern`: 4x4 threshold table with values 0..15 in a fixed order.
   - `Ordered16::Apply(color, uv, amount)`: picks a table entry by pixel position, maps it to (-1..1), and scales colour by `1 + that * amount`.
   - Dependency: `#include "Common.fxh"` and `GetResolution()`. Neither is in this folder, so the file cannot compile here as-is.
   - Note: the index is `pattern[pos.x*W + pos.y]`, i.e. the table is read transposed relative to row-major; harmless for a fixed table but worth knowing.
4. Usefulness: reference. Reason: the 4x4 ordered Bayer-style pattern is a textbook idea; TriDither is the better choice for our output stage, and this file has an external dependency we do not have.

## TriDither.fxh

1. Licence/credit: header lines 2-4: "Triangular Dither", "By The Sandvich Maker", "Ported to ReShade by TreyM". Licence: none stated.
2. Purpose: triangular-PDF dither added before reducing colour bit depth (for example to 8 or 10 bits).
3. Provides:
   - `DitherTimer` uniform (timer source) for animation.
   - `remap(v,a,b)` macro.
   - `rand21(uv)`: sin-based hash, averaged over two components.
   - `rand11(x)`: fract(x * 1/41) style 1D hash.
   - `permute(x)`: integer-style permutation via ((34x+1)x) mod 289.
   - `TriDither(color, uv, bits)`: builds two independent uniform noise values per channel, applies a low-end and high-end soft fade (lo, hi) so the dither is near-zero at 0 and 1, and returns `lerp(uniform, triangular, min(lo,hi)) / bitstep`.
   - Usage pattern from the header: add the returned value to colour as the last step before output.
4. Usefulness: worth porting as an idea. Reason: triangular dither is the right shape for removing banding at our final 8-bit output; the endpoint fade is a useful detail. Write our own WGSL hash; the sin-hash here is fragile on some GPUs.

## ReShade.fxh

1. Licence/credit: none in header; framework file for ReShade 3+. Licence: none stated in file (ReShade itself has its own licence in upstream; not checked here).
2. Purpose: ReShade framework glue: version guard, depth-buffer linearisation, buffer size macros, back buffer and depth samplers, and a fullscreen-triangle vertex shader.
3. Provides:
   - Version guard (requires ReShade 3.0+) and `RESHADE_VERSION` / `SUPPORTED_VERSION` macros.
   - Depth input macros: upside-down, reversed (default on), logarithmic, scale, offset, pixel-offset, far plane 1000.
   - Size macros: `BUFFER_PIXEL_SIZE`, `BUFFER_SCREEN_SIZE`, `BUFFER_ASPECT_RATIO`; `ReShade::AspectRatio`, `PixelSize`, `ScreenSize`.
   - Samplers `ReShade::BackBuffer`, `ReShade::DepthBuffer`.
   - `GetLinearizedDepth(uv)`: applies the offsets, optional reversal, and the linearisation d / (far - d*(far - 1)).
   - `PostProcessVS`: generates a full-screen triangle from SV_VertexID with no vertex buffer.
4. Usefulness: none for our code, except as a reminder of the depth linearisation formula (reference). The depth path is not needed for colour grading. The fullscreen-triangle trick is a standard idea we already know.

## ReShadeUI.fxh

1. Licence/credit: none in header; historical comments reference ReShade GUI commits and forum releases. Licence: none stated.
2. Purpose: macros that map ReShade's `ui_type` annotations to the correct widget per ReShade version (input, drag, slider, combo, radio, list, color).
3. Provides: `__UNIFORM_{INPUT|DRAG|SLIDER|COMBO|RADIO|COLOR|LIST}_{BOOL|INT|FLOAT}{1..4}` macros, with version branches (4.0.1, 4.0.0, 4.2.0, 4.3.0) chosen by `SUPPORTED_VERSION`. Note: `__UNIFORM_LIST_BOOL1` is redefined at the end as combo.
4. Usefulness: none. Reason: compatibility shim for old ReShade GUI annotations; our UI is Bevy/egui-style, not ReShade.

## Blending.fxh

1. Licence/credit: lines 5-13: "Blending Header (version 0.8)"; "Header Authors: originalnicodr, prod80, uchu suzume, Marot Satil"; cites the Khronos NV_blend_equation_advanced spec and nathanm.com. Licence: none stated.
2. Purpose: a larger, self-contained blend library (31 modes incl. Normal, grouped by type) with a UI combo macro and a `Blend()` dispatcher that lerps by opacity.
3. Provides:
   - `BLENDING_COMBO(...)` macro: a grouped uniform combo listing all modes.
   - Helpers: `Aux` (the Lab-style soft-light helper), `Lum` (0.33333/0.33334/0.33333 luminance), `SetLum`, `SetSat` (a branchy max/mid/min saturation remap), `Sat`, `min3`, `max3`.
   - Modes: Darken, Multiply, ColorBurn (guarded by b > 0), LinearBurn, Lighten, Screen, ColorDodge (guarded by b < 1), LinearDodge, Addition, Reflect, Glow, Overlay (step-based lerp), SoftLight (two-branch), HardLight, VividLight, LinearLight, PinLight, HardMix, Difference, Exclusion, Subtract, Divide (with +0.01 guard), DivideAlt, DividePS, GrainMerge, GrainExtract, Hue, Saturation, ColorB, Luminosity.
   - `Blend(mode, input, output, blending)`: switch dispatch, default is Normal (lerp), each case lerps input toward the mode result.
   - Note: `SetSat` is a long nested branch chain; the file is 589 lines and the mode switch is cases 0..30 with a numbering that differs from `PD80_00_Blend_Modes.fxh` (for example Difference and Exclusion exist here but not there).
4. Usefulness: reference. Reason: the broadest blend-mode list here, but the formulas are a mix of styles (some Photoshop, some with ad hoc guards) and the file is long; prefer one clean implementation from the spec.

## Macros.fxh

1. Licence/credit: lines 1-5: "BASIC MACROS FOR RESHADE 4 / AUTHOR: TREYM", "Modified by dddfault". Licence: none stated.
2. Purpose: a large set of preprocessor macros for declaring ReShade UI uniforms (int, float, float2, float3, colour, bool), samplers, textures, techniques and passes.
3. Provides: `UI_COMBO`, `UI_RADIO`, `UI_BOOL`, `UI_INT_{I,S,D}`, `UI_FLOAT_{I,S,D,FULL}`, `UI_FLOAT2_*`, `UI_FLOAT3_*`, `UI_COLOR`, each with a `CAT_*` variant that adds a category; `SAMPLER`, `SAMPLER_UV` (with address mode), `TEXTURE`, `TEXTURE_FULL`, `TECHNIQUE`, `PASS`, `PASS_RT`. Top comment has a usage guide.
4. Usefulness: none. Reason: boilerplate for ReShade UI declarations; we have no equivalent need.

---

## Include graph

Edges found by grepping `#include` across the Shaders folder:

- `ReShade.fxh`: included by ArtisticVignette.fx, CAS.fx, Clarity2.fx, DPX.fx, Deband.fx, FakeMotionBlur.fx, LiftGammaGain.fx, MagicBloom.fx, SunsetFilter.fx, Technicolor2.fx, Vignette.fx, VRS_Map.fx (inside a block), and PD80_LUT_v2.fxh.
- `ReShadeUI.fxh`: included by ArtisticVignette.fx, ChromaticAberration.fx, DPX.fx, Deband.fx, FakeMotionBlur.fx, LiftGammaGain.fx, MagicBloom.fx, SunsetFilter.fx, Technicolor2.fx, Vignette.fx.
- `PD80_LUT_v2.fxh`: included only by PD80_02_Bonus_LUT_pack.fx.
- `PD80_00_Noise_Samplers.fxh`: included only by PD80_LUT_v2.fxh.
- `PD80_00_Color_Spaces.fxh`: included only by PD80_LUT_v2.fxh.
- `PD80_00_Base_Effects.fxh`, `PD80_00_Blend_Modes.fxh`: no includer in this folder.
- `Blending.fxh`: no includer (only a comment mentions it).
- `Macros.fxh`: no includer.
- `ACES.fxh`, `Dithering.fxh`, `TriDither.fxh`: no includer. (TriDither is referenced only in a comment.)
- `ArcaneBloom.fxh`: no includer (comment only).
- `qUINT_common.fxh`: included by qUINT_lightroom.fx.
- `VRS_Map.fx` includes `VRS_Map.fxh`.
- Missing dependencies (not in this folder): `Common.fxh` (from Dithering.fxh), `Overwatch.fxh` (from GloomAO.fx), `Fxaa3_11.h` (from FXAA.fxh).

PD80 helpers in use: PD80_LUT_v2 pulls in ReShade.fxh, PD80_00_Noise_Samplers.fxh and PD80_00_Color_Spaces.fxh. PD80_00_Base_Effects and PD80_00_Blend_Modes are not used by any file here.

Shader sources of dynamic textures (from `source =` annotations): `pingpong` (PD80 noise, VRS_Map, Macros pp), `timer` (TriDither, GloomAO), `framecount` (GloomAO, VRS_Map.fxh, qUINT), `frametime` (qUINT), `random` (Deband), `key` (GloomAO). Named texture files: `pd80_bluenoise.png`, `pd80_bluenoise_rgba.png`, `pd80_gaussnoise.png` (PD80 noise), `pd80_example-lut.png` (PD80 bonus LUT), `MagicBloom_Dirt.png` (MagicBloom).

---

## Textures

Folder: `...\SCRIPTS\reshade-shaders\Textures` (dimensions read from PNG IHDR headers; JPG size not parsed; no images otherwise opened). Licence of every texture: none stated, treat as unknown.

PD80 family (used by PD80 headers): pd80_bluenoise.png 512x512; pd80_bluenoise_rgba.png 512x512; pd80_gaussnoise.png 512x512; pd80_cinelut.png 4096x1984; pd80_clean_lut.png 4096x64; pd80_example-lut.png 4096x3200; pd80_neutral-lut.png 512x512; pd80_permtexture.png 256x256.

Other PNG/JPG (dimensions):
- ArcaneBloom_Dirt.png 1280x720
- AreaTex.png 160x560
- AspectRatio.jpg (size not parsed)
- DarkNRich.png 1365x768
- DigitMap.png 379x5
- Dirt.png 1920x1080
- DirtA.png 1920x1080
- DirtOVB.png 1920x1080
- DirtOVR.png 1920x1080
- FontAtlas.png 512x512
- Layer.png 1280x720
- LensDB.png 1024x576
- LensDB2.png 1024x576
- LensDOV.png 1920x1080
- LensDUV.png 1920x1080
- LensSprite.png 1920x1080
- MagicBloom_Dirt.png 1024x576
- MatCap.png 256x256
- MultiLut_atlas1.png 1024x544
- NeoBloom_LensDirt.png 1280x720
- NormalMap.png 1024x1024
- Overlay.png 1280x720
- Pong.png 128x128
- SearchTex.png 64x16
- Sketch_Shadow.png 1024x1024
- Stage.png 3840x2160
- UIDetectMaskRGB.png 1920x1080
- UIMask.png 960x540
- alpha-checkerboard.png 1920x1080
- bayer16x16.png 16x16
- bluenoise.png 32x32
- cursor.png 108x108
- image.png 1440x1080
- lut.png 1024x32
- monochrome_gaussnoise.png 512x512
- neutral_lut.png 1024x32
- ssdonoise.png 4x4
- waow.png 2550x1710
- dummy (0 bytes, no extension; placeholder)

Textures named in shader code: pd80 noise set, pd80_example-lut.png, MagicBloom_Dirt.png. Others not referenced by the PD80/shared headers in this folder.
