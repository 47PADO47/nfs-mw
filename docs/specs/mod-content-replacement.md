# Spec: replacing and adding models and textures

Decision: [ADR 0015](../decisions/0015-mod-content-replacement.md). Mod-facing: [api-replace.md](../modding/api-replace.md). Worlds:
[mod-content-replacement-worlds.md](mod-content-replacement-worlds.md). Evidence tags: `[ours]` is this repository's decision or
measurement, `[verified]` is read from the game's data (see the cited page).

## 1. Layers

| Layer | Where | What |
|---|---|---|
| Manifest types and parser | `libs/blackbox-modding/src/content/replace/` (`ReplaceSet`, `ModelEntry`, `TextureEntry`, `parse`, `from_model_swap`) | schema and ranges; no game names |
| World manifest, patch, merge | `content/world.rs`, `content/world_patch.rs` | pure functions `apply`, `plan_packs` |
| Texture pictures | `libs/blackbox-tpk/src/dds.rs` (`read_dds`), `TextureSource::into_texture` | DDS as a pack texture |
| Fitter | `crates/nfsmw/src/modding/content/replace/{fit,place}.rs` | the four fits; the only NFS:MW-specific part is the `_DEINST` rule in `active.rs` |
| Host registry | `.../replace/{catalog,load,active,textures}.rs` | scan, read, merge, apply |
| Options and choice | `crates/nfsmw/src/modding/backends/replace.rs` | `declare_replacements`, `publish_replacements`, `replacement_report` |
| Streamer hook, shared uploads | `scenes/world/residency/{start,replace,shared_upload}.rs` | `replace::rewrite`, `sync_replace`, `add_textures` |

The fitter is host-side because it works on the game's `Solid`; it takes no game names and moves to `libs/blackbox-solid` when a
second game needs it. [ours]

## 2. Manifest

`replace/<id>.toml`, `format = 1`; the schema, ranges and refusals are in [api-replace.md](../modding/api-replace.md#a-set) and the
module docs of `replace/mod.rs`. Decisions: `[[model]]` takes `target`, `targets`, `target_hash`, `target_hashes` (exactly one form)
so one entry covers every level of detail that shares a model; a target is replaced once per set; a set needs `[files].geometry` when
it has models or objects; a set with only textures needs no geometry; `kind` is required unless `always`; ids are lower case. [ours]

## 3. Models

- Per kind rule: the manifest names the fit; kinds do not have host presets. Suggested: trees `height`/`row`; bushes, lamps, signs,
  barriers, buildings, props `stand`; authored-to-frame `exact`. Cars used as props are solids like any other: `stand`. [ours]
- **`stand`** (`place::stand`): s = min(spread * w_old / w_new, spread * d_old / d_new, h_old / h_new, 4); refused when s < 0.25 or
  s * h_new < min_height * h_old; the model's box centre goes to the old box centre in x and y, its lowest point to the old floor. [ours]
- **`exact`**: no change; refused unless the model's box is inside the old box grown by (spread - 1) / 2 of its size per side + 5 cm. [ours]
- `height` and `row` are the earlier trees rules unchanged (cell grid, golden-angle turn, at most 32 copies and 60,000 vertices, a
  third of failed stretches keeps the old model) [verified on the real tiles, effect-variants.md].
- LOD slots: each level of detail of a game object is its own solid, so a set lists each level in `targets` (the 2012 trees list up to
  four levels per species). Added objects have the scenery slots `model`, `mirror`, `coarse`.
- Collision of a replaced model is the game's (the collision is separate from the solid). Added objects: `collision = "none"` or the
  geometry file's own static collision pack; `"prop"` is refused as in scenery packs.
- Cars as props: a car's `GEOMETRY.BIN` solids can be a set's geometry; its textures go in the set's pack. No wheels, no physics. [ours]

## 4. Textures

Rules as in the mod page. Implementation: `TextureSlot` (a `OnceLock` per target) is created when the set loads; the first loader
thread that parses a section holding the hash reads and decodes the file (`read_picture`), later sections clone the result. A
replaced texture is put into the section's `textures`, so the later steps (decode for upload, upload) see an ordinary texture; its
`name_hash` is the original's. Added textures (the set's pack, `name = ...` entries) join the global textures before the shared
upload and are uploaded once at a change during a drive. The upload rule of the residency (a hash that is resident is not uploaded
again) is why a replacement of a shared texture follows at the next drive. [ours]

Resolution rule: the GPU texture has the picture's size; the renderer's texture detail (`texture_discard`) drops top mips of mod and
game textures alike. Nothing else assumes a size (UVs are normalised); a texture whose size is not a multiple of 4 is decoded to
RGBA8 by `Texture::needs_decode`. Priority and fallback: the last set to name a hash wins; a picture that cannot be had leaves the
game's texture. Not built: animated (`TextureAnim`) frames and scrolls keep working by hash, but a replacement of one frame has not
been looked at in the game; cubemap and volume textures are refused; per-texture lazy sources for *added* textures are read at
activation.

## 5. Status

Built and unit tested: manifest, catalog, options, merge, four fits, textures, trees through the one mechanism, report text.
Open, in this order: (1) **`[[add]]` placement**: parsed and kept (`ReplaceSet::adds`); the host has to turn a set's objects into
synthetic map packs (`PackSource` in memory, sections from the 2700 band skipping `taken`, one pack per 512 m cell) when a world
opens: the pieces exist (`scenery::assemble`); (2) the `bb.content` 1.2 table (section 8); (3) refreshing the shared sets in a drive;
(4) a `replace` console command listing sets (the report text is the data).

### 5.1 Added with the road signs and traffic lights (`docs/formats/chameleon-signs.md`)

- **Markers.** A replacement keeps the authored lights of the solid it replaces: the model's own position markers, placed like its
  vertices (the replacement keeps the replaced solid's pivot, and the files store a marker's translation with the pivot added, so the
  host adds it), or, when the model has none, the markers of the replaced solid where they were (`fit.rs`, `markers_of`).
- **Height.** A model counts as having a height when its **box** is taller than a centimetre (a signal head hangs below its origin, so
  its top is at or below zero); the old test was its top above the origin.
- **First publication.** The sets the saved choices name are made the replacements in effect when the mods are opened
  (`modding/open.rs`), before any frame, so a viewer or a drive that starts at once loads the shared library `X0` (where most signs, lamps
  and props are) with them. The frame update still publishes a change.

## 6. Worlds

See [mod-content-replacement-worlds.md](mod-content-replacement-worlds.md).

## 7. Errors

Every refusal has one reason, logged as `mod <id>: ...` and kept for the report: a manifest that does not parse or has an unknown key,
a value taken by an earlier mod, a game with no reader (`source_game`), a missing geometry or pack file (`not read`), a model not in
the geometry, a model whose texture is not in the pack, a texture file that does not decode (once, at first use), a conflict (which
set won). The game keeps playing with the game's own content for what failed.

## 8. `bb.content` 1.2

Appended to `BbContent`: `capabilities(ctx) -> u32` with bits `LAZY_FILES 1`, `REPLACE_SETS 2`, `TEXTURES 4`, `WORLD_PATCHES 8`,
`ALT_MAPS 16`, and `report(ctx, sink, sink_ctx) -> BbStatus` that writes the lines of `Bridge::replacement_report(owner)` through
the lazy-file sink type. Main thread only; old mods are unaffected, a mod built for 1.2 on a 1.1 host gets `Unsupported`.
**Not built in this round**: the host function exists (`replacement_report`), the ABI field, SDK wrapper and snapshot test do not.
Until then a mod learns support by the manifest being read (the log line) and mods ship their files regardless.

## 9. Tests

`blackbox-modding` (`replace/tests.rs`, `world.rs`, `world_patch_tests.rs`), `blackbox-tpk` (`dds_tests.rs`), host
(`replace/{tests,place_tests,catalog_tests,textures_tests}.rs`, `backends/replace_tests.rs`), the 2012 mod
(`trees/tests.rs`: the set equals the old profile; `signs/tests.rs`, `signs/geometry_tests.rs`: the signs and lights packs on authored models, host `replace/marker_tests.rs`). Real-game checks are `#[ignore]` (`replace/real_tests.rs`, `replace/real_dump_tests.rs`, `signs/real_tests.rs`).
