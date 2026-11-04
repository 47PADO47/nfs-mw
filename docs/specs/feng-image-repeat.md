# Repeating FEng images

Sources: installed PC `Dialog.fng`, `MainMenu.fng` and `HUD_SingleRace.fng` object data and scripts, read through the
project's `dump-screen`; the texture packs' `TextureInfo` records (`docs/formats/textures.md`); the existing shared UI
sampler; and [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0, `Frontend/FEngRender.cpp`
(`RenderImage`) and `FEng/FEImage.h`.

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
- **[verified]** The texture carries it instead. Each `TextureInfo` record has a `TilableUV` byte (offset 0x4F). Over
  every image object of the installed screen packages whose rectangle, as stored or as any script key moves it, leaves
  the unit square, the byte matches how the art is meant to be drawn:
  - `BaseStripe`, `Noise_Tile`, `FE_Noise_Tile` and `horz_line`, which tile in both directions, have 3.
  - `LED_main` and `LED_main_Sliders`, which are stretched across up to five times their width but never down, have 1.
  - The pursuit board's `Outrun_Backing`, whose appear and leave slide the texture down to twice its height and back,
    has 0. So do `TAG_1`, which slides from one height above, and `LED_DELTA`, which slides by a third of its width.
  - Of the 725 HUD and menu textures read, 690 have 0, 6 have 1 and 29 have 3. None has 2.
- **[inferred]** Bit 0 makes the texture repeat across and bit 1 down; an axis whose bit is clear clamps to the
  texture's edge. Only bit 0 alone is seen, so bit 1 alone is taken by symmetry.
- **[verified]** `Outrun_Backing` is opaque in its top 18 of 32 rows (0.5625) and transparent below, in the stock pack
  and in the HD copy the private texture work uses alike. Its leave ends at the rectangle `(0, 0.59375)..(1, 2)`, which
  under the clamp shows only transparent rows: the bar is gone. Its colour stays at full alpha throughout, so a
  presenter that repeats it draws the stripes again below and the bar never leaves.

## Presenter implementation policy

Keep the shared GPU UI sampler unchanged; it clamps to the edge. An image whose rectangle stays inside the unit square,
or leaves it only on axes its texture does not repeat on, keeps the single-quad path and that clamp. Otherwise split it
at every integer UV boundary of each repeating axis, preserving its original transform, opacity and draw order. Each
resulting quad samples one tile. An axis that does not repeat stays one span with its coordinates kept. Negative
scrolling and reversed UV axes must preserve phase and direction. Add one border texel around the decoded texture:
from the opposite edge on a repeating axis, so linear filtering across a boundary sees it rather than an edge-clamped
seam, and from the same edge on a clamping one. Remap the tile's UVs into the interior, so coordinates past the edge of
a clamping axis land on that border texel.

The padded texture is uploaded once per resolved texture. Subdivision is bounded
to 4096 quads per source image. Nonfinite or larger UV ranges use the existing
single-quad fallback rather than allocating an unbounded mesh. A texture no pack holds (a picture the remake draws
itself) has no `TilableUV` and clamps.

## Verification boundary

Synthetic regressions cover tile counts, fractional and reversed rectangles, negative scrolling, wrapped and clamped
filter borders, a clamping axis kept whole, an untiled texture kept on one quad, caching and the original image's
extent. Installed regressions identify both stripe rectangles, read the `TilableUV` bytes of the textures above, and
check that the pursuit board's backup backing and the cool-down's hiding bar show only transparent texels once their
leave has run. An original-game pixel comparison and the sampler state the original sets remain unverified.
