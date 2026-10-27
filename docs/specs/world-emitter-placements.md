# World emitter placements

The streamed world contains `EmitterLibrary` chunks (`0x0003BC00`). The
layout and loading rules below come from the CC0 reference's
`Ecstasy/EmitterSystem.h` and `EmitterSystem.cpp`. They are also checked
against the PC track stream. No game data is included here.
See [provenance](../provenance/world-emitter-placements.md).

The 16-byte-aligned payload starts with four little-endian 32-bit words:
endian state, version, library count and section number. Version 5 is present
in the inspected PC stream. Each library has an 80-byte record followed by
its 48-byte trigger records. Its group key is at offset 0, section at 8,
trigger count at 10, and row-vector local-to-world matrix at 16. The matrix's
translation is its fourth row. Trigger records contain radius at 12, world
position at 16, reset time at 32 and probability at 36.

The original associates a group with each library and updates it as sections
stream. Nonzero ordinary sections can instantiate a group when its position
is near a player view; the reference uses its authored `FarClip` plus 20 and
a view test. Triggered effects also have trigger-specific activation rules.
Group keys resolve to the installed `emittergroup` class. Zero-section and
special-section behavior, trigger activation, exact distance/view admission,
particle budget and image matching need separate confirmation.

The reader must bound counts and reject truncated data. It must preserve
trigger records rather than treating every placement as always active. A
parsed placement alone is not evidence that its effect appears in gameplay.

## Initial gameplay implementation

For untriggered ordinary section placements, create emitters from the authored
group, at the authored matrix, when within its FarClip plus 20 units of the
player camera. Stream retirement and distance removal retire their particles.
Use deterministic nearest-first admission, at most 64 placements, 16 emitters
per group and 128 particles per emitter. These limits and the distance-only
admission are remake approximations, not claims of original pool or field of
view parity. Missing groups and textures produce no substitute effects.
Trigger-based placements are deliberately left out until their activation and
reset rules are verified. The world simulation clock controls particle motion.
