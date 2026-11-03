# Weather GPU work

The enhanced cloud volume evaluates phase coefficients and sun colour once per
view ray. Density evaluates the height envelope before fetching base and erosion
noise: a zero envelope returns the same zero density without those texture reads.
This also applies to cloud self-shadow and ground-shadow density samples.
Cloud map dimensions, update bands, density filtering, view/light sample counts,
wind and opacity termination remain unchanged.

Screen-space road rays project their origin and direction once, then evaluate
the projected line at each march/refinement distance. Range, sample spacing,
intersection thickness, refinement and confidence fades remain unchanged.
Floating-point reassociation can move an intersection at a depth boundary.

Wet materials avoid the legacy reflection texture sample when enhanced shading
replaces it. Exactly dry receivers and exactly zero reflection masks skip unused
water/environment work; spatial derivatives are evaluated before divergent exits.
Both RT and screen-space roads use these material savings. RT intersections,
texture atlas quality and reflected geometry selection are unchanged.

F9 GPU counters include shared cloud-map generation and road-reflection snapshot
plus receiver redraw. These are asynchronous observations inside the main frame;
they do not include separate legacy capture submissions or imply a universal FPS
improvement. Compare matching, fully loaded scenes, settings and render sizes.

Road RT hit shading first checks the existing exact-depth current-frame colour
reuse. A fully reusable hit skips texture, material and shadow work that would
otherwise be overwritten. Partial reuse retains both contributions; offscreen
hits keep independent material shading. Car hit shading is unchanged.

Enhanced road fallback now reads the existing capture depth. It fades captured
surfaces to a broad sky estimate when receiver displacement is large
relative to their captured distance. This prevents a prop beside the car probe
from becoming an oversized panorama across distant puddles. The fade depends on
angular parallax, not prop names or hand-selected materials. It runs on fallback
pixels only; fully weighted RT hits skip the fallback entirely. Static uploaded
environments and distant captures retain their previous response. The car's
local-correction toggle does not disable the road safety check.

The sky estimate interpolates six static-environment face colours from a small
uniform buffer. It retains the existing 16-texture RT binding budget.

This is a fallback safeguard, not a second geometry view: it cannot recover
scenery hidden behind a captured prop. RT/SSR intersections still supply local
geometry where available. No extra capture, ray, temporal history or reduced
reflection range is introduced. Capture passes bind dummy depth to avoid reading
the same depth image they are writing.
