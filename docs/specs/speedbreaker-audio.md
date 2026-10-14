# Speedbreaker audio and presentation

## Complete mode transition **[PC checked]**

The breaker mode is the player's Speedbreaker, including the original event
whose name also contains "pursuit breaker". Its begin/end message sends common
sound input 3/4, which starts authored effect IDs 2/1. The UVES pulse (input 1,
effect ID 0) belongs to visual heat pulses and must not replace these transitions.
The player state and sequence event producers remain distinct; a scenery
sequence's existing begin/end sound must not replace the player's transitions.

Main controller input 8 drives six installed envelopes: engine attenuation,
selected effect pitch, other mode levels and filters. Synchronize every envelope
with that trigger on the shared real-time controller clock, rather than sharing
only the pitch envelope. Keep depths, durations and curves in installed data.
The local engine's pitch channel follows its separate NOS routing; its large
Speedbreaker volume reduction is what makes the held engine recede. A universal
engine pitch multiplier would contradict this installed map.

The audible music cutoff comes from installed music-state object1 output9.
Its filter master consumes main event6, whose trigger is the same Speedbreaker
input8. Keep the output attached to the shared audio mixer and follow the
installed envelope instead of independently applying a music filter fader.
The PC music consumer passes this packed output directly to the stream setter.
The internal630→1800 Hz fader is updated in the original but is not consumed
by this audible path; reproducing its field alone does not justify applying
it to the host music bus. The earlier host integration did so incorrectly.
The stream setter uses the original software first-order low-pass described
in [voice-low-pass.md](voice-low-pass.md). Coefficient changes are immediate;
bypass removes history and filtered re-entry allocates zero history. Apply
this to licensed/pursuit sources and newly started tracks, retaining explicit
per-stream/shared-bus lifecycle and callback-grouping validation boundaries.

## HUD **[PC checked]**

Expose the installed Speedbreaker meter and icon groups. Rotate its multi-image
mask from 175 degrees at zero charge to zero at full charge (the drag HUD uses
-48 degrees instead). The linear gauge, when its group is in the active layout,
scales width by charge and preserves its left edge. Select the installed low
charge script at charge <= 0.3, and the normal script above it. The icon uses its
charged script above zero and the initialization script at zero; this is a
charge indicator, independent of active driving mode.

The active mode transition is separate from charge. On entry, send the installed
HUD package its fade-out message followed by ACTIVATE; on exit send fade-in
followed by DEACTIVATE. These authored responses fade the normal instruments and
minimap and expose the central horizontal charge bar. Send messages once per
state edge; repeated per-frame posts would restart the authored animations.
Keep the package roots available to the scripts, and do not override their
alpha during an active transition. Charge arithmetic alone is not an active-HUD
parity check: verify the installed active, exit and interrupted transitions.
The central stopwatch image's installed initialization chain enters a looping
360-tick script which changes colour and size. Leave that authored pulse
running through the runtime; do not substitute a host-generated sine wave.
The layout's fade scripts run for 600 authored ticks. Advance the UI with its
full frame delta, retaining fractional ticks: capping a slow frame's delta
changes transition and pulse timing. Matching PC UI callers establish 1200 ticks
per real second independently of these asset lengths; see the FEng runtime spec.
Thus the fade lasts 0.5 seconds and the pulse loops every 0.3 seconds.

## Camera and visual bounds **[PC checked]**

The original visual treatment drives a separate breaker target from the active
player state. Its authored transition is measured in world time, produces the
vignette weight, and contributes radial blur separately from the normal speed
blur. A camera drive-state blend approaches one at four per supplied camera
second while active and zero at one per second while inactive; its zoom factor
is 1 - 0.4 * blend. The POV producer divides installed spatial LAG by that
factor and multiplies its FOV by it; HEIGHT and ANGLE retain their installed
values. The negative lag therefore backs the camera away while narrowing FOV.
Both operations are necessary: FOV-only scaling makes the car too large. The main
animation caller accumulates real frame seconds separately from simulation
seconds, so this camera blend runs on real time, including render frames with
no physics tick. The remake retains it on the drive object and applies it to
installed POV values. The existing custom chase/dynamic adapters apply the same
horizontal lag division and FOV multiplication to their existing pose. Their
height and viewing direction are preserved; no camera mode switch is introduced.
Loading/paused frames freeze its progression. Camera position dynamics remain
separate and the surrounding custom follow is not native Cubic camera parity.
The native renderer receives the authored world-time target and installed mask
through a retained additive screen pass. Its [green/blue contribution follows
the recovered shader](speedbreaker-visuals.md); the earlier invented four-tap
blur and alpha-darkening have been removed. Do not
substitute decorative speed streaks or an invented color tint.

## Original producer and authored routing **[PC checked]**

The Speedbreaker begin/end event switches the sound game mode to the original
breaker mode and restores the preceding mode on exit. The main volume controller
publishes 32767 at controller 0 input 8 while this mode is active and zero after
exit. This input is distinct from the activation one-shot and from simulation
time scaling.

The installed free-roam mixer map uses this input for main-state event 2. Its
pitch offset, curve and attack/release durations remain asset data. Consumers
include traffic pitch and selected player effects. Player Ginsu/sample engine
pitch slots have their own routing; do not apply a universal playback multiplier
or multiply authored pitch a second time.

## Shared envelopes **[decision]**

Evaluate every main breaker event once using unscaled real audio time, including
frames without a physics step. Local car, traffic and collision map instances
use the same Q15 positions and signed levels. A newly created source inherits the
current phase rather than restarting the attack. Local controller and simulation
clocks remain independent. Source recordings, controller lifetimes and reverb
processing continue on their established audio timelines.

Use the retained 60 Hz audio-controller timeline, keeping historical activation
words when catch-up is bounded. Never discard elapsed time or replace old
activation words with the newest frame while repaying a stall.

The generic mixer exposes a bounded event-output snapshot and optional override
by state, instance and event index. Overrides replace only that event output
and skip local advancement. Unknown events are rejected; removing the override
restores local evaluation. Game-specific state numbers remain outside the library.

Record activation, envelope position/level and output capability in the trace.
Test activation/hold/release/re-entry at 5..360 render FPS and across stalls,
new collision/traffic sources during a hold, separate simulation/real deltas,
and actual authored playback at 44.1/48 kHz.

## Boundaries

This implements authored Speedbreaker mode routes, common begin/end effects,
selected original moment recordings, HUD activation/fades, music-filter control,
paired spatial lag/FOV and shader-verified visual contribution. Special cinematic
modes, full camera follow, whole-frame grading and matched full-game PCM remain
coverage items. It does not certify
all audio behavior associated with every original sound game mode.
