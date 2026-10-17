# Original car lighting and material delivery

Sources: matched retail PC executable, static inspection; installed light-material
records and paint attributes; `dbalatoni13/nfsmw` (CC0, reference only).
See [provenance](../provenance/car-lighting.md). This supersedes the guessed rig
and unconfirmed material mapping in [car-assembly.md](car-assembly.md) section8
for the selectable original modes. Custom Glossy and Simple retain their behavior.

## Three-light rig

The PC car path consumes the first three entries of its shaper rig, not SH9.
The gameplay rig has three sun-relative entries. Each row is theta degrees,
phi degrees, RGB, intensity:

| Light | Theta | Phi | RGB | Intensity |
|---|---:|---:|---|---:|
| 0 | 0 | 350 | .85, .90, 1 | .83 |
| 1 | 50 | 220 | .55, .60, 1 | .40 |
| 2 | 310 | 220 | .65, .70, 1 | .45 |

The frontend quick-race/default rig instead uses world-space entries:

| Light | Theta | Phi | RGB | Intensity |
|---|---:|---:|---|---:|
| 0 | 90 | 48 | 1, 1, .95 | .98 |
| 1 | 134 | 185 | .83, .83, 1 | .50 |
| 2 | 27 | 175 | .83, .83, 1 | .35 |

Car-lot, safehouse and shop defaults match these entries in the inspected PC
image; backroom negates theta. Context selection is separate from car identity.
The remake's car viewer uses the default frontend rig; it does not reproduce
every career frontend lighting scene.

For a world entry, construct spherical direction with azimuth theta+180 and
polar angle90-phi. For a sun-relative entry, first convert the direction towards
the sun to azimuth A and polar angle P. Construct azimuth A+theta+180 and polar
angle -P-phi. Normalize the result. RGB is authored RGB times intensity; ambient
is zero. Direction matrices rotate into each car part's local space; a rigid
world-space equivalent is sufficient for the shader. Nonuniform transforms need
separate validation. Recompute directions from current sun state every frame,
including when custom shadows, contact shadows and skylight are disabled.

The original converts angles through unsigned16-bit engine angles and uses
lookup trigonometry. The initial remake retains angle quantization with standard
floating-point trigonometry; table/atan approximation differences are unverified.
There is no per-frame increment or smoothing in the rig.

## Material constants

Resolve each group's authored light material. `CARSKIN` resolves the selected
paint's `LIGHT_MATERIAL_NAME`; other hashes retain their own profiles, including
chrome and rims. In original modes, `WINDSHIELD` resolves the selected window
tint part's `LIGHT_MATERIAL_NAME` (part slot131), including preset overrides.
Custom Glossy retains its existing authored windshield profile. Unknown
materials use a visible documented fallback.

For RGB diffuse and specular, minimum = MinScale times MinRGB, range = MaxScale
times MaxRGB minus minimum. Diffuse minimum alpha is unscaled MinAlpha; alpha
range is .25 times (MaxAlpha-MinAlpha). Environment ramps use the first color
channel because the selected original vertex programs read only `.x`.
SpecularPower and EnvmapPower retain their authored values and verified runtime
global multipliers (initially one). MetallicScale/SpecularHotSpot are not consumed
by these selected programs. Original shader families remain separate choices;
their arithmetic is specified in the original car shader specification.

All specular minimum/range RGB also multiply the authored `CarSpecScale`
timeofdaylighting scalar. It follows the same clear/overcast and high/low-sun
blend as world atmosphere. Deliver this as frame state, avoiding material
recreation when the sun or cloud amount changes. Invalid or missing scalar
data uses neutral one and remains a documented unavailable input.

The standalone car viewer samples authored presets at its fixed preview sun
and clear-cloud setting when an install source exists. That selected preview
context is not a claim of exact career frontend time/weather state.

## Live mode changes

A settings change rebuilds the render appearance of already loaded player,
traffic, cop, trailer and helicopter rigs. Shared rig identity, source geometry
and actor physics remain intact. Repeating the current mode does not upload
resources again. Detached vehicle fragments retain mesh snapshots; replace
their mesh references by corresponding rebuilt part meshes before drawing,
preserving transforms and physics. Paused settings refresh performs these
appearance updates without advancing simulation.

## Camera, reflection and remaining limits

Selected CAR programs receive actual camera position transformed into each
part's local space. A separate dynamic-light context constructs a clamped eye
for its environment matrix (gameplay2.78, frontend2.78..8); CAR's raw eye upload
does not use that clamped value. Do not apply that clamp to CAR shader facing.

Original gameplay and frontend select different generated environment textures.
The remake's existing world capture/procedural viewer environment remains a
separate reconstruction input; original capture orientation, update cadence,
projected shadow textures and exact frontend context are remaining parity gaps.
Skin gloss alpha and customization textures retain
their existing limitations unless independently specified and validated.

## Validation

Test original material mapping without changing custom conversion, rig intensity
and orientation changes, frontend independence from sun direction, cloud/dusk
CarSpecScale blending and invalid inputs. Installed scene checks must confirm
original rig delivery even with every custom lighting option off and changing
sun state. Original shader fixtures validate arithmetic separately from these
runtime input checks. No static evidence alone establishes visual parity.
