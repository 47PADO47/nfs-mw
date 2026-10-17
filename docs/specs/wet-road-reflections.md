# Original PC wet-road reflections

Status: producer behavior recovered by static inspection; material sampling is
already covered by independent original-program fixtures. This specification
precedes capture implementation. See [provenance](../provenance/wet-road-reflections.md).

## Capture contract

The road image is a separate projected **2D** scene, not the car environment
cube. The inspected enabled target is 320 by 240, one mip level, ARGB8, with a
matching D16 depth surface. It is cleared to opaque black with depth one before
capture. Disabling the original option allocates a one-pixel white fallback.
The inspected WorldReflect updater still supplies RoadDampness independently
of this option. Consequently, deliberately suppressing the reflected material
term when disabled is a remake choice; it is not established as the source
option-off material response.

The source road view shares the main player's camera and precipitation object.
It does not reflect that camera about a single estimated ground plane.
Its projection reads the main camera's field of view, but computes aspect from
the road target's dimensions with the same 1.15 aspect multiplier used by player
views. At stock 4:3 output this matches the main view's projection. Widescreen
reference projection patches require a separate check; using the corrected main
projection is a compatibility choice, not established by this stock trace.
The image is subsequently sampled in the main view's normalized screen
coordinates. A global mirror-plane or normal-distorted reflection is a different
effect and must not silently replace this path.

The render dispatcher refreshes the active road image before the main scene.
There is no recovered lower refresh-rate budget or temporal accumulation here.
The view is eligible when road detail is nonzero, the road view/target is valid,
and player RoadDampness is greater than 0.001. A settled dry state contributes
zero. A dry frame immediately after rain may still have a visible reflection.

## Authored reflection geometry

Reflection culling and geometry selection differ from the main view. Preserve
the scenery instance's source flags and all four model slots.

The ordinary road view uses flags `0x08000850`, with additional view/event flags
where applicable. If an instance has either `0x08000000` or `0x40`, expand its
effective flags with both bits. Reject the instance when
`((effectiveFlags XOR 0xFFFFFF60) AND viewFlags AND 0x080000FF) != 0`.
This includes the original low-byte inversion and the PC bit-27/bit-6 alias.

1. Reject projected size below two; add ten when source flag `0x02000000` is
   present; then require at least 32 projected pixels in the road view.
2. Instance flag `0x80` selects model slot 2. Otherwise flags `0x1000100` select
   slot 0; ordinary instances select slot 3. An absent selected slot stays absent.
3. Instances with flag `0x200` submit no model matrix and keep their authored
   geometry placement. Other instances retain their rotation/translation except
   that the matrix's local-Z/world-Z component changes sign. This is the element
   at column 2, row 2 under the remake's column-vector convention. It does not
   negate translation or reflect the entire world about one common elevation.

These rules accommodate authored reflected geometry. Reusing the main visible
list, always choosing its mesh, or mirroring every object about the player's
road height does not implement the inspected behavior. Preserve source exclude
flag filtering and view-specific culling; a bounded resident-only capture must
record that residency/preculler coverage remains incomplete.

Projected size uses radius `r = infoRadius + 6` and camera-to-instance center
vector. Reject a sphere wholly behind the view (`dot(to,forward) < -r`). With
distance `d = length(to)`, size is the view's stored projection scale H when
`d-r <= r`, otherwise `r*H/(d-r)`, truncated to integer. The PC producer of H
has not been established here; deriving it from capture height and FOV remains
a reconstruction choice until calibrated against source culling.

## Capture materials

The original road-target draw dispatch substitutes these effect families:

| Surface effect | Capture family |
| --- | --- |
| World, ID 0 | WorldNoFog, ID 8 |
| GlossyWindow, ID 5 | its own family |
| WorldNormalMap, ID 3 | WorldNormalMapNoFog, ID 30 |
| WorldReflect, ID 1 | WorldNormalMapNoFog, ID 30 |
| Car, ID 4 | its own family, technique level zero in this view |
| Other IDs, including Tree 2, Prelit 6 and NoFog 8 | their own family |

The verified WorldNoFog program transforms positions by the supplied WVP; it
does not perform a hidden mirror operation. Its color is twice diffuse texture
times vertex color. It is therefore not equivalent to drawing ordinary lit
World materials with only their fog switched off.
The separate shadow target also substitutes car/window families, but that is
not the road target. Reflection capture must never recursively sample the image
currently being rendered or run the player HUD/post-processing chain.

The CAR family explicitly selects technique level zero outside player view one.
Its already-verified vertex-lit arithmetic applies in this road view; the
player's shadowed CAR technique must not be reused merely because that mode is
selected for the main view. Custom car shading remains an explicit enhancement.
The ID 30 resource selects a fixed-function technique with both vertex and pixel
programs disabled. Applying that original effect on D3D9 confirms stage zero
MODULATE2X for texture times diffuse RGB, ordinary MODULATE for their alpha,
disabled later stages, and disabled lighting/fog. Independent asymmetric pixel
fixtures confirm `RGB = clamp(2 * texture.rgb * vertex.rgb)` and
`A = texture.a * vertex.a`, including separately zeroed texture/vertex alpha.
It therefore uses the existing NoFog arithmetic, not tangent normal lighting.
The technique does not establish the caller's blend/cull state; retain each
submitted range's existing blend/cutout contract rather than inferring state
from an uninitialized effect-state query.

The native capture keeps dedicated camera/instance uniforms and submits its
commands before updating the main view. Normal materials with no explicit
reflection texture sample this live target only in their reflective model;
explicit texture bindings remain unchanged. Capture substitutes the normal
families before binding material resources, so the target cannot sample itself.
Resizing, blur changes and disabling rebind retained materials without mesh or
texture-handle churn. Zero size disables the image; native dimensions are
bounded to 2048 pixels per edge and the device limit. Unsupported backends
report no projected-reflection capability. With no live image the native
receiver binds black, an explicit remake behavior described above.

## Detail and blur

Road detail zero disables capture. All enabled levels draw the reflected sky
and eligible scenery. Values above one add the vehicle-render pass when wet.
The inspected car callback preserves the body origin and negates its complete
local Z axis, with reflection render flags. The car renderer builds wheel/brake
local transforms, including their ordinary steering/spin/travel inputs, then
composes them with that reflected body root. Do not independently reverse those
inputs. For already assembled attached parts `originalRoot * localPart`, applying
`reflectedRoot * inverse(originalRoot)` on the left is algebraically equivalent
to rebuilding each part with the reflected root. This transform is per vehicle;
it does not apply to detached fragments. Per-vehicle eligibility and reflection
LOD remain separate boundaries.
The remake feed reuses each displayed vehicle's interpolated attached parts and
damage visibility. Player, ground traffic, ground police and hitched or loose
trailers enter the vehicle detail tiers without a second mesh upload or physics
update. Trailers retain independent roots. Detached debris and the helicopter
are excluded; their separate source reflection callbacks are not established.
Animated/world effects and vehicle flares have additional source enable gates;
their full material/content coverage is not established here.

The sky uses a separate source reflection placement: scale 0.06 on X/Y,
negative 0.06 on Z, translated to the camera position with an additional 130 on
world Z. This is another authored/model-specific transform, not a shared plane.

On the capable path, detail three applies two image-filter passes after the
initial scenery/vehicle image and before later effects. It is a blur, not an
extra geometric detail tier. A horizontal four-tap pass has weights
`[1/3, 1/6, 1/3, 1/6]` and texel offsets `[1.5, 3, -1.5, -3]`;
the vertical pass uses the same weights/offsets on Y. Exact effect-technique
selection and sampling state still require fixture validation. Image size
increases must recompute texel offsets; they are asset-independent enhancements.

## Dampness and high frame rates

The stock producer uses a desired wetness of one while rain intensity is nonzero,
and zero otherwise. Each weather call updates the current value by
`wetness += (desired - wetness) * 0.005`. Falling values below 0.05 snap to zero.
Disabling precipitation resets rain and current dampness immediately. A source
override can assign current dampness directly; ordinary rain history and an
explicit comparison override must remain separate.

In the matched executable the weather updater is called from the render path,
after the main scene, so copying this recurrence once per remake render frame
would change wet/dry timing with FPS. A remake must explicitly choose a documented
reference cadence, for example fixed 60 Hz or an elapsed-time conversion with
retention `0.995^(60*dt)`. That is an FPS-independent adaptation of the measured
recurrence, not evidence that the stock update was fixed 60 Hz. Preserve zero-dt
pause behavior and the drying cutoff. Test 30/60/144/360 FPS through both rain
onset and drying; compare elapsed-time state and capture eligibility.

## Receiving surfaces

Use the [WorldReflect](world-reflect-materials.md) homogeneous main-clip UV and
existing masks. The reflected contribution before final SDR saturation is
`2 * reflectedRGB * (2*specularMask.r) * diffuse.a * vertex.a * RoadDampness * E`.
Reflection alpha and normal-map alpha are unused. Surface normals do not distort
the reflected coordinate. Original projected shadow visibility affects direct
light/highlight independently and must not darken this reflection term again.
Direct highlight RGB still receives the source cloud attenuation even when
dampness is zero.

## Validation and remaining boundaries

- CPU tests: source slot/flag rules, local matrix sign change, quality gates,
  wetness timing, pause, disable/reset and resource lifecycle.
- Synthetic GPU tests: screen coordinate orientation, camera motion, authored
  reflected geometry, masks/alpha, source gamma-space color, no HUD/recursion,
  capture material substitutions and blur footprints.
- Installed tests: actual slot-3 geometry and bindings resolve; no invented
  substitute assets; a rainy reference route covers slopes, intersections,
  moving vehicles and post-rain drying.

Static research does not establish live reflection pixels, complete original
preculling/visibility, vehicle reflection LOD/eligibility, effect poses, runtime sampler state or the preferred
enhanced reference's modifications. Persistent dry reflections in an enhanced
reference must remain an explicit compatibility/enhancement choice, separate
from stock dampness behavior. Full parity must remain unclaimed until these
boundaries are validated.
