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

World one-shot voices share a pool bounded to 25 stitch pieces and release after
their authored modules finish. Existing car pools remain separate; original
global priority arbitration across pools remains a parity gap. Scene changes
stop world voices. Keyed continuous model
effects and arbitrary non-stitch audioevent classes remain separate work; this
implementation does not turn them into repeated one-shots. Original live audio
and keyed-effect parity remain unverified.
