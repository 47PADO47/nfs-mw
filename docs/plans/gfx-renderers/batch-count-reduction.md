# Plan: reduce the Bevy renderer's per-frame batch count

The Bevy renderer backend is CPU/render-thread bound by Bevy's per-frame GPU resource churn, not by our
own code. The Tracy capture ([../../benchmarks/2026-10-10-bevy-vs-native.md](../../benchmarks/2026-10-10-bevy-vs-native.md),
[../../profiling-bevy.md](../../profiling-bevy.md)) puts the single largest system at
`prepare_preprocess_bind_groups` (~0.63 ms/frame), which creates most of the ~398 `create_bind_group` calls per
frame. Those calls scale with the number of **draw batches**. Reducing the batch count is therefore the
most direct lever on the dominant cost, and unlike the alternatives it needs no `bevy_pbr` fork (ADR 0004 keeps
Bevy as a pinned dependency) and helps the native renderer too.

Our own per-frame uploads are *not* the problem: a static check (recorded in the benchmark doc) found them to be
~a dozen `write_buffer` calls per frame, against Bevy's ~575. So "reduce our writes" is not pursued.

## What splits a batch

A draw cannot be merged with another — so it costs its own batch and bind group — for two independent reasons:

1. **Material.** `BlackboxMaterial` is one asset per `MaterialKey { texture, blend, shading, glossy }`
   ([`apply/assets.rs`](../../../libs/blackbox-bevy-render/src/apply/assets.rs) `material_for`). Texture is the
   dominant multiplier: ~hundreds of distinct textures → ~hundreds of materials. `blend` (4) and `shading` (4)
   are small multipliers.
2. **Mesh.** Distinct meshes have distinct vertex buffers and cannot be instanced together no matter what
   material they use. Each unique building / tree / road-segment mesh is its own batch.

These need different fixes, and **we do not yet know the split** between them. Building a material-side fix is a
gamble until we know how much of the ~400 is material-driven versus mesh-driven. Hence the plan is
measurement-first.

## Step 1 — measure the split (implementable now, no GPU to compile)

Instrument the instance pool so that, per frame over the entities drawn that frame, it reports:

- total draws (entities drawn),
- distinct `(mesh, material)` pairs — the batch count Bevy cannot reduce by instancing,
- distinct meshes,
- distinct materials,
- the largest few batches (instances per `(mesh, material)`), to see whether instancing is happening at all.

Approach: store the `(AssetId<Mesh>, AssetId<BlackboxMaterial>)` of each spawned entity on its `Placed` entry
(it is known at spawn in `instances.rs`), then each frame tally over entries `seen == epoch` and
`log::info!` the summary every N frames (N ≈ 120) so it is readable without flooding. No new ECS query and no GPU
needed to build. One offscreen run on the RTX host (`view-world --renderer bevy --drive`, focus-safe) prints the
numbers.

**Reading the result:**

- If distinct `(mesh, material)` ≈ total draws **and** distinct materials ≫ distinct meshes → **material-driven**;
  bindless wins (Step 2a).
- If distinct meshes ≈ distinct `(mesh, material)` ≈ total draws → **mesh-driven**; bindless barely helps and we
  need mesh merging (Step 2b).
- If the largest batches already hold many instances → Bevy is instancing well and the ceiling for batch
  reduction is lower than hoped; reconsider scope.

## Step 2 — reduce the batches (gated on Step 1's numbers)

**Ruled out by Step 1 (2026-10-10):** over a full city drive (238 frame samples), `batches == meshes` on every
single sample — draws 282–1576, batches/meshes 228–923, materials 114–296. The split is **mesh-driven**
everywhere, so **2a cannot help**: even in the densest frame (843 batches) collapsing the ~240 materials leaves
843 mesh-bound batches. **2b is the path.**

**2a. Bindless material (if material-driven).** Bind all textures as a binding array and select per draw by an
index, so the texture leaves `MaterialKey`. Materials collapse from ~hundreds to ~16 (`blend` × `shading`, plus a
few glossy). `probe` already reports a `binding_arrays` capability
([`probe.rs`](../../../libs/blackbox-bevy-render/src/probe.rs)); nothing uses it yet. Keep the current per-texture
path as a fallback for adapters without binding arrays, selected from the probe.

**2b. Static-mesh merging (the path).** Step 1 showed the batch count equals the distinct visible-mesh count, so
the fix is to draw fewer, larger meshes. Group the frame's *static* instances (keyed scenery with a stable
transform — not transient or moving objects) by material `(texture, blend, shading, glossy)`, bake each source
`DrawRange`'s vertices into world space, and concatenate them into one merged mesh per group: ~531 meshes →
~207 per-material draws (and, combined later with bindless, a handful per blend/shading).

Where it could live:

- **In the backend** (`libs/blackbox-bevy-render`) — dynamic batching of the already-culled instance set the
  scene hands over each frame. Self-contained, testable in isolation, no game-side change. Risk: it trades
  Bevy's per-batch bind-group churn for per-rebuild vertex transforms and buffer uploads, so it must rebuild
  only when the visible set changes (streaming pops tiles), not every frame, and be measured to confirm the
  trade is net positive.
- **At stream time** (`crates/nfsmw` + `libs/blackbox-scenery`) — merge static scenery into per-chunk meshes
  when a tile loads, so the renderer just gets fewer, bigger meshes. Amortizes the merge over streaming and
  keeps the backend simple, but touches the streaming/LOD path and must respect material and LOD boundaries.

Either way: merging loses per-object culling/LOD within a group (merge at chunk granularity to bound this),
makes vertex buffers larger, and the merge key must include everything that splits a pipeline. Verify with
Step 3 and re-run the batch-stats counters to confirm the batch count actually drops.

**Recommended first step:** a backend dynamic-batching prototype behind a flag, merging the visible static set
per material and rebuilding on visible-set change, measured on one zone against the GPU-builder baseline. If the
net is positive, consider moving the merge to stream time to amortize it.

Either sub-step lands as small commits with the parity tests kept green.

## Step 2b scope (stream-time merge)

Grounded in the pipeline (section parse on a worker thread → `Residency::upload_arrived` uploads meshes and
builds `Vec<Placed>` → per-frame `visibility::collect` frustum-culls, runs `LodRules::choose`, emits one keyed
`Instance` per visible object → `render`).

- **Hook:** `Residency::upload_arrived` (`crates/nfsmw/src/scenes/world/residency.rs`), right after `upload_meshes`
  + `place`, where the tile's raw geometry (`SectionData.solids`: vertices/indices/groups), the placements'
  world transforms (`Placed.transform`), and the material lookup all coexist. The matching teardown already
  exists in `unload_unwanted` → `SectionResources::release`. Mirror in `finish_shared` only if needed.
  `SectionData` is dropped at the end of `upload_arrived`, so the merge must run there, not lazily.
- **Merge key:** `(TextureHandle, BlendMode, Shading)` — the `DrawRange` fields. World scenery is `Prelit`/`Sky`
  and opaque/alpha-test, so many groups across a tile share a key.
- **What it does:** per tile, for candidate placements, transform each group's vertices into world space by the
  placement transform and concatenate by material key into one merged `MeshDesc` (one `DrawRange` per key),
  splitting at 65,535-vertex windows (`indices: &[u16]`, `base_vertex: i32`). The merged chunk meshes are emitted
  as their own keyed instances; their source placements are **removed from the per-frame LOD/cull path** so
  nothing is drawn twice.
- **Candidate criteria (to keep LOD/culling where it matters):** only small, opaque/alpha-test, LOD-stable,
  non-prop scenery, committing to one LOD slot at bake time. Exclude props (need per-object `hidden`), large
  objects (want frustum culling and `None`/coarse-slot savings), and objects whose solid comes from the shared
  sets (baking duplicates shared geometry per tile).
- **Obstacles (from the pipeline map):** LOD freeze within a merged group; loss of per-object frustum culling
  (merged chunk is one AABB); shared-mesh duplication; the u16 index window; and collapsed `InstanceKey`
  identity (fine for native/immediate, changes temporal semantics for retained backends).

### Increments

1. Tile-level merge of opaque `Prelit`, non-prop, LOD-stable scenery at a fixed LOD, behind a flag; merged chunks
   drawn, source placements suppressed; measure one zone with the batch-stats counters + screenshot parity + fps.
2. Tune candidate criteria / add per-LOD-band merged meshes if the LOD freeze is visible.
3. Extend to the shared sets if worthwhile.

## Step 3 — verify

- Offscreen screenshot parity: `view-world ... --screenshot` on native and Bevy, compared with
  `cargo xtask img-diff` (reference mean ~1.6 at `2152,1399`, [../../bevy-backend.md](../../bevy-backend.md)
  "The real city"). A correct change leaves the picture within tolerance.
- fps re-run of the benchmark (visible window, run manually per the "never take focus" rule) to confirm the
  batch-count drop actually moved fps.
- The GPU parity tests (`cargo test -p blackbox-bevy-render -- --include-ignored`).

## Logistics

- The PR 10 session is active in the main checkout; do this work in an isolated worktree to avoid colliding.
- This also benefits the native renderer and feeds the milestone-8 renderer decision
  ([../../decisions/0003-renderer-after-milestone-8.md](../../decisions/0003-renderer-after-milestone-8.md)).

## Status

- [x] Step 1 — batch counters (`apply/instances.rs`, `apply/state.rs`)
- [x] Step 1 — numbers captured (RTX host, full city drive, 238 samples): `batches == meshes` on every sample;
  draws 282–1576, batches/meshes 228–923, materials 114–296. **Mesh-driven everywhere**: bindless (2a) cannot
  reduce the mesh-bound batch count; the lever is static-mesh merging (2b).
- [x] Step 2b — prototyped (behind `NFSMW_MERGE_STATIC`) and **measured: low-upside, not pursued.** A city
  drive merged only 107 of 8187 placements (1.3%); the batch count stayed at the baseline (avg ~639,
  `batches == meshes`). The exclusions are structural, not tuning: `shared` dominates (and is correct to
  exclude — shared-set models are already instanced into one batch, so baking them per tile would *add*
  batches and duplicate geometry), while `radius`/`prop` protect per-object frustum culling on the large
  unique structures. The mergeable set (small, tile-local, non-shared, non-prop) is ~1% of the scene. The
  batch count is dominated by geometry that already batches well (shared, instanced) or must stay per-object
  (large, culled), so it cannot be cut without hurting instancing or culling.
- **Conclusion:** reducing batch count is a dead end for this content. The remaining levers attack the
  per-frame bind-group *rebuild* cost directly: the CPU instance builder (sidesteps `gpu_preprocess`
  entirely — the deferred CPU-builder investigation above) or caching the bind groups (a vendored `bevy_pbr`
  patch). See docs/benchmarks/2026-10-10-bevy-vs-native.md "Next".
