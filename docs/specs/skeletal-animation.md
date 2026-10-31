# Skeletal animation runtime (`blackbox-skeleton`)

An engine-generic runtime for a skinned, animated character: a bone hierarchy, sampled clips, pose blending, blend trees
as data, two-bone IK and CPU linear-blend skinning. It knows no game and no file format of a game; converters (for RIDE 6:
`mods/ride6-bikes/src/rider/`) write its [pack](../formats/ride6-rider.md) (section "Runtime pack").
The maths are the standard ones (quaternion slerp, the matrix palette of linear-blend skinning, the law of cosines for a
two-bone limb); nothing in the crate comes from a game's code. Code: `libs/blackbox-skeleton/src/`.

## Data model

- `Transform { rotation: Quat, translation: Vec3, scale: Vec3 }`. A point maps as `p' = R (S p) + T`. `a.compose(b)` is the
  composition `a` after `b` (parent times child): `R = Ra Rb`, `T = Ra (Sa Tb) + Ta`, `S = Sa * Sb` (per axis; shear is
  dropped, as in Unreal's `FTransform`). `inverse` is exact for a uniform scale only; skinning inverts matrices instead.
  `lerp` is slerp for the rotation (shortest arc) and linear for translation and scale.
- `Skeleton`: bones with a parent that comes earlier, a name (unique) and a rest transform relative to the parent.
  `component_transforms` walks the list once. `branch_mask(root, w)` gives a per-bone weight for layering.
- `Pose`: one local `Transform` per bone. `blend(a, b, w)`, `blend_masked(a, b, mask, w)` (weight `mask[i] * w` per bone),
  `add_additive(delta, w)`: `R = slerp(I, Rd, w) * Rb`, `T = Tb + Td w`, `S = Sb * lerp(1, Sd, w)`.

The crate is unit- and handedness-agnostic: the converter picks the frame (RIDE 6: car frame, metres).

## Clips

A clip has a `length` in seconds, `tracks` (bone index and three channels), `additive`, `root_motion` (informational) and
`notifies`. A channel is `None` (the pose keeps its value), `Constant`, `Uniform` (n values from time 0 to `length`: the
position is `t / length * (n - 1)`, interpolated between the two neighbours; a clamp holds the ends) or `Keyed` (sorted
times, held before the first and after the last key).

- `sample(skeleton, t, looping)`: starts from the rest pose (identity for an additive clip), then `apply` overwrites the
  channels the clip has. Time wraps by `rem_euclid(length)` when looping, otherwise clamps.
- `notifies_between(from, to)`: notifies with `from < time <= to`; when `to < from` the clip looped, so
  `(from, length]` then `[0, to]`.
- `to_additive(skeleton, base, frames)`: samples the clip at `frames` even steps and stores per bone
  `R = pose.R * base.R^-1`, `T = pose.T - base.T`, `S = pose.S / base.S`; bones that never differ get no track. Applying the
  result at weight 1 on `base` gives the clip back (tested).

## Blend trees

`Graph { param_names, root: Node }`; a caller fills `params: &[f32]` (index = position in `param_names`) and calls
`evaluate(skeleton, clips, params, time)`.

`Value` is a constant or `clamp(param * scale + bias, min, max)`; `Value::remap(i, (a0, a1), (b0, b1))` maps `a0..a1` to
`b0..b1` and clamps. `TimeMode` is `Loop { rate }` (clip time `time * rate`, looped), `Fraction(value)` (clip time
`clamp(value, 0, 1) * length`, an explicit time) or `Hold` (frame 0).

| Node | Result |
|---|---|
| `Clip { clip, time }` | the clip's pose; a bad index gives the rest pose |
| `Blend { a, b, weight }` | `a.blend(b, clamp(weight, 0, 1))` |
| `ThreeWay { negative, zero, positive, param }` | `p = clamp(param, -1, 1)`; `zero` when 0, else `zero` blended toward `negative` (p < 0) or `positive` (p > 0) by `|p|` |
| `Additive { base, delta, weight }` | `base` plus `delta` (an additive pose) |
| `Layer { base, layer, mask, weight }` | `base.blend_masked(layer, mask, weight)` |
| `Space { space, x, y, time }` | the blend space's samples at bilinear weights, all at the same phase |
| `Select { options, index }` | the option at `round(index)` clamped |

`BlendSpace` has two axes (`min`, `max`, `divisions` cells, so `divisions + 1` positions; 0 divisions ignores the axis)
and samples at grid positions. `weights(x, y)` clamps the inputs, takes the four corners of the cell, drops corners
without a sample and renormalises. A space plays its clips in sync: the phase is `time * rate / mean length` (mean
weighted by the weights), and each clip is sampled at `phase * its length`.
`validate` checks clip indices, parameter indices and mask lengths.

## Two-bone IK

`TwoBoneChain { root, mid, tip }` (`mid` child of `root`, `tip` child of `mid`). `solve_positions(root, mid, tip, target,
pole)`: with `a`, `b` the bone lengths and `d = clamp(|target - root|, |a - b| + e, a + b - e)` (`e = 1e-4 (a + b)`), the
middle joint is `root + dir * a cos A + bend * a sin A` where `cos A = (a^2 + d^2 - b^2) / (2 a d)`, `dir` points at the
target, and `bend` is the part of `pole - root` (falling back to the current bend) perpendicular to `dir`. The tip goes to
`root + dir * d`. `apply_two_bone` turns that into two rotations (`from_rotation_arc` about the root, then about the middle
joint), scales them by `alpha` (slerp from identity), writes them to the local rotations and optionally sets the tip's
component-space orientation. Unit scale is assumed.

## Skinning

`inverse_bind_from_rest(parents, rest)` builds the inverse of each mesh bone's bind component transform.
`skin_matrices(component, bone_map, inverse_bind)` gives `component[bone_map[i]] * inverse_bind[i]` (identity when the
mesh bone is not in the skeleton). `skin_vertices` blends the matrices by the vertex weights, normalised by their sum (up
to 8 influences), applies the blend to the position and rotates the normal by it (exact for uniform scale); a vertex with
no weight stays in bind position.

## Tests

Transform inverse, composition against matrices and lerp; skeleton order and duplicate checks; masked blend and additive;
uniform and keyed sampling, wrapping, notifies across a loop, additive round trip; blend-space weights; each blend-tree
node; graph JSON round trip; two-bone IK reaching, stretching, pole side, alpha 0, tip rotation; skinning at rest, under
rotation, weight normalisation; pack and clip round trips and truncation at every length.
