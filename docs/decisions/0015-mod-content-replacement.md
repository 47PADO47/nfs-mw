# ADR 0015: One way to replace and add models, textures and worlds

Status: accepted (2026-11-08). Builds on [ADR 0005](0005-native-mod-plugins.md) (files first, `bb.content`) and the effect-variant
machinery of [effect-variants.md](../modding/effect-variants.md). Spec: [mod-content-replacement.md](../specs/mod-content-replacement.md),
[mod-content-replacement-worlds.md](../specs/mod-content-replacement-worlds.md). Mod-facing: [api-replace.md](../modding/api-replace.md).

## Context

Three mechanisms had grown separately: the **trees swap** (a `TREES/<id>/profile.toml`, its own option and its own code path from
the option to the streamer), **scenery packs** (objects placed per tile, no way to replace anything) and **worlds** (a manifest plus
packs, no way for a second mod to change one). The 2012 content mod wants to replace trees, bushes, signs, lamps and road textures
and to add props; the RIDE 6 and Fairhaven mods want to change worlds they generate. Every new kind would have meant new host code.

## Decision

1. **One declaration: the replacement set** (`replace/<id>.toml`, `bb.content` 1.2). It lists the game's models and textures it
   replaces (by name or hash), the files that hold the replacements, and objects it adds. It belongs to a **kind**, a plain name the
   mod picks. **A kind is data, not code.**
2. **One option per kind**, `<kind>_variant`, built on the existing variant machinery: the game's `original` plus a value per set a
   mod offers. The host adds the values from the manifests, so a mod needs no code. `always` sets have no option (a mod's own fixes).
3. **One mechanism in the host**: the chosen sets merge into one `Active` set of replacements applied by the streamer's rewrite hook
   on every section a loader thread parses. The trees swap is that mechanism with `kind = "trees"`; the old `TREES/` profile is read
   as a set so existing mods keep working unchanged.
4. **One fitter** with four rules (`height`, `row`, `stand`, `exact`), generalising the per-stretch logic of the trees and adding the
   standing-on-the-floor rule. Per-kind behaviour is the choice of rule in the manifest, not host code.
5. **Textures by name hash**, any size, with the replaced texture's draw state kept by default, BC1/2/3 passed through, other
   pictures converted, files read lazily on the first section that needs them (so a lazy file can be a byte-range read of another game).
6. **Worlds**: a world manifest gains `base` (an alternative map is the city plus packs), `environment` and start `grid`; a **world
   patch** (`world-patches/<id>.toml`) lets another mod change a world: rename, start, environment, slots, remove/replace/add packs.
   Ownership: a patch applies from the lowest priority up; the last to set a thing wins and the clash is reported.
7. **Conflicts are deterministic and reported**: lowest priority first, then load order, then id; last wins; every clash is logged and
   in the mod's report.
8. **Versioning**: the files are readable by a 1.1 host (which ignores them). `bb.content` 1.2 appends two functions, `capabilities`
   (bit flags) and `report` (text), so a mod can ask what the host does and why a set was not taken. A breaking manifest change would
   be `format = 2`; the ABI major is not touched.
9. **Genericity**: parsers and merge rules live in `libs/blackbox-modding` with no game names; the fitter, the streamer hook, `_DEINST`
   copies and the option plumbing are NFS:MW bindings in `crates/nfsmw`. A mod for another Black Box game declares the same files.

## Consequences

- A new family of props or textures is a manifest, no host change (see [api-replace.md](../modding/api-replace.md#adding-a-new-kind)).
- Replacing collision, physics or sound of a prop is out of scope: only models and textures, and objects through scenery packs.
- Textures in the shared sets follow a choice by a texture redirect drawn over the one the upload made (pictures made on a worker thread);
  tile content follows within seconds; the shared sets' models and the global textures follow at the next start.
- The trees' option text stays the game's catalogue; kinds a mod declares show the mod's label.
- Rejected: a separate option machinery per kind (what the variants did), and a code-only registry (`content.props().register(..)`):
  both make every mod write code for what is data.
