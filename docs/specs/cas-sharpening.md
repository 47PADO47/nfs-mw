# Contrast-adaptive sharpening (CAS)

Design of an optional final sharpening pass for the native resolution (render scale 100 %). It is a
new effect: the original game has no sharpening, so nothing here is a claim about the original.
It is the same family of algorithm as RCAS, which already runs after FSR 1 upscaling
([upscaling.md](../upscaling.md)). The two differ, see [Relation to RCAS](#relation-to-rcas).

- **Sources read:** AMD FidelityFX CAS (MIT licence, "Copyright (c) 2017-2019 Advanced Micro Devices, Inc.")
  as shipped in the ReShade effect `CAS.fx` (MIT header kept, with the ports and fixes listed there:
  SLSNe's first ReShade port, Marty McFly's reordering and the gather trick, Lord of Lunacy's linear
  light fix). We read the effect for its maths and parameters. No code is copied; the WGSL will be written
  from this spec.
- **Data inputs:** none from the game. It reads the rendered scene only.
- **Licence of this spec:** project licence (MIT OR Apache-2.0). AMD's notice applies to the algorithm's
  source, so if the WGSL ends up as a close port of AMD's source, keep AMD's notice and add it to
  [NOTICE](../../NOTICE). A from-spec implementation needs only a credit.

## What the algorithm does

Sharpening adds back local contrast. CAS picks the strength **per pixel**: where the neighbourhood is
already sharp it adds little, where it is flat or soft it adds more. The aim is an even level of
apparent sharpness across the image, with fewer halos than a fixed unsharp mask.

## Inputs

- `scene`: the colour image, in the space the renderer produces (see [Colour space](#colour-space)).
- Two user values, both in `0..1`:
  - `contrast`: how much high-contrast areas are allowed to take sharpening. Default `0`.
  - `sharpness`: the blend between the untouched pixel and the sharpened one. Default `1`.

## Algorithm

For each output pixel, read the 3x3 block around it, with the centre `e` and the neighbours labelled

```
a b c
d e f
g h i
```

Taps outside the image are clamped to the edge (the usual sampler clamp).

**1. Soft minimum and maximum.** Two values per colour channel, computed independently per channel:

- `mn1 = min(b, d, e, f, h)` (the plus shape), then `mn = mn1 + min(mn1, a, c, g, i)`.
- `mx1 = max(b, d, e, f, h)`, then `mx = mx1 + max(mx1, a, c, g, i)`.

Both sums are twice the true min and max of the neighbourhood. The doubling is folded into the constants
of the next steps; keep it or account for it, but don't mix the two.

**2. Per-pixel amount.** `amp = saturate( min(mn, 2 - mx) / mx )`, per channel.

- `min(mn, 2 - mx)` is the distance from the local signal to the nearest limit. A pixel near black (`mn`
  small) or near white (`mx` near 2, which is 1 after halving) has little room to move, so `amp` drops.
- Dividing by `mx` makes the number relative to the local brightness, so bright and dark areas are judged
  on the same scale.

**3. Weight.** `w = -sqrt(amp) / peak`, with `peak = 8 - 3 * contrast`.

- At `contrast = 0` (`peak = 8`) `w` runs from `-0.125` to `0`. At `contrast = 1` (`peak = 5`) it runs
  down to `-0.2`, so the same pixel is sharpened more. The ReShade tooltip describes this as
  "more high contrast sharpening". Note the name is misleading: `contrast` changes the strength, not the
  contrast of the image.
- `w` is never positive, so the filter always sharpens, never blurs.

**4. Filter.** The kernel is a plus shape with weight `w` on the four edge neighbours `b, d, f, h` and `1`
on the centre, normalised by its sum `4w + 1`:

```
out = clamp( (w * (b + d + f + h) + e) / (4 * w + 1), 0, 1 )
```

Because the weights add to 1 the filter keeps flat areas unchanged: if all nine taps are equal to `v`,
`out = (4w v + v) / (4w + 1) = v`.

**5. Blend.** `result = mix(e, out, sharpness)`. `sharpness = 0` returns the input, `1` the full filter.

Only the centre pixel is written. The pass reads nine taps and writes one, so it is a single fullscreen
pass with no extra buffers.

## Edge cases the shader does not handle

- **Black neighbourhood.** When `mx = 0` the division in step 2 is `0 / 0` (or `0 * inf` in the reference's
  reciprocal form). The result is `NaN`, which then spreads. The implementation must guard it: if
  `mx <= 0`, set `amp = 0`, so `w = 0` and the pixel is unchanged. The same guard is needed for any
  divide-by-zero in the reciprocal path. RCAS uses an epsilon for this; CAS needs an explicit branch or a
  `max(mx, eps)`.
- **Values above 1.** The output is clamped to `0..1`. That is right for display-referred output
  after tone mapping. Before tone mapping (HDR `Rgba16Float`) the clamp would cut highlights, so the pass
  must run after tone mapping, or the clamp must be dropped.
- **Gather.** The ReShade code uses `textureGather` for four of the nine taps on DX10+. It is an
  optimisation only. The WGSL can use nine plain loads; use gathers only if profiling asks for it.

## Colour space

ReShade's version samples the game's sRGB back buffer as linear (`SRGBTexture = true`) and writes sRGB back.
The sharpening then works on linear light, which is what AMD's documentation asks for and what Lord of
Lunacy's fix introduced. The rewrite's scene is `Rgba16Float` (linear) when post-processing is on, so
the pass works in linear light. With post-processing off the scene goes to the surface directly, so the
pass must not be enabled there without a conversion (see [Placement](#placement)).

Test both on a gradient: a sharpened edge in linear light looks thinner and less haloed than one done
in gamma-encoded values. Choose by eye on screenshots, and record the choice here.

## Placement

Proposed chain, after tone mapping and anti-aliasing, the last pass before the resolve. Reasons:
sharpening after FXAA doesn't sharpen the aliasing it removed, and after tone mapping the clamp is safe.
This is a choice to confirm with screenshots, not a fact from the sources.

When render scale is below 100 % and the upscaler is `fsr1`, RCAS already runs and CAS must be off
(two sharpeners would stack).

## Settings

| Setting | Config key | Environment | Command line | Values (default first) |
|---|---|---|---|---|
| Sharpening | `post_sharpen` | `NFSMW_POST_SHARPEN` | `--post-sharpen` | `off`, `cas` |
| Strength | `post_sharpen_strength` | `NFSMW_POST_SHARPEN_STRENGTH` | `--post-sharpen-strength` | `0.0` to `1.0`, default `1.0` (mapped to `sharpness`) |

`contrast` stays fixed at its default unless the test pass shows a need for a second control. Two controls
are one more than most users need. The menu row, if added, follows the existing post-processing rows
([post-processing.md](../post-processing.md)).

## Relation to RCAS

RCAS (`libs/blackbox-gpu-passes/src/shaders/fsr1.wgsl`) is the FSR 1 sharpener. It uses a different
kernel: a plus shape with a limiter that stops overshoot, and its strength is given in stops
(`rcas_stops`). CAS uses an adaptive amount from a soft min and max. Both are AMD MIT code, with
the same role. The rewrite should not ship two sharpeners that both run on the same frame. If CAS is added,
either make it an alternative to RCAS for native rendering, or drop it and keep RCAS.

## Tests

- **Flat input is unchanged.** A constant colour field returns exactly the same colour for any `contrast`
  and `sharpness`.
- **White and black are unchanged.** `1.0` and `0.0` fields are untouched (the guard in step 2).
- **Sharpness 0 is identity.** `mix` with `0` returns the input bit-for-bit.
- **Output stays in `0..1`** for random inputs in `0..1`, and never `NaN` (test the black guard).
- **Edge is steeper, no ringing.** A step edge gets a larger local contrast, with the overshoot bounded by
  the clamp, measured on a row of pixels.
- **Screenshot check** through `nfsmw view-world --post-sharpen cas --screenshot`, compared with the
  unsharpened frame, on a road with signs and car detail.

## Open questions

- Do we want `contrast` as a user control, or is the default right for all scenes?
- Is the linear-light result preferred over a gamma-space version on the game's art? Decide with screenshots.
- Should the setting be on by default? No: the game's look is the default, as with the other post effects.
