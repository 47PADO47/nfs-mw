# Player car lights

## Sources and scope

**[decomp, PC checked]** The selected PC executable's car flare constructor,
car flare renderer, light-profile loader and flare renderer agree with the
marker roles in the CC0 dbalatoni13/nfsmw reference. The installed
`light_flares_cg` records supply appearance; do not bake their colors and sizes
into source. Provenance: [player-car-lights](../provenance/player-car-lights.md).

## Lamps and state

Head markers include left, right and centre. Brake markers include left, right
and centre. Reverse markers are separate. Preserve each marker's position and
third basis vector through assembly and world transforms. Normal car flares are
one-sided, with intensity multiplied by the nonnegative facing cosine raised
to the profile's `Power`. There are no steering-triggered amber lamps at the
head/brake markers.

Head intensity is one while enabled. Outer tail intensity is 0.5 at rest and
one while braking; the centre brake lamp is dark at rest. Reverse lamps light
only in reverse. Damage can independently suppress original lamp flags; until
the remake supplies those flags, lamp breakage remains unimplemented.

## Authored flare appearance

Headlights use `headlight_inner` and `headlight_outer`; racer brake lamps use
`brakelight_inner` (not `brakelight_traffic_inner`); reverse lamps use
`reverselight_inner`. Read `MinSize`, `MaxSize`, `MinScale`, `Maxscale`, `Power`,
`ZBias`, `colour` and `flare_texture`. Texture slots select
`HEADLIGHTFLAREINNER`, `HEADLIGHTFLAREOUTER`, `HEADLIGHTGLOW` respectively.

Size interpolates between the authored endpoints with directional intensity.
These sizes are quad half-extents, not diameters. Directional power precedes
the caller intensity multiplier. The installed inner head profile has zero
alpha; the original additive blend uses source alpha, so it contributes no
visible main-scene flare. Keep that authored zero instead of forcing a glow.
Depth divided by view focal scale is clamped in order to the authored minimum
then maximum scale; inverted endpoints are valid. Offset toward the camera by
`ZBias` without changing the authored offset; normal scene clipping applies.
RGB scales with intensity; alpha also doubles and fades linearly over
squared distances 32000 to 40000. Reject behind-camera and back-facing lamps.
The PC view focal scale is half the current output width divided by the
tangent of half the horizontal camera FOV. Equivalently, for the remake's
vertical FOV, use half the current output height divided by its half-angle
tangent. Do not substitute a fixed reference resolution. A zero-sized view
produces no flare geometry. Exact original screen rotation, occlusion queries
and reflection-view flares remain unverified.

## Lamp surfaces

Keep installed headlight and brake `_ON`/`_OFF` textures available. Brake input
selects both brake diffuse and glass state. Headlight glass uses the original
headlight-glass ON replacement where supplied. The PC light-state routine uses
that authored glass texture, whereas the independent other-platform reference
overwrites it with the front-window texture. Keep the available replacement
when the authored glass texture is absent. Upload distinct variants once;
state changes select existing meshes rather than upload geometry each frame.
Missing variants retain the existing available texture, never a white fallback.

The PC textured headlight footprint is separate from lens flares and is gated
by the caller's reflection flag. The normal player scene passes zero and must
not draw `2PLAYERHEADLIGHT1` on the road. The original auxiliary pass passes one
and can draw that texture at model height zero, forward coordinates 3..23 and
side coordinates -4.9..4.9, color RGB 204 / alpha 64, rejecting world-up dot
below 0.707. The remake has no matching reflection-pass integration, so leave
that footprint absent. Texture presence and the routine's existence do not
establish its eligibility in normal gameplay. Exact auxiliary-pass depth,
blend and surface behavior remain unverified.

## Unwanted HUD streaks

Remove the procedural radial UI overlay driven by pursuit effect strength.
It can draw even at zero vehicle speed. Preserve the independent graphics
speed-lines option; the selectable upstream streak style uses actual speed,
not the retained pursuit effect value.
