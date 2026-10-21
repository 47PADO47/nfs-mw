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
