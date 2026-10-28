# chameleon-effects

Reader for the **BearEffect** resource (kind `0x301`) of the Chameleon engine (NFS: Most Wanted 2012): the particle effects
of `GLOBALEFFECTS.BNDL`. It takes the bytes of the resource's main block and decodes the emitters, their operations and the
parameters of the operations that are understood (lifetime, radius and its curve over the life, motion, spawn rate and
shape, colour gradient and alpha curve, flipbook, rotation, velocity inheritance) and of the wheel effects (`wheel`: the wheel
spawner of the tyre smoke, its rates and where on the tyre a particle starts).

Format and evidence: [docs/formats/chameleon-effects.md](../../docs/formats/chameleon-effects.md). No game data is in this
crate; the tests that read the install are `#[ignore]` and need `MW2012_GAME_DIR`.

The `corona` module reads the `Corona` objects of a car bundle (flare, glow and beam layers, the four-lane curve blocks and the chain that
evaluates them, the siren flash patterns): [docs/formats/chameleon-corona.md](../../docs/formats/chameleon-corona.md); `examples/corona.rs`
prints a car's records, layers and patterns.

Collision effects (the spawner on the colliding mesh, the bounce, the spark streak and the bindings) are in `collision.rs`, documented in
[docs/formats/chameleon-collision-effects.md](../../docs/formats/chameleon-collision-effects.md).
