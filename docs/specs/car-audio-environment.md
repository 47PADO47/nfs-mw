# Car engine environment, requested sends and output DSP

The environment lookup and wet output contract is extended by
[spatial parameter maps](spatial-parameter-maps.md),
[FXEDIT processing](fxedit-processor.md) and
[shared output](car-audio-output.md). Older pending-output notes below describe
the initial producer audit; current implementation limits live in the
[coverage ledger](../car-audio-coverage.md).

This separates the game's environment producers, the sample graphs' requested
voice properties, and the output processor that renders wet audio. A nonzero
requested send is not evidence that the original output actually processed it.

Sources: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0,
decompiled platform reference), `CARSFX_Engine`, `SFXCTL_Engine`,
`SFXCTL_HybridMotor`, `SFXCTL_Tunnel`, `SFXObj_Reverb`, the generated `CAR`
interfaces, and the common sound-library effect bus. The PC producer and level
table were independently checked against the matching original executable.
No source or installed assets were copied into this document. Provenance:
[car-audio-environment.md](../provenance/car-audio-environment.md).

## 1. Engine sample class producer **[decomp, PC checked]**

`CAR` contains 26 signed integer parameters. In addition to RPM, torque,
sample-layer volume, pitch offset and limiter level, a Ginsu-active engine sets:

| Index | Parameter | Producer |
|---|---|---|
| 13 | FX_DRY | AEMS environment dry Q15 minus32767, clamped−32767..0 |
| 14 | FX_AMOUNT | environment AEMS wet times engine map slot1 volume, shifted15, clamped0..32767 |
| 15 | AZIMUTH | engine map slot0 azimuth, clamped0..65536 |
| 21 | FILTER | lesser of engine map slot5 frequency and distance cutoff, clamped0..25000Hz |
| 22 | FILTER_RND | 11000 when the hybrid AEMS-filter flag is set, otherwise0 |
| 23 | FILTER_Dist | not updated by this producer; constructor value0 |
| 24 | FILTER_Trig | explicitly0 |

The dry class parameter is signed attenuation. It is different from the
positive Q15 dry gain requested by an individual player node after the class
graph runs. Do not apply a second linear attenuation to the raw class value.
Similarly, raw filter class parameters must pass through the installed graph;
they are not substitutes for each player's resulting low/high-pass requests.

The hybrid dual-engine update clears its AEMS-filter flag every update.
The single-engine update has no write to that flag in the inspected body.
No additional speed, limiter or random trigger is justified by that absence;
retain the flag explicitly and label any unsupported initial-state policy.

The console reference omits some of these assignments and describes different
dry setter bounds. The PC producer takes precedence for these parameters.

CAR pitch index16 retains the decoded mixer ratio times16383 minus16383,
bounded−16383..16383. At unity its value is0. Transmission pitch is different:
it receives the decoded Q12 ratio4096 at unity directly, bounded as §3 states.

## 2. Engine distance cutoff **[decomp, original numeric replay]**

The positional controller's camera distance is in centimetres. Let `d` be
that distance times0.01 metres, and `x = clamp((d−6.5)/44,0,1)`. Quantize
`x*32766` with the original float-to-integer conversion, evaluate mixer shape6
in hundredths of a decibel, then convert that result to Q15 with the original
quantized mixer conversion. The cutoff is the original integer conversion of
`Q15/32767*23275 + 725`, bounded0..32767. The final CAR filter also applies the
map-slot minimum and0..25000 bound from §1.

The lookup-table quantization matters: an analytic curve with a24000Hz near
endpoint is not the original numeric result. Full original-routine replay,
including its unmodified curve and level conversion calls, produced:

| Camera distance (m) | Cutoff (Hz) |
|---|---|
| 0,5.6,6.5 | 23920 |
| 10 | 18605 |
| 20 | 7398 |
| 30 | 2257 |
| 40 | 837 |
| 50.5,100 | 725 |

This proves these static numeric vectors, not live listening equivalence.
Use the actual positional-controller distance; an arbitrary fixed chase-camera
distance cannot represent all views.

The existing shared mixer gain table is a mathematical reconstruction and
differs from the original by small Q15 units. Current distance wiring reuses
that shared conversion rather than copying an extracted binary table; the
regression bounds the resulting difference by3Hz at these nine points.
This is an explicit numeric approximation, not an exact table-replay claim.
The shared cosine curve uses the independently verified divisor1022;
using1024 changes the intermediate curve and caused a161Hz near-camera
error. That was a curve reconstruction error, not the small gain-table gap.

## 3. Transmission class **[decomp, PC checked]**

`CAR_TRANNY` is created for player context when selected engine audio enables
`Tranny`. Constructor defaults: low-pass25000, wet0, dry32767, single-shot0.
Its nine parameters are class, magnitude, volume, azimuth, pitch offset,
low-pass, wet, dry and single-shot. Every producer update sets:

- Magnitude: forward speed times15, original integer conversion, clamp0..10000.
- Volume: engine map slot3, original class bound0..32787.
- Azimuth: engine map slot0, clamp0..65536.
- Pitch offset: decoded engine map pitch, clamp−16383..16383.
- Low-pass: engine map slot5 frequency, clamp0..25000.
- Wet and dry: AEMS environment Q15 outputs, each clamp0..32767. Wet here
  does not get the extra engine sample-layer slot1 multiplication from CAR.
- Single-shot:1 when the shifting controller is active, otherwise0.

These are class inputs; installed transmission graphs choose their own players,
samples, gain, pitch and environmental requests.

## 4. Environment level producer **[decomp, PC table checked]**

The tunnel controller maintains separate Ginsu/AEMS wet and dry outputs.
Targets use an authored12-row table. Wet/dry columns are hundredths of a dB;
fade columns are milliseconds. The matching PC table is:

| Zone | Ginsu wet | Ginsu dry | AEMS wet | AEMS dry | Fade out | Fade in |
|---|---|---|---|---|---|---|
| car-show small, bass, bass2; garage, small garage | −300 | 0 | −300 | 0 | 120 | 120 |
| tunnel | 0 | −200 | 0 | −200 | 300 | 120 |
| small tunnel | 0 | −200 | 0 | −200 | 120 | 120 |
| close hills | −900 | 0 | −900 | 0 | 120 | 120 |
| hills | −1200 | 0 | −1200 | 0 | 120 | 120 |
| open city, dense city | −900 | 0 | −900 | 0 | 120 | 120 |
| alley | −300 | 0 | −300 | 0 | 120 | 120 |

The numeric PC targets differ from the console reference's tunnel/hills/city
values. Convert them through the original mixer hundredths-dB conversion.
Apply the spatial parameter-map offset to wet only, bound−10000..0 before
conversion. Targets do not mean wet and dry are complementary mix percentages.

Changing environment starts a linear fade to wet0/dry1, using the current
row's fade-out time. Once complete and the preceding update's ramping flag
has cleared, the lead controller requests a global
processor reset/patch switch, starts the destination fade-in, and targets the
new row. Outside transitions, each current level approaches its target by up
to0.25 linear gain per controller update. Quantize interpolated levels toQ15.
Every second update increments each wet output by1, bounded32767, preserving
the original alternating low-bit update behavior.

The global reverb object multiplies both wet outputs by its map slot0Q15 when
enabled for the lead car outside front-end/car-show. This occurs after the
tunnel producer; engine consumers must see the resulting outputs.

## 5. Spatial zone and mixer context **[decomp]**

Tunnel detection checks authored track-path zones in priority order: tunnel,
overpass, small overpass, garage, dynamic. A zone is eligible when its elevation
is0 or at least the car's height. Dynamic zones with visit-info1 are excluded.
Tunnel maps to tunnel reverb; both overpasses to small tunnel; garage/dynamic
to garage. On exit, restore the spatial `Reverb` parameter-map zone.
That map supplies zone column0 and wet-offset column2. An invalid zone falls
back to small car-show in front-end, otherwise open city.

The tunnel mixer controller publishes tunnel/overpass/garage and in-tunnel
flags, occlusion, and player AEMS dry/wet. AI-racer/cop occlusion is a separate
bounded world-line test. Do not infer these from wheel surface or an arbitrary
height/roof heuristic. The spatial map/zone adapter is specified in
[spatial-parameter-maps.md](spatial-parameter-maps.md). A failed metadata load
still carries explicit unavailable context rather than an invented enclosure.

## 6. Requested wet versus effective processing **[decomp, bounded]**

The PC includes a software custom-effect graph path: the global effect object
loads authored `SOUND/FXEDIT` patches, the parser allocates stateful processors,
and a service routine runs their function pointers and mixes output buffers.
The per-voice wet setter converts0..127 to linear level, multiplies it by voice
gain, stores the effect-bus level and queues a platform update. This is stronger
evidence than an unused request field, but does not prove that a particular
captured session installed/enabled the patch or ran that service.

Each FXEDIT file contains a56-byte big-endian header, separate processor/link
lists for up to six channel layouts, variable-size processor records and
12-byte links. The installed15-file inventory parsed exactly to file ends.
Observed processor IDs are0,1,3,5,6,7,8,9. Verified bounded forms include
scalar gain(6), a stateful one-pole coefficient filter(1), a delay-buffer
feedback form(7), and a channel-routing form(9); complete numeric kernels,
remaining IDs, connection semantics and callback buffer layout still require
independent replay before implementing a matching interpreter.

A generic reverb preset is not those authored graphs. Preserve requested wet
and patch identity separately from effective backend capability. An unsupported
processor/patch must stay explicitly unsupported rather than silently being
replaced and described as original. Dry, azimuth and filter producers remain
useful independently of this missing wet processor.

## 7. Current output wiring **[decision, bounded]**

Ginsu carries the engine map azimuth to Kira's pan control, and uses the
environment dry level quantized as `Q15 >> 8`, divided by127. It retains
the existing hybrid/map low-pass; the newly reconstructed distance cutoff
is applied to CAR's graph filter parameter, not added a second time to Ginsu.
Center gain is preserved, with a constant-power stereo pan law independently
tested in the remake. Original speaker pan law and filter/DSP coefficients
remain unverified; this is useful explicit output wiring, not speaker equality.

CAR indices13/14/15/21 have the producers above. Index22 stays0 as an explicit
unsupported cold-start flag policy, with no invented trigger. Index23 keeps
constructor0 and24 is0. Environment requests use fixed60Hz updates with
retained fractional time/debt; unavailable spatial context is dry32767/wet0,
with no requested patch. Completing an authored fade-out publishes the patch
request and starts fade-in; wet capability remains false until a matching
processor is available. Actual installed spatial context is read by
[`environment_context`](spatial-parameter-maps.md) from the Reverb layer and
track-zone polygons. Dynamic enclosures use the garage patch but ordinary
enclosed mixer input0, not the garage input2. Enter/exit transitions select a
patch; a different enclosure kind while continuously enclosed preserves the
entry patch. Entering an enclosure begins with wet offset0; city-map offsets
apply after the fade reaches steady state. The shared controller preserves
these transitions independently of render rate.

## Validation

Check signed dry bounds; CAR wet's slot1 factor versus transmission wet;
azimuth wrapping; map/distance filter minimum; the distance vectors above;
hybrid flag behavior; each PC table row; wet offsets and clamp; fade-out,
patch-switch and fade-in order; full dry with unavailable environment.
Installed tests should require actual class/bank/patch selections and nonzero
coverage. Requested-player traces validate producers and graph evaluation.
Impulse and block-boundary original DSP replays are needed for FXEDIT kernels;
process-isolated PCM and controlled listening are needed for audible mixing,
tails, spatial response and live equivalence. Keep those claims separate.
