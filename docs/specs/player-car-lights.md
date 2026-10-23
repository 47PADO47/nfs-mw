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
only in reverse. Damage independently suppresses original lamp flags after
the on-state has been selected. Marker identities are left/right/centre head
bits 1/2/4, left/right/centre brake bits 8/16/32, left/right reverse bits 64/128,
and police red/blue/white bits 4096/8192/16384. Bright red/blue markers share
their corresponding color's bit. A marker without a mapped VehicleFX bit
does not acquire an invented damage classification.

For the player, braking means the processed regular brake control, not the raw
pedal action. The input controller can apply automatic braking when coasting
slowly or stopped without a pedal held. That control keeps brake textures and
flares lit at a stop. Automatic reverse swaps the pedals before this predicate;
handbrake clears regular braking. Use the same processed state for lamp surfaces
and flares. Do not infer lamp state directly from speed or add a stopped-only
brightness multiplier. Released outer tail flares retain their separate 0.5 base.

**[decomp]** The damage behavior reports every mapped lamp broken when the
vehicle is explicitly destroyed; otherwise it reads a separate per-lamp damage
mask. Destruction and repair must reach the renderer, including taillight trail
copies, without inferring broken lamps from cosmetic panel/glass levels or an
AI loss-of-control accident. The authored named-light producer and its
collision path are specified in [vehicle lamp damage](vehicle-lamp-damage.md);
do not add arbitrary zone-to-lamp thresholds outside those authored actions.

## Native admission and occlusion boundary

The ordinary native head-flare base is one for both traffic and other cars.
The inspected suppression branch checks the cinematic `INIS` singleton, not
an hour-of-day threshold; explicit light-on flags can override that base.
The remake's custom day clock and its headlight hotkey remain extensions.
Do not invent an original night switch from the sun phase or clock hour.

The native point-admission helper tests six view-frustum planes. It is not a
collision-world visibility ray or an asynchronous occlusion query. Normal
flares then enter the ordinary textured polygon path; the rewrite's textured
effects already depth test against scene geometry without writing depth.
Keep authored Z bias and native clipping. A new CPU ray must not be described
as original occlusion behavior without evidence of an original producer.
Precise native render-state equivalence remains an optical validation item.

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
produces no flare geometry. Exact original screen rotation, native render-state
equivalence and reflection-view flares remain unverified.

## Lamp surfaces

Keep installed headlight and brake `_ON`/`_OFF` textures available. Brake input
selects both brake diffuse and glass state. Headlight glass uses the original
headlight-glass ON replacement where supplied. The PC light-state routine uses
that authored glass texture, whereas the independent other-platform reference
overwrites it with the front-window texture. Keep the available replacement
when the authored glass texture is absent. Upload distinct variants once;
state changes select existing meshes rather than upload geometry each frame.
Missing variants retain the existing available texture, never a white fallback.
The PC surface-state consumer keeps head replacements independent of broken
head flags. Brake diffuse and glass replacements check each of bits 8/16/32
independently. With the current shared brake mesh state, all three broken
brake bits must select OFF even while the brake is held; partial-side surface
selection remains open. Do not switch all head surfaces OFF on destruction.

The PC textured headlight footprint is separate from lens flares and is gated
by the caller's reflection flag. The normal player scene passes zero and must
not draw `2PLAYERHEADLIGHT1` on the road. The original auxiliary pass passes one
and can draw that texture at model height zero, forward coordinates 3..23 and
side coordinates -4.9..4.9, color RGB 204 / alpha 64, rejecting world-up dot
below 0.707. The remake has no matching reflection-pass integration, so leave
that footprint absent. Texture presence and the routine's existence do not
establish its eligibility in normal gameplay. Exact auxiliary-pass depth,
blend and surface behavior remain unverified.

## Traffic and other AI vehicles

**[decomp, PC checked]** Ordinary AI vehicles use the same directional marker
flare renderer and installed head/reverse profiles. Installed car usage type 2
selects `brakelight_traffic_inner` for brake markers; other usage types retain
`brakelight_inner`. This is a model classification, not a police/pursuit role.
Use the admitted [brake-lamp state](traffic-brake-lamps.md) for both flare
intensity and cached brake surface selection. Steering does not change native
head, brake or reverse flares.

The world adapter must visit each independently posed trailer too. Use the
trailer body's current brake input, which clears when its physics releases the
coupling. Original reverse-light propagation through a tractor coupling remains
unverified; do not infer it from tractor steering or motion direction.

Keep existing authored cop-light particle emitters and emergency eligibility.
Do not layer the placeholder fixed-period red/blue circles over those emitters.
The optional amber steering indicators remain a clearly separate rewrite
extension with their existing setting/timing. The normal scene must not add
the former procedural traffic headlight wedges: native textured footprints
belong to the auxiliary path described above. Named light-damage actions follow
[vehicle lamp damage](vehicle-lamp-damage.md). General crash-triggered lamp
failures, per-side damaged surface selection, hazard/indicator timing and exact
optical equivalence remain separate parity gaps. Emergency modulation has its own
evidence and installed-profile coverage.

## Unwanted HUD streaks

Remove the procedural radial UI overlay driven by pursuit effect strength.
It can draw even at zero vehicle speed. Preserve the independent graphics
speed-lines option; the selectable upstream streak style uses actual speed,
not the retained pursuit effect value.
