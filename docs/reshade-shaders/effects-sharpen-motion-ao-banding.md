# ReShade effects: sharpening, motion, depth, AO and banding

Part of [the ReShade shader pack survey](README.md). Read-only notes on 12 files from the local pack
(source folder not committed; nothing from it is copied into the repo). Licence tiers decide what may be
reused; see the [index](README.md#licences).

Licence tiers used below:
- Permissive (MIT-style text): CAS.fx, VRS_Map.fx, VRS_Map.fxh, Deband.fx
- Creative Commons NoDerivatives: GloomAO.fx
- Unclear / self-contradictory: FakeMotionBlur.fx
- None stated in file: UIDetect.fxh, qUINT_common.fxh, ArtisticVignette.fx, Vignette.fx, ChromaticAberration.fx, DPX.fx

Project rule reminder: learn ideas only; no copying of code from GPL, unlicensed or uncertain-licence files; write WGSL from a spec.

---

## CAS.fx

1. Licence/credit: "Copyright (c) 2017-2019 Advanced Micro Devices, Inc." followed by the standard MIT permission text ("Permission is hereby granted, free of charge..."). Ported to ReShade by SLSNe; later edits by Marty McFly, OopyDoopy, Lord of Lunacy, CeeJay.dk. Licence type: MIT (permissive, keep notice if code is copied).

2. Purpose: AMD FidelityFX Contrast Adaptive Sharpening, a cheap local sharpen that adapts strength per pixel to local contrast.

3. Algorithm:
   - Read a 3x3 neighbourhood a b c / d e f / g h i around the pixel e. On DX10+ the e, f, h, i samples come from three texture gather calls (R, G, B) at texcoord + 0.5 pixel; otherwise plain fetches.
   - Soft min and max per channel: min over the plus shape (d, e, f, b, h), then min over the corners (a, c, g, i) combined with it; the plus-shape result is added to itself (this doubles the value, the code's "2x" factor). Max is done the same way.
   - Headroom amplitude: amp = saturate( min(mn, 2 - mx) / mx ), where mx is the soft max. Then amp = 1/sqrt(amp) (rsqrt).
   - Peak = -3 * Contrast + 8. Negative lobe weight w = -1 / (amp * peak).
   - Normalise: rcpWeight = 1 / (4w + 1).
   - Filter (cross-shaped, weight w on the four neighbours, 1 on the centre): window = b + d + f + h; out = saturate( (window * w + e) * rcpWeight ).
   - Final = lerp(e, out, Sharpening).
   - Maths is linear-light: the input sampler decodes sRGB and the pass writes sRGB (per the author's fix note).
   - Key constants: 3, 8, 2 (headroom clamp), 4 (cross weight count), +1 (normalisation), 0.5 (gather offset).

4. Parameters:
   - Contrast (uniform float, "Contrast Adaptation", 0.0 to 1.0, default 0.0): higher values adapt to high-contrast areas more strongly (changes the peak).
   - Sharpening (uniform float, "Sharpening intensity", 0.0 to 1.0, default 1.0): blend between original and sharpened.

5. Passes: one pass. Vertex: PostProcessVS. Pixel: CASPass. Reads the back buffer (TexColor : COLOR, sRGB sampler). Writes the back buffer (SRGBWriteEnable = true). No intermediate textures.

6. Macros: __RENDERER__ (>= 0xa000 selects the gather path), BUFFER_RCP_WIDTH and BUFFER_RCP_HEIGHT (via ReShade.fxh), ReShade.fxh include.

7. Textures: none beyond the back buffer.

8. Usefulness: WORTH PORTING AS AN IDEA. MIT licence, small, well-known algorithm; a good post-process sharpen option for the rewrite. Reason: low cost, adaptive, and easy to express in WGSL from the description above (textureGather exists in WGSL, or 9 loads). ReShade-only parts: tex2Doffset, tex2Dgather*, __RENDERER__, SRGBWriteEnable, technique/pass syntax.

---

## FakeMotionBlur.fx

1. Licence/credit: "Copyright (C) 2015 Ganossa (mediehawk@gmail.com)". The text grants permission to use and sell, then the next paragraph says "Permission needs to be specifically granted by the author ... to copy, modify, merge, publish, distribute ...", which contradicts the first grant. Licence: UNCLEAR. Treat as not licensed for reuse.

2. Purpose: fake motion blur that smears the image along x and y in proportion to how much the frame changed since the previous frame, using a feedback buffer of previous outputs.

3. Algorithm:
   - diff = sum over RGB of |prevSingle - curr| * 2, then clamped to mbRecall (so diff is in [0, mbRecall]).
   - Eleven fixed weights (w0 = 0.082607 at the centre, falling to 0.005627 at tap 10). History sample starts as prev * w0.
   - Two blur radii: pixelBlur = softness * 13 * diff / width; pixelBlur2 = softness * 11 * diff / height.
   - For z = 1..10, add the history at +/- z*pixelBlur horizontally and +/- z*pixelBlur2 vertically, weighted by w[z]. Note that the x and y loops are summed, not multiplied.
   - Output = lerp(curr, prev, diff + 0.1). The 0.1 floor means even static frames take some history.
   - Net effect: history is a blurred feedback loop; more change means more smear.

4. Parameters:
   - mbRecall (uniform float, "Motion blur intensity", 0.0 to 1.0, default 0.40): clamp on diff, i.e. the strength of the effect.
   - mbSoftness (uniform float, "Blur strength of consequential streaks", 0.0 to 2.0, default 1.00): scales blur radius.

5. Passes:
   - CopyFrame: back buffer -> currTex (RGBA8 full size).
   - Combine: reads currTex, prevSingleTex, prevTex; writes back buffer.
   - PrevColor: MRT. prevSingleTex = currTex; prevTex = back buffer (this is the Combine output, so it is the previous output).

6. Macros: BUFFER_WIDTH, BUFFER_HEIGHT, BUFFER_RCP_WIDTH, BUFFER_RCP_HEIGHT; ReShadeUI.fxh and ReShade.fxh; [unroll] loop attribute.

7. Textures: currTex, prevSingleTex, prevTex (all RGBA8, back buffer size, created in the effect).

8. Usefulness: REFERENCE ONLY. Licence unclear, so no code or structure should be copied. The idea (temporal-difference-driven smear) is generic, but a proper motion-vector blur from the game's own velocity data would be better in the rewrite.

---

## UIDetect.fxh

1. Licence/credit: none stated.

2. Purpose: a header of hard-coded pixel coordinates and colours used to detect which game menu is on screen for "COD4:MW", 1920x1080 only (the file's own comment says so).

3. Algorithm: none in this file. It declares 8 sample points (PIXELNUMBER = 8) as static arrays:
   - UIPixelCoord: float3(x, y, uiId) with uiId 1 (TAB mission details), 2 (ESC menu), 3 (options/controls), 4 (main menu).
   - UIPixelRGB: the expected colour for each point, 0 to 255 (e.g. 255,204,102 for the gold text; 255,255,250 for the main-menu white).
   - A consumer would sample those pixels and match colours. Matching logic is not here.

4. Parameters: none.

5. Passes: none (header only).

6. Macros: PIXELNUMBER 8.

7. Textures: none.

8. Usefulness: NONE. Game-specific (COD4:MW coordinates, not NFS:MW) and trivial. Idea only if the rewrite ever needs HUD-state detection, and then it would be built from our own UI layout data.

---

## qUINT_common.fxh

1. Licence/credit: no licence text in this file. Changelog covers versions 2.0.0 to 2.0.6 and mentions qUINT (the upstream project is github.com/martymcmodding/qUINT, per the file's error message). Licence: none stated here. Check the upstream repo before any reuse.

2. Purpose: shared helper header for ReShade effects: depth-buffer access and linearisation, version guards, common screen constants and a fullscreen-triangle vertex shader.

3. Algorithm:
   - depthtex_uv: optional flip (1 - y), scale, and offset of sample coordinates (for games with mismatched depth buffers).
   - linearize_depth(d):
     - d *= RESHADE_DEPTH_MULTIPLIER (default 1).
     - If logarithmic: d = (exp(d * ln(C+1)) - 1) / C with C = 0.01.
     - If reversed: d = 1 - d (default reversed = 1).
     - Linear normalised depth: d = d / (Far - d * (Far - N)) with N = 1.0 and Far = RESHADE_DEPTH_LINEARIZATION_FAR_PLANE (default 1000); result saturated, so it is a 0 to 1 fraction of the far plane.
   - PostProcessVS: a single oversized triangle from SV_VertexID covering the screen.
   - Version checks use #error if the header and effect versions mismatch, and require ReShade 4.4+.

4. Parameters: none as user UI. Compile-time macros (see 6). Uniforms FRAME_TIME and FRAME_COUNT are provided (source "frametime" and "framecount"). With __RESHADE_FXC__ it provides UI_RESHADE_DEPTH_INPUT_IS_REVERSED as a checkbox for FreeStyle/Ansel.

5. Passes: none. Provides PostProcessVS and sampler/depth functions for other effects.

6. Macros: RESHADE_QUINT_COMMON_VERSION (206); RESHADE_DEPTH_INPUT_IS_UPSIDE_DOWN (0), _IS_REVERSED (1), _IS_LOGARITHMIC (0), _LINEARIZATION_FAR_PLANE (1000.0), _MULTIPLIER (1), _INPUT_X_SCALE and _Y_SCALE (1), _X_OFFSET and _Y_OFFSET (0), _X_PIXEL_OFFSET and _Y_PIXEL_OFFSET (0); __RESHADE__, __RESHADE_FXC__, BUFFER_WIDTH and friends.

7. Textures: BackBufferTex (COLOR), DepthBufferTex (DEPTH). These are declared here; other effects must not redeclare them.

8. Usefulness: REFERENCE. The linearisation formula and the depth-input flags are a useful checklist for how depth conventions differ between games, which matters if the rewrite reads depth. The ReShade plumbing itself is not needed. Licence must be confirmed before copying anything.

---

## GloomAO.fx

1. Licence/credit: "Overwatch & Code out side the work of people mention above is licenses under: Attribution-NoDerivatives 4.0 International" and "https://creativecommons.org/licenses/by-nd/4.0". Author: Jose Negrete AKA BlueSkyDefender (Depth3D), October 2021. The header credits Holden (normal from depth), Ritschel/Grosch/Seidel (SSDO paper), Jimenez (interleaved gradient noise), Wronski (Poisson samples), Sanchez (Wagner Poisson blur), Dang (adapted median), yvt.jp (TAA), and others. Licence type: CC BY-ND 4.0 (attribution required, NO derivatives). Any modified copy may not be distributed, so porting code is ruled out. Ideas from cited papers are fine.

2. Purpose: Screen-space directional occlusion (SSDO) ambient occlusion with a normal reconstruction from depth, an edge-aware denoiser and temporal accumulation. The file also contains the Depth3D on-screen help/watermark overlay, which is out of scope.

3. Algorithm (in pipeline order):
   - Depth linearisation (Depth_Info): optional barrel distortion, optional position/size correction, optional flip; then a Near/Far mapping. Near = 0.125 / Depth_Map_Adjust (default 7.5); Far = 1. Choice of DM0 (normal) or DM1 (reversed). Result is saturated linear depth (lerped with the raw value at 0.925).
   - Normals and colours (NormalsColorsDepth): 2x2 taps (centre, right, left, up, down). Picks the best horizontal and vertical neighbour by depth difference, builds a triangle from the three points, computes a cross-product normal, and blends with a fallback normal when they differ by >= 0.7. Normal is packed into RG16f with PackNormals: enc = n.xy * rsqrt(8 n.z + 8) + 0.5. Colour and an intensity mask (1 - 0.333 * luma-ish, against a threshold) go to texColorsSSDO.
   - SSDO (main loop): samples = clamp(SSDO_Levels, 1, 64), default 32. For each sample i: ray = pix * (radius / samples) * IGN * Rotate2D(POISSON[i], IGN) / D0, flipped to the hemisphere by the sign of dot with the normal. Fetch the neighbour's packed normal and depth at a mip level chosen by the sampling mode. Position from offset and depth difference. AO = smoothstep(0, 1, dot(normalise(pos), normal)). Range check r = max(0, range - |pos|) / range, applied squared. Accumulate (1 - local colour) * AO * r^2. Final SSDO = 1 - average.
   - Constants: Interleaved Gradient Noise magic (0.06711056, 0.00583715, 52.9829189); Poisson table of 64 points; attenuation 1e-5; radius default 2500 (in buffer-scaled units).
   - Denoise (CEAGD_H, then CEAGD_V): separable edge-aware Gaussian. Taps from -n/2 to n/2 with n = EvenSteven[SamplesXY], so the tap count is 2 * SamplesXY (default 3 gives 6 steps around the centre). Weight = gaussian(i, sqrt(n)) * exp(-depthDiff / 0.0125) * exp(-normalDiff / 0.5). Output lerped to centre where a depth mask (DM) is set. Pow(SSDO_Power) is applied in the H pass.
   - Temporal (TAA_SSDO): history blend factor Per = 1 - Persistence (default 0.125). Neighbourhood clamp in YUV (encodePalYuv: min and max of 8 neighbours), then mixRate = clamp(1 / (1/mix + 1) + clampAmount, 0.05, 0.5). Output in YUV decoded back.
   - AccumulatedFramesSSDO copies the back buffer to accuTexSSDO (history).
   - Composite (SSDOMixing and Composite): Blend_Mode is fixed at 3 (static), so the output is color * ssdo (multiply). Before that: optional Squared input (SSDO_X2 modes), HSL saturation (SSDO_Saturation), optional dither noise (MCNoise, scale 0.0225). Final lerp with the intensity mask.

4. Parameters (main UI names as in code; types and ranges from the file):
   - Sampling Quality (SSDO_MipSampling, combo 0 to 5, default 4): full / half / quarter mip levels, plus adaptive variants.
   - Samples (SSDO_Levels, int slider 0 to 64, default 32): number of rays per pixel.
   - Sampling Radius (SSDO_SampleRadius, float 1 to 5000, default 2500).
   - Texture Details (SSDO_2DTexture_Detail, 0 to 1, default 0; only exists if Force_Texture_Details = 1, otherwise a constant 0).
   - Near Details (NCD, float2 0 to 1, default (0.125, 0.0)): X is near-object detail, Y is weapon-hand range.
   - Depth Trimming (SSDO_Trimming, 0 to 0.5, default 0.1): range check width.
   - Depth Fade-Out (SSDO_Fade, -1 to 1, default 1.0; tooltip says 0.5): depth-based fade.
   - Square Input (SSDO_X2, combo 0 to 3, default 0): squares input or favours light or dark areas.
   - Total Power (SSDO_Power, 0 to 5, default 1.5).
   - Color Power (SSDO_ColorPower, 0 to 5, default 1.5): bounce colour intensity.
   - Saturation (SSDO_Saturation, 0 to 5, default 1.0).
   - Intensity Mask (SSDO_Intensity_Masking, 0 to 1, default 0): masks out bright light sources.
   - Depth Map Selection (Depth_Map, combo DM0/DM1, default 0).
   - Depth Map Adjustment (Depth_Map_Adjust, 1 to 500, default 7.5).
   - Depth Map Offset (Offset, -1 to 1, default 0).
   - Max Depth (SSDO_Max_Depth, 0.5 to 1, default 0.999): depth beyond which AO is skipped.
   - Depth Map Flip (Depth_Map_Flip, bool, default false).
   - Debug View (Debug, combo 0 to 2, default 0).
   - Denoise Power (SamplesXY, int 1 to 6, default 3).
   - TAA Power (Persistence, 0 to 1, default 0.125; 1 = off).
   - Dither SSDO (Dither_SSDO, bool, default true).
   - Hidden or fixed: Blend_Mode 3; HDR_Toggle 0; Horizontal/Vertical size and barrel-distortion values only if those macros are on.

5. Passes (from the technique, in order):
   - SSDO_Normals_Depth_Colors (MRT: texNormalsSSDO, texColorsSSDO, texDepthSSDO). Reads depth and back buffer. (Force_Texture_Details branch: SSDO_Colors then SSDO_Normals_Depth.)
   - SSDO: reads normals, depth and colours. Writes texSSDO (half res by default).
   - SSDO_EADFDDS (CEAGD_H): texSSDO -> texCEAGD_H_SSDO (half res).
   - SSDO_EADFDUS (CEAGD_V): texCEAGD_H_SSDO -> texCEAGD_V_SSDO (full res).
   - TAA_SSDO: reads texCEAGD_V and SSDOaccuFrames (history). Writes back buffer (no RenderTarget set, so the pass output lands in the back buffer).
   - AccumilateFrames: back buffer -> accuTexSSDO (history for next frame).
   - SSDO_Out (Out): reads accuTexSSDO and colour; writes back buffer. Uses text overlay only if the Depth3D triggers are on.

6. Macros (defined or checked in the file): Overwatch.fxh (optional include, via #if exists); OS; Force_Texture_Details (0); SSDO_Buffer_Size (2); DB_Size_Position (0); BD_Correction (0); Text_Info_Key (93); Disable_Debug_Info; Minimize_Web_Info; Compatibility; HDR_Toggle (0); DC; SP; LBC; LB_Correction; PI; pix; Scale; Alternate; and many trigger macros (RE, NC, RH, NP, ID, SP, DC, HM, DF, NF, DS, LBC, LBM, DA, NW, PE, FV, ED all 0). ReShade.fxh and ReShadeUI.fxh (via the usual includes) are assumed.

7. Textures (all created in the effect):
   - accuTexSSDO: RGBA8, full size, 4 mips (temporal history).
   - texNormalsSSDO: RG16f, full size, 4 mips.
   - texColorsSSDO: RGBA8, full size, 8 mips.
   - texDepthSSDO: R32f, full size, 4 mips.
   - texSSDO: RGBA8, buffer / SSDO_Buffer_Size (default half), 2 mips.
   - texCEAGD_H_SSDO: RGBA8, half size.
   - texCEAGD_V_SSDO: RGBA8, full size.
   - Reads game textures: BackBufferTex (COLOR), DepthBufferTex (DEPTH).

8. Usefulness: REFERENCE ONLY. CC BY-ND forbids distributing derivatives, so no copying and no adaptation of this file. The ideas (normal reconstruction from depth, half-res AO with edge-aware denoise, temporal clamp in YUV, hemisphere-directed sampling) are described in the cited papers and can be implemented from those papers by our own team. Reason for reference rather than port: licence, plus large amount of ReShade-only plumbing and Depth3D-specific overlays.

---

## VRS_Map.fx

1. Licence/credit: "Copyright (c) 2020 Advanced Micro Devices, Inc." with the standard MIT permission text. Port and framework by Lord of Lunacy (github.com/LordOfLunacy/Insane-Shaders). Optical-flow method from Jose Negrete (BlueSkyDefender). Licence type: MIT. Derived from AMD FidelityFX-VariableShading.

2. Purpose: generate a per-tile variable-rate-shading (VRS) map. For each 8x8 tile, decide whether it can be shaded at 1x1 or 2x2 based on local luminance variance, and optionally reduce variance on fast-moving pixels. A debug overlay shows the result.

3. Algorithm (compute pass, one dispatch thread per pixel grouped into 8x8 tiles):
   - Take 2x2 luminance blocks (lum = dot(rgb, (0.299, 0.587, 0.114))) across the tile plus a 1-block border (sample count = TILE + 2 per axis).
   - For each 2x2 block: delta.x = max of horizontal differences, delta.y = max of vertical differences, delta.z = max - min (2x2 range).
   - Motion (only if optical flow is on): v = |motionVec| * g_MotionFactor, subtracted from deltas and from max.
   - Combine with neighbours (burn-in guard): dMin = max(0, own min - neighbour min), dMax = max(0, neighbour max - own max); delta = max(0, delta + dMin + dMax) over the 4-neighbourhood in LDS.
   - Reduction: per 2x2 sub-block within the tile take the maximum delta (atomicMax on an order-preserving int encoding of floats).
   - Decision per sub-block: if var (2x2 range) < cutoff: rate 2x2 (both axes). Else if varH > varV: 1x1 horizontal, and 2x1 or 1x1 vertical depending on whether varV > cutoff; else the reverse. (Only 1x and 2x steps are implemented; 4x codes exist in the header but are not produced.)
   - Shading-rate code = (x << 2) | y with RATE1D values 0 (1x), 1 (2x), 2 (4x). So 1x1 = 0, 1x2 = 1, 2x1 = 4, 2x2 = 5.
   - Output texel stores float4(varH, varV, var) * 4 in rgb and rate/255 in alpha.
   - Motion proxy (VRS_ReadMotionVec2D): not a true motion vector. It uses the linearised depth change between two frames (ping-pong R8 depth buffers), S_Velocity = 12.5 * lerp(1, 80, 0.5) = 506.25, V = saturate(|depth diff| * S). Output as a 1-channel "velocity" in x.
   - Liveness check: VRSUpdated (1x1 R32f) stores the frame count; consumers only use the map when its stored frame is current or one behind.

4. Parameters:
   - Variance Cutoff (g_VarianceCutoff, float slider 0 to 0.1, step 0.001, default 0.05): max luminance variance allowed for a reduced rate.
   - Motion Factor (g_MotionFactor, float slider 0 to 0.1, step 0.001, default 0.05): only with optical flow on; otherwise a constant 0.
   - Show Overlay (ShowOverlay, bool, default true): draw the debug overlay.
   - Compile-time switch VRS_USE_OPTICAL_FLOW (default 0) and _VRS_COMPUTE (auto-set from renderer version and ReShade >= 4.8.2).

5. Passes:
   - Pass 1 (compute, VRS_GenerateVrsImage, 8x8 threads): reads back buffer (sBackBuffer) and depth ping-pong (optional). Writes VRS (storage) and VRSUpdated (storage, frame count).
   - Pass 2 (graphics, mainVS / mainPS): reads back buffer and VRS. Writes an overlay colour per shading rate, with a 35 percent blend and a grid line every 8 pixels.

6. Macros (VRS_Map.fx and .fxh): __SUPPORTED_VRS_MAP_COMPATIBILITY__ (11), __VRS_MAP_COMPATIBILITY_VERSION__ (11), VRS_MAP, _VRS_INCOMPATIBLE, _VRS_COMPUTE, VRS_USE_OPTICAL_FLOW, TILE_SIZE (8), DIVIDE_ROUNDING_UP, VRS_IMAGE_SIZE, THREAD_GROUPS, VRS_LAST_UPDATE, VRS_IS_UPDATED, VRS_MAKE_SHADING_RATE, RENDERER, __RENDERER__ (>= 0xb000 and < 0x10000, or >= 0x14300, selects compute), __RESHADE__ >= 40802. Also the 'exists' include test.

7. Textures:
   - VRS: RGBA8, size ceil(W/8) x ceil(H/8).
   - VRSUpdated: R32f, 1x1.
   - VRS_Depth0 and VRS_Depth1: R8, full size (only with optical flow).
   - BackBuffer: game colour (read).

8. Usefulness: REFERENCE, with a small idea worth keeping. Rate tiers and the variance rule are generic. The rewrite targets wgpu, which does not expose variable-rate shading, so the GPU feature itself is not usable. The idea (a per-tile variance mask to drive cheap work or a quality decision) could be reused in our own code. Licence is MIT if we ever port, with the notice kept.

---

## VRS_Map.fxh

1. Licence/credit: same AMD 2020 MIT text and header as VRS_Map.fx, plus the same Lord of Lunacy and BlueSkyDefender credits.

2. Purpose: common header that other effects include to read the VRS map (or fall back when it is unavailable). Defines the constants, the compatibility check and helper functions for shading rate and debug colouring.

3. Algorithm:
   - Compatibility: include guard checks for version 11, sets _VRS_INCOMPATIBLE.
   - Gating: _VRS_COMPUTE = 1 if the renderer is compute-capable and ReShade >= 4.8.2, else 0. If 0 or incompatible, the VRS_Map::ShadingRate functions just return the off rate (0 or the provided default).
   - ShadingRate(texcoord, VarianceCutoff, UseVRS, offState): if the map is not current or UseVRS is false, return offState. Otherwise read the map texel at texcoord * VRS_IMAGE_SIZE, divide the stored variances by 4, and apply the same rule as the compute pass (var < cutoff gives 2x2; else pick per axis).
   - DebugImage: colour-codes each rate (1x1 dark red, 1x2 dark yellow, 2x1 brown, 2x2 green, 2x4 blue-ish, 4x2 purple, 4x4 teal), blends 35 percent with the original, and draws a 1-pixel grid (half brightness) every TILE_SIZE pixels.
   - Deprecated overloads (no offState) keep old callers working.

4. Parameters: none as UI (the VRS_Map.fx file owns the sliders).

5. Passes: none.

6. Macros: the list in VRS_Map.fx section 6, plus the RATE1D constants and VRS_MAKE_SHADING_RATE(x,y) = ((x << 2) | y).

7. Textures: VRS (RGBA8), VRSUpdated (R32f), sVRS and sVRSUpdated samplers. Also uniform VRS_FrameCount (framecount source).

8. Usefulness: REFERENCE, same as VRS_Map.fx. The rate-encoding constants and the decision rule are the reusable ideas. MIT licence if copied, with notice.

---

## ArtisticVignette.fx

1. Licence/credit: none stated (no author or licence header; only the ReShade technique tooltip and UI text).

2. Purpose: a configurable vignette overlay with multiple shapes and Photoshop-style blend modes, using a colour with an alpha channel as strength.

3. Algorithm:
   - Read the back buffer colour.
   - Aspect correction: ratio = (aspect > 1) ? (W/H, 1) : (1, H/W). ratio_uv = (uv - 0.5) * ratio + 0.5; uv = lerp(uv, ratio_uv, VignetteRatio). So VignetteRatio 0 is anamorphic (stretched), 1 is corrected (round).
   - Shape distance (switch): None: 1. Radial: distance(0.5, uv) * HalfPi. Top/Bottom: |y - 0.5| * 2. Left/Right: |x - 0.5| * 2. Box: max(|x - 0.5|, |y - 0.5|) * 2. Sky: distance to (0.5, 1). Ground: distance to (0.5, 0).
   - Gradient: vignette = smoothstep(start, end, distance).
   - Blend colour vig_color by mode: Mix (constant colour), Multiply (c * v), Darken (min), Lighten (max), Overlay (base < 0.5 ? 2cv : 1 - 2(1-c)(1-v)), Screen (1 - (1-c)(1-v)), Hard Light (base > 0.5 ? 2cv : 1 - 2(1-c)(1-v)), Soft Light ((1 - 2v) c^2 + 2 v c).
   - Final: color = lerp(color, vig_color, vignette * alpha).
   - Note: the Hard Light branch tests the image pixel, not the overlay colour, so it is not the textbook hard light. Treat as this file's own variant.

4. Parameters:
   - Color (VignetteColor, float4, default (0, 0, 0, 1)): RGB colour; alpha = strength.
   - Blending Mode (BlendMode, combo: Mix, Multiply, Darken Only, Lighten Only, Overlay, Screen, Hard Light, Soft Light; default 0 = Mix).
   - Start/End (VignetteStartEnd, float2, 0 to 3, step 0.01, default (0, 1)): gradient range.
   - Ratio (VignetteRatio, float, 0 to 1, default 0.0).
   - Shape (VignetteShape, combo: None, Radial, Top/Bottom, Left/Right, Box, Sky, Ground; default 1 = Radial).
   - Help text (_Help): UI only.

5. Passes: one pass. Vertex: PostProcessVS. Pixel: MainPS. Reads back buffer, writes back buffer.

6. Macros: ReShadeUI.fxh uniform macros (__UNIFORM_COLOR_FLOAT4, __UNIFORM_COMBO_INT1, __UNIFORM_DRAG_FLOAT2, __UNIFORM_SLIDER_FLOAT1); BUFFER_WIDTH, BUFFER_HEIGHT, BUFFER_RCP_WIDTH, BUFFER_RCP_HEIGHT; ReShade::AspectRatio and ReShade::BackBuffer.

7. Textures: none beyond the back buffer.

8. Usefulness: WORTH USING AS AN IDEA (own implementation). No licence, so do not copy the file; the maths is simple and can be written from this description. Useful as a lens-vignette option for the rewrite.

---

## Vignette.fx

1. Licence/credit: "Vignette version 1.3 by Christian Cann Schuldt Jensen ~ CeeJay.dk". No licence text. Licence: none stated.

2. Purpose: darkens the screen edges to mimic a camera lens. Seven variants.

3. Algorithm (single pass; the if-blocks are independent, only one matches Type):
   - Type 0 (Original, radial): d = (uv - Center) * (aspect factor, Ratio) / Radius; dist = dot(d, d); colour *= 1 + pow(dist, Slope * 0.5) * Amount.
   - Type 1 ("New round"): per axis t = -t^2 + t; colour *= saturate( (k * Ratio * x + y) * 4 ), k = (H/W)^2.
   - Type 2 ("TV style"): t = -t^2 + t per axis; colour *= saturate(x * y * 100).
   - Types 3 and 4 (Untitled 1 and 2): t = |uv - 0.5|; a dot-product of t terms emulating a corner test; tc = saturate(tc - 0.495) (type 4 subtracts an extra 0.0002); colour *= (1 - 200 tc)^4 + 0.25 (type 3) or + 0.0 (type 4).
   - Type 5 (Untitled 3): as type 3 but using a multiply-add form of the same test; colour *= (1 - 200 tc)^4 + 0.25 with the sign reversed inside.
   - Type 6 (Untitled 4): tex_xy = sum of (-t^2 + t) per axis; colour *= saturate(4 * tex_xy).

4. Parameters:
   - Type (combo: Original, New, TV style, Untitled 1 to 4; default 0).
   - Ratio (float, 0.15 to 6.0, default 1.0): width to height ratio of the radial shape.
   - Radius (float, -1 to 3, default 2.0): lower means stronger from centre.
   - Amount (float, -2 to 1, default -1.0): strength; negative darkens.
   - Slope (int, 2 to 16, default 2): steepness; odd values cost more FPS (per tooltip).
   - Center (float2, 0 to 1, default (0.5, 0.5)): only used by Original.

5. Passes: one pass. Vertex: PostProcessVS. Pixel: VignettePass. Reads and writes back buffer.

6. Macros: ReShadeUI.fxh and ReShade.fxh; BUFFER_RCP_WIDTH and BUFFER_RCP_HEIGHT.

7. Textures: none.

8. Usefulness: IDEA ONLY. No licence, and the same effect as ArtisticVignette in simpler form. If we want a vignette, write one from the described maths. Not worth porting code.

---

## ChromaticAberration.fx

1. Licence/credit: "Chromatic Aberration by Christian Cann Schuldt Jensen ~ CeeJay.dk". No licence text. Licence: none stated.

2. Purpose: shifts the red and blue channels in opposite directions to imitate a cheap lens or sensor.

3. Algorithm: read the original colour. Red is sampled at uv + pixelSize * Shift; green is kept; blue at uv - pixelSize * Shift. Output = lerp(original, shifted, Strength).

4. Parameters:
   - Shift (float2, -10 to 10 pixels, default (2.5, -0.5)): offset in pixels for the R channel (B gets the negative). Fractional values give a soft look.
   - Strength (float, 0 to 1, default 0.5).

5. Passes: one pass. Vertex: PostProcessVS. Pixel: ChromaticAberrationPass. Reads and writes back buffer.

6. Macros: BUFFER_PIXEL_SIZE; ReShadeUI.fxh and ReShade.fxh.

7. Textures: none.

8. Usefulness: IDEA ONLY, and trivial to write ourselves. No licence. Low value unless the rewrite wants a lens effect. Nothing to port.

---

## Deband.fx

1. Licence/credit: "Deband shader by haasn", source https://github.com/haasn/gentoo-conf (mpv deband-pre.glsl). "Copyright (c) 2015 Niklas Haas" with the MIT permission text. ReShade port and optimisation by JPulowski. The header also says "Do not distribute without giving credit to the original author(s)." Licence type: MIT, with an attribution request. Keep the notice if code is copied.

2. Purpose: removes colour banding in smooth gradients by averaging pixels from random nearby directions when they look like the same gradient, then adds a dither to hide the remaining steps.

3. Algorithm:
   - Thresholds (per preset, in 0 to 255 units, then divided by 255):
     - Low: avg 0.6, max 1.9, mid 1.2.
     - Medium: avg 1.8, max 4.0, mid 2.0.
     - High: avg 3.4, max 6.8, mid 3.3.
     - Custom: the three custom sliders.
   - Random seed h = permute(permute(permute(x) + y) + drandom / 32767), where permute(x) = ((34x + 1) x) mod 289.
   - Random angle dir = rand(permute(h)) * 2pi, where rand(x) = frac(x / 41). Direction o = (cos dir, sin dir).
   - For i = 1 to iterations (1 to 4):
     - Radius dist = rand(h) * range * i (range default 24); offset pt = dist * pixel size.
     - Four reference samples at ±o and ±perp(o) (SE, NW, NE, SW).
     - ref_avg = mean of four. ref_avg_diff = |ori - ref_avg|. ref_max_diff = max over the four |ori - ref|. ref_mid_diff1 = |(SE + NW)/2 - ori|. ref_mid_diff2 = |(NE + SW)/2 - ori|.
     - Thresholds scaled by i: avg*i, max*i, mid*i.
     - factor = pow( Π over the four diffs of saturate(3 (1 - diff / threshold)), 0.1 ).
     - res = lerp(ori, ref_avg, factor). Then h = permute(h).
   - Ordered dither (after the loop): grid = frac(dot(uv, screenSize * (1/16, 10/36) + 0.25)); shift = 0.25 / (2^8 - 1) per channel with alternating sign, lerped by grid position (times 2), added to res.
   - Debug view: returns ref_avg (the blurred low-pass result).
   - Observed quirk: each iteration reads from ori and uv only, so iterations do not compound; the final res is from the last iteration. Check this before relying on iterations > 1.

4. Parameters:
   - Debanding strength (threshold_preset, combo Low, Medium, High, Custom; default 0 = Low).
   - Initial radius (range, float 1 to 32, step 1, default 24).
   - Iterations (iterations, int 1 to 4, default 1).
   - Average threshold (custom_avgdiff, 0 to 255, step 0.1, default 1.8, Advanced).
   - Maximum threshold (custom_maxdiff, 0 to 255, step 0.1, default 4.0, Advanced).
   - Middle threshold (custom_middiff, 0 to 255, step 0.1, default 2.0, Advanced).
   - Debug view (debug_output, bool, default false, Advanced).
   - Hidden: drandom (random source, 0 to 32767) for per-frame seed.

5. Passes: one pass. Vertex: PostProcessVS. Pixel: PS_Deband. Reads back buffer (ReShade::BackBuffer via tex2Dlod); writes back buffer.

6. Macros: BUFFER_PIXEL_SIZE, BUFFER_SCREEN_SIZE; ReShadeUI.fxh, ReShade.fxh; dither_bit = 8.0 (local const).

7. Textures: none beyond the back buffer.

8. Usefulness: WORTH PORTING AS AN IDEA. The algorithm is MIT and documented in the file. For a 2005 game with gradient banding (sky, fog, HUD glows), a deband pass plus ordered dither is a concrete improvement. Reimplement from the description, keep the attribution, and do not copy the GLSL-derived structure. Reason: solves a real problem on our target content.

---

## DPX.fx

1. Licence/credit: "DPX/Cineon shader by Loadus". No licence text. Licence: none stated.

2. Purpose: film-look grade that mimics a Cineon/DPX log-curve response and adds colourfulness and saturation.

3. Algorithm (single pass):
   - Input colour in, then a contrast pull toward mid grey: B = B (1 - Contrast) + 0.5 Contrast.
   - Per-channel sigmoid S-curve with curve steepness RGB_Curve and centre RGB_C: Btemp = 1 / (1 + exp(Curve / 2)); B = [ 1 / (1 + exp(-Curve (B - C))) ] / (1 - 2 Btemp) + (-Btemp) / (1 - 2 Btemp). This normalises the sigmoid so 0 maps to 0 and 1 to about 1.
   - Chroma: value = max(B.r, B.g, B.b); colour = B / value; colour = pow(|colour|, 1 / Colorfulness); rebuild c0 = colour * value.
   - Saturation pass: c0 = mul(XYZ matrix, c0); luma = dot(c0, (0.30, 0.59, 0.11)); c0 = (1 - Saturation) luma + Saturation c0; c0 = mul(RGB matrix, c0). The matrices are given as constants (a 3x3 XYZ-style forward matrix and a second 3x3 inverse-style matrix). The saturation is done in the transformed space, which is unusual.
   - Output lerp(input, c0, Strength).

4. Parameters:
   - RGB Curve (RGB_Curve, float3, 1 to 15, default (8, 8, 8)).
   - RGB C (RGB_C, float3, 0.2 to 0.5, default (0.36, 0.36, 0.34)).
   - Contrast (float, 0 to 1, default 0.1).
   - Saturation (float, 0 to 8, default 3.0).
   - Colorfulness (float, 0.1 to 2.5, default 2.5): exponent on chroma.
   - Strength (float, 0 to 1, default 0.20, tooltip "Adjust the strength of the effect").

5. Passes: one pass. Vertex: PostProcessVS. Pixel: DPXPass. Reads and writes back buffer.

6. Macros: ReShadeUI.fxh and ReShade.fxh; no custom defines.

7. Textures: none.

8. Usefulness: REFERENCE ONLY. No licence, so no copying. The idea of a normalised per-channel S-curve plus a chroma-preserving max split is generic and could be written from this description if we want a grade option. Low priority.

---

## Cross-file notes

- Dependencies: almost every file includes ReShade.fxh and/or ReShadeUI.fxh and uses PostProcessVS (declared in qUINT_common.fxh and in ReShade.fxh; the duplicate copies are the same idea). Entry points are techniques with pass blocks, tex2D and tex2Dlod sampling, and ReShade's BackBuffer. None of this maps directly to wgpu; the portable part is the maths.
- Licence summary for porting decisions:
  - Permissive (MIT-style, keep notices): CAS.fx, VRS_Map.fx, VRS_Map.fxh, Deband.fx.
  - No derivatives (CC BY-ND 4.0): GloomAO.fx. Ideas from cited papers only.
  - Unclear: FakeMotionBlur.fx. Do not copy.
  - None stated: UIDetect.fxh, qUINT_common.fxh, ArtisticVignette.fx, Vignette.fx, ChromaticAberration.fx, DPX.fx. Write from description only, and for qUINT check upstream licence first.
- Recommended from this group:
  1. CAS.fx: contrast-adaptive sharpen, MIT, cheap, easy to re-derive in WGSL.
  2. Deband.fx: banding removal plus ordered dither, MIT, directly relevant to gradients in a 2005 engine. Note the iteration quirk.
  3. VRS_Map.fx: variance-to-rate rule and encoding are a useful idea, but the GPU feature is not available in wgpu, so low priority.
- Game-specific content: UIDetect.fxh is COD4:MW-specific and should not be used. The Depth3D overlay in GloomAO.fx and the help text in ArtisticVignette.fx are UI only.
- Overlap: Vignette.fx and ArtisticVignette.fx do the same job; ChromaticAberration.fx and DPX.fx are simple per-pixel maths. Writing one small vignette and one CA pass from spec would cover all four.
- Depth conventions (qUINT_common.fxh, GloomAO.fx): if the rewrite reads depth, check reversed-Z and far-plane handling in our own renderer. The linearisation formula in qUINT is a reference point, not a source to copy.
- Nothing in this group reads NFS:MW game data, and nothing is a source of assets. Nothing in this group should be committed to the project repo.
