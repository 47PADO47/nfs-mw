# Original camera noise

**[PC binary checked]** The original Cubic mover applies its noise after
constructing the follow matrix. This is independent of the custom Chase camera
shake. The native implementation uses deterministic interpolated integer noise,
not a sine wave or a random sample per display frame.

## Waveform

For integer lattice point `i`, use wrapping 32-bit arithmetic:

1. `n = (i << 13) XOR i`.
2. `h = (n * (n*n*15731 + 789221) + 1376312589) AND 2147483647`.
3. The lattice value is `1 - h/1073741824`.

Linearly interpolate the two adjacent lattice values using floor of the phase,
including negative phases. Sum six octaves whose frequency doubles and amplitude
halves from initial one. Native octave accumulation is stored as a 32-bit float
at every octave; retain higher precision inside the lattice interpolation.

## Vehicle and terrain response

Linearly interpolate the following scalar responses at speeds 0,20,40,60,80 m/s,
saturating outside that range. They are behavioral response parameters, not
the viewport pairs in `camerainfo`.

| Speed | Motion amplitude | Terrain amplitude | Motion frequency | Terrain frequency |
|---|---|---|---|---|
| 0 | 0 | 0 | 0.6 | 1 |
| 20 | 0.005 | 0.5 | 1.1 | 1.1 |
| 40 | 0.03 | 0.8 | 1.2 | 1.2 |
| 60 | 0.07 | 1 | 1.3 | 1.3 |
| 80 | 0.02 | 1 | 1 | 1.4 |

Drag multiplies motion amplitude by 1.5. Above 5 m/s, let
`a=clamp(forward_acceleration,0,20)*0.05`; add `0.15*a` to motion amplitude and
blend motion frequency toward 0.5 by `a`. Over-rev adds 0.15 amplitude and
multiplies motion frequency by 0.7. Above 5 m/s, active nitrous replaces motion
amplitude/frequency with 0.3/2; braking then takes precedence with 0.3/5.

Terrain amplitude and frequency multiply the matching two `CAMERA_NOISE` values
from the current `simsurface`. Missing attributes use the native scalar-zero
fallback. Bumper/Hood multiply both motion and terrain amplitudes by 0.25.

For four channels (screen horizontal translation, screen vertical translation,
pitch degrees, yaw degrees), the motion frequency multipliers are
`(1.8,2,2.125,2)` and amplitude multipliers `(0.03,0.025,0.68,0.28)`.
The terrain multipliers are `(3,5,7,5.5)` and `(0.007,0.01,0.3,0.4)`.
Evaluate each waveform at real elapsed seconds times its frequency, multiply
its amplitude, then add the two corresponding channels.

Convert pitch/yaw degrees to whole binary-angle units using the native two-stage
truncation. Scale their sine by `sin(current_FOV/2)/sin(reference_FOV/2)`, then
take inverse sine back to angle units. The reference full FOV is 10922 units.
Compose local screen translation, pitch rotation and yaw rotation in native
view-matrix order. In the renderer's right-handed column-vector convention this
is `Ry(-yaw) * Rx(pitch) * Translation(horizontal,-vertical,0) * view`.
Keep target distance when deriving the resulting eye/target/up.

## Helicopter response and retained channel

The ordinary per-view handheld contribution is zero in this executable's
default runtime table. The helicopter contribution is separate: each active
police helicopter within horizontal distance 40 m of the previous published
camera eye applies amplitude
`(1-clamp(speed/30,0,1)*0.8)*(1-distance/40)`.
Its frequency multipliers are `(3.141,2.971,0.84234,0.92345)` and amplitudes
`(0.01,0.05,1.1,2.7)`. Each helicopter composes an additional noise transform.
Tunnel/overpass cover suppresses this helicopter stage entirely.
This path replaces the first channel group and retains the previous camera
update's second terrain group; it runs before the current vehicle noise pass.

## Timing and limits

Use retained unscaled camera-clock seconds so Speedbreaker changes the vehicle
inputs while this phase continues on the original real-time clock. Camera reset
clears retained channel parameters. Resetting phase on a newly created host
controller is a host initialization choice; it does not reproduce the original
process's absolute real-timer phase. Native trigonometric lookup approximations
may differ slightly from standard library trigonometry.

The waveform has isolated unchanged-native regression vectors. Matrix regression
vectors use unchanged native application code with only its external standard
4x4 multiplication primitive supplied by the harness. These are synthetic
instruction checks; matching live scenes remain separate evidence.
