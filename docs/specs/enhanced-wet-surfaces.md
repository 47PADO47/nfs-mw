# Enhanced wet surfaces

Independent authored-material extension; not original-game parity.

## Inputs and eligibility

- Explicit saved Enhanced wet surfaces switch. Original PC style disables it; Enhanced style enables it.
- Only the authored reflective normal-material family participates. Existing specular red, diffuse alpha and
  vertex alpha remain reflection eligibility factors. Pools expand nonzero eligibility toward full water
  reflectance because the original mask already attenuates a nonphysical additive reflection. Zero mask
  regions remain excluded. These channels are not depth or roughness maps.
- Upward-facing geometric normals gate film and pooled water. Walls retain the original result.
- Dampness retains the original elapsed-time recurrence. A separate pool history builds after saturation,
  responds to rain amount, and drains after rainfall. Pausing freezes both histories and ripple motion.
- The Environment wetness override sets both histories for comparison without destroying stored history.

## Spatial response

Two smooth world-space noise scales approximate irregular patch coverage. They do not identify actual low
spots, drainage, curbs or puddle depth. The same integer lattice hash and smooth interpolation define CPU
wheel-contact coverage and GPU coverage. GPU edges widen with the pixel footprint to suppress distant aliasing.
Coverage is independent of camera motion and time. Patch size and coverage are advanced controls.

Damp asphalt darkens in an approximate linear-light domain. Pooled water smooths the tangent normal toward
its geometric plane. A water-like Fresnel response increases reflection at grazing angles. Film samples a small
spatial filter of the existing capture; pools retain finer detail. Small world-space oscillations affect the
water normal during rain and fade below the pixel footprint. This is a cheap ripple approximation, not drop
impact simulation. It does not introduce UV offsets that pretend to correct reflection parallax.

The authored renderer and projected capture remain display-encoded. The extension decodes/encodes its local
lighting calculation with a 2.2 power approximation; it is not an HDR radiance capture or a full PBR conversion.
Enhanced mode bypasses the capture-wide blur: film filtering happens at the receiver and pooled water
retains the original capture detail. The original mode keeps its fixed blur unchanged.

## Contact effects and limits

Player, traffic and police tyre effects keep authored surface groups and speed/slip limits. Enhanced mode
scales emissions using persistent dampness and local patch coverage, so spray can continue after rainfall.
Existing actor shelter checks suppress spray under cover. Drying does not depend on reflection quality.

There is no world-space rain-exposure map yet. Visual water history is global: procedural puddles can therefore
appear on eligible sheltered roads. Per-actor shelter is conservative and does not prove the surface is dry.
Rendered material eligibility and collision surface groups come from different original data; their boundaries
can differ. No claim of physically simulated drainage or exact contact-to-pixel agreement is made.

## Enhanced road reflections

Perspective main views snapshot opaque colour and reverse-infinite depth after opaque cars and before
transparent scenery, particles and UI. Eligible reflective road materials are redrawn against the unchanged
depth buffer. Deterministic world-space steps and seven crossing refinements trace the receiver's
water normal up to a configurable range (320 m by default, 80-800 m). Step counts scale with range
(96-480 samples), preserving the previous sampling density instead of stretching sparse samples.
The final 65% of the range blends smoothly into the environment by default; the advanced blend control
allows 10-90%. Screen-edge blending spans 9% of the image. These controls persist with appearance settings.
Longer ranges increase GPU cost and cannot recover hidden or off-screen geometry.
The ray origin bias grows with the receiver pixel footprint to avoid hitting its own depth texel on distant roads. Depth discontinuities, screen edges and the distance limit fade or reject hits.
Pixels the mask leaves with zero reflectance skip the march and use the environment cubemap alone, which they
weight by zero. A small threshold is not used: over a dark road, a reflectance under 1% still shows once encoded. Misses sample the current environment cubemap by reflected world direction. Film uses a small spatial filter;
pools retain detail. There is no frame history, ray jitter, ray tracing or game-asset dependency.

The base draw uses disabled ray controls and the static sky environment, preventing snapshot and cubemap feedback. Foreground depth prevents
the receiver redraw from overwriting cars. Original materials, orthographic views and explicit material
reflection textures retain their existing path. The game skips the old projected capture in enhanced mode.
The original road detail/resolution controls therefore apply only when the enhanced path is off. Turning road
reflections off still suppresses reflection contributions; dampness and surface darkening remain independent.

Screen rays cannot reflect hidden or off-screen objects. The local environment is an approximation across
sloping roads and distant receivers, and can update at its configured capture cadence. Transparent objects
are absent from the ray snapshot. Streaming limits still apply. No exact multi-bounce or off-screen parallax
claim is made.

## References and provenance

Conceptual references only; no source code copied:

- Sebastien Lagarde, [Water drop 3a](https://seblagarde.wordpress.com/2013/03/19/water-drop-3a-physically-based-wet-surfaces/):
  porous wet-surface darkening and layered water response.
- [Water drop 3b](https://seblagarde.wordpress.com/2013/04/14/water-drop-3b-physically-based-wet-surfaces/):
  separate dampness and accumulation, material-parameter approximations.
- [Water drop 4a](https://seblagarde.wordpress.com/2013/09/08/water-drop-4a-reflecting-wet-world/):
  angle-dependent reflection and rough-surface filtering.

These articles are references only, with no assumed permission to copy their code. The lattice hash,
coverage, history integration and WGSL implementation here are independently authored approximations.
