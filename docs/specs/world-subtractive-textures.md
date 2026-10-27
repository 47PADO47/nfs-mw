# Subtractive world textures

The PC texture blend selector uses a separate destination-modulating mode for
`AlphaBlendType` 3. The installed PC texture setup selects source factor ZERO
and destination factor INVSRCCOLOR, with blending enabled. Thus the colour
written to the framebuffer is `destination * (1 - source.rgb)`. Black source
pixels preserve the destination, and white source pixels darken it. This is
not ordinary alpha blending and does not use source alpha as the blend factor.
The regular material shader still determines the source colour, including its
texture, vertex colour and lighting. The draw tests depth but does not write it.

The installed oil-stain material at the safe house uses blend type 3; drawing
it with ordinary alpha blending makes its black background cover the ground.
The renderer must route type 3 through the destination-modulating blend state.
The unrelated overbright mode 4 remains outside this specification.

Evidence and limitations are recorded in
[the provenance note](../provenance/world-subtractive-textures.md).
