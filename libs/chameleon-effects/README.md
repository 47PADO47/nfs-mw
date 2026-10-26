# chameleon-effects

Reader for the **BearEffect** resource (kind `0x301`) of the Chameleon engine (NFS: Most Wanted 2012): the particle effects
of `GLOBALEFFECTS.BNDL`. It takes the bytes of the resource's main block and decodes the emitters, their operations and the
parameters of the operations that are understood (lifetime, radius and its curve over the life, motion, spawn rate and
shape, colour gradient and alpha curve, flipbook, rotation, velocity inheritance).

Format and evidence: [docs/formats/chameleon-effects.md](../../docs/formats/chameleon-effects.md). No game data is in this
crate; the tests that read the install are `#[ignore]` and need `MW2012_GAME_DIR`.
