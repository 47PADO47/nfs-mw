# Original gameplay camera data and projection

## Sources and scope

**[PC executable checked]** The native camera-anchor POV getter resolves seven
`ecar` reference fields and consumes the selected `camerainfo` collection.
This specification replaces the earlier guessed collection-name lookup and
vertical-FOV interpretation in `camera-views.md`. Provenance is recorded in
`docs/provenance/original-camera-data.md`.

## Profile selection

The seven POV indices resolve these fields, in order:

| Index | View | `ecar` field |
|---|---|---|
| 0 | bumper | `CameraInfo_Bumper` |
| 1 | hood | `CameraInfo_Hood` |
| 2 | close | `CameraInfo_Close` |
| 3 | far | `CameraInfo_Far` |
| 4 | superfar | `CameraInfo_SuperFar` |
| 5 | drift | `CameraInfo_Drift` |
| 6 | pursuit | `CameraInfo_Pursuit` |

Resolve the reference's class and collection keys, including inherited `ecar`
attributes. A car may refer to a collection whose name does not contain its own
name. Never synthesize a `<car>_<view>` name to choose a native profile.
The remake can retain generic views when no car collection or usable reference
exists; this is a host recovery policy, not verified original malformed-data
behavior.

Each profile has float pairs `ANGLE`, `FOV`, `HEIGHT`, `LAG`, `LATOFFSET`,
`STIFFNESS`, and boolean pairs `TILTING`, `SELECTABLE`. Index one is selected
only for horizontal split-screen viewport mode three; other modes use zero.
The pairs are not vehicle-speed endpoints. Keep both values in the data reader.
The text `CollectionName` identifies the record and does not control the pose.

`SELECTABLE` marks ordinary camera-cycle availability. Internal superfar, drift
and pursuit profiles remain loadable even when not selectable. Selection and
forced camera use are separate decisions. `TILTING` enables the native up-vector
modifier. `STIFFNESS` is a native controller input, not an interpolation fraction
between fullscreen and split-screen fields.

## Native target coordinates

Local axes are forward, lateral and up. Before acceleration/temporal modifiers,
the eye offset is `(LAG, LATOFFSET, HEIGHT)`. The outside views (indices two
through six) target `(0, 0, HEIGHT + tan(ANGLE) * LAG)`. Bumper and hood target
`(LAG + 1, 0, HEIGHT - tan(ANGLE))`. Angle uses the original unsigned full-turn
angle conversion before native trigonometry.

No half-car-length adjustment is present in this target calculation. The native
vehicle producer forms the anchor translation from rigid-body position plus the
rotated local vector `(0, center_of_gravity.y, center_of_gravity.z)` in simulation
coordinates. It suppresses the lateral center-of-gravity component. Simulation
vectors then map to render-world vectors as `(z, -x, y)`. The remake must pass
this physical body anchor, rather than subtracting the collision-box pivot as
its model-drawing path does. Adding half a car length is not a substitute for
that conversion.
The Speedbreaker target factor divides LAG and multiplies FOV before these
calculations, while HEIGHT and ANGLE retain their selected values.

## PC projection

The PC render path treats the stored angle as the horizontal field of view.
It computes a separate vertical angle by multiplying the horizontal angle by
`viewport_height / viewport_width`. Main/player views additionally multiply
that ratio by the stock floating-point constant `1.15`. This is an angle-space
operation, not the usual tangent-space aspect correction.

After half-angle conversion to the original integer angle units, horizontal
scale is `1 / tan(horizontal_half_angle)` and vertical scale magnitude is
`1 / tan(vertical_half_angle)`. D3D9's viewport convention gives the original
vertical matrix coefficient a negative sign; the remake renderer may express
the corresponding optical scales in its own coordinate/depth convention.
The two axes must remain independent. Passing the asset FOV unchanged as a
vertical perspective angle does not reproduce this projection.

The full-angle output is quantized to an unsigned 16-bit turn. The two native
half-angle conversions truncate independently toward zero. The base mover uses
a near plane of `0.5` metres. The renderer's infinite reverse-depth convention
can remain in use while preserving these optical axes and near-plane distance.

Before the renderer receives the field of view, the cubic mover performs an
additional angle round trip: truncate the current scalar to a 16-bit angle,
halve it, evaluate tangent, convert that tangent back to angle, and double.
Its sine uses a ninth-order Taylor polynomial with single-precision stored
coefficients and the full-turn angle scale. The tangent's numerator is stored
as a float before division. The inverse-angle conversion quantizes its ratio
to 16 fractional bits and interpolates integer angle endpoints at ratios
`index / 256`; endpoints equal rounded mathematical arctangents. Those endpoints
can be generated from the equation, without distributing an original lookup
table. The round trip can reduce FOV by a few angle units, so it is a separate
step from the render projection's independent half-angle quantization.

## Playable-reference widescreen projection

The user-selected default for original gameplay modes matches the playable
reference's WidescreenFix configuration: `FixFOV=1`, `Scaling=0`. Its separate
`CAMERA.Enable=0` setting disables added camera rotation, not FOV correction.
Keep the native profile, pose, angle finalization and near plane above; change
only the final optical scales. Custom Chase and Dynamic FOV retain their own
controllers and projection.

For finalized unsigned full-angle `F` and display aspect `A = width / height`,
the reference gameplay projection uses:

```
horizontal_half = truncate(F * 0.5)
vertical_half = truncate(F * float(1.15) * float(0.43434))
x_scale = ((4.0 / 3.0) / A) / tan(horizontal_half)
y_scale_magnitude = float(1.215) / tan(vertical_half)
```

Half angles use unsigned turn units. Constants are single precision, with the
vertical product evaluated before the truncating conversion. These scales
preserve vertical framing across display aspect ratios; horizontal framing
expands with aspect. Express both independently in the renderer using vertical
FOV `2 * atan(1 / y_scale_magnitude)` and effective aspect
`y_scale_magnitude / x_scale`. Retain the native near plane and supplied up
vector. The stock PC projection remains separately tested as a native oracle,
rather than silently changing its expected values to match the reference.

At 1920×1080 and finalized angle14196 (approximately78° authored FOV), the
reference optical FOV is approximately94.37° horizontal and67.27° vertical.
Its projected dimensions differ substantially from the unmodified PC lens;
the `Scaling=0` label does not mean that FOV correction is disabled. Installed
original-game files need no changes for this choice.

## Verification boundaries

Isolated unchanged native spans were executed with synthetic camera inputs for
18 projection cases (three FOVs, three aspect ratios, main and auxiliary views)
and five outside/inside target cases, plus twelve degree-to-angle cases.
They establish equations and axis ordering,
not matched live driving behavior. Native integer-angle quantization and lookup
table trigonometry introduce small differences from host floating-point tangent.
Full follow, collision, lookback and car visibility belong to the mover pipeline.
