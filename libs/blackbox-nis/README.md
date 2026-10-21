# blackbox-nis

EA NIS cutscene scenes of EA Black Box games: the `AnimScene` container of a bundle (its header and its character
records), the scene clock, and the placement of a scene in the world. Takes bytes, never opens files, and has no
rendering or Bevy dependency.

Status: the scene header, the character records, the clock (frames to seconds, a playback run with skip) and the
marker placement (scene, render and world frames) are read and tested on synthetic bytes. The real-install check is
in `crates/nfsmw-data/tests/real_install/nis_scenes.rs`. Not here: the scene's camera, its animation clips, its
event sequence, and the game rules for cars and cops.

Formats: [`docs/specs/nis-scenes.md`](../../docs/specs/nis-scenes.md).
Timing and placement: [`docs/specs/nis-player.md`](../../docs/specs/nis-player.md).

License: MIT OR Apache-2.0.
