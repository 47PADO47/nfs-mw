# Weather GPU work

The enhanced cloud volume evaluates phase coefficients and sun colour once per
view ray. Density evaluates the height envelope before fetching base and erosion
noise: a zero envelope returns the same zero density without those texture reads.
This also applies to cloud self-shadow and ground-shadow density samples.
Cloud map dimensions, update bands, density filtering, view/light sample counts,
wind and opacity termination remain unchanged.

When the enhanced sky's daylight factor reaches zero, the cloud march still
integrates opacity but skips the sunlight-density samples and scattering.
The enhanced RT soft-shadow filter checks visibility and separation first.
A sample inside the current extrema cannot expand them, so the filter skips
that sample's surface fetch and compatibility calculation. Both paths keep
the same samples that contribute to the image.

Screen-space road rays project their origin and direction once, then evaluate
the projected line at each march/refinement distance. Range, sample spacing,
intersection thickness, refinement and confidence fades remain unchanged.
Floating-point reassociation can move an intersection at a depth boundary.

The march takes `clamp(ceil(range * 0.4), 64, 320)` steps (128 at the default
320 m range; it was `clamp(ceil(range * 0.6), 96, 480)`, 192), with the same
quadratic spacing, followed by eight bisections instead of seven. Measured on an
RTX 3070 (Vulkan) with a synthetic wet street (pooled WorldNormal road, buildings
both sides, 12 cm poles with bright heads, bright windows, an end wall), batch
timing: the wet road's cost over a dry one fell from 2.02 to 1.58 ms at
2560x1440 (frame 2.32 to 1.89 ms) and from about 4.45 to 3.45 ms at 3840x2160 (frame
5.10 to 4.12 ms). Image differences against the old count: 0.1% of pixels
differ by more than 8/255 (mean 0.08/255), at object edges inside reflections
and the scattered single-pixel holes the march already had; the area of reflected
bright features changed by 0.3%; frame-to-frame change over five camera
positions 5 cm apart was the same as before (no added swimming).
96 and 64 steps (Far Cry 6's published choice) saved 0.67 and 0.85 ms at
2560x1440 but lost 2.3% and 4.5% of reflected bright area, visibly thinning
the reflections of thin horizontal lamp heads, so they were not taken. The loop
already leaves when a ray leaves the screen or passes the near plane; there is
no cheap bound on the farthest scene depth for a further exit, and a per-pixel
step count would move sample positions between neighbouring pixels (noise
without temporal accumulation), so neither was added. Not measured in game.

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

## Trace only where the reflection can show (2026-11-16)

The wet receiver now asks for a ray (RT road reflections) or a screen march (the
screen-space path) only where the result could be seen. Before the trace it
knows the pixel's wet light without the reflection (`diffuse + spec + glint`,
linear) and the reflection's weight (`reflectance * specular occlusion`). It
traces when a reflection half a unit away from the probe would move the final
pixel by 3/255 or more after encoding and the dampness mix; otherwise the
probe (`road_probe_at`, the same fallback a missed ray takes) answers alone.
Pools and grazing film carry enough weight to trace; so does film on a dark
receiver, where a small linear change is a large encoded one (the synthetic
black-road test `hybrid_road_reflects_visible_geometry_and_preserves_foreground`
still sees its wall). The faint damp film on a lit road, about 1% reflectance
when looked down on, keeps the probe. A flat 1% weight threshold measured the
same in game but lost that dark-road case, so it was not kept.

Measured in game at 3840x2160 on an RTX 3070 (Vulkan), BMW at the street lamp
(2298,3415) or highway (2418,3415), no traffic, median frame time of a 12 s
bench, two runs each (before / after):

| Scene | Before | After |
| --- | --- | --- |
| Night rain, overcast, RT road reflections (user night config) | 28.04, 27.82 | 26.47, 26.49 |
| Day rain, overcast, RT road reflections (user day config) | 23.03, 23.04 | 21.47, 21.44 |
| Night rain, RT road reflections off (screen march) | 32.31, 32.08 | 28.42, 28.45 |
| Clear dry day (wet path unused) | 15.91 to 16.13 | 15.82 to 16.04 |
| Tunnel, dry day, seven runs each | median 17.31 | median 17.37 |

The road redraw's GPU timestamp went from 4.1 to 2.6 ms with RT and from 8.2
to 4.3 ms with the screen march. The tunnel's +0.06 ms is at the noise level.

Image change, 4K screenshots, mean absolute difference per pixel in /255 and
the share of pixels off by more than 2/255, against two runs of the old binary
(the noise floor): night wet without rain particles 0.031 (0.18%) against a
floor of 0.029 (0.18%); night rain within the floor (0.16 against 0.18); day
wet 0.026 (0.17%) against an unusually quiet floor of 0.0013 (0.011%), spread
as faint single-pixel changes over the road and invisible in 1:1 crops at 10x
amplification; screen-march night wet 0.024 (0.09%) against 0.0018 (0.009%),
no visible change in 1:1 crops. Dry scenes are unchanged.

Measured while looking for this (night rain, RT on, same bench): the redraw of
the reflective receivers was 4.1 ms, of which about 2.65 ms was the ray trace
(about 2.3 traversal, 0.3 hit shading), 1.0 ms the rest of the shading and
0.46 ms raster. Puddle coverage, ripples and screen colour reuse cost nothing
measurable; a 80 m reflection range saved 0.6 ms (not taken: it shortens the
reflections). Tried and not kept:

- Opaque ray flags for the opaque BLAS geometry: 0.1 ms, and it changes which
  triangles take the sidedness test.
- A cheap base-pass shade for receivers the redraw overwrites: up to 1.1 ms
  (flat colour) or 0.3 ms (dry shading, skipping only the wet layer). That
  colour is also what other screen-space reflections read before the redraw:
  a tree and the lamp pole in the night shot changed (mean 0.18/255 in a tree
  crop against a floor of 0.006), and road-on-road reflections would show the
  stand-in. Not worth 0.3 ms.
