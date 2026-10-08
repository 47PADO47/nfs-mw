# blackbox-feng

FEng, the user-interface engine of EA Black Box games (the menus and the HUD of Need for Speed: Most Wanted):
a reader for its packages and fonts, a runtime for the animation scripts and messages, and a **retained tree**
of drawable nodes. **No rendering, no windowing, no Bevy, no wgpu:** a host draws the tree.

- `Package::parse` reads an `FEngPackage` chunk (`0x00030203`); `Package::parse_compressed` an
  `FEngCompressedPackage` (`0x00030210`, JDLZ or HUFF). Objects, resources (with the run-time texture and font
  handles), scripts (base and delta keys, events), message responses and message targets are plain data.
- `Font::parse` reads an `FEngFont` chunk (`0x00030201`); `Font::layout` lays a string out into glyph rectangles
  with texture coordinates, the way the game does (justification, newlines, word wrap, kerning, leading).
- `Runtime` loads packages, starts every object in its `INIT` script and advances a 960-tick clock:
  step, linear and move-to interpolation, once / loop / ping-pong / chained scripts, events that become
  messages, response lists with if / else / end-if, buttons with focus messages.
- The host binds values by object name hash (`set_text`, `set_label`, `set_hidden`, `set_rotation_z`,
  `set_alpha`, `set_colour`, `set_texture`, `run_script`), posts messages, takes the messages that leave
  (`take_outgoing`: to the game, to sound, package commands) and reads `Runtime::tree`: per node the kind,
  resolved text, transform, accumulated colour, depth and the far-to-near `draw_order`.

```rust
let package = blackbox_feng::Package::parse(chunk_payload)?;
let mut runtime = blackbox_feng::Runtime::new();
let id = runtime.load(package);
let needle = runtime.find(id, blackbox_feng::fe_hash_upper("3rdPersonNeedle")).unwrap();
runtime.set_rotation_z(needle, 1.2);
runtime.update(1.0 / 60.0);
let tree = runtime.tree(id);
for &i in &tree.draw_order {
    let node = &tree.nodes[i]; // draw node.kind with node.world and node.world_colour
}
```

Coordinates are the FEng screen: 640 x 480, origin at the centre, y down, depth larger = farther. Takes bytes,
never opens files. Texture and font resources are named by handle (`resource_handle`); the host maps them to its
own textures.

Specs: [formats/frontend.md](../../docs/formats/frontend.md), [specs/feng-runtime.md](../../docs/specs/feng-runtime.md);
provenance: [provenance/feng.md](../../docs/provenance/feng.md). Tests: synthetic packages always; the `#[ignore]`d
ones read an install (`NFSMW_GAME_DIR`) and parse every package and font of it.

Not implemented: list boxes, movies, the mouse, the multi-image mask (the tree carries it), package commands
(reported to the host), clip regions (the game does not use them).

License: MIT OR Apache-2.0.
