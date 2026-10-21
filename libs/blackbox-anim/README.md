# blackbox-anim

EAGL4 animation of EA Black Box games: animation objects, skeletons, animation banks, channel codecs and
pose evaluation. Takes bytes, never opens files, and has no rendering or Bevy dependency.

Status: being built. Reads EAGL ELF objects and applies their relocations. Skeletons, animation banks, channel
codecs and pose evaluation are not written yet.

Format: [`docs/formats/animation.md`](../../docs/formats/animation.md).
Specification: [`docs/specs/eagl-animation.md`](../../docs/specs/eagl-animation.md).

License: MIT OR Apache-2.0.
