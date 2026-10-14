# Car audio contact and playback contract

Sources: the CC0 dbalatoni13/nfsmw reference (SoundCollision,
SFXObj_Collision, Physics/Behaviors/Effects and CARSFX_Engine),
https://github.com/dbalatoni13/nfsmw; existing engine-sound,
engine-sound-effects, engine-sound-ginsu and car-sound-mixer specs;
static comparison with the original PC 1.3 executable. No source is copied.

## Contact records

Capture each resolved contact before aggregation discards its correspondence:
kind (world, object or ground), other-body identity, material, point, normal,
relative point velocity and per-actor impulse divided by effective contact mass.
Preserve that velocity change separately from the normal impulse in N s.
Original reference response can temporarily modify mass; use the response's
per-actor value for object contacts, rather than dividing by permanent mass.

An impact and a sliding contact are separate outputs. An impact gates on the full
relative velocity squared and uses the selected linkage's response range. A scrape uses relative
tangent speed, including lateral travel and rotation, even without a new normal
response. Do not derive scrape strength from vehicle forward speed.

Each simulation tick publishes its own telemetry and contacts. This integration
uses the checkout's fixed simulation step for contact expiry and divides elapsed
audio time across the recorded states. A future variable-step adapter must supply
its actual simulation delta instead.
Consumers process the ordered history once, including contacts that begin and
end between two render frames. Render frames with no new tick do not replay an
impact. PCM playback continues on the audio sample clock. Simulation slowdown
does not by itself imply the original sound device uses slowed time or pitch.

## Linkage resolution

The pvehicle action arrays contain 32-byte records. In addition to the selector
and effect references, retain the final two floats as the minimum and maximum
response values. Resolve a matching material and its parent surfaces before
using the default record. Missing actions or records produce no authored effect;
an arbitrary first record is not a valid substitute.

Retain audioimpact description flags, stitch lists and their Q15 volumes.
Retain audioscrape CSIS_EFFECT identifiers. A scrape identifier enables the
authored effect; it is not itself executable code or a bank sample index.

The reference impact gate requires relative velocity squared above 1 and a
positive ramp of the response between the linkage limits. Do not apply another
mass-based volume floor or global N s normalization. Low-intensity rejection
uses the selected collection's WALL and SMOKABLE flags together, with intensity
quantized to 0..127. These gates and the linkage ramp are confirmed in the matched
PC routine. Ranges no wider than approximately f32 1e-6 are invalid.

The reference scrape gate uses tangent speed above 1 m/s, a valid increasing
linkage range and speed at least its minimum. Intensity is the clamped ramp
with a minimum of 0.1 while active. Contacts refresh a 0.2 s simulation lifetime;
the release counter expires after 0.25 s of audio-state time, strictly below zero.
Both constants and boundaries are confirmed in PC control flow. The release
counter is a lifetime, not a volume multiplier. The backend adds a 40 ms stop
envelope to avoid a discontinuity; that envelope is a port decision.

Bottom-out classification requires more than two grounded wheels and the body's
up vector dotted with the contact normal above 0.2. Its intensity uses normal
closing speed; its source point is below and toward the rear of the body. Other
resolved contacts retain their actual point. Position-controller distance and
azimuth use the planar render axes, independently of height.

## Collision mix and authored modules

Collision state 7 distinguishes car-car, car and movable-world events through
description inputs. Intensity is published as int(magnitude*127)*255.
Output volume multiplies the selected collection's stitch-level volume.
Output pitch, azimuth and wet level belong to that collision, not a generic
player-engine mixer frame. Scrapes publish those parameters plus intensity and
a low-pass value to the authored FX_Scrape module. Preserve the module's sample
selection; success executing an engine module does not prove scrape coverage.

Maintain an active scrape voice or module across contacts; update its parameters
rather than creating a voice per tick. Release uses the documented lifetime.
Do not accidentally compound the original effect fade with multiple generic
stop fades. Distinct simultaneous sources may overlap; repeated probes of the
same body contact should not create duplicate independent impacts.

## Engine and control integration

Consume both the engine controller's low-pass output and the map's engine
filter output. Match units before combining them. Retain the current researched
redline takeover and the fallback when the sample layer is unavailable.

AEMS RPM follows the synthesizer's actual pitch/frequency, including its
recording-range behavior and below-minimum playback ratio. Dynamic mixer pitch
is a separate parameter; do not apply it twice to the CAR RPM input.

Engine requests contain coherent frequency, pitch and volume state. No callback
may observe fields from different updates. Frequency-request revisions reflect
controller updates, not arbitrary callback buffer sizes. A stalled callback
resumes from the latest coherent request, without replaying an unbounded backlog.
The audio callback
must not block on the game thread or allocate for normal parameter changes.

Populate turbo tuning from the selected turbosfx data. Use decoded sample
duration for blow-off lifetime where available. Keep the documented
throttle-based audio torque rather than substituting physical engine torque.

## Wind and ordinary road modules

Reference-derived wind parameters use full velocity magnitude: clamp below 2 m/s
to 2 and above 40 to 39.95 (exactly 40 stays 40). Intensity is trunc(v/40*1040).
Multiply trunc(v/40*32767) into the separate left, right and rumble map volumes
with Q15 arithmetic. The authored FX_WIND module receives those values, the two
azimuths and Q12 pitch; it selects its own loops. No synthetic wind pitch or
average channel volume replaces those parameters.

Ordinary road IDs 0..7 run FX_ROADNOISE once per side. Type is zero and ID selects
the terrain loop. Keep generated volume, Q12 pitch, side azimuth and map wet level;
gravel enables secondary noise. A side retains its prior loop while its wheels
disagree. Airborne sides have zero ordinary-loop volume. ID8 retains the existing
stitch-loop path. Road transitions and the additional wet layer remain separate
work; no reference-PC numeric equivalence is claimed for this module wiring.

## Verification and remaining boundaries

Replay identical timestamped inputs at 5..360 render FPS and across long frames,
different device rates and callback sizes. Verify event counts, material
selection, contact lifecycle, controller time and actual sample output.
Export unclipped float evidence before integer WAV quantization. Measure
combined headroom and voice failures, rather than using audibility as proof
that the mix cannot clip.

Exact PC spatial attenuation, environmental reflections/reverb, Speedbreaker
audio treatment and unsupported gameplay state producers remain separate
research questions. Unknown behavior is documented, not replaced with an
unlabeled parity claim.

The host applies azimuth panning and guarded low/high-pass filters. Authored
dry/wet words are retained but environmental reverb is not yet rendered. Kira's
filter transfer function is a backend approximation, not a measured PC match.
Skids, transmission/whine, road transitions, wet-road layers and component-specific
upgrade crossfades remain outside this batch. Discrete one-shots are delayed by
their position within the ordered render batch. Continuous authored modules still
receive batched parameter updates on the game thread; their exact within-frame
waveform at very low render FPS has not been established as an original match.
