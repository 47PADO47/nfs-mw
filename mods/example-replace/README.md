# example-replace

A worked example of the replacement sets ([docs/modding/api-replace.md](../../docs/modding/api-replace.md)): it replaces **one street lamp
model** and **one road texture** of the 2005 city with a model and a picture it makes itself. It holds no model, no picture and no game
data: `src/generate.rs` builds a lamp with `blackbox_solid::write_geometry_pack`, a texture with `blackbox_tpk::write_tpk` and a
checker PNG with `png`, and `src/lib.rs` writes them and the two manifests into the mod's own folder when the game loads it.

```text
replace/street-lamps.toml      kind "street_lamps": [[model]] targets -> EXAMPLE_SLIM_LAMP, fit = "stand"
replace/road-textures.toml     kind "road_textures": [[texture]] target -> files/EXAMPLE/CHECKER.png, format = "bc1"
files/EXAMPLE/GEOMETRY.BIN     the lamp          (written at load)
files/EXAMPLE/TEXTURES.BIN     its texture       (written at load)
files/EXAMPLE/CHECKER.png      the road picture  (written at load)
```

Build and install:

```sh
cargo build --release --manifest-path mods/example-replace/Cargo.toml
# copy mod.toml and target/release/example_replace.dll into <mods folder>/example.replace/
```

In the game, Options > Gameplay shows **Street lamps** and **Road textures** (a row appears only while a mod offers a value), each with
`Original` and the mod's value. The choice is saved as `[mod_values.nfsmw] option_street_lamps_variant = "slim"`; a change during a drive
re-streams the map tiles and shows within seconds.

`LAMP_TARGETS` and `ROAD_TEXTURE` in `src/generate.rs` are the names of the game's solid and texture the example replaces. To see what a
replacement does without the game running, the `replace` tests of `crates/nfsmw` build the same kind of set over generated sections.

Tests: `cargo test --manifest-path mods/example-replace/Cargo.toml` (the manifests are accepted by the game's parser, the files read back).
To write the files without the game (for example into a folder you pass as `--mods-dir`):
`EXAMPLE_REPLACE_OUT=<folder> cargo test --manifest-path mods/example-replace/Cargo.toml --release write_the_mod -- --ignored`.
