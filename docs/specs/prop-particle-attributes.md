# Prop particle wrapper and texture attributes

Sources and boundaries: [provenance](../provenance/prop-particle-attributes.md).
This describes attribute interpretation and PC particle presentation. Event
dispatch and emitter timing are separate from these rules.

## Wrapper intensity and motion

An `effects` collection links its `emittergroup` through a `RefSpec`.
`EmitterQuadratic` and `AudioQuadratic` are four-float vectors; `InheritVelocity`
and culling distances are floats. Resolve inherited attributes before applying
these rules. Particle and audio intensity are independent.

For magnitude length `x` and coefficients `[a,b,c,d]`, the wrapper intensity is
`clamp(((x*d+c)*x+b)*x+a, 0, 1)`. The ordinary identity polynomial is
`[0,1,0,0]`, including the one-shot path's fallback when no override is present.
A positive constant coefficient can produce particles at zero event magnitude.
Do not suppress an event solely because its input intensity is zero before
evaluating its wrapper. A zero direction remains zero when normalization cannot
produce a direction.

The inherited velocity passed to the emitter group is the owner's linear
velocity multiplied by a positive wrapper `InheritVelocity`; a nonpositive
coefficient supplies zero. This is not point velocity at a named marker and is
not scaled by effect intensity. Tracking transforms an effect's position and
direction with its owner, independently of this inherited velocity.

At particle birth, ordinary non-live emitters separately vary and clamp their
`MotionInherit` coefficient to `[0,1]`, then add that coefficient times the group's
inherited velocity to their launch velocity. Preserve both inheritance stages.
`MotionLive` requires its own implementation; it must not silently use the
ordinary birth-only rule.

## Named effect positions

A named `EPlayObjectEffect` position refers to an immediate child of the target
model's collision geometry. Compare the serialized name hash directly with the
child bounds' name hash. There is no conversion into the render marker hash
space and no recursive descendant or render-solid marker fallback. A missing
model geometry or child prevents that effect from playing.

Transform the child's stored local position by the target model transform. Its
stored orientation determines the local emission direction, which is rotated by
the same model transform and scaled by the event magnitude. With no named
position, use the model origin and transformed local up direction. The vehicle
draw model exposes the geometry supplied by its owner's boundable interface;
this lookup does not independently select a render component.

## Group intensity ranges

`Emitters` is an array of emitter references; `IntensityRanges` is an array of
two-float vectors. If the arrays have equal lengths, use each emitter's matching
range. A range with `min >= max` becomes `[0,1]`. If the lengths differ, every
emitter uses `[0,999999]`. The endpoints are inclusive: spawning rejects intensity
below the minimum or above the maximum, and independently rejects nonpositive
intensity. These are eligibility limits, not a remapping of intensity into a new
zero-to-one interval.

## Texture records and atlas cells

`ParticleTextureRecord` is eight bytes: little-endian texture name hash at offset
0 and texture-list index at offset 4. The index is not an animation frame.
`ParticleAnimationInfo` is eight bytes: little-endian square-grid enum at offset
0, FPS byte at offset 4, random-start byte at offset 5, and two padding bytes.
Read these game-specific records through `Value::as_raw()` and validate their
eight-byte length and expected type before interpreting them.

The grid enum supports 0, 2, 4, 8 and 16. Grid zero uses the whole texture.
Otherwise, frame `f` selects column `f % grid` and row `f / grid`. Standard packed
UV bounds are zero and `0xffffffff`; high 16 bits encode U and low 16 bits encode
V, each divided by 65535. The native cell width is
`(end_u-start_u)/grid` for both axes. Add the column and row offsets to the
rectangle's starting coordinates, truncate each coordinate times 65535, and
pack the resulting bounds. Preserve this row order without an extra V flip.

At birth, let `n = grid*grid`. With random start enabled, sample a floating frame
in `[0,n-1)`; otherwise use zero. The birth cell truncates this floating frame,
while the stored 16-bit phase is `trunc(frame/(n-1)*65535)`. FPS zero still uses
the selected birth cell; it does not disable atlas sampling.

Each update computes `increment = trunc(dt*FPS/n*65535)`. Add it to the old phase
using a wider integer. If that sum plus another increment exceeds 65535,
subtract 65535 once, then narrow to 16 bits. Select the updated cell with
`trunc(phase/65535*(n-1))`. This unusual wrap is intentional PC behavior, not
ordinary modulo arithmetic.

The separate texture-page loader accepts records containing four UV floats,
flags, a texture hash and padding. Do not infer that these records are active
from their format alone. The inspected ordinary PC prop path starts from the
full texture; console texture-page use and other packs remain a separate scope.

## Verification boundary

Installed prop effect families exercise animated debris and sparks, including
random atlas cells with FPS zero. Validate animated and static sprites, wrapper
polynomials, range boundaries, and the two inheritance multipliers separately.
Matching these rules does not establish identical final pixels: camera basis,
blend mode, texture contents, lighting and postprocessing still affect the image.
