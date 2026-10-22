# Fixed vinyls and garage colors

## Evidence and scope

The installed PC parts and texture packs provide the inputs below [file].
The CPU composition and garage controls are an independent implementation
[rewrite]. Matched original-game pixel parity is not established.
The existing assembly rules remain in [car-assembly.md](car-assembly.md).

## Vinyl inputs

Slot 77 selects a part with part id 79. `TEXTURE_NAME`/`TEXTURE` identify
artwork, and `NUMREMAPCOLOURS` states its recolorable channel count [file].
Vehicle packs `VINYLS.BIN` and `PREVINYL.BIN` hold a bitmap named
`<base model>_<part name>` and a corresponding `_MASK` bitmap [file].
Lookups use the full name hash; stored display names can be truncated.
Only decodable bitmap/mask pairs with equal dimensions are offered [rewrite].
Fixed-livery vehicles have no generated paint and cannot use this path.

For authored artwork (zero remap colors), retain its RGB. For recolorable
artwork, use bitmap R/G/B as weights for up to three selected colors.
Default colors come from installed vinyl swatch slots 79–81. The grayscale
mask supplies coverage; the artwork bitmap alpha is not coverage [rewrite].
Blend this overlay over generated body paint, retaining the overlay separately
so changing the body color preserves the artwork. Spoilers and roof scoops
keep their body color without receiving the body UV artwork [rewrite].

There is one fixed vinyl layer. There is no translation, rotation, scaling,
layer stack or per-panel decal editor. Unsupported inputs are excluded rather
than offered with incomplete rendering. Blending uses 8-bit RGB; original
shader precision, gloss swatches and matched reference captures remain open.

## Rim paint and gauges

Only aftermarket wheel parts can receive rim paint. Replace MAGSILVER,
MAGCHROME and MAGGUNMETAL references in the wheel solids, including driving
LODs, with an isolated material. Multiply its diffuse RGB by the selected
preset/custom RGB. Other materials and other solids remain intact [rewrite].

Load available `HUDS_Custom_00` through `HUDS_Custom_10` packs. The existing
HUD binding selects the tachometer face, fill and needle textures. Optional
backing, needle and digit RGB values apply after authored scripts while
preserving alpha. Removing an override restores its original RGB [rewrite].
The Original HUD theme uses these textures. Drag keeps its separate artwork;
its named needle/digit objects can still receive color overrides.

Presets retain these fields as optional additions to schema version 1.
Older presets retain stock defaults [rewrite]. Gauge-only changes avoid a
vehicle reload. Model/color changes retain the existing garage transaction.
