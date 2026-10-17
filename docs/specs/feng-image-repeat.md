# Repeating FEng images

Sources: installed PC `Dialog.fng` and `MainMenu.fng` object data, read through
the project's `dump-screen`; the existing shared UI sampler; and
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
`Frontend/FEngRender.cpp` (`RenderImage`) and `FEng/FEImage.h`.

## Evidence and contract

- **[verified]** The dialog stripe's authored UV rectangle is
  `(0, 0)..(16.8125, 6.1875)` for a `538 x 198` image. Its intended tile
  therefore covers `32 x 32` FEng units. Clamping that rectangle shows one tile
  in the corner and stretches its final row/column over the rest of the panel.
- **[verified]** The main menu footer stripe has UVs
  `(0, 0)..(8.90625, 1)`; its `285 x 26` rectangle has the same clamp problem.
- **[decomp]** `RenderImage` passes the authored UV rectangle into its polygon
  draw. It does not consult `FEImage::ImageFlags` for texture addressing.
  No per-object clamp/repeat bit is established by this evidence.
- **[ours]** Ordinary FEng images whose UV rectangle leaves the unit square
  repeat that rectangle. Images wholly inside it retain the existing path;
  fonts, input atlases, egui and composed masks retain their current sampling.

## Presenter implementation policy

Keep the shared GPU UI sampler unchanged. Split a repeating image at every
integer UV boundary, preserving its original transform, opacity and draw order.
Each resulting quad samples one tile. Negative scrolling and reversed UV axes
must preserve phase and direction. Add one wrapped texel around the decoded
texture and remap the tile's UVs into that interior, so linear filtering across
a boundary sees the opposite edge rather than an edge-clamped seam.

The padded texture is uploaded once per resolved texture. Subdivision is bounded
to 4096 quads per source image. Nonfinite or larger UV ranges use the existing
single-quad fallback rather than allocating an unbounded mesh.

## Verification boundary

Synthetic regressions cover tile counts, fractional and reversed rectangles,
negative scrolling, wrapped filter borders, caching and the original image's
extent. Installed package regressions identify both affected stripe rectangles.
An original-game pixel comparison and unknown addressing flags remain unverified.
