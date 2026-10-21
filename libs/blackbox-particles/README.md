# blackbox-particles

Particle emitters of EA Black Box games, as a deterministic simulation: an [`EmitterSpec`] (the
parameters of one emitter: rate, life, speed, cone spread, drag, gravity, keyed size, angle and colour
curves), an [`Emitter`] that spawns and steps particles, and the [`Sprite`]s it hands to a renderer.

Pure maths: no files, no GPU, no game data. Where the numbers come from (a game's attribute database)
and how a sprite is drawn (texture, blend) is the caller's business. The rules follow the behaviour
documented in `docs/specs/exhaust-flames.md` (sections 5 and 6).

Start delays, on/off and one-shot control follow `docs/specs/particle-emission-control.md`.
Quantized square atlases and random static cells follow
`docs/specs/prop-particle-attributes.md`. Sprites expose their selected UV rectangle.
Disc spread, the original global random stream and packed particle motion remain
outside this simulation's current coverage.

License: MIT OR Apache-2.0.
