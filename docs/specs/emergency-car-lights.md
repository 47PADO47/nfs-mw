# Emergency car lights

The vehicle's installed `LIGHT_COPRED`, `LIGHT_COPBLUE` and `LIGHT_COPWHITE` state bits enable
independent lens flares at the corresponding model markers. Broken bits override each colour.
Emergency flares have no additional night gate in the original PC car-flare function. These are
separate from the installed `fxcar_coplight*` particle groups; particle lifetime must not be enlarged
to substitute for the missing flares.

Use simulation time plus a persistent per-car phase, including time while the lamps are disabled.
The original initializes the car phase to a random value in `[0, 1)` seconds. Evaluate the following
continuous functions at draw time; do not advance a Boolean flash state once per rendered frame.
The host samples the shared fixed timeline at the same interpolation fraction as
the car pose. Its persistent phase is distributed from the physical instance ID
into `[0,1)`; this preserves independent car phases without claiming the original
global random-number sequence.

- Let `p = cos((TAU / short_period) * time)^2`, where the stored short period is the single-precision
  value of `PI / 12` and the long period is the single-precision value of `PI / 4`.
- Red is `p` when `cos((TAU / long_period) * time) > 0.2`, otherwise zero. Blue uses the opposite
  phase, adding single-precision `PI` to the gate angle.
- White uses `sin(3 * p * cos(12 * (time + 1.5 * flare_index)))`, clamped to visible intensity
  `[0, 1]`. The index is the original traversal ordinal across flares, not just white markers.
- Enabled white emergency lights also modulate the left headlight by `cos(12 * time)` and the right
  by `1 - cos(12 * time)`. Clamp the resulting visible intensity, rather than clamping the cosine
  before multiplying. This mode depends on the white enabled bit even if the white flare is broken.
- `COPLIGHTORANGE` is steady and independent of the emergency enable mask.

The native traversal multiplies an accumulated red or blue intensity when encountering each normal
colour marker; a bright marker uses that accumulated colour multiplied by the pulse again. Keep
marker order if reproducing that repeated attenuation. The pure pulse API also supports independent
evaluation for callers that do not have the original traversal order.

Read `coplightred_inner`, `coplightblue_inner`, `coplightwhite_inner`, `coplightbrightred`,
`coplightbrightblue` and `coplightorange` from installed `light_flares_cg` collections, retaining
their authored texture, colour, size, power and bias. Native emergency markers have flag `2`, whereas
ordinary directional lamps have flag `4`. The native flare function applies directional intensity
gain only for flag `4`; emergency flare intensity therefore bypasses the facing-dot gate. Depth/range
visibility and normal lamp damage are shared with the vehicle flare renderer.

Validation covers disabled and broken colour gates, alternating colours, white ordinal dependence,
headlight modulation, finite input handling and installed profile availability. Local `sin`/`cos`
does not assert bit-identical equivalence with the original angle-table trigonometry. Full visual
parity, dynamic world illumination and reflection passes remain separate validation work.
