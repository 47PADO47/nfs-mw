# blackbox-anim

EAGL4 animation of EA Black Box games: animation objects, skeletons, animation banks, channel codecs and
pose evaluation. Takes bytes, never opens files, and has no rendering or Bevy dependency.

Status: reads EAGL ELF objects, skeletons, animation banks and compound channels, decodes the delta codecs the install
uses (DeltaLerp, DeltaQuat, DeltaF1, DeltaF3) and samples clips into poses.

Format: [`docs/formats/animation.md`](../../docs/formats/animation.md).
Specification: [`docs/specs/eagl-animation.md`](../../docs/specs/eagl-animation.md).

License: MIT OR Apache-2.0.
