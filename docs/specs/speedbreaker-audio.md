# Speedbreaker audio pitch

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

## Shared envelope **[decision]**

Evaluate the main pitch event once using unscaled real audio time, including
frames without a physics step. Local car, traffic and collision map instances
use its same Q15 position and signed level. A newly created source inherits the
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

This implements the authored Speedbreaker pitch route. Original music filter
fades, additional mode-dependent volume changes, special cinematic modes and
matched full-game PCM remain separate coverage items. It does not certify
all audio behavior associated with every original sound game mode.
