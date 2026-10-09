# Post-processing

Design of the optional image effects of this rewrite. They are new; the original game has none that
this reproduces, so nothing here is a claim about the original.

- **Sources read:** public descriptions only: Krzysztof Narkowicz, "ACES Filmic Tone Mapping Curve"
  (2016); the Call of Duty: Advanced Warfare post-processing talk (SIGGRAPH 2014, the 13-tap downsample);
  Timothy Lottes, "FXAA" (NVIDIA white paper, 2009) and the public walkthroughs of FXAA 3.11.
- **Data inputs:** none from the game; the effects read the rendered scene.

## Behaviour

The scene is drawn into an HDR image (`Rgba16Float` when the GPU can render to, blend into and filter it,
else the surface format). A chain of fullscreen passes then reads it and the last pass, `resolve`,
clamps to the display range and writes the surface. The effects sit at the front of the chain, in this
order, each present only when enabled:

1. **Bloom.** Prefilter: per tap, `weight = max(soft, b - T) / b` with `b` the largest colour channel,
   `T` the threshold (0.8) and `soft = clamp(b - T + K, 0, 2K)^2 / (4K)`, `K = T / 2` (a soft knee), each
   tap capped at 32. The image is reduced to half size with a 13-tap filter (centre 1/8, four diagonal
   taps 1/8 each, four edge taps 1/16, four corner taps 1/32), then down to at most six half-size images
   (stopping when an axis reaches four pixels). Going back up, each image is added to the one above
   with a 3x3 tent filter (1 2 1 / 2 4 2 / 1 2 1 over 16). The result is added to the scene scaled by
   `intensity / 4` (the levels add up, so the user's 1.0 is a quarter of the raw sum).
2. **Tone mapping.** `colour = aces(max(c, 0) * exposure)` per channel with
   `aces(x) = clamp(x (2.51 x + 0.03) / (x (2.43 x + 0.59) + 0.14), 0, 1)`. `aces(1) = 0.804`.
3. **FXAA.** Per pixel: skip when local luma contrast is below `max(0.0312, 0.125 * max luma)`; estimate
   sub-pixel aliasing from the 3x3 neighbourhood (strength 0.75); decide whether the edge is horizontal or
   vertical; move half a texel across the edge; walk along it both ways (up to 12 steps, growing from 1 to
   8 texels) until the luma departs from the edge's mean by a quarter of the steepest gradient; shift the
   sample across the edge by `0.5 - (distance to the nearer end) / (edge length)` when that end lies on
   the right side of the edge; finally sample there. Colours are clamped to 0..1 for the luma decisions
   because, without tone mapping, highlights may exceed 1.

Bloom and tone mapping work in linear HDR values; FXAA assumes display-range colours, so it follows
tone mapping. The game's art is authored for a non-sRGB pipeline and the surface is UNORM, so "linear"
here means "the values the game always wrote".

## Constants

| Constant | Value | Reason |
|---|---|---|
| Bloom threshold | 0.8 | the scene is authored in 0..1, so only bright areas should glow |
| Bloom steps | low 0.5, medium 1.0, high 1.8 | chosen by eye on the BMW M3 GTR in the parking lot |
| Bloom levels | at most 6 | covers about 1/64 of the screen width of blur |
| FXAA edge threshold, minimum, sub-pixel | 0.125, 0.0312, 0.75 | the usual "default quality" values |
| Exposure range | 0.1 to 8 | renderer API; the game keeps 1.0 |

## Defaults

All three are off, so the frame passes through only `resolve` and stays pixel-identical to the frame
without this feature. A filmic curve changes the authored look, bloom adds light to a bright sky and
FXAA softens textures; none is a fair default for a game that is being restored faithfully.

## How to check it

Compare the screenshots of [post-processing.md](../post-processing.md): the default must equal an
earlier build's capture; ACES must darken and flatten; bloom must brighten skies and lights without
changing dark areas; FXAA must change edge pixels only (flat areas stay equal).
