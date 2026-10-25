# Scenery sway (trees, bushes, boats)

- **Sources read:** the retail PC 1.3 `speed.exe` in Ghidra (proprietary, read for behaviour only): the weather
  update that advances the sway clock, the scenery instance draw that applies the sway, the rotation helpers it
  calls and the rain update that supplies the wind vector. dbalatoni13/nfsmw (CC0, decompiled):
  `Ecstasy/Ecstasy.hpp` (`eSolid::Flags`) and `Ecstasy/Ecstasy.cpp` (`SetupSceneryCullInfo`). ExOptsTeam's
  NFSMWExtraOptions (`WorldAnimationSpeed`, which writes the sway rate). No code copied.
- **Data inputs:** `eSolid::Flags` (u16 at 0x0E of `SolidInfo`, [../formats/models.md](../formats/models.md)); the
  instance `ExcludeFlags` and matrix ([../formats/maps.md](../formats/maps.md)); the `Wind` layer of the track's
  spatial parameter maps ([spatial-parameter-maps.md](spatial-parameter-maps.md)).

## Behaviour

The tree effect has no time or wind constant, so foliage does not move in a shader **[verified]**: the 31
embedded effects name no such parameter. The motion is a small rotation the CPU applies to the instance matrix of
flagged solids each time they are drawn.

**Which solids.** A solid sways when its `Flags` has bit `0x80` **[verified]**. The test is made on the solid of the
LOD slot the view chose, so a model can sway at one LOD and not at another. In the PC 1.3 install 31 of the 20,377
streamed world solids carry it: trees and bushes (`XT_` art: redwoods, cedars, maples, poplars, junipers, windbreak
rows, bush clusters) and boats (`XO_LAKEBOAT`, `XO_SAILBOATB_VC`, `XO_CANNERYBOAT`, `XO_TUGBOATSTEAMC`,
`XO_BOAT_SMLB`), whose sway reads as bobbing. Instances with flag `0x200` are drawn without a matrix and never
sway.

**Which views.** The draw applies the sway only in views whose scenery cull flags include `0x4000`
**[verified]**. The PC frame setup passes `0x6000` for the two player views (`0x1000` for every view while one
unidentified global is set), `0x4100` for the view drawn when the shadow detail is above 0 (the shadow-map view
**[unconfirmed]**), and no `0x4000` for the third view, the environment maps or the road reflection. So the player
sees the sway, mirrors and reflections show the resting pose.

**The clock.** A phase in degrees advances by `45 * dt` every in-game weather update and is set back to 0 once it
passes 360 **[verified]**. 45 is the global that Extra Options' `WorldAnimationSpeed` overwrites.

**The rotation.** For an instance drawn with world matrix `M` (row-vector convention, rows are the local axes, the
fourth row the position), with all angles below converted to the engine's 16-bit binary angle as
`units = trunc(trunc(degrees * 65536) / 360)` taken modulo 65536:

```
seed   = trunc(M.position.x * 60) rem 360                 (integer remainder: keeps the sign of x)
angle  = binary(1.5 * sin(binary(seed + phase)))           (at most 1.5 degrees)
W      = normalize(view wind vector), or (1, 0, 0) when it has no length or the view has none
L      = M's 3x3 with elements [0][1] and [1][0] negated   (the inverse of a pure yaw)
w      = W * L                                             (not normalized again)
S      = Axis(w, angle) * Rz(angle)
M'     = S * M
```

`Axis(v, a)` is the helper's axis rotation, with `c = cos a`, `s = sin a`, `t = 1 - c`:

```
[ t x x + c     t x y - z s   t x z + y s ]
[ t x y + z s   t y y + c     t y z - x s ]
[ t x z - y s   t y z + x s   t z z + c   ]
```

and `Rz(a)` is `[c s 0; -s c 0; 0 0 1]`. The seed spreads neighbouring trees over the cycle by their x position;
the 8-second period comes from the 45 degree per second rate.

**The wind vector.** The view's rain object owns it. Each rain update samples the `Wind` layer's field 1 (degrees)
at the view's position **[verified]**. The sample is low-passed by distance moved: the first sample is taken whole;
afterwards `value = p * sample + (1 - p) * value` with `p = moved / 50` when the position moved less than 50 units
since the last sample, else `p = 1`. With `units = binary(value)` the vector is `(sin, cos, 0) * 0.01` and is then
transformed by the inverse of the camera's rotation that the rain keeps (its translation cleared). With no `Wind`
layer the previous vector keeps being scaled by 0.01 and decays to zero, so the sway falls back to `(1, 0, 0)`.
Which way that camera matrix maps is not settled: the rain needs camera-space wind, so the vector is taken as the
wind in the camera's frame (right, up, forward) **[derived]**. The axis is therefore camera-relative, a quirk of
reusing the rain's vector.

## Constants

| Value | Meaning |
|---|---|
| `0x80` | `eSolid::Flags` bit: sways |
| `0x200` | instance flag: drawn without a matrix, never sways |
| `0x4000` | view cull flag: draw the sway |
| 45 degrees/s | sway clock rate (`WorldAnimationSpeed`) |
| 360 | clock wrap |
| 1.5 degrees | amplitude |
| 60 | seed scale on the instance x position |
| 50 | wind sample blend distance |
| 1 | `Wind` layer field holding the angle |

## The rewrite

- The rate is a layout constant; the world animation clock, which `WorldAnimationSpeed` already scales, drives it.
  The phase is `(clock * 45) mod 360` instead of a reset to 0, which differs from the original only by the
  fraction of one frame that the reset drops.
- The player views sway. The rear-view mirror, reflections and environment maps do not, as in the original.
- The casters the player view's pass adds for the sun sway with it. The fixed sun-shadow casters that the shadow
  feed keeps between scenery changes stay at rest, and the custom cascades are an enhancement in any case.
- The culling boxes are not grown for the sway; at 1.5 degrees the swing stays within a fraction of a unit of the
  stored boxes for most trees.

## How to check it

- Synthetic: a zero phase and a zero-x instance give `binary(0) = 0` and the identity; the sway never passes
  1.5 degrees about each of its two axes; instances 1.5 units either side of x = 0 sit at the peak and the trough;
  the result is a rotation (orthonormal, determinant 1) for a pure yaw `M`.
- Installed: L2RA's `Wind` layer exists with a float field 1, and the 31 flagged solids load with the flag.
- In the game, park facing a stand of redwoods or the boats at the docks and compare the period (8 seconds at
  the default rate) and the phase spread between neighbours against the original.
