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

**2b. Static-mesh merging (if mesh-driven).** Merge static tiles that share a material into larger meshes so one
batch covers many former draws. This is a bigger change touching how `AddMesh`/instances are built; scope it only
if Step 1 shows mesh-driven splits dominate.

Either sub-step lands as small commits with the parity tests kept green.

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
- [ ] Step 2b — merge static meshes that share a material
- [ ] Step 3 — parity + fps verified
