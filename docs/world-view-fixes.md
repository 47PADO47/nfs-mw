# Sun glare and rear-view presentation

The native renderer accepts caller-supplied `SunGlare` artwork layers. The world
selects the installed `SUNMAJORRAYS`, `SUNCENTER`, `SUNMINORRAYS`, `SUNHALO` and
`SUNRING` textures. No artwork is distributed with the implementation.

The original-game screenshots and installed texture metadata establish the
artwork and the visible starburst/halo appearance. Layer sizes, tint and brightness
are independent tuning, not a reconstruction of the original numeric constants.
This does not claim complete original optical-effect parity.

The current directional sunlight places the effect. A small depth neighborhood
around the source controls visibility for the entire flare. Enhanced clouds use
the shared cloud map's transmittance; the original sky uses weather visibility.
The effect is drawn into the main scene before post-processing, never into
auxiliary captures. The existing sun size and brightness controls apply. Closing
the world clears the effect before releasing its textures.

The rear-view presenter derives its glass contour from the installed half-bezel's
alpha channel. It excludes transparent texture padding and detector-bar holes.
The capture aspect follows the resulting glass bounds. Both GPU and readback
paths use this same presenter geometry instead of a second rounded rectangle.
The picture includes the bezel's partially transparent inner fringe and the
bilinear filter footprint between texel centres. Adjacent row slabs share edges
without overlap, so this coverage does not double-blend during the fade-in.

Reference checks: [HD Reflections' PC setup](https://github.com/AeroWidescreen/NFSHDReflections/blob/main/SOURCE/NFSMW/dllmain.cpp)
changes capture dimensions; it does not establish the glass alpha contour.
[Speedbreaker's ultrawide notes](https://github.com/SpeedBreakerProject/speedbreaker/blob/main/runtime/game/ultrawide.cpp)
describe a separate clip-space picture quad and HUD frame in the Xbox 360 path.
[NFSMW-Recompiled](https://github.com/madelrandel-blip/NFSMW-Recompiled) also targets
Xbox 360. Those paths are not PC bezel-layout specifications. All three are
GPL-3.0 references only; no implementation was copied. The inner contour and
filter coverage above are independently derived from the installed PC artwork.

Rear-view range checks use the closest point of world bounds, for both tiles and
individual objects. A world-space mesh can have nearby vertices and an origin
at zero; its origin alone must not remove it from a limited-distance mirror.

The missing architectural surfaces were an alpha-cutout coverage problem. World
walls can use vertex alpha `127/255`, so a `0.5` test removes even their solid
texels. World, no-fog world and prelit world families use a `0.375` threshold,
referenced from noclip.website's independent implementation. Genuine transparent
texels remain rejected. Raster and RT sun-shadow coverage follows the same rule.
Other material families retain their existing thresholds.

Validation includes synthetic source occlusion/weather/disable tests, world cutout
coverage, native mirror tint paths, a world-space bounds regression,
and an optional installed HUD contour test. Exact original optical intensities
and all replacement HUD textures still require visual comparison.
