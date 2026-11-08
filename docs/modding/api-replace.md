# Mod API: replacing and adding models and textures (`bb.content` 1.2)

Part of the [modding catalogue](README.md). Design: [ADR 0015](../decisions/0015-mod-content-replacement.md) and
[specs/mod-content-replacement.md](../specs/mod-content-replacement.md). This page is for mod authors, and for the agent that adds the
next kind of replacement (see [Adding a new kind](#adding-a-new-kind)).

**Status.** Built and unit tested: the set manifest, models (four fits), textures (PNG, DDS, texture packs; lazy), the option per kind,
priority and conflict rules, the old `TREES/` profile read as a set. Not yet run in the game window except where the spec says so.
Alternative maps, world patches and the world `environment` are applied when a world opens ([api-content-worlds.md](api-content-worlds.md#alternative-maps-environment-and-world-patches-bbcontent-12)); `bb.content` 1.2 `capabilities` and `report` are in the table and the SDK (`Content::capabilities`, `Content::replacement_report`).

## The idea in one paragraph

A mod drops **one small file per set** in `replace/` next to its `files/`: `replace/<set>.toml`. A set names the models and textures of
the game it takes the place of (or adds), and the files that hold the replacements. Every set belongs to a **kind** (`trees`, `bushes`,
`street_lamps`, `road_textures`: any lower case name the mod picks). Each kind is one list option in the Gameplay options,
`<kind>_variant`, with the game's own value `original` and one value for every set a mod offers; the player picks one per kind. The
option shows only while some mod offers a value. **A mod needs no code**: the host reads the manifests and adds the values itself.

## A set

```toml
format = 1

[set]
kind = "street_lamps"             # the option group: option id street_lamps_variant
kind_label = "Street lamps"       # what the option is called (the first set of the kind that says counts); default: the kind, spaced
value = "mw2012"                  # this set's value in the option (default: the file name); a-z 0-9 _ -
value_label = "NFS Most Wanted 2012"
priority = 0                      # -1000..1000: who wins when two chosen sets replace the same thing
fit = "stand"                     # default fit of the models: height | row | stand | exact (below)
always = false                    # true: no option, applies whenever the mod is loaded (a mod's own fixes, never another game's content)
source_game = "nfsmw"             # optional; a set made for a game this host has no reader for is refused

[files]                           # paths under the mod's files/, ending .BIN (the install's formats)
geometry = "REPLACE/LAMPS/GEOMETRY.BIN"   # the new solids (as a car's GEOMETRY.BIN); write them with blackbox_solid::write_geometry_pack
textures = "REPLACE/LAMPS/TEXTURES.BIN"   # the textures they use (as a car's TEXTURES.BIN); added to the world once

[fit]                             # optional limits, every key optional
spread = 1.25                     # 1..4  how much wider than the old footprint a replacement may be
squeeze = 0.5                     # 0.1..1 how far a tree may be squeezed sideways before it is made smaller
min_height = 0.7                  # 0..1  share of the old height under which the old model is kept

[[model]]                         # replace models of the game
targets = ["XT_LAMP_1A", "XT_LAMP_1B"]   # the game's solid names (case-sensitive); or target = "X", target_hash = 0x1234, target_hashes = [..]
model = "NEWLAMP_L0"              # the solid of `geometry` that takes their place
fit = "stand"                     # optional: this entry's fit

[[texture]]                       # replace a texture of the game ...
target = "ROAD_ASPHALT_01"        # by name, or target_hash = 0x...
file = "REPLACE/ROAD.png"         # .png, .dds or .bin under files/; or texture = "NAME" (a texture of [files].textures)

[[texture]]                       # ... or add a new one the set's models use
name = "NEWLAMP_GLASS"
file = "REPLACE/GLASS.dds"

[[add]]                           # place a model of `geometry` in the world (a scenery [[place]]: model, pos, yaw|rotation, scale, coarse, mirror, collision)
model = "NEWLAMP_L0"
pos = [120.0, -40.0, 3.0]
```

At least one `[[model]]`, `[[texture]]` or `[[add]]`. Unknown keys refuse the set; a refused set is logged (`mod <id>: replacement set
<path> is not added: <reason>`), listed by `Bridge::replacement_report`, and the others still load. A value (`kind` + `value`) that an
earlier mod already provides is refused. A target is replaced once per set.

## Models

- A replacement keeps the **identity** of the solid it replaces: its name hash, flags, pivot, polygon count and density. Only the geometry
  changes. Textures: every texture a model names must be in the set's pack or among its added textures when the set has a `textures` pack
  (a set with no pack uses the game's own textures).
- `_DEINST` world-space copies of a solid (the game bakes most trees into per-place copies) are replaced too, fitted stretch by stretch.
- **Fits** (what `fit` means; `FitLimits` is the `[fit]` table):

| `fit` | For | Rule |
|---|---|---|
| `height` | one tree | scaled evenly to the old height, origin on the ground, never wider than `spread` times the old footprint (squeezed, then made smaller) |
| `row` | a stretch of trees | one copy per stretch of the old geometry, on that stretch's floor |
| `stand` | lamps, signs, barriers, buildings, bushes | scaled **evenly to fit inside the old box**: footprint within `spread` times the old, height within the old; centred on the old box; its lowest point on the old floor. The model's origin does not matter |
| `exact` | a model authored in the old solid's frame | used as it is, only when its box stays inside the old box grown by `spread` |

A model that cannot be fitted within the limits is **not used**: the solid keeps the game's own (a rule, not an error; nothing is
logged). In a world-space copy every fit is cut into stretches like `row`/`height`.

## Textures

- **Targets** are looked up by name hash in every section the game streams and in the global textures. Any size: a larger or smaller
  picture is stretched over the same surface. Mips: a PNG gets a full chain; DDS and packs keep theirs.
- **Formats.** BC1/BC2/BC3 (DXT1/3/5) pass through. PNG, uncompressed DDS, BC4, BC5, BC7 are decoded to RGBA8 and stored as BC1 (no alpha),
  BC3 (alpha), or A8R8G8B8 when a side is not a multiple of 4 (`format = "auto"`). `format = "bc1" | "bc3" | "rgba8"` forces one;
  `max_size = N` halves a picture (or drops top mips) until a side is at most N.
- **State.** The replacement keeps the replaced texture's name hash **and its draw state** (alpha mode, blend, sidedness, tiling): the
  game's materials were authored for it. `state = "file"` takes the picture's own.
- **When.** A file is read and decoded the first time a section holding the texture loads, on a loader thread, once for all sections. The
  file may be a [lazy file](api-content.md#lazy-files): a mod can make a texture from another game's file by byte range at that moment.
  A file that cannot be read is logged once and the game's texture stays.
- **At start.** The sets the saved choices make take effect when the install is handed to the mods (`Mods::set_install`), before any scene
  loads the world: the shared sets and the global textures are rewritten once, when they load, so a set that took effect in the first frame
  would miss what loads before it (found by running a texture set offscreen: with the saved choice only published in the first frame the
  first screenshot showed the game's own road). Run offscreen with the choice saved
  and the replacement is in the first screenshot.
- **Limits.** A choice made during a drive re-streams the map tiles, so tile textures change within seconds. Textures of the shared
  sets and the global textures were replaced when they loaded and follow at the next drive.

## Options, choice and conflicts

- Option id `<kind>_variant`, list, first value `original`; shown while it has more than one value. The choice is saved as
  `[mod_values.nfsmw] option_<kind>_variant = "<value>"`. The Gameplay page draws them after its own rows (the variants group).
- Per kind exactly one set is chosen, plus every `always` set. The chosen sets merge: lowest `priority` first, then the mod that loads
  earlier, then set id; **the last to name a target wins**, and the clash is logged (`replacements: model 1234ABCD: a:x replaces what
  b:y replaced, and wins`) and listed by the report.
- A mod built before 1.2 that adds a value to `trees_variant` in code and ships `TREES/<VALUE>/profile.toml` still works: the game reads the
  profile as a set of kind `trees` ([effect-variants.md](effect-variants.md#profile-format-of-the-trees)). New mods declare a set.
- Replacements apply to sections as they load; added textures are uploaded once. `[[add]]` objects become map packs when a world opens (the city and alternative maps, not standalone worlds): grouped by 512 m cell, each pack loads while the camera is within 150 m of its objects; a choice made in a drive shows at the next world. The geometry file must be a plain file (not a lazy stub).

## Versioning, errors

- Files only: a 1.1 host ignores `replace/`. `bb.content` 1.2 adds `capabilities` and `report` (spec section 8) so a mod can check what the
  host does and read why a set was not taken. Errors go to the log with the mod id and the reason, and to `report`; the game goes on.
- `format = 1` is the only number read; a new field is added in a minor version and a mod sets `format` only when it needs one.

## Adding a new kind

A kind is a name, not code. For one more family of props or textures (say bushes, signs, road textures):

1. **Find the targets.** The game's solid names and texture names are in the install's section files (`blackbox_solid::read_solids`,
   `blackbox_tpk::read_texture_packs`); hash with `blackbox_hash::bstring_hash` if you only have hashes. Group the names of the same
   family (all levels of detail of a bush are separate solids: list each in `targets`).
2. **Convert the models** with your converter into solids (`write_geometry_pack`) and textures (`write_tpk`, or ship PNG/DDS), origin and
   scale in metres, z up. Pick the fit: trees `height`/`row`, everything with a footprint of its own `stand`, authored-to-frame `exact`.
3. **Write `replace/<kind>-<value>.toml`** as above: `kind`, `value`, `value_label`, `[files]`, `[[model]]`/`[[texture]]`. One file per
   offered value; two mods may offer values of one kind.
4. **Declare the files in your converter's output**, under `files/`; write the manifest last (the host re-reads manifests while a chosen
   value has no set, so a converter that works on a thread may finish after the game starts). The 2012 mod's example:
   `mods/nfsmw-2012-content/src/trees/` (`replace_manifest`).
5. **Test**: parse with `blackbox_modding::content::parse_replace_set`, build with the host's `Active` (`crates/nfsmw/src/modding/content/replace/tests.rs`
   shows generated solids and sections), then run offscreen: `nfsmw view-world --muted --screenshot out.png` with the value chosen
   (`[mod_values.nfsmw] option_<kind>_variant`).
6. Nothing in the game changes: no option row, no host code. Only a kind that needs new **behaviour** (collision of a replaced prop, a
   different draw path) needs host work: add it in `replace/active.rs` and document it in the spec.

## Worked examples

**Replace a street lamp** (`mods/example-replace` writes this):

```toml
format = 1
[set]
kind = "street_lamps"
value = "slim"
value_label = "Slim lamps"
fit = "stand"
[files]
geometry = "EXAMPLE/GEOMETRY.BIN"
textures = "EXAMPLE/TEXTURES.BIN"
[[model]]
targets = ["XB_LAMPPOST_A_00"]
model = "SLIM_LAMP"
```

**Replace a road texture** with a PNG, no geometry:

```toml
format = 1
[set]
kind = "road_textures"
value = "fresh"
value_label = "Fresh asphalt"
[[texture]]
target = "ROAD_ASPHALT_01"
file = "EXAMPLE/ASPHALT.png"
format = "bc1"
max_size = 1024
```

**Add a prop**: `[[add]] model = "BENCH" pos = [x, y, z]` in a set with `[files] geometry`.
**Models authored in the old solid's frame** (`mods/nfsmw-2012-content/src/signs/`, kinds `road_signs` and `traffic_lights`): when the mod knows the
box of each solid it replaces it can turn, scale and place its model itself, and say `fit = "exact"`: the host uses the model as it is
when it lies inside the old box grown by half of `spread - 1` and 5 cm. Such a model may hang below its origin (a signal head), and its
position markers (authored lights, `docs/specs/environment-prop-lights.md`) are kept, placed like its vertices.

**Add a prop** (parsed; placement by the host is still open, see the spec): `[[add]] model = "BENCH" pos = [x, y, z]` in a set with `[files] geometry`.
