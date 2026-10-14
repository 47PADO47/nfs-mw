# Speedbreaker vignette and radial treatment

## Controller **[PC checked]**

The visual target follows the player Speedbreaker state independently of the
camera zoom. Read `visuallookeffect` collection `0x90d06c71` at runtime: its
`length`, four matrix-row `graph` y ordinates and `radialblur_scale` control the
transition. Target changes restart the transition clock in world seconds.
Before the authored length expires, evaluate the cubic Bezier ordinates at
elapsed/length, reversing the parameter when current output exceeds the target.
After the length expires, retain the exact target. This preserves the original
restart behavior on interrupted entry/exit instead of adding a smooth ramp.

The output is the vignette weight. Radial strength is output times the authored
scale, clamped to zero through one. The radial UV offset comes from collection
`uves_radialblur`. Load the installed global `UVESVIGNETTE` texture; missing or
malformed profiles/textures disable the treatment with a diagnostic. This
effect does not select a new desaturation/tint profile merely because the
player activates Speedbreaker.

Twenty isolated native-PC vectors check unchanged transition/cubic arithmetic
with synthetic attribute layouts, including target reversals and endpoints.
Installed regressions check actual profile and texture availability. Pauses
and missing local simulation data freeze the world-time transition; frame rates
do not change its duration or discard elapsed simulation time.

## PC shader composition **[bytecode and live object checked]**

The recovered high-quality PC pixel shader adds the mask's green channel times
the vignette weight to each composed color channel, then applies brightness and
the mask's blue gain. Mask red and alpha do not control this operation. In
particular, opaque corners are not blackened by one minus alpha. The installed
texture's blue gain is already part of ordinary Visual Treatment; applying it
again for Speedbreaker would attenuate the scene twice.

The retained native-backend pass adds the exact Speedbreaker contribution
`mask.green * vignette_weight * mask.blue` to the existing scene before the HUD.
Its neutral state is an exact identity. It does not change the user's ordinary
grading selection or reproduce unrelated pursuit brightness pulses. Underlying
grading and its Off/Low modes remain separate parity boundaries; the contribution
assumes ordinary brightness one.

The previous centered four-tap radial kernel has been removed. Neither the
recovered high-quality pixel shader nor its vertex shader performs that sampling.
An authored radial scalar alone does not establish a sampling kernel. Other
original blur/render passes require active-path evidence before implementation.
No generated replacement texture or shader bytecode is shipped.

Pipelines, textures and uniforms remain retained across frames and settings
changes; scene exit clears the resource. Optional backends without this pass
remain a coverage boundary. Matched gameplay image sequences are still required
to establish full composited look equivalence.

GPU regressions cover inactive identity, exact green/blue composition, repeated uniform
updates, resize, graphics-setting changes and scene-resource removal on Vulkan
and Direct3D 12.
