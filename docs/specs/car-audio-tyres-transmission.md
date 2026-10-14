# Authored tyre and ordinary transmission audio

Behavioral specification, researched 2026-10-14. Sources and limits:
[provenance](../provenance/car-audio-tyres-transmission.md). No game assets or
reference implementation are redistributed.

## Tyres

**[verified installed structure]** The selected audiosystem AEMS_SkidBanks entry
contains FX_SKID: 22 class words, seven players, ordinary kind-zero same-bank
sample entries. Load the existing selected level-zero bank; do not infer a tyre
upgrade bank change from performance upgrades. The saved reference clamps its
audio bank level to 0..1 but does not establish MW PC's producer of that level.
The adapter accepts an explicit bank path and skid type; normal driving uses
type0. Drift type1 is an explicit caller input, not a guessed driving detector.

**[reference producer]** Class words in order:

| Words | Meaning |
|---|---|
| 0,1,2 | raw mixer volumes forward(slot5), side(slot7), backward(slot6) |
| 3,4,5 | azimuth(slot1), skid type, raw pitch offset(slot8) |
| 6,7,8 | hop(initial0), truncated speed mph, opposite side(initial0) |
| 9,10,11 | truncated understeer, oversteer, maximum retained left/right Aud_Skid_Type |
| 12,13,14 | front signed forward slip, signed lateral slip, load |
| 15,16,17 | rear signed forward slip, signed lateral slip, load |
| 18,19,20,21 | lowpass25000, highpass0, dry32767, raw mixer wet(slot9) |

These volumes already include the authored mixer mapping. Do not multiply by
the generic skid loop volume or mixer makeup a second time.

**[reference producer]** Wheel order is front-left, front-right, rear-right,
rear-left. Cut a symmetric deadzone of 0.2*tolerated slip from physics forward
slip. Scale the result by200 and clamp to signed[-1023,1023]; scale negative
physics lateral skid by81 and clamp to the same range. Load ramps linearly from
4000N ->0 to10000N ->1023, clamped. Average both wheels of each axle, preserving
sign and including their values even when only one wheel is grounded. Truncate
the resulting axle values. Clear all six axle words only when every wheel is
airborne. Understeer/oversteer must come from physics, not reconstructed angles.
Retained terrain types, including blown-tire replacement, belong to the caller's
side-terrain controller; the adapter consumes its resolved left/right types.

**[reference producer; PC packet scale observed]** Understeer and oversteer are
fractions0..1. Both return0 unless signed vehicle speed>0 and rigid-body speed
magnitude>1m/s. Understeer averages the raw signed lateral skid of front wheels
and their actual steering angles; when mean steer * mean skid <0, its value is
min(abs(mean skid)/body speed,1), otherwise0. Oversteer is
min(abs(mean rear-wheel skid)/body speed,1), without a steering-sign test.
Use the signed lateral contact speed from the wheel physics, before the sound
packet's negation. A sideways-force magnitude or square-rooted slip cannot
recover that sign. The game adapter therefore forwards WheelState.lateral_speed
into WheelInput.skid. FX_SKID still truncates these fractions directly to integer
words; do not convert them to percentages or Q15. In one matched PC driving
recording,3,561 packets have maxima0.636829/0.532310, so both truncated fields
remain0 even during observed sliding. This supports scale, not full calculation
equivalence for every physics situation.

The dynamic mixer signals use the absolute maximum front/rear *signed axle
averages*, shifted by5 and clampedQ15. The same forward result goes to object
inputs0 and2; do not split positive forward and negative backward slip. Input1
smooths absolute lateral maximum by500. Input3 smooths max(forward,lateral) by3000;
it is not the separately calculated axle-load class word. Smoothing is per
controller update; the exact matched PC task-order/previous-field lag remains
unverified. The existing generic controller supplies fallback loops and
some mixer signals; its absolute-per-wheel averaging differs from this signed
average. Integration must publish the authored signed signals when available.
The smoothing operation advances by at most the given integer amount toward
its target, symmetrically, with no interpolation factor or dt multiplication in
the original helper. The port feeds this operation through its sound controller's
60Hz clock; zero-tick frames retain the signal and multi-tick frames repeat the
operation for each owed tick (at most12 per update, retaining debt). The choice
of60Hz is the existing port clock policy, not a newly verified native audio rate.

## Ordinary transmission

**[reference producer]** Local-player engines with engineaudio.Tranny enabled
instantiate ENGINE/CAR_TRANNY.abk, class CAR_TRANNY. It has nine words:
car_class, magnitude, volume, azimuth, pitch_offset, lowpass, wet, dry, single_shot.
Magnitude is truncate(signed forward speed m/s *15), then clamped0..10000 by
the matched PC class setter. Reverse therefore produces0, without inferring
forward speed from gear. Volume is the active engine object's slot3, bounded
0..32787 (the observed PC setter's unusual upper bound); azimuth slot0 is bounded
0..65536, decoded pitch-offset slot4 to−16383..16383, lowpass slot5 to0..25000; tunnel
controller supplies wet/dry. single_shot is1 when the shifting controller is
active, otherwise0. Car class is the selected engineaudio.CarID. Forward speed
is a signed projection supplied by physics; speed magnitude and gear sign are
insufficient substitutes. This graph is separate from the reverse whine.
Matched PC bounds supersede unbounded console producer writes; see
[car audio environment §3](car-audio-environment.md#3-transmission-class-decomp-pc-checked).

## Lifecycle and failure behavior

One persistent module per family/car, using the shared AEMS interpreter and Sfx
output. Commit parameters through the existing sound timer. Drop stops authored
voices. Missing bank/module/sample or host failure drops the module, logs once,
and permits generic commands; remove generic skid commands only while the authored
skid adapter succeeds. A failure does not silently suppress the fallback family.
Changing selected bank or engine profile reloads the owning car audio.

## Evidence bounds

Installed-bank structure and interpreter compatibility are measured. The saved
producer reference establishes numeric contracts but is not a complete matched
MW PC behavioral trace. Captured original players request skid and transmission
voices; allocator-reuse metadata is quarantined and pre-existing transmission
voices lack captured starts. RNG sequences, original sound timer, skid-bank
upgrade selection, mixer signal ordering, environmental DSP and absolute gain
remain bounded limitations. Install tests verify actual graph decoding/PCM,
synthetic tests verify producer vectors and fallback suppression behavior.
Installed render-rate regressions feed identical constant telemetry over4s at
5/60/144/360FPS and44.1/48kHz. They compare steady-state PCM RMS after1s warmup,
voice-start counts and player capacity, then drop at3s and require silence after
3.5s. The15% RMS and two-start tolerances are port regression acceptance bounds,
not measured original-game tolerances. These checks are pending serialized local
validation until their run results are recorded separately.
