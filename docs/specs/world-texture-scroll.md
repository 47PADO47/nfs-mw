# World texture scrolling

The installed PC track stores animated UV settings in each 124-byte texture
info record: mode at 0x52, signed step at 0x58, and signed S/T speeds at
0x5a/0x5c. The PC texture setup converts speed using 1/1024 and step using
1/256. Smooth mode 1 adds the fractional part of elapsed seconds times speed
to the diffuse UV. Snap mode 2 first truncates elapsed seconds divided by the
step. A zero step disables snap scrolling. Use the signed fractional part;
repeat sampling handles negative UVs. Mode 0 is stationary. Mode 3 contains
static offset/scale values and is outside this change.

The Rosewood campus fountain uses `TRN_WATERFALL_A`, smooth mode, with a
T speed of 960: 0.9375 texture repeats per second. Its mesh is
`XW_CAMPUS_RINGWALLC_1B_00`. The neighbouring fountain emitter separately
produces mist and sparkle particles.

Texture scrolling runs with the world animation clock and preserves the
texture's normal sampling, filtering and mipmaps. Plain PC texture packs in
resident world sections are the initial scope. Compressed packs, alternate
material families and exact cross-platform behavior need separate validation.

See [provenance](../provenance/world-texture-scroll.md).
