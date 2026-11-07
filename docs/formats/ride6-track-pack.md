# The RIDE 6 track pack

What `tools/ride6-tracks/export.py` writes from the user's own RIDE 6 install, and what `mods/ride6-tracks` reads to make a
world of the game. The pack is **our own format**, small and plain; it is made on the user's machine and never committed.
Where the data comes from: [ride6-tracks.md](ride6-tracks.md). Plan: [plans/tracks-and-fairhaven-feasibility.md](../plans/tracks-and-fairhaven-feasibility.md).
Evidence tags: [README](../README.md#evidence-tags).

## Why a pack

RIDE 6 is Unreal Engine 5 cooked data (IoStore, Oodle, unversioned properties that need the game's `.usmap`); the only reader is
CUE4Parse (C#). So the tracks cannot be streamed from the install by Rust code. The exporter reads the install once and keeps
only what a world of this game needs: the ground triangles, one picture per drawn ground layer, the surface under each triangle,
the lap. The mod then converts the pack to the game's own section formats, once, in the background.

## Layout

```
<pack>/pack.json     the materials, the lap, the counts
<pack>/ground.r6m    every ground triangle, grouped by material
<pack>/tex/<name>.png  the pictures, lower-case file stems, at most `--max-texture` pixels (default 1024)
<pack>/models.r6m    the distinct meshes of the placed objects (barriers, buildings, props), same layout as ground.r6m
<pack>/instances.r6i every placement of those objects
```

## Space and units **[verified]**

The glTF the exporter's tool writes is `(x, z, y) / 100` of Unreal's centimetres; the pack keeps the **game's render space**:
`render = (ue.x, -ue.y, ue.z) / 100`, metres, x forward, y left, z up (right-handed). Checked by laying the track spline over the
ground: the nearest-vertex distance of its 44 keys is 0.75 m on average with this mapping and 49 m or more with any other
signed permutation. Winding is counter-clockwise seen from outside (glTF's), kept through the turn.

## `ground.r6m`

All little endian.

| Offset | Type | Field |
|---|---|---|
| 0 | `"R6TM"` | magic |
| 4 | u32 | version, 1 |
| 8 | u32 | group count `G` |
| 12 | `G` x 16 bytes | group headers: u32 material (index in `pack.json` `materials`), u32 vertex count, u32 index count (a multiple of 3), u32 flags (bit 0: the group is collision) |
| then, per group | | `vertex count` x 12 bytes f32 x,y,z; `vertex count` x 8 bytes f32 u,v; `vertex count` x 4 bytes i8 normal x,y,z (/127) and a 0; `index count` x u32 |

Texture coordinates are final: a planar material has `u = x / tile_m`, `v = y / tile_m` already applied, so the loader does not
need to know about it. Indices are per group.

## Objects: `models.r6m` and `instances.r6i` **[verified on Okayama]**

The track's object levels (`Props_Barriers`, `Pitlane_barriers`, `Buildings`, `Props`) hold static mesh actors and **instanced mesh
components** (the barriers along the splines: `IgnitionSplineDecorationPlacementComponent`s fill hierarchical instanced meshes with a
transform per instance). The exporter keeps each distinct mesh once in `models.r6m` (the layout of `ground.r6m`; a group's flags
carry the model in bits 8 and up) and each placement in `instances.r6i`:

| Offset | Type | Field |
|---|---|---|
| 0 | `"R6TI"` | magic |
| 4 | u32 | version, 1 |
| 8 | u32 | placement count |
| 12 + 52 n | u32, 3 x f32, 9 x f32 | the model (index in `pack.json` `models`), the position (render space, metres), the three rows of the rotation and scale (row i is where the model's axis i points, scaled: the scenery format's rows) |

An instance's translation is **in world space** (checked: the first instances lie 25 to 30 m from the lap on average; adding the
component's relative location makes it 54 m). Unreal's rotator and quaternion are turned into rows with `x, -y, z`; the model's own
y axis turns round with the handedness, so its row is the negated Unreal axis (`props.py`, `render_rows`). `pack.json` `models[]` is
`{name, solid}`: barriers, the pit lane's and buildings are solid. Okayama: 9,084 placements of 233 models (8,161 barrier pieces),
368,000 triangles.

## `pack.json`

| Key | Meaning |
|---|---|
| `format` | 1 |
| `track` | the track's name in the install (`Okayama`) |
| `triangles` | total of the groups |
| `materials[]` | `name`, `texture` (a stem in `tex/` or null), `color` (rgb, used when there is no texture), `tile_m`, `uv` (`planar` or `uv0`), `surface` |
| `lap.spline` | `{closed, length_m, points: [[x, y, z]]}` the track spline sampled every 2 m (render space) |
| `lap.grid` | `[[x, y, z, heading_degrees]]` the starting grid, in grid order (heading 0 is +x, counter-clockwise) |
| `lap.line` | the same for the racing line, the authors' ideal trajectory `IdealTrajectory_1000cc_1` (3.72 km on Okayama) |
| `sea` | `{z, color}` of the track's water (the median height of its `*_Water` meshes), when it has any |
| `models` | `[{name, solid}]` of `models.r6m` |
| `lap.layout` | the sub-level the lap was read from (`SplinesA` is the full circuit) |

`surface` is one of the game's own `simsurface` names: `asphalt`, `concrete`, `grass`, `dirt`, `sand`, `gravel`, `stone`, `metal`
([collision.md](collision.md#surface-types-verified)); the mod turns it into the hash a collision pack stores.

## How the ground materials are read **[ours, measured on Okayama]**

- A slice of ground is drawn with `BASE_Terrain3_4L`, four pictures (`Layer01..04 - BaseColor - Texture`) blended by the **vertex
  colour** of the mesh (red, green, blue: layers 1 to 3; the alpha is 255; what is left is layer 4 **[unconfirmed]**: the order of
  the blend was guessed and checked by eye). The exporter gives each triangle the layer with the largest mean weight, so a
  material becomes up to four groups (`<material>#L<n>`) with hard edges between them. The pictures tile over the **world**: UV1
  of those meshes is the position in centimetres.
- The tile length is `10 m / scale` of the layer's `UV - ScaleOffset` (1 when absent): a guess that looks right, not a reading
  of the shader **[unconfirmed]**.
- The tile projection follows the normal: ground is projected from above (x and y), a steep face from the side, so a cliff is
  not a smear of the picture's edge.
- Tarmac and kerb masters draw one picture (`Tarmac - BaseColor - Texture`, `Layer01 - BaseColor - Texture`); kerbs keep their own UV0.
- Surface: the slot's `PhysMaterial` when the material has one (`PM_T_Tarmac` asphalt, `PM_T_Concrete` concrete, `PM_T_Grass`
  grass, `PM_T_Gravel` gravel, `PM_T_Sand` sand, `PM_T_Kerb` concrete, `PM_T_MetalGrid` metal); for a layered slice the layer's
  picture decides (`T_Grass*`, `T_Forest*` grass; `T_Soil*` dirt; `T_Sand*` sand; `T_Rock*` stone; `T_Tarmac*` asphalt), because the
  physical material of the layered parent is one value for the whole slice.
- Material overrides of the mesh component replace the mesh's own slots.
- Not in the pack: the water meshes themselves (only the sea's height), painted-line overlays, decals, normal and PBR pictures, the
  grass, the vegetation and the crowd.

## Measured **[ours]**

Okayama, from a warm package cache: the export takes 5 s; 658,294 triangles (the 25 ground meshes: twelve near slices, three far slices,
seven kerbs, the runnable and paddock meshes, the terrain), 56 groups, 17 pictures. `ground.r6m` 18.6 MB, pictures 22 MB (PNG, at
most 1024), `pack.json` 0.2 MB.

All **39 tracks export** with the same rules (`tools/ride6-tracks/survey.py`, 3 to 11 s each, 16 to 196 MB of pack): the ground of the
terrain level, whatever its mesh names. Triangles run from 147,000 (Vallelunga) to 7.56 million (Cappadocia Trails; Stelvio Park has 4.75
million, Blue Wave Circuit 2.25 million, Nevada 2.04 million: the open maps). **30 have a lap spline and 22 a starting grid**; the others keep
their splines in levels or under names the exporter does not read yet (no `TrackSpline` in `SplinesA`, `Splines` or `SplinesB`), and the
importer starts them at the first lap sample, or at the middle of the first asphalt group.
