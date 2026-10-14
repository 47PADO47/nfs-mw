# Road transitions and weather wind

Sources: original PC 1.3 road/wind producers, installed AEMS banks and attribute
collections; behavioral cross-reference [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)
(`CARSFX_Roadnoise.cpp`, `CARSFX_WindNoise.cpp`, `STICH_PlayBack.cpp`, CC0,
decompiled reference only). Implemented from this specification, not copied code.

## Ordinary road lifecycle

Keep one selected ordinary road module per side. A new road ID creates a new
`FX_ROADNOISE` instance; changing the ID of an already running graph does not
recreate its player. Preserve the same instance while the ID remains unchanged.
When replaced, request the old graph's destructor and continue its updates until
it finishes. Losing wheel support sets volume to zero without changing the ID.
Selecting no road sound releases the selected instance.
Keep each selected graph's own constructor parameters immediately, including a
graph replaced before its first audio tick; retirement must not inherit the
previous graph's ID or gain.

An initialized ordinary graph runs its authored destructor. Original numeric
graph replay confirms an initialized gravel graph completes release; deleting
before its first update can leave its constructor/destructor triggers competing
and keep the graph alive. The remake discards an instance that has never ticked
and owns no host resources instead of creating voices during its retirement.
This is a caller lifecycle adaptation with no audible resources to release,
not a timeout or a change to the original arithmetic nodes.

The eleven members are ID, volume, Q12 pitch, azimuth, type, secondary noise,
speed, low-pass, high-pass, dry, wet. Ordinary type is zero. Pitch and volume come
from the existing per-side wheel/speed producer and the installed road mixer;
wet comes from its wet output. Dirt/gravel select secondary noise, latched until
the selected graph changes. A side accepts a new terrain when its two wheel
keys agree; differing wheel keys retain the prior selection. Without
surface keys the existing gravel-loop fallback is retained explicitly.

Two independent wet-road modules use ID 4. Their volume is generated side volume
times 24000, then the wet-layer mixer output, each multiplication shifted by 15.
The PC's road-volume table confirms 24000; this is separate from environmental
reverb wet gain. The layer can be silent when its environment/mixer input is zero.

## Surface and tyre transitions

Optional game-side context supplies four actual surface keys and authored
`Aud_RoadNoise_TransON/OFF` values, plus tyre damage states. Wheel order is front
left, front right, rear right, rear left; sides use indices [0,3] and [1,2].
Among wheels whose surface and road-loop ID changed, the last wheel on that side
selects the transition. At least one wheel on the side must be grounded. If
either old OFF or new ON value is DONTPLAY (4), suppress it; otherwise prefer new
ON when present, then old OFF. NONE is -1. Initial context establishes history
and never invents an entry transition.

A new puncture selects transition 2; a new fully blown tyre selects 3 and takes
precedence. Context must distinguish these states rather than inferring a
puncture from an ambiguous blown boolean. Missing context disables these events.

Recreate one `FX_ROADNOISE_TRANS` graph per side on a new transition. Type is 1,
secondary noise and speed are zero, low-pass 25000, high-pass zero, dry 32767.
PC base gains are 12000 for curb ON/OFF (0/1), 24500 for spike strip (2), 32767
for blown (3). Multiply by 32767 and then by the appropriate mixer output using
Q15 shifts. Mixer slots are 2 for ordinary transitions, 13 for puncture, 14 for
blown. Pitch increases linearly from 3600 at 12 mph to 4400 at 60 mph, clamped
at the endpoints. Azimuth and reverb use the selected side's road outputs.
The authored graph controls completion and release; no guessed sample delay.

## Stitch road

Road ID 8 uses `aud_stitch_loop` collection key 0x4B41DEC8, not a road-bank sample.
Read its `stitch` ID and signed `overlap` time from installed attributes. Keep
two alternating stitch slots. Start one immediately; decrement the repeat timer
by truncated audio delta in milliseconds; strictly below zero resets it to the
authored overlap and starts another stitch in the last free slot. Active pieces
use the same installed collision stitch module and local piece variation as
ordinary impacts. Missing attributes/stitch data are reported, not replaced by
an arbitrary loop. Precise RNG equality remains unverified.

The remake supplies this numeric timer at a fixed 60 Hz, retaining fractional
tick debt and limiting each update to 64 ticks while keeping outstanding debt.
This caller adaptation preserves its normal 16 ms truncation per tick and
avoids rendering rate altering the repeat cadence. Exact native behavior with
variable audio callback periods remains unverified.

## Weather wind

Run `FX_WIND_Weather` from `IG_GLOBAL/WIND_00_MB.abk` alongside car-speed wind.
The four members are volume, pitch offset, width, intensity. Weather volume is
player mixer object 12 output 0 times a speed factor: 1 at 0 mph, falling linearly
to 0.25 at 70 mph and remaining there above 70. Truncate and clamp to Q15.
Pitch offset stays zero, width is 4000, intensity zero. These constructor,
producer and slope values are confirmed in the matching PC binary and four
isolated original-instruction vectors. Do not substitute the unused reference
weather-volume constant or invent rainfall strength. With no mixer, weather
volume defaults to zero; runtime weather/environment inputs remain a caller
dependency.

The installed player weather master has a base of -2500 and combines shared
submixes, player physics, main controller input 5 and event envelopes. Honor
that authored output even when no separate weather-wind input is supplied;
the graph does not justify an invented rainfall-to-wind multiplier.

## Validation and boundaries

Test graph replacement, unchanged-ID continuity, release completion, transition
preference/suppression and tyre edges, side wheel ordering, PC transition gains
and pitch endpoints, weather vectors, and installed road/transition/weather
graph selection through the actual host. Original tyre/controller history and
weather environment input production require the caller to provide real state.
Backend reverb, original RNG and variable audio-period equality remain separate
coverage; graph integration alone does not establish audible parity.
