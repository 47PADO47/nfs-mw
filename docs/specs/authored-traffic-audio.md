# Authored traffic sound producers

Sources: the owner's PC 1.3 executable (static inspection and passive sound
captures), installed `MAPOUTPUT.mxb` and sound banks **[verified]**; the CC0
[matching reference](https://github.com/dbalatoni13/nfsmw), `CARSFX_TrafficFX`,
`SFXCTL_3DObjPos`, `STICH_PlayBack`, `STITCH_WSH` and generated class headers
**[decomp]**. This specification precedes the independent implementation.

## Ownership correction

`FX_TRAFFIC` controls traffic engine loops, not horns. Horns create `ENV_STATIC`
with the vehicle's `HornType`, common-environment type 0. Consequently treating
`HornType` as an entry of the traffic engine bank is incorrect. The common bank's
graph selects the sample; values such as truck horn 11 are valid class IDs.

Every simulated step publishes stable vehicle identity, installed engine/horn/
whoosh IDs, position, forward direction and velocity. Positions use world Z-up
space. Ordered step snapshots and events retain their elapsed time even when
multiple physics steps occur in one display frame. No render-rate cooldowns or
invented full-volume horn/near-miss constants participate in authored playback.

## Mixer and class data

The traffic main state is 5: engine object 0, whoosh object 1, horn object 2.
Slots 0/1/2/3/4 are
azimuth/engine volume/horn volume/whoosh volume/pitch respectively **[decomp]**.
Its 3D controller receives car distance, camera distance (centimetres), their
azimuths, and source-relative velocities in cm/s. Approaching means the current
distance decreased since the preceding step: negate the relative speed magnitude.
Crossing from negative to positive or positive to negative sets the appropriate
car/camera flag, in addition to the attached bit. Camera velocity is measured from
successive listener positions. No synthetic Doppler pitch is added after the map.

`FX_TRAFFIC` words: ID, volume, pitch offset, azimuth, range, low pass, high pass,
dry, wet. ID is `TrafficEngType`. Range truncates vehicle mph times 7.3142858 and
clamps to 0..1024; pitch offset is decoded mixer Q12 pitch minus 4096. Constructor
filter defaults are 25000/0/32767/0. Lifecycle ends when its emitter disappears.

`ENV_STATIC` horn words: ID, volume, Q12 pitch, azimuth, type, low pass, high pass,
dry, wet. ID clamps to 0..50; type 0. Horn output uses slot 2 and a 150 ms linear
fade after the chosen duration. The original autonomous horn checks use a 3 s
per-car cooldown, 30 percent attempts or a 5 s global timeout, player forward
speed greater than 10 m/s, range 20 m, and direction gates **[decomp]**. The
remake implements the non-drag forward/rear gate; drag and AI horn requests remain
separately tracked. Duration is 1.2..4 s, chosen once per accepted event.

## Whoosh stitches

The sound-stitch family is explicit in the data header: family 0 collision,
1 whoosh, 2 static. Read every matching bundle, not only its first bundle.
Traffic/AI-car whoosh chooses stitch 45..56. Relative speed above 10 m/s and
within 10 m, at most once per 3 s, triggers playback **[decomp]**. Speed intensity
is `clamp((speed-10)/50,0,0.99)*127`; block index is truncation of intensity/128
times two, then cycle through six variants. This is a rotating selection, not
a fresh random variant each display frame. Other installed `WooshType` values
select their own documented base/block/variant dimensions.

Each stitch piece creates `AEMS_StichWoosh` with words type, sample ID, volume,
pitch, azimuth, offset, dry, wet, low pass, high pass. Piece volume and pitch
variation are chosen once; update preserves the selected local pitch, multiplies
local volume by current mixer whoosh volume (Q15), and adds source/piece azimuth
modulo 65536. Authored offset remains independent of pitch. Stop when the graph
finishes, the source disappears, relative speed falls to 10 m/s or the game pauses.

## Validation boundaries

The fixed step preserves ordered events independently of display FPS; synthetic
tests cover variant cycling, signed moving-source inputs, horn gates/fades and
invalid telemetry. Installed regressions exercise actual module voices and
cleanup. Remaining parity gaps include original camera-velocity averaging,
global random generator sequence, two-player mixing, exact AI horn requests,
traffic truck auxiliary `FX_TRUCK_FX`, generic scenery whoosh trigger producers,
and shared wet DSP/backend laws. Module presence is not an audible parity claim.
