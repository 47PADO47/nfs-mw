# Speedbreaker moment audio

## Observed behavior [decomp]

Activation has two independent sounds: the common FX_UVES begin controller and
an installed `aud_moment_strm` recording. Interactive music selects collection
`purs_break_start_music`; other music modes select `purs_break_start_fx`.
The recipient stops that recording when Speedbreaker exits. It does not send
the database's end-moment collections; the common exit controller remains.

The moment collection supplies `stream`, `param` and `VolSlot`. NISAudio's speech
index selects samples whose parameter bit indices match both stream and param.
Sample selector fields begin at byte14, stride `parameter_count+2`; each field
is a bit index, limited to31. The installed SoundFX event has two selectors.
Offsets and PCM decoding use the existing speech index and stream reader.
The bank's alignment unit is `256*(1+byte7)`; it applies to sample offsets.
This differs from the speech recordings' usual256-byte unit. Ignoring that
field points into compressed audio instead of a stream header.

Moment audio uses the sound-effects bus, centered for unpositioned collections.
It reads its gain from main MomentStrm object6 at the collection's volume slot.
The stream interface consumes the Q15 gain shifted right8, limited to127.
Keep that output current using the retained main audio mixer; slider volumes
remain on the effects bus and must not be applied twice.
Even an unpositioned stream's volume master can include a spatial rolloff.
The activation moment's `UseUserPos` producer follows the player-car position.
Publish its planar distances and azimuths to the original main position
controller2, with its valid-position flag at input15. Match the current camera
state. Unpositioned controls centered output azimuth, not whether spatial
volume is evaluated. Leaving that flag unset explicitly silences this master.
Its playback rate is1; it does not inherit the music low-pass or blanket slow
motion pitch. Cache decoded recordings, retaining only one breaker moment voice.
Activation and exit follow mode edges, independent of render FPS. The native
2-second retrigger check applies only while the same moment remains current.
Once its stopped channel is cleaned up, a fresh activation may play again;
do not impose a global2-second cooldown after exit.
World loading may suppress the request; broader NIS/moment priority arbitration
and exact audio-stream scheduling are separate integration gaps.
The remake includes pending pursuit-music selection and currently playing
interactive voices when choosing the branch. Full original music-state graph
selection remains unverified, particularly race/front-end/mode transitions.

## Validation

Synthetic selectors must distinguish stream and parameter masks and respect
sample-zero flags rather than shifting flags to the following offset. Installed
checks must select and decode both branches, use authored gain, stop on exit
and prove no repeated start while held at5/60/144/360 FPS. Such checks establish
implementation behavior, not matched original PCM or native DSP equivalence.
