# SweetFX

SweetFX is a suite of post-process image effects (anti-aliasing, sharpening, colour grading, film and retro looks) written as ReShade shader files (`.fx`, `.fxh`). It is not a standalone injector and has no ASI plugin or game-memory code. It runs inside ReShade, which does the injection and the DirectX hooking. This page records what the repository states and what its shader source shows, so that nfs-mw can decide whether to re-specify any of these effects in its own renderer.

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/CeeJayDK/SweetFX |
| Author | CeeJayDK (Christian Cann Schuldt Jensen, CeeJay.dk) [README] |
| Type | ReShade shader suite (post-process shaders, not an ASI plugin or tool) [README][source] |
| Licence | MIT, copyright 2014 CeeJayDK [source: LICENSE] |
| Language | Shader source (HLSL-style `.fx` / `.fxh`). The GitHub language bar was empty in the captured page. [inferred from file types] |
| Target game build | None stated. The README says the suite works with ReShade in general; no game or version is named. |
| Latest visible activity | Not shown in the captured page. The page reported 34 commits, 270 stars, 42 forks, 13 watchers. The "Latest commit" message and date were not visible. |
| Repository contents | `Shaders/SweetFX/` (30 files), `Textures/SweetFX/`, `LICENSE`, `README.md` [source] |

## What it does

Each item lists the effect, the source file that implements it, and the evidence tag.

1. **SMAA anti-aliasing**. File `Shaders/SweetFX/SMAA.fx` and `SMAA.fxh`. Credited to Jimenez et al. (2011). [README][source]
2. **FXAA anti-aliasing**. File `FXAA.fx` and `FXAA.fxh`. Credited to Timothy Lottes (Nvidia). [README][source]
3. **AMD FidelityFX Contrast Adaptive Sharpening**. File `CAS.fx`. Credited to AMD, ported as a ReShade effect. [README][source]
4. **LumaSharpen**. File `LumaSharpen.fx` (version 1.5.0 in its header). Luma-weighted unsharp-mask style sharpening. [README][source]
5. **Levels**. File `Levels.fx`. Black and white point adjustment, may clip. [README][source]
6. **Curves**. File `Curves.fx`. S-curve contrast without clipping. [README]
7. **Vibrance**. File `Vibrance.fx`. Saturates or desaturates according to existing saturation. [README]
8. **Lift Gamma Gain**. File `LiftGammaGain.fx`. Shadow, midtone and highlight control. [README]
9. **Tonemap**. File `Tonemap.fx`. Gamma, exposure, saturation, bleach, defog; may clip. [README]
10. **HDR (FakeHDR)**. File `FakeHDR.fx`. Tonemapped HDR-like look. [README][source]
11. **Technicolor**. Files `Technicolor.fx` and `Technicolor2.fx`. Three-strip film process look. [README][source]
12. **DPX**. File `DPX.fx`. Cineon DPX film look, can give a warm tone. [README][source]
13. **Monochrome**. File `Monochrome.fx`. Black and white film look. [README]
14. **Sepia**. File `Sepia.fx`. [README][source]
15. **Advanced CRT**. File `CRT.fx`. Arcade CRT look. [README][source]
16. **Nostalgia**. File `Nostalgia.fx`. Old computer or console look. [README][source]
17. **Cartoon**. File `Cartoon.fx`. Outline effect based on the Dolphin Auto Toon shader. [README][source]
18. **Vignette**. File `Vignette.fx`. [README][source]
19. **Border**. File `Border.fx`. Black screen edge or custom letterbox. [README][source]
20. **Layer**. File `Layer.fx`. Overlays an image onto the game. [README][source]

Source files in the folder that the README does not list as main effects: `ASCII.fx`, `ChromaticAberration.fx`, `ColorMatrix.fx`, `Compare.fx`, `FilmGrain.fx`, `Splitscreen.fx`, `Template.fx`. Their function is not verified in this review. [source: folder listing]

The README also lists two utilities by the same author that live in the separate reshade-shaders package and are not in this repository: DisplayDepth and Splitscreen. [README]

## How it does it

**Injection and hooking.** SweetFX itself has no injection code. The README states that it runs as ReShade shaders and that ReShade loads them. The repository contains no DLL, no ASI loader code, no hook code and no game-function addresses. [source: file listing; README] The specific ReShade wrapper DLL or DirectX version is not stated in the repository. [README: "not stated"]

**Pipeline.** Each effect is a single fullscreen pass. LumaSharpen is the verified example:
- Vertex shader `PostProcessVS`, pixel shader `LumaSharpenPass`, one technique `LumaSharpen`. [source]
- It reads the back-buffer pixel, computes a blur from one of four sampling patterns, subtracts the blur to get detail, weights the detail by luma (`CoefLuma`, BT.709 by default), clamps it, adds it back and saturates. [source]
- Optional debug view `show_sharpen` replaces output with a grayscale map of sharpened areas. [source]

**Shared state.** Settings are ReShade uniforms declared in each `.fx`. The `.fxh` files are shared headers (`SMAA.fxh`, `FXAA.fxh`). [source]

**Presets.** Presets are text files of setting values. A preset is only compatible with a close ReShade version. Older SweetFX 1.x presets use `USE_` defines to enable effects. [README]

Claims tagged [README] above are stated by the repository documentation and have not been checked against running code. Claims tagged [source] were read in the shader files.

## Configuration

Settings are not stored in a SweetFX config file. They are set in ReShade's UI on the Home tab, and saved as ReShade preset values. [README]

Verified uniform defaults for LumaSharpen [source]:

| Uniform | Range | Default | Meaning |
|---|---|---|---|
| `sharp_strength` | 0.1 to 3.0 | 0.65 | Sharpening strength |
| `sharp_clamp` | 0.0 to 1.0 (step 0.005) | 0.035 | Maximum per-pixel sharpening |
| `pattern` | 0 to 3 | 1 (Normal) | Sampling pattern: 0 Fast, 1 Normal, 2 Wider, 3 Pyramid |
| `offset_bias` | 0.0 to 6.0 | 1.0 | Sampling radius scale |
| `show_sharpen` | bool | false | Debug view of sharpened areas |
| `CoefLuma` (define) | float3 | (0.2126, 0.7152, 0.0722) | Luma weights, BT.709 |

Defaults for other effects were not extracted in this review. The README gives no key list beyond these.

## How we could implement it in nfs-mw

Spec-first. The repository is MIT-licensed, so its code may be used under that licence with the copyright notice kept. Our project rules still apply: specify behaviour and facts first, then implement from the spec, and keep the licence notice if any code is ported. [docs/licensing.md] Confirm the project's current licence policy in `docs/licensing.md` before copying any shader text.

**Behaviours to specify.** For each effect we intend to ship:
- The input and output of the pass (colour space, whether it reads the back buffer, whether it is applied before or after the UI).
- The formula, stated in terms of inputs and parameters, not as a transcription of shader code.
- Parameter ranges and defaults, as in the table above.
- Ordering of passes (anti-aliasing before sharpening, grading after, border last).

**Data or formats needed.**
- A parameter set per effect, with a versioned on-disk preset format.
- A way to express "effect enabled" and "effect order".
- Optional texture input for Layer and Border-style effects.

**Natural home.**
- The post-processing pass graph and the generic effects (sharpen, grade, vignette, film grain, border) are engine-generic. They belong in `libs/` as a renderer-agnostic effect library, for example under a `post-process` folder, one effect per file to respect the 500-line limit.
- Anything tied to NFS:MW's own colour grading, HUD or menu layout belongs in `crates/`. `crates/` depends on `libs/`, never the reverse.
- Renderer-specific pass wiring belongs in a renderer backend crate, per ADR 0004 (`docs/decisions/0004-swappable-renderers.md`).

**Risks.**
- Porting HLSL-style code to our renderer requires care with colour space and gamma. The shaders assume a particular input space.
- Anti-aliasing (SMAA, FXAA) needs depth or motion information for some variants. Check which inputs our renderer provides before choosing a variant.
- Overlapping effects can produce double-sharpening or clipping. Specify the order and clamp rules explicitly.
- Our rewrite must not depend on ReShade. The ReShade-specific parts (UI, preset format, injection) do not apply to a clean-room rewrite and should not be reproduced.

**Loader-specific parts.** Injection and the ReShade runtime do not apply to nfs-mw. The rewrite has its own renderer and does not inject into the original game. This is the same reason the ASI loader items in other mod pages do not carry over.

## Open questions and what could not be verified

- The latest commit date and message were not visible. Activity level is unknown.
- The GitHub language bar was empty, so the language is inferred from file extensions.
- The README does not name supported games, DirectX versions or ReShade versions.
- Defaults were only extracted for LumaSharpen. The other 29 shader files were not read.
- The role of `ASCII.fx`, `ChromaticAberration.fx`, `ColorMatrix.fx`, `Compare.fx`, `FilmGrain.fx`, `Splitscreen.fx` and `Template.fx` was not checked.
- The README says Most shaders moved to developer-managed repositories in ReShade 4.6. The current status of the copies in this repository is not verified.
- No runtime test was run, and nothing was downloaded or executed, per the task rules.

## Sources

- https://github.com/CeeJayDK/SweetFX (repository page)
- https://raw.githubusercontent.com/CeeJayDK/SweetFX/master/README.md
- https://raw.githubusercontent.com/CeeJayDK/SweetFX/master/LICENSE
- https://github.com/CeeJayDK/SweetFX/tree/master/Shaders/SweetFX (folder listing)
- https://raw.githubusercontent.com/CeeJayDK/SweetFX/master/Shaders/SweetFX/LumaSharpen.fx
</content>
