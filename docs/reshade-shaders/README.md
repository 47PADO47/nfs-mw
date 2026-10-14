# ReShade shader pack: survey and ideas

A survey of a local ReShade/GShade effect pack (33 `.fx` and `.fxh` files plus a texture folder), read in full
for what it does, what is worth taking, and what may be reused. It is not NFS:MW-specific. The pack is
for ReShade's D3D9/10/11 hooks, so none of it runs in this renderer as-is. The useful parts are algorithms and
ideas, and every one needs a written spec and our own WGSL before it goes in.

Related: [ReShade and injected effects](../reshade.md) (how an external ReShade setup meets this renderer),
[post-processing](../post-processing.md) (what the renderer already ships) and the
[CAS spec](../specs/cas-sharpening.md), the one piece of this pack that has a written spec.

## Licences

The rule from [licensing.md](../licensing.md): we may learn from any file, but only permissively licensed code
may be copied, and even then we write WGSL from a spec.

| Tier | Files | What we may do |
|---|---|---|
| Permissive (MIT) | `CAS.fx`, `VRS_Map.fx`, `VRS_Map.fxh`, `Deband.fx`, `MagicBloom.fx`, `ACES.fxh` | Re-derive from a spec, keep the notice and credit in NOTICE |
| Attribution, no derivatives | `GloomAO.fx` (CC BY-ND 4.0) | Ideas from the cited papers only |
| Share-alike | `SunsetFilter.fx` (CC BY-SA 4.0) | Do not copy |
| Self-contradictory licence | `FakeMotionBlur.fx` | Do not copy |
| Vendor copyright | `FXAA.fxh` (NVIDIA) | Read for the published algorithm, copy nothing |
| All rights reserved | `qUINT_lightroom.fx` | Do not copy |
| None stated | the rest (about 20 files, incl. the PD80 helpers, `Clarity2`, `ArcaneBloom`, `LiftGammaGain`, `Technicolor2`, `DPX`, the vignettes, `ChromaticAberration`, `UIDetect`, `qUINT_common`, `ReShade*.fxh`, `Macros`, `Blending`, `Dithering`, `TriDither`) | Write from a description only |

The textures have no stated licence either, so none is shipped or copied. The shader pack itself is not committed.

## Ideas worth taking

Ranked by value to this renderer. Each row says what the repo already has, from a grep of the docs and the
post shaders. Only "no" and "partly" rows are new work.

| # | Idea | Source | Already in the repo? | Next step |
|---|---|---|---|---|
| 1 | **Banding removal and dither at the final 8-bit output** (blue-noise or triangular dither, gradient-aware debanding) | `Deband.fx`, `TriDither.fxh`, `Dithering.fxh` | No deband or dither pass found in the post docs or shaders | Spec a dither step in the resolve pass; the 2005 sky, fog and HUD glows band |
| 2 | **Contrast-adaptive sharpening** | `CAS.fx` | Partly: RCAS runs only after FSR 1 upscaling below 100 % render scale | [CAS spec](../specs/cas-sharpening.md), written. Do not run it together with RCAS |
| 3 | **Check our FXAA against the published recipe**: luma stored in alpha (perceptual, gamma-space), run after tone mapping on LDR data, threshold ranges | `FXAA.fxh` (algorithm only) | Partly: [post-processing.md](../post-processing.md) names FXAA 3.11 but not these rules | Read our FXAA shader against the list in [fxaa-reference.md](fxaa-reference.md) |
| 4 | **Bloom pyramid with mip-based exposure adaptation** | `MagicBloom.fx` | Bloom pyramid yes; adaptation no | Spec an adaptation option. Decide a frame-time-based lerp (MagicBloom's is per frame) |
| 5 | **3D LUT grade**: bake a grade once over an identity cube, then one lookup per pixel; tiled-atlas lookup with luma and chroma split | `qUINT_lightroom.fx` (idea only), `PD80_LUT_v2.fxh` | No LUT grade pass; `visual-treatment.md` is a fixed colour treatment | Worth it if we want user looks. Spec from the description |
| 6 | **Temporal motion blur** from frame differences | `FakeMotionBlur.fx` | No | Prefer real per-object velocity from the game's data; the pack's idea is a fallback only |
| 7 | **Depth linearisation checklist** (reversed, logarithmic, upside-down, far plane) | `qUINT_common.fxh`, `ReShade.fxh` | Yes, in [reshade.md](../reshade.md) | Nothing new |
| 8 | **Screen-space AO with normal reconstruction from depth** | `GloomAO.fx` (papers only) | Yes, GTAO in [ambient-occlusion.md](../ambient-occlusion.md) | Nothing new; do not port GloomAO |
| 9 | **Vibrance weighted by (max - min)**, asymmetric negative range | `PD80_00_Base_Effects.fxh` | No | Small; fold into any saturation control |
| 10 | **Standard fitted ACES curve** | `ACES.fxh` (MIT), the Narkowicz fit | Yes, [post-processing.md](../post-processing.md) | Nothing new |
| 11 | **Screen-space vignette and chromatic aberration** | `ArtisticVignette.fx`, `Vignette.fx`, `ChromaticAberration.fx` | No | Write from the description if a lens look is wanted |
| 12 | **Blend-mode library** | `PD80_00_Blend_Modes.fxh`, `Blending.fxh` | No | Only with a feature that needs it (a grade or HUD compositing) |

Low value for us: `VRS_Map` (wgpu does not expose variable-rate shading), `UIDetect` (CoD4 coordinates),
`Macros.fxh` and `ReShadeUI.fxh` (ReShade UI macros), `Clarity2`'s local contrast (its code has invalid branches,
and the idea is covered by the bloom and sharpening rows).

## Pack parts that reference their own dependencies

Some files depend on files that are not in the folder: `Dithering.fxh` (needs `Common.fxh`), `GloomAO.fx`
(needs `Overwatch.fxh`), `FXAA.fxh` (needs `Fxaa3_11.h`). They cannot be compiled from this folder alone, which is
one more reason to write from the description. The include graph and the texture list are in
[shared-headers-and-textures.md](shared-headers-and-textures.md).

## Coverage: where each file is documented

| Document | Files |
|---|---|
| [effects-sharpen-motion-ao-banding.md](effects-sharpen-motion-ao-banding.md) | `CAS.fx`, `FakeMotionBlur.fx`, `UIDetect.fxh`, `qUINT_common.fxh`, `GloomAO.fx`, `VRS_Map.fx`, `VRS_Map.fxh`, `ArtisticVignette.fx`, `Vignette.fx`, `ChromaticAberration.fx`, `Deband.fx`, `DPX.fx` |
| [effects-bloom-grade-clarity.md](effects-bloom-grade-clarity.md) | `Clarity2.fx`, `qUINT_lightroom.fx`, `MagicBloom.fx`, `ArcaneBloom.fxh`, `LiftGammaGain.fx`, `Technicolor2.fx`, `SunsetFilter.fx` |
| [shared-headers-and-textures.md](shared-headers-and-textures.md) | `PD80_00_Base_Effects.fxh`, `PD80_00_Blend_Modes.fxh`, `PD80_00_Color_Spaces.fxh`, `PD80_00_Noise_Samplers.fxh`, `PD80_02_Bonus_LUT_pack.fx`, `PD80_LUT_v2.fxh`, `ACES.fxh`, `Dithering.fxh`, `TriDither.fxh`, `ReShade.fxh`, `ReShadeUI.fxh`, `Blending.fxh`, `Macros.fxh`, and the texture folder |
| [fxaa-reference.md](fxaa-reference.md) | `FXAA.fxh` |
| [CAS spec](../specs/cas-sharpening.md) | the sharpening design from `CAS.fx` |

## Open questions

- Where does the final resolve clamp to 8 bits, and does it dither? That decides whether row 1 needs a new pass
  or a change to the existing one.
- Does our FXAA meet the luma and tone-mapping rules in row 3? Not checked yet.
- Which looks do users want: a fixed grade, or a LUT slot? That decides row 5.
