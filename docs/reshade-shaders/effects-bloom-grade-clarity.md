# ReShade effects: bloom, grading and local contrast

Part of [the ReShade shader pack survey](README.md). Read-only notes on 7 files (source folder not committed;
nothing is copied into the repo). Licence summary is in the [index](README.md#licences).

Licence summary (decides what may be copied):

| File | Licence stated in file | Copy status for us |
|---|---|---|
| Clarity2.fx | none stated | do not copy |
| qUINT_lightroom.fx | "All rights reserved" | do not copy |
| MagicBloom.fx | MIT (luluco250, 2017) | permissive; still write our own WGSL from spec per project rule |
| ArcaneBloom.fxh | none stated in this file | do not copy |
| LiftGammaGain.fx | none stated | do not copy |
| Technicolor2.fx | none stated | do not copy |
| SunsetFilter.fx | CC BY-SA 4.0 | share-alike; do not copy code |

---

## 1. Clarity2.fx

1. Licence/credit: "//Clarity2 by Ioxa" / "//Version 08.09.17 for ReShade 3.0" / "// Lightly optimized by Marot Satil for the GShade project." No licence stated. Treat as unlicensed.

2. Purpose: local-contrast ("clarity") enhancer that boosts or softens mid-frequency detail by blending a blurred copy of the image into the original with Photoshop-style blend modes.

3. Algorithm:
   - Pass 1 (Clarity1): take the backbuffer at half resolution (sampled at texcoord multiplied by ClarityOffsetTwo) and store luma (dot with weights 0.327, 0.656, 0.016) in an R8 texture (or RGB if ClarityRGBMode).
   - Pass 2 (Clarity2): separable Gaussian blur, vertical direction only. Kernel is symmetric; radius selects one of five precomputed kernels (4, 6, 11, 15, 18 weights in a sequence). Taps use the "linear sampling" trick: pairs of neighbouring taps are merged into one bilinear fetch at a fractional offset (e.g. 1.4895848401 px, 3.4757135714 px).
   - Pass 3 (Clarity3): same blur, horizontal direction.
   - Pass 4 (ClarityFinal): reads blurred value B (sampled at texcoord divided by ClarityOffsetTwo). Build mask:
     - sharp = (luma + (1 - B)) / 2, a 50/50 mix of luma and inverted blur (a high-pass-like signal).
     - Optional MaskContrast: a "vivid light" style remap of sharp, lerped in by MaskContrast.
     - If dark or light halo intensity is non-zero: apply smootherstep-like curve c = s^3 (6s^2 - 15s + 10 composed as s*(s*(6s-15)+10)) and lerp toward it by the dark intensity (for s < 0.5) or light intensity (for s >= 0.5).
     - Dither: subtract a small hash noise scaled by DitherStrength (luma mode uses the blue-channel noise term only).
     - Blend with luma using the selected blend mode (soft light, overlay, hard light, multiply, vivid light, linear light, addition). Default is 2 (hard light).
     - Blend-If: a mask from the luma of the original (dark threshold ClarityBlendIfDarkTwo, light threshold ClarityBlendIfLightTwo, soft range BlendIfRange) limits where the effect applies.
     - Output: luma = lerp(luma, sharp, Strength); result = new luma times the original chroma ratio (orig.rgb / luma). Chroma is preserved exactly.
   - Note: the dark/light tests use "if(ClarityDarkIntensityTwo || ...)" style branches; these are runtime branches, not compile-time.

4. Parameters:
   - ClarityRadiusTwo: int slider 0..4, default 3. Selects blur kernel size.
   - ClarityOffsetTwo: float slider 1..5 (step 1), default 8. Coordinate scale for the half-res pre-scale (see note in Cross-file notes).
   - ClarityBlendModeTwo: combo, 0 Soft Light, 1 Overlay, 2 Hard Light, 3 Multiply, 4 Vivid Light, 5 Linear Light, 6 Addition. Default 2.
   - ClarityBlendIfDarkTwo: int 0..255, default 50 (shadows excluded below this).
   - ClarityBlendIfLightTwo: int 0..255, default 205.
   - BlendIfRange: float 0..1, default 0.2.
   - ClarityStrengthTwo: float 0..1, default 0.4 (final mix).
   - MaskContrast: float 0..1, default 0.
   - ClarityDarkIntensityTwo: float 0..10, default 0.4.
   - ClarityLightIntensityTwo: float 0..10, default 0.
   - DitherStrength: float 0..10, default 1.0.
   - Debug only (UseClarityDebug=1): DebugMode combo None/ClarityMask/BlendIfMask/Blur.
   - PreprocessorDefinitions: a combo that is only a note to the user.

5. Passes:
   - Clarity1: reads ReShade::BackBuffer (scaled coords) -> writes Clarity2Tex (half res, luma).
   - Clarity2: reads Clarity2Tex -> writes Clarity2Tex2 (vertical blur).
   - Clarity3: reads Clarity2Tex2 -> writes Clarity2Tex (horizontal blur).
   - ClarityFinal: reads Clarity2Tex (blur) and ReShade::BackBuffer -> writes backbuffer.

6. Preprocessor macros:
   - UseClarityDebug (default 0): adds debug UI and debug returns.
   - ClarityRGBMode (default 0): 0 = luma-only, 1 = RGB processing. Changes texture format (R8 vs RGBA8) and many branches.
   - Internal: ClarityLuma (float3 weights), CF/ClarityFloat aliases, noiseR/G/B dither hashes, Offset macros (many named sets for the kernels).
   - Uses BUFFER_PIXEL_SIZE, BUFFER_WIDTH, BUFFER_HEIGHT.

7. Textures:
   - Clarity2Tex and Clarity2Tex2: half-resolution, format R8 (or RGBA8 in RGB mode), CLAMP addressing, POINT min / LINEAR mag.
   - Backbuffer (ReShade::BackBuffer) read in Clarity1 and ClarityFinal.

8. Usefulness: reference (worth noting the idea, do not copy). Reason: a "clarity" mask built from luma and inverted blur, then a blend mode, is a standard local-contrast idea. We can write our own from the spec in WGSL if we want the effect. Known issues found while reading (verify before reuse):
   - In the Hard Light branch (non-RGB), the line "const sharp = lerp(A,B,C);" redeclares an existing variable. Read as written, this does not look valid HLSL.
   - In the Overlay branch (RGB mode), "const const float3 B" is a duplicated qualifier. Also invalid.
   - Clarity1 samples the backbuffer at texcoord times ClarityOffsetTwo, and ClarityFinal samples the blur at texcoord divided by ClarityOffsetTwo. The two scales cancel, but most of the half-res texture then holds clamped edge texels. Treat the pre-scale as a quirk to check numerically, not as a design.
   - ReShade-only APIs: ReShade.fxh, ReShade::BackBuffer, PostProcessVS, tex2Dlod, ReShadeUI-style annotations (ui_type, ui_min, ui_tooltip), technique/pass syntax, "pooled"-free textures, and HLSL-style static const arrays inside functions.

---

## 2. qUINT_lightroom.fx (Lightroom, by Marty McFly / Pascal Gilcher)

1. Licence/credit: "Lightroom by Marty McFly / P.Gilcher" / "part of qUINT shader library for ReShade 4" / "Copyright (c) Pascal Gilcher / Marty McFly. All rights reserved." Licence: all rights reserved, no open licence stated. Do not copy anything from this file. The shared qUINT library (qUINT_common.fxh, not in this folder) also needs its own licence check.

2. Purpose: colour-grading toolbox. Applies curves, global white balance, saturation/vibrance, a seven-band hue/saturation/exposure palette, and vignette. Bakes the whole grade into a 64x64x64 colour cube stored as a 4096x64 strip, so the grade can be reapplied or exported as a LUT.

3. Algorithm:
   - Vertex stage: a fullscreen triangle from SV_VertexID. It also computes seven hue anchors (per-band hue shift factors) as nointerpolation values.
   - Curves (per channel, applied to R, G and B separately):
     - Levels: normalise input between black and white level (each in 0..255 scale).
     - Exposure is 2^E; gamma is 2^(-G) applied as x^gamma.
     - Five tonal zones with smoothstep masks: blacks (peak near 0), shadows (peak 0.25), midtones (peak 0.5), highlights (peak 0.75), whites (near 1). Each zone adds its own exponent to a single power curve: x = x^(2^(sum of zone weight times zone value, minus 1)).
     - Contrast: lerp between x and the smoothstep S-curve 3x^2 - 2x^3 by the contrast amount.
   - White balance: converts to HSL and moves hue towards orange (0.061) for warm or blue-ish (0.561) for cool, blended by temperature amount. Tint uses hues 0.311 (green) or 0.811 (magenta). Note: the tint branch tests the sign of the temperature value, not the tint value (line 783). Likely a copy error.
   - Saturation and vibrance: S' = S + S*sat, then S' = S^(2^(-vibrance)).
   - Palette (per pixel, HSL):
     - Seven hue bands at 0, 1/12, 2/12, 4/12, 6/12, 8/12, 10/12 of the wheel, each with a triangular weight (linear falloff over neighbouring bands; the yellow band is asymmetric).
     - Each band: hue is replaced by a hue factor from the vertex stage (shifted toward the neighbour band), saturation S' = S + S*sat_band, luminance L' = L * 2^(sqrt(S) * exposure_band * (1 - L) * S).
     - Output: weighted sum of the seven band colours.
   - LUT bake (PS_ProcessLUT): starts from an identity cube (draw_lut_4096x64 writes the coordinates), runs the full grade on each cube coordinate, writes the result into LutTexInternal.
   - LUT apply (PS_ApplyLUT): reads backbuffer colour, optionally draws a LUT preview tile strip (draw_lut), then reads the baked cube with manual trilinear-like interpolation: two adjacent blue slices, linear blend by the fractional blue index.
   - Display (PS_DisplayStatistics): optional clipping overlay (red where channel sum is at least 3, blue where it is 0), optional luma curve graph, optional histogram graph, and vignette.
   - Vignette: maps uv to -1..1, applies aspect ratio, computes length, linearstep between inner and outer radius, applies power curve, then blends by one of four modes (multiply, subtract, screen-like, luma-preserving).
   - Histogram (optional): 20x20 sample grid by default, each sample contributes a triangular kernel into 128 bins per channel; then smoothed.

4. Parameters (all float "drag" unless noted; defaults in brackets):
   - Palette: per-hue Hue Control, Exposure, Saturation for Red, Orange, Yellow, Green, Aqua, Blue, Magenta: range -1..1, default 0 each (21 floats total).
   - Curves: Global Black Level [0] and White Level [255] (0..512 step 1); Global Exposure [0], Gamma [0], Blacks/Shadows/Midtones/Highlights/Whites Curve [0], Contrast [0] (all -1..1).
   - Colour: Global Saturation [0], Global Vibrance [0], Temperature [0], Tint [0] (all -1..1).
   - Vignette (ui_category Vignette, off by default): Enable [false], Show radii [false], Inner Radius [0] (0..2), Outer Radius [1] (0..3), Width [0] and Height [0] (0..1), Amount [1] (0..1), Curve [1] (0..10), Blend Mode combo [1] = Subtract (Multiply, Subtract, Screen, LumaPreserving).
   - LUT (category LUT): Enable LUT Overlay [false], LUT tile size [16] (8..64), LUT tile count [16] (8..64), LUT scroll [0] (0..5).
   - Debug: Enable Luma Curve Display [false], Enable Black/White Clipping Mask [false].
   - Histogram (only if ENABLE_HISTOGRAM=1): Enable [false], Samples [20] (32..96 in UI, default 20), Curve Height [15] (5..50), Curve Smoothness [5] (1..10).

5. Passes (technique "Lightroom"):
   - PProcessLUT: VS_Lightroom -> PS_ProcessLUT, writes LutTexInternal (4096x64).
   - PApplyLUT: VS_Lightroom -> PS_ApplyLUT, reads backbuffer and LutTexInternal, writes backbuffer.
   - PHistogramGenerate (only if ENABLE_HISTOGRAM=1): PostProcessVS -> PS_HistogramGenerate, reads backbuffer, writes HistogramTex (128x1).
   - PHistogram: VS_Lightroom -> PS_DisplayStatistics, reads backbuffer (and HistogramTex if enabled), writes backbuffer.
   - Note: vignette and clipping only appear in the display pass, so they are not baked into the LUT. The LUT contains curves, white balance, saturation and palette only.

6. Preprocessor macros:
   - ENABLE_HISTOGRAM (default 0): compiles histogram code and UI.
   - HISTOGRAM_BINS_NUM (default 128).
   - RESHADE_QUINT_COMMON_VERSION_REQUIRE 200 (requires qUINT_common.fxh).
   - Uses qUINT::PIXEL_SIZE and qUINT::sBackBufferTex from qUINT_common.fxh.

7. Textures:
   - LutTexInternal: 4096x64, RGBA8, sampler sLutTexInternal (the baked 64^3 cube strip).
   - HistogramTex: 128x1, RGBA16F, optional.
   - Backbuffer via qUINT::sBackBufferTex.

8. Usefulness: reference, with one idea worth porting from the spec. Reason: the LUT-bake pipeline (run the grade once over an identity cube, store it, then apply by a single cube lookup) is a clean design for our own grade pass, and the curve/zone and palette formulas are useful to read as description. Do not copy code. ReShade-only dependencies: qUINT_common.fxh, tex2D/tex2Dlod, SV_VertexID-based fullscreen triangle, nointerpolation array varyings, technique tooltip annotations, ui_* annotations, ReShade macros (BUFFER_WIDTH/HEIGHT, ReShadeUI).

---

## 3. MagicBloom.fx (luluco250, MIT)

1. Licence/credit: "Magic Bloom by luluco250" / "Attempts to simulate a natural-looking bloom." / "MIT Licensed: Copyright (c) 2017 luluco250" (full MIT text in the header). Licence: MIT. Permissive, but our rule is to write our own WGSL from a spec, so use this as an idea source only.

2. Purpose: natural-looking bloom using a multi-level Gaussian-like blur pyramid, optional adaptive exposure (bloom gets dimmer on bright scenes), and optional lens dirt.

3. Algorithm:
   - Threshold and blur level 1: sample backbuffer at half resolution with a 9x9 separable-looking kernel (product of a 9-tap 1-D Gaussian with sigma = 2). Raise each channel to the power fBloom_Threshold (contrast of bloom), multiply by fBloom_Intensity.
   - Levels 2 to 8: each level samples the previous level with the same 9x9 kernel at a further step scale (4, 8, 8, 16, 32, 64, 128 times the pixel size). Each level is half the resolution of the one before.
   - Blend: sum all 8 levels, divide by 8, tone-map with an Uncharted 2 style curve (A=0.15, B=0.5, C=0.1, D=0.2, E=0.02, F=0.3, W=11.2), normalised so white point W maps to 1. The exposure fed in is fExposure divided by the adapted luminance (or 100 when adaptation is off).
   - Dirt (optional): screen-blend the bloom with a dirt texture scaled by fDirt_Intensity.
   - Final: screen-blend the bloom result onto the backbuffer (1 - (1-a)(1-b)).
   - Adaptation (optional):
     - Luma per pixel (Rec.709 weights 0.2126, 0.7152, 0.0722) is written to a 256x256 R32F texture with a full mip chain.
     - A single sample at the screen centre, at mip level (lowest mip minus precision), gives an average over a chosen area (precision 0 = whole-frame average).
     - Scale by sensitivity, clamp to Min/Max, then lerp with last frame's value using speed (no delta time, so it does not depend on framerate).
     - Save the result into LastAdapt for the next frame.
   - Constants: sigma = 2, 9 taps per axis. Kernel values (precalculated): 0.0269955, 0.0647588, 0.120985, 0.176033, 0.199471 (mirrored). Accum constant 1.02352.
   - Arithmetic note (my check, not run): the 1-D kernel sums to about 0.97702, so 1.02352 normalises the 1-D sum, not the 2-D product. The 2-D blur therefore comes out about 2.3 percent darker than unity. The non-precalculated path normalises properly.

4. Parameters:
   - Bloom Intensity: float 0..10, default 1.0.
   - Bloom Threshold: float 1..10 (step 0.1), default 2.0. Exponent applied to colour (higher = fewer bright pixels contribute).
   - Dirt Intensity: float 0..1, default 0 (only if MAGICBLOOM_NODIRT=0).
   - Exposure: float 0..1, default 0.5 (only if NOADAPT=0). Target exposure.
   - Adaptation Speed: float 0.001..1, default 0.1.
   - Adapt Sensitivity: float 0..3, default 1.0.
   - Adaptation Min/Max: float2 0..1, default (0, 1).
   - Adaptation Precision: int 0..lowest_mip (about 9), default lowest_mip * 0.3 (about 2.7 in code).
   - Debug Options: combo None / Display Bloom Texture.

5. Passes (technique MagicBloom):
   - Blur1: PostProcessVS -> PS_Blur1 reads ReShade::BackBuffer, writes tMagicBloom_1 (half res). Also thresholds and applies intensity.
   - Blur2 to Blur8: each reads the previous level (sMagicBloom_1 to _7), writes tMagicBloom_2 to _8.
   - Blend: reads backbuffer and all 8 bloom levels plus optional adapt value and dirt; writes backbuffer.
   - GetSmall (if adapt): reads backbuffer, writes tMagicBloom_Small (luma, mipped).
   - GetAdapt (if adapt): reads tMagicBloom_Small (mip sample) and LastAdapt; writes tMagicBloom_Adapt (1x1).
   - SaveAdapt (if adapt): copies Adapt into LastAdapt.

6. Preprocessor macros:
   - MAGICBLOOM_ADAPT_RESOLUTION (default 256): size of the luma texture; must be a power of two.
   - MAGICBLOOM_BLUR_PRECALCULATED (default 1): use constant kernel, or compute the Gaussian in the shader.
   - MAGICBLOOM_NODIRT (default 0): drops dirt texture, UI and blend.
   - MAGICBLOOM_NOADAPT (default 0): drops adaptation textures, UI and passes.
   - Internal: iBlurSamples = 4, lowest_mip (computed with a bit-trick CONST_LOG2), double_pi, luma_value.
   - Includes ReShade.fxh and ReShadeUI.fxh.

7. Textures:
   - tMagicBloom_1 to _8: RGBA16F, pooled, sizes half, quarter, ... 1/256 of the backbuffer.
   - tMagicBloom_Small: R32F, adapt resolution with lowest_mip mip levels (adaptation only).
   - tMagicBloom_Adapt, tMagicBloom_LastAdapt: R32F, 1x1 (adaptation only).
   - tMagicBloom_Dirt: loaded from "MagicBloom_Dirt.png" (external asset, not in this folder; its licence is separate and unknown). Dirt only.

8. Usefulness: worth porting as an idea. Reason: the multi-level blur pyramid (each level at half resolution, summed, then tone-mapped) and mip-based luminance averaging for adaptation are both directly useful for a wgpu bloom pass. Write our own WGSL from the spec. Note the adaptation uses a fixed per-frame lerp (not delta-time), which we should decide on deliberately for the rewrite. ReShade-only APIs: ReShade.fxh/ReShadeUI.fxh, ReShade::BackBuffer, PostProcessVS, pooled textures, MipLevels, source="..." texture loading, tex2Dlod with explicit mip, __UNIFORM_* annotations.

---

## 4. ArcaneBloom.fxh (luluco250 family)

1. Licence/credit: "Header file for ArcaneBloom by luluco250" / "This file exposes some of the interals of the shader so you can use it in your own". No licence text in this file. Do not assume MIT from MagicBloom. Treat as unlicensed.

2. Purpose: a reusable header (no technique) exposing ArcaneBloom's textures, samplers and maths helpers to other shaders. Not a standalone effect.

3. Algorithm (helpers only, no full pipeline in this file):
   - Textures: five bloom levels at 1/2, 1/4, 1/8, 1/16, 1/32 of buffer size, RGBA16F (a 1/64 level is commented out). Optional adaptation texture R32F (1x1 by default).
   - Tone helpers: Reinhard c/(1+c); inverse Reinhard c/max(1-c, eps); inverse Reinhard on luma (scales colour by lum/(1-lum)).
   - Blur helpers:
     - box_blur: 4 bilinear taps at plus/minus half-pixel offsets, averaged.
     - gaussian_blur: 13 taps along a direction, unrolled, using a hard-coded weight table (0.017997 .. 0.132981, symmetric). Start offset is minus 6 taps.
   - Arithmetic note (my check): the 13 weights sum to about 0.970501, not 1.0. Callers must normalise or accept about 3 percent loss.
   - Luma: Rec.709 weights (0.2126, 0.7152, 0.0722).
   - normal_distribution: standard 1-D Gaussian density with variance squared used as sigma (argument is spread, not variance, so the name is misleading).
   - linearstep helper, cPI constant.

4. Parameters: none in this file. ARCANE_BLOOM_USE_ADAPTATION (default 1) is the only macro-level switch.

5. Passes: none in this file. Other shaders include it and define their own passes.

6. Preprocessor macros:
   - ARCANE_BLOOM_USE_ADAPTATION (default 1): when 1, defines the adaptation texture and sampler.
   - Internal DEF_BLOOM_TEX and DEF_BLOOM_SAMPLER (macros, undefined after use).
   - Uses BUFFER_WIDTH and BUFFER_HEIGHT.

7. Textures (declared, namespace ArcaneBloom::_ for storage, samplers sBloom0..sBloom4, sAdapt):
   - tArcaneBloom_Bloom0 to Bloom4 (RGBA16F, half to 1/32), tArcaneBloom_Adapt (R32F).
   - Samplers use POINT filtering for adaptation, clamp addressing.

8. Usefulness: reference. Reason: the useful part is the layout (a bloom pyramid shared through a header and namespace), which is already covered by MagicBloom. The helper maths is trivial and easy to write from spec. It is a clean example of sharing textures between effects, but it is unlicensed, so do not copy.

---

## 5. LiftGammaGain.fx (3an and CeeJay.dk)

1. Licence/credit: "Lift Gamma Gain version 1.1" / "by 3an and CeeJay.dk". No licence stated. Do not copy.

2. Purpose: per-channel lift (shadows), gamma (midtones) and gain (highlights) colour correction.

3. Algorithm:
   - Read backbuffer colour c.
   - Lift: c' = c * (1.5 - 0.5*L) + 0.5*L - 0.5, where L is the lift triplet. At L = 1 this is identity. Then clamp to 0..1.
   - Gain: c'' = c' * G.
   - Gamma: c''' = |c''|^(1/Gm), where Gm is the gamma triplet. Then clamp to 0..1.

4. Parameters (all float3 sliders 0..2, default 1,1,1):
   - RGB Lift (shadows), RGB Gamma (midtones), RGB Gain (highlights).

5. Passes: one pass, PostProcessVS -> LiftGammaGainPass, reads backbuffer, writes backbuffer.

6. Preprocessor macros: none.

7. Textures: backbuffer only.

8. Usefulness: none, or trivially reimplemented. Reason: a three-triplet lift/gamma/gain is standard and we can write it from the description in a few lines; there is nothing to learn from the file itself. ReShade-only: ReShade.fxh, ReShade::BackBuffer, PostProcessVS, __UNIFORM_SLIDER_FLOAT3.

---

## 6. Technicolor2.fx (Prod80, optimised by CeeJay.dk)

1. Licence/credit: "Technicolor2 version 1.0" / "Original by Prod80" / "Optimized by CeeJay.dk". No licence stated. Do not copy.

2. Purpose: emulate the look of early three-strip Technicolor film, with strong colour separation and a per-channel cross-subtraction.

3. Algorithm (described in words, with channel swizzles):
   - Clamp input colour c to 0..1. Let t = 1 - c.
   - Build a cross-product term: each output channel gets the product of its own colour and the inverses of the other two channels (red: r(1-g)(1-b), and so on by swizzle).
   - Scale that term by ColorStrength per channel (higher strength = darker, more saturated colours). A second copy is scaled by Brightness.
   - Subtract the cross-channel swizzled versions of the strength-scaled term from the colour, then add the brightened term, then subtract one more swizzled term. This is the "color separation" step.
   - Mix the result with the original colour by Strength.
   - Saturation: mix each channel with the luma-like dot(color, 0.333) by Saturation. Note this is approximately one third of the channel sum, not an exact Rec.709 luma.
   - No final clamp is written; the backbuffer format clamps on write.

4. Parameters:
   - ColorStrength: float3 colour (default 0.2, 0.2, 0.2). Higher = darker and more intense.
   - Brightness: float 0.5..1.5, default 1.0.
   - Saturation: float 0..1.5, default 1.0.
   - Strength: float 0..1, default 1.0.

5. Passes: one pass, PostProcessVS -> TechnicolorPass, reads backbuffer, writes backbuffer.

6. Preprocessor macros: none.

7. Textures: backbuffer only.

8. Usefulness: reference only. Reason: a channel cross-product "look" is an artistic effect with no reusable structure beyond the swizzle pattern. Low priority for NFS:MW. ReShade-only: ReShade.fxh, PostProcessVS, __UNIFORM_COLOR_FLOAT3.

---

## 7. SunsetFilter.fx (Jacob Maximilian Fober)

1. Licence/credit: "SunsetFilter PS v1.0.1 (c) 2018 Jacob Maximilian Fober," / "This work is licensed under the Creative Commons" / "Attribution-ShareAlike 4.0 International License." Licence: CC BY-SA 4.0 (share-alike). Do not copy code; it would bring share-alike obligations into our repo.

2. Purpose: a linear gradient colour overlay (two colours, angle, position, sharpness) screen-blended onto the image, giving a sunset-style tint.

3. Algorithm:
   - Read backbuffer colour.
   - Aspect-correct uv (shifts y by the aspect ratio), then recentre to -1..1 and multiply by Scale (gradient sharpness).
   - Tilt vector from angle (negated, in radians): (sin a, cos a).
   - Mask value m = dot(tilt, centred uv) + Offset.
   - Remap m to a 0..1 ramp: m' = Overlay(m * 0.5 + 0.5). The "Overlay" here is a custom piecewise quadratic ease, not a standard blend. Checked points: 0 maps to 0, 0.5 maps to 0.5, 1 maps to 1 (a monotonic S-shape, my check).
   - Gradient colour = lerp(A, B, m') (or with m' inverted if Flip).
   - Output = Screen(source, gradient colour), i.e. 1 - (1-a)(1-b).

4. Parameters:
   - Colour (A): float3, default (1, 0, 0) red.
   - Colour (B): float3, default (0, 0, 0) black.
   - Color flip: bool, default false.
   - Angle: int -180..180, default -7.
   - Gradient sharpness: float 0.5..1.0 (step 0.005), default 1.0. Scales the centred coordinates (higher = steeper gradient).
   - Position: float 0..0.5, default 0. Offset of the gradient.

5. Passes: one pass, PostProcessVS -> SunsetFilterPS, reads backbuffer, writes backbuffer.

6. Preprocessor macros: __RESHADE__ (version check for the step option on Angle and Position).

7. Textures: backbuffer only.

8. Usefulness: none for direct porting (share-alike licence and a tint effect). Reason: a two-colour linear gradient screen blend is trivial to write from a spec if we ever want it. Note: ColorA uses __UNIFORM_COLOR_FLOAT3 and ColorB uses ui_type "color" (inconsistent, harmless). ReShade-only: ReShadeUI.fxh, ReShade::AspectRatio, __RESHADE__ macro, PostProcessVS, ui_category annotations.

---

## Cross-file notes

- Shared maths: Screen blend (1 - (1-a)(1-b)) appears in MagicBloom, SunsetFilter and Clarity's blend set. Overlay / soft / hard light appear in Clarity and SunsetFilter with different formulas. Write one canonical blend library for our WGSL, from the spec.
- Shared blur approach: Clarity uses a linear-sampled separable Gaussian with paired taps. MagicBloom uses a 9x9 2-D Gaussian per level. ArcaneBloom has a 13-tap table. For our rewrite, a separable pass with linear-sampled taps is the cheaper design; MagicBloom's pyramid of half-resolution levels is the bloom design.
- Shared luma: Clarity uses weights (0.327, 0.656, 0.016) (a different luma set). MagicBloom and ArcaneBloom use Rec.709 (0.2126, 0.7152, 0.0722). Lightroom uses HSL. Pick one luma definition for the rewrite and document it.
- Normalisation: MagicBloom's accum constant and ArcaneBloom's unnormalised 13-tap table both lose a few percent of gain. Compute kernel normalisation in code, not by hand.
- Adaptation: only MagicBloom adapts exposure. Its lerp per frame ignores frame time (framerate-dependent by design). Decide a delta-time version for the rewrite.
- Colour grading: Lightroom (LUT bake) and LiftGammaGain / Technicolor2 are the grading group. For the rewrite, one grade pass with a 3D LUT or direct math is enough. The LUT-bake idea from Lightroom is the one worth considering.
- Licence risk: Clarity2, Lightroom, ArcaneBloom, LiftGammaGain, Technicolor2 are unlicensed or "all rights reserved", so only ideas (described in our own words) may be used. MagicBloom (MIT) is permissive but we still write from spec. SunsetFilter is share-alike and must not be copied.
- Bugs and quirks found while reading (not fixed, read-only): Clarity2 hard-light and overlay branches (invalid redeclarations, RGB mode); Lightroom tint branch tests the temperature sign (line 783); MagicBloom 2-D kernel gain about 0.977 (accum constant); ArcaneBloom 13-tap weights sum to about 0.9705.
- Top three for our rewrite (ideas only): (1) MagicBloom's pyramid bloom plus mip-based adaptation; (2) Lightroom's LUT-bake pipeline; (3) Clarity's high-pass-plus-blend-mode mask as a local-contrast reference.
- Dependencies on ReShade APIs throughout: ReShade.fxh / ReShadeUI.fxh, ReShade::BackBuffer, PostProcessVS, tex2D/tex2Dlod, technique and pass syntax, ui_* annotations, __UNIFORM_* macros, pooled textures, MipLevels, source="..." texture loading, qUINT common (Lightroom), SV_VertexID triangles. None of these carry over to wgpu directly; the maths does.
