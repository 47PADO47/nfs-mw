# Authored nitrous, purge, shift and sweetener playback

Sources read: the matching original PC executable (SHA256
80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c),
and [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0,
decompiled reference), CARSFX_Nitrous.cpp, CARSFX_Shifting.cpp,
MAIN_AEMS.h, ENGINES_AEMS2.h and EAXSoundEnums.hpp. Read for behavior;
no implementation copied. Companion specifications:
[engine-sound-effects.md](engine-sound-effects.md),
[engine-sound-aems.md](engine-sound-aems.md), and
[traffic-sound-events.md](traffic-sound-events.md).

## Interfaces and routing [decomp, PC checked]

All class words are signed integers. Gains use Q15, decoded playback pitch
uses Q12. The sound banks remain runtime inputs from the user's install.

| Class | Ordered words | Bank |
|---|---|---|
| FX_NITROUS | type, volume, azimuth, stop, pitch, lowpass, highpass, dry, wet | selected global nitrous bank |
| FX_PURGE | volume, azimuth, stop, pitch, lowpass, highpass, dry, wet | same nitrous bank |
| FX_SHIFTING_01 | id, volume, pitch, azimuth, type, camera | selected shift-pattern bank |
| CAR_SWTN | id, car class, physical RPM, volume, azimuth, pitch offset, rotation | engine SweetBank[0] |

Nitrous constructors initialize stop/pitch/highpass/wet to zero, lowpass
25000, dry32767. Each update replaces volume from nitrous mixer slot1,
azimuth from slot0, decoded pitch from slot3 and lowpass from slot4.
Bounds are type0..2, volume0..32767, azimuth0..65535, pitch0..16384,
lowpass0..25000. Type0 is the currently established player selection.

The rising active edge creates a new FX_NITROUS. The falling edge sets
stop=1, requests class destruction and lets the authored graph retire its
voices. Reactivation replaces the previous object, as the original does.
The empty request creates FX_PURGE if no purge object is active. Its
volume is slot2, azimuth slot0, playback pitch4096; default filters/dry/wet
are the constructor values above. A persistent boolean empty signal is
consumed on its rising edge because the original clears its request.
Neither effect substitutes a fixed sample index.

The separate 400ms/1000ms boost interpolator has no established mixer
consumer. Do not invent a boost-pitch contribution; authored class data
and the dynamic mixer determine the sound.

## Shift and sweetener events [decomp, PC checked]

PC FX_SHIFTING_01 id is **1 for an upshift, 0 for a downshift**, 2 for
brake mash; this corrects the reversed older direct-sample description.
Pitch4096, type0, camera1 when original POV equals0, otherwise0.
Volume/azimuth are bounded0..32767 and0..65535.

For a clunk, truncate physical RPM before calculating
`s = 0.1 + 0.9 * clamp((RPM - 1500)/6500, 0, 1)`. The native cosine-table
index is `truncate(512 - 512*s)`. Its entry is
`max(0, floor(32767*cos(index*pi/1022)))`.
Multiply the selected shift-pattern Q15 volume by that entry, shift15,
then multiply by raw mixer up-slot1 or down-slot2 and shift15 again.
Brake mash uses raw slot10 directly. A new clunk replaces the previous
clunk. Brake release drops the brake object. Each sweetener has its own
slot; a new event of the same kind replaces that slot.

Sweetener id0 is disengage/engine-off, id1 engage/accelerate. Class number
is engine CarID, bounded0..100; physical RPM0..10000; gain0..32787;
azimuth0..65536; pitch offset/rotation0. Gain is engine Vol_ShiftSweets
times a linear Q15 slope26000 at4000RPM to32767 at7000RPM, then times
raw mixer slot4(disengage),3(engage),5(accelerate),6(engine-off), with
integer truncation and shift15 after each multiplication. Disengage uses
physical RPM for the slope; the other producers use RPM_AtShift. Every
class receives current physical RPM in its RPM word. Existing engine
controllers supply the event gates/delays; this adapter does not invent
new events or multiply legacy sample-command gains a second time.

## Forced induction and reverse [decomp, PC checked]

FX_TURBO_01 has six words: id0..5, volume0..32767, PSI0..1024,
azimuth0..65535, rotation0..1024, physical RPM0..10000. Spool id0
exists while the turbo object is enabled (non-default turbosfx); unlike
the previous direct loop, its gain is **not multiplied by spool percent**.
The graph receives PSI and shapes the spool audio itself.

At each retained fixed sound tick, increase charge by engine torque*0.01
when engine torque>20, otherwise decrease by data Leak_Rate. Clamp first
to0..ChargeTime then to0..ChargeTime*(0.4+0.6*clamp((truncate(RPM)-1000)*0.01,0,1)).
Spool percent is charge/ChargeTime. Reaching the RPM-dependent limit
arms a 1000ms hold followed by 2000ms linear table-index ramp0..307.2.
Duck volume is data Vol_Spool times the indexed cosine Q15 table,
shift15. Falling below full ChargeTime clears peak state, preserving
the original RPM-limit behavior. Spool parameters are ducked volume
times raw turbo-slot1 shift15, PSI=truncate(percent*1024), azimuthslot0,
rotation=truncate(physical torque*10.24), and physical RPM.

Blowoff semantic event IDs0..2 correspond to class IDs1..3. Existing
fixed-clock event selection remains its producer; this preserves its
documented RNG limitation. Before resetting charge, capture percent
and physical RPM after the current falling-torque tick's leak. Base gain is truncate(data Vol_Blowoff1 for id1,
Vol_Blowoff2 for ids2/3 times percent). Per-update gain is base times
raw slot2/3 shift15, then times the stop ramp. When physical throttle
exceeds20 and no audio shift is active, ramp gain1..0 over150ms;
the graph or ramp completion retires the object.

CAR_WHINE has nine words: class0, physical RPM0..10000, gain0..32787,
azimuth0..65536, pitch offset0, rotation0, lowpass25000, wet0,dry32767.
Class is constructor0, not the player's CarID. Reverse creates the
class from SweetBank[1]; leaving reverse drops it. Gain is raw shift
slot8 times physical sound RPM divided by the **physical car's MaxRPM**,
or0 if MaxRPM is nonpositive. That physical maximum is an explicit
producer input, not engineaudio.MaxRPM. No hand-tuned pitch ratio.

Original POV0 is bumper and1 hood; the physics view signal is0/4000,
with32767 for POV>=2. The adapter's boolean camera word can therefore
be supplied by view_value==0 for currently supported bumper/hood/chase.
This is separate from the mixer-map camera-state number0 (chase).

## Timing, ownership and limits [decision]

The adapter takes ordered simulation events and raw mixer outputs. It
claims an event only after loading its authored class; the central router
records unavailable paths and decides fallback policy. Existing objects
advance through the shared frame-independent authored module clock,
retaining elapsed-time debt at low FPS. Completed/failed objects drop
their voices. No collection grows with the render rate.

Coverage must distinguish controller routing from audible equivalence.
Original variable sound-timer scheduling, RNG sequences, output-backend
pan/filter laws and environmental DSP remain separate parity questions.
The selected nitrous type/upgrade producer beyond type0 and specialized
camera POV states require explicit source inputs. Synthetic tests cover routing bounds
and Q15 arithmetic; installed tests exercise real class lifecycle/PCM at
5/60/144/360FPS and44.1/48kHz. Unrun tests are not evidence of a pass.
