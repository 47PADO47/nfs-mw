# Sequenced scenery one-shot audio

Sources: installed PC tollbooth event sequence, inherited effects/audioevent
collections and the CC0 WorldConn.cpp one-shot effect service. No recordings,
original bytecode or decompiled code are distributed.

An EPlayObjectEffect with no instance ID creates a world one-shot. Its audio is
independent of whether its particle group resolves or renders. Retain event
order, resolved marker position and intensity. Apply the effect wrapper's
AudioQuadratic to the nonnegative normalized event magnitude, clamp to [0,1],
and require a positive result. AudioCullDist uses the sound-view distance and
does not borrow a particle draw distance. Missing audio references stay silent.

Select the wrapper's AudioFX face using its owner's frame and the effect normal,
falling back to AudioFX_DEFAULT when the face has no reference. The target may be
audioevent, audioimpact or audioscrape. Retain its DESCRIPTION and Volumes and
choose from STITCH_LEVEL_0..3, stopping at the first empty list. Selection shares
the global impact counter. Play authored stitch pieces through the existing
AEMS collision-stitch module and retained spatial mixer; do not replace a
missing event with a generic collision sound.

The installed tollbooth's detach sequence requests sfx_tollbooth_brk. Its
audioevent has EVENT and SMOKABLE descriptions and three authored level-zero
stitches. This layer accompanies physical car/prop collision callbacks; it is
not evidence that the booth's mass or breakaway force should be increased.

## Stream sweeteners

An impact collection's `StreamSweetner[]` lists 16-byte records: an
`aud_moment_strm` reference (class, collection, unresolved pointer) and a 32-bit
threshold. When the selected sound is described `EVENT`, or described `TWO_CAR`
in a hit the player's car takes part in, take the first record in
list order whose threshold is at most the event intensity (the 0..1 audio
magnitude scaled to 0..127) and start that moment recording at the event
position, alongside the stitch. Skip it while world data is still streaming in.
The moment voice uses the existing moment-stream mixer object, authored gain
slot and positioned pan; it follows the camera while it plays and stops with the
scene. Admission belongs to the shared moment stream below. The installed
`sfx_tollbooth_brk` sound carries one record,
the positioned `tollbooth` moment with threshold zero, so every booth break
streams it. The description gate is (primary target and `TWO_CAR`) or `EVENT`:
the player's car-to-car impacts stream `car2car`, while a wall impact, even one
that lists a sweetener, streams nothing. The rewrite applies it to the player
car's impacts (always the primary target) and to `EVENT` world effects.

A sequence's `EMomentStrm` event names a moment by string (its key word is zero
in the installed data); stream that collection from the object's position
through the same voice, for example `logs_drop` when the log pile breaker
collapses. `expl_gas_station` is not streamed there: the rewrite requests it
with the petrol-station explosion. `unpause` names no recording: it releases the
moment queued at the breaker marker within 25 m on both ground axes of the
event (the last such marker in table order).

## Shared moment stream

Every `aud_moment_strm` recording shares one stream: breaker and booth crashes,
car crashes, sequence moments, the petrol-station blast, the pursuit breaker's
starting sting and thunder. A request is admitted unless

- it names the moment that is current and that moment started less than two
  seconds ago, except `tollbooth` and `window`, which restart on every request;
- it comes from the marker queue while a queued moment waits and a player is
  nearer that queued marker than this request's marker;
- another (cinematic) stream plays;
- a moment is current and the request comes from the marker queue without
  `CanInterupt`, or the current moment's `strmpriority` is higher than the
  request's.

An admitted request becomes current at once and stops the recording it replaces;
equal priority replaces. A request naming the queued moment releases it instead,
at its marker. The two-second clock starts when a recording starts playing. Each
frame: the clock runs down; a queued moment is dropped when no player is within
20 m of its marker on either ground axis; a current moment that finished playing
is cleared; a positioned moment is stopped when every player is more than 120 m
away. Then, for every breaker marker whose moment is not current and that has a
player within 20 m on both ground axes, an admitted queue request makes that
moment current and queued (silent, decoded ahead), stopping what played. The
breaker's end stops the starting sting only while the sting is current.

Breaker markers are the track position markers (`TRACKS/L2RA.BUN`) of fifteen
kinds, each queuing one moment: gas station (`expl_gas_station`), fire hall and
police station ground (`comm_tower_crash`), water tower, porch, signage (`gate`),
scaffolding, gazebo, fish market, boat, trailer park, amphitheatre, strip mall,
drive-in and torus. Limits: only the first player is a listener; the original's
race-status gate (no moments in a race that is not actively racing) is not
applied.

World one-shot voices share a pool bounded to 25 stitch pieces and release after
their authored modules finish. Existing car pools remain separate; original
global priority arbitration across pools remains a parity gap. Scene changes
stop world voices. Keyed continuous model
effects and arbitrary non-stitch audioevent classes remain separate work; this
implementation does not turn them into repeated one-shots. Original live audio
and keyed-effect parity remain unverified.
