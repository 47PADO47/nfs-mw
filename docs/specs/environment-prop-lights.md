# Environment prop lights

Status: implemented for the native main view, including the world viewer without
a player car. Installed-data and Vulkan/DX12 rendering regressions are included.

Sources: the user's installed PC data **[verified]**, read-only PC executable
analysis **[decomp]**, and the CC0-1.0
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reference's
`World/Scenery.cpp` and `Ecstasy/eLight.cpp` / `eLight.hpp`.
No reference code is copied. See [provenance](../provenance/environment-prop-lights.md).

## Two separate mechanisms

### Model markers

The original scenery submission path enumerates the selected model's position
markers. It subtracts the solid pivot translation from each marker translation,
then applies the instance matrix. The resulting point becomes a pooled flare.
Its type is the marker integer parameter plus 14 **[decomp, PC checked]**.

| Marker parameter | Flare type | Meaning |
|---|---|---|
| 0 | 14 | Orange cat's-eye reflector |
| 1 | 15 | Red cat's-eye reflector |
| 2 | 16 | Blue cat's-eye reflector |
| 3 | 17 | Blinking amber |
| 4 | 18 | Blinking red |
| 5 | 19 | Blinking green |

Meaning names come from the reference enum. A four-file PC data audit found
21 marker records on 20 distinct world model names: 13 amber, six red, one
green and one orange. These count model records, not placed world lights.
Examples include `XS_CHEVRCNRA_1_00`, `XS_MEDIANPOLE_1B_00`,
`XS_MEDIANPOLER_1B_00` (two markers), `XS_FLASHLIGHTA_1B_00`,
`XS_MEDIANPOLEGREEN_1B_CK_00`, and `XO_TRAFFICLIGHTC_1B_00` **[verified]**.
This is a bounded inventory, not proof that all installed files use these roles.

The three shared blink channels accumulate world elapsed time. Each toggles
when its accumulator is strictly greater than 0.5, 0.4 or 0.45 seconds,
respectively, then resets that accumulator to zero. Initial static state is off.
Red uses channel zero; green uses channel two. Amber selects a channel with
`abs(trunc(world_x) % 3)` **[decomp]**. PC control flow and interval constants
were checked; live frame timing and initial presentation were not measured.
Preserve overshoot discard and strict comparison when implementing these rules.

Blinking parameters 3 through 5 use directional flare flag 4. Their direction
is the normalized horizontal displacement from the instance origin to the
transformed light point. Other parameters use flag 2 **[decomp, PC checked]**.
Do not substitute the marker's matrix axis for that direction.

The PC pool has 50 entries. Submission depends on the selected model and view.
Exact submission priority, multi-view sharing and overflow policy need attention
before claiming original parity. Installed `light_flares_cg` profiles supply the
appearance; the existing car profile list is not a complete environment mapping.

#### Installed appearance and admission

The PC profile loader and its type table select exactly one `light_flares_cg`
collection for each role **[decomp, PC checked]**:

| Marker parameter | Collection |
|---|---|
| 0 | `catseye_orange` |
| 1 | `catseye_red` |
| 2 | `catseye_blue` |
| 3 | `traffic_light` |
| 4 | `traffic_light_red` |
| 5 | `green_flare` |

Read the same eight appearance fields and three texture slots documented for
[car flares](player-car-lights.md#authored-flare-appearance). Every installed
environment profile selects `HEADLIGHTGLOW`. The installed red/blue cat's-eye
profiles are zero-sized and transparent; preserve those authored values.
Blinking lights use nonnegative direction-to-camera cosine raised to `Power`.
Reflectors use caller intensity directly, without marker-facing attenuation.

Normal player-view alpha doubles and fades linearly over squared distance
32000..40000; radius uses ordered minimum/maximum depth-scale bounds, and
`ZBias` offsets toward the camera. Environment-map views have separate
3900..4000 squared-distance bounds; supporting that view is outside the current
main-scene implementation scope **[decomp, PC checked]**.

Normal player views apply an additional camera-centre cone to cat's-eye types
14..16 when an active camera mover supplies camera parameters. Compute the dot
product of normalized camera-to-light displacement with camera direction.
Reject values below 0.96; otherwise intensity is `(cosine - 0.96) / 0.04`.
Blinking types 17..19 bypass this gate. This is camera-centred admission, not
an original-game headlight illumination test **[decomp, PC checked]**.

### Animated light surfaces

`XO_TOLLBOOTH_LIGHT_1B_00` has no position marker. One of its texture references
is `SGN_SIGNAL_CROSS_`, an authored two-frame animation at two frames per second
**[verified]**. Do not invent a blinking point light for this surface.

The audit also found billboard, lobster-sign and rocket-girl light animations.
The remake reads texture animations and redirects their frames each world update.
Installed native GPU regressions verify the isolated signal face and an actual
streamed D51 toll placement at 0, 0.5 and 1 seconds. Both material modes visibly
change and loop on Vulkan and DX12. No toll-surface runtime fault was reproduced
on this revision, so no speculative appearance change was added.

## Runtime integration

- The guessed `sign_lamps` overlay is replaced by installed model markers.
  Each resident mesh retains its own pivot-corrected marker metadata.
- Main-view scenery selection supplies the active LOD, group visibility and live
  prop transform. Broken hierarchy nodes and fragments use their own mesh data.
  Removing/replacing resident meshes retires their marker metadata.
- The world blink clock advances with elapsed simulation time, independently of
  player cars and the texture-animation-speed option. A 50-marker admission bound
  applies before blink gating. Exact original pool ordering is not claimed.
- Installed flare textures use existing depth-tested additive effect rendering.
  Appearance shares the car flare projection, range fade and authored bounds.
- Enabled extended junction signals replace authored blinking flares only on
  matched signal heads. Unmatched signals retain their authored markers.

## Source organization

- `nfsmw-data::world::lights`: marker roles, pivot subtraction and profile lookup.
- `world/resident` and `residency/lights`: mesh-lifetime marker ownership.
- `world/environment_lights`: world blink clock, main-view selection and drawing.
- `world/materials/toll_*_tests`: installed toll animation/render regressions.

## Validation and remaining limits

Synthetic tests cover nonzero pivots, rotated transforms, multiple markers,
negative X channel selection, strict toggle boundaries, overshoot, pool capacity,
mesh replacement, and selected/unselected models. Installed tests resolve named
model roles/profiles/textures and exercise main-view group/hidden-prop selection
without a player. Native GPU cases check lit/off phases and opaque depth occlusion.

No original/remake matched visual comparison or frame-time benchmark was run.
Detached fragment behavior uses the rendered fragment mesh/pose but lacks a
dedicated installed collision comparison. Rear-view, environment captures and
split-screen still share the existing main-camera effect feed; their per-view
flare selection/orientation is outside this implementation. Original pool
priority and camera-mover-specific reflector admission remain parity limits.
