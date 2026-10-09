# blackbox-particles

Particle emitters of EA Black Box games, as a deterministic simulation: an [`EmitterSpec`] (the
parameters of one emitter: rate, life, speed, cone spread, drag, gravity, keyed size, angle and colour
curves), an [`Emitter`] that spawns and steps particles, and the [`Sprite`]s it hands to a renderer.

Pure maths: no files, no GPU, no game data. Where the numbers come from (a game's attribute database)
and how a sprite is drawn (texture, blend) is the caller's business. The rules follow the behaviour
documented in `docs/specs/exhaust-flames.md` (sections 5 and 6).

Not modelled: start delays, on/off cycles, one-shot emitters, the disc spread, animated textures.

License: MIT OR Apache-2.0.
