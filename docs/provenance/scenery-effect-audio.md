# Sequenced scenery sound evidence

Spec: [scenery effect audio](../specs/scenery-effect-audio.md).

The installed tollbooth_seq responds to WORLD_DETACHED with a zero-instance
EPlayObjectEffect naming sfx_tollbooth_brk. The effect's AudioFX_DEFAULT points
to an audioevent collection, DESCRIPTION EVENT and SMOKABLE, with three
STITCH_LEVEL_0 entries, full authored Volumes, AudioCullDist 30 and an identity
AudioQuadratic. The normal physical tollbooth profile independently retains
400 kg mass, 15000 N s breakaway and NO_CAR_EFFECT. Installed contact regression
reports positive callbacks from both endpoints; those inputs were not retuned.

The CC0 dbalatoni13/nfsmw WorldConn.cpp World_OneShotEffect independently
processes AudioQuadratic, AudioCullDist and ChooseAudioAttributes, then creates
Sound::AudioEvent even if the visual emitter is absent. The matched PC image's
effect callbacks and wrapper layout were previously inspected for prop particle
and collision audio work; executable identity is recorded in prop-lifetime.md.

The rewrite previously queued ObjectEffect particles without any audio request.
Its collision link reader alone does not cover standalone audioevent effects.
Implementation follows the spec with an independent world one-shot queue and
the existing authored stitch decoder, playback module and spatial mixer.
Installed tests exercise a real tollbooth placement and its sequence through
that queue to headless PCM, with missing-effect and inaudible controls. This is
offline pipeline evidence; no live original-game listening comparison is claimed.

Stream sweeteners: the CC0 dbalatoni13/nfsmw `SFXObj_Collision.cpp` collision
object walks `StreamSweetner` from the last record to the first, keeping the
last one whose threshold the intensity reaches (the first in list order), when
the event is described `EVENT` or matches its car gate, and sends a moment-stream
event at the contact point unless world data is streaming. An installed census
of all 131 `audioimpact` collections found sweeteners on six car/wall impacts
and two `EVENT`+`SMOKABLE` sounds, one of them the tollbooth break (the
`tollbooth` moment, threshold 0). The record layout was read from those installed
values. Installed tests decode and play the tollbooth moment through replay PCM;
no live listening comparison is claimed. The retrigger rule (two seconds, with
`tollbooth` and `window` exempt) and the single shared moment stream follow the
same reference's `SFXObj_MomentStrm.cpp`; a read-only look at the PC 1.3
executable's `ShouldStreamPlay` found the same two exempt keys. The description
bit names (primary target, `TWO_CAR`, `EVENT`) come from the reference's
`SoundCollision.hpp/.cpp`; the `EMomentStrm` event fields from its generated
event header. A private passive capture of the original game (no game code or
data retained) confirmed the behaviour: hard player car crashes streamed
`car2car` (twice per crash, once per car's event), a booth chain restarted
`tollbooth` on each booth, and the log pile breaker's sequence streamed
`logs_drop`.

Shared moment stream: admission, commit, hold/release and the per-frame queue
follow the reference's `SFXObj_MomentStrm` (`ShouldStreamPlay`,
`CommitStreamReq`, `ReceiveMoment`, `UpdateParams`, the marker-to-moment table
and the 20 m queue radius); `CARSFX_Rain` and the breaker receiver show thunder
and the starting sting use the same stream. Marker positions reuse the existing
`TrackPositionMarkers` reader. The capture showed the arbitration: the starting
sting refused `car2car` requests, `logs_drop` replaced the sting, a gate moment
was queued near its marker (distance-checked requests), replaced a playing
`tollbooth` crash, and `unpause` released it without a new request. Installed
tests compare the installed priorities with those observations. The race-status
gate and a second player's queue are not modelled.
