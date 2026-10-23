//! A scene pass's draw lists: every draw of one view's instances, sorted into a list per pipeline. The colour
//! pass walks each pipeline's bucket, and the normal-map, window and sky passes walk their own lists, instead
//! of each re-walking every instance and its draws. Each list keeps the order the old per-pipeline walk drew in,
//! so blended draws stay in the order they were given.

// The instance store lives beside the draw lists: a view's lists draw instance ranges of the slot list the store
// gave that view.
#[path = "instance_store.rs"]
pub(super) mod instance_store;
#[path = "instance_view.rs"]
pub(super) mod instance_view;

use super::cull::Face;
use super::draw_mask::DrawMask;
use super::meshes::GpuMesh;
use super::pipelines::{BLEND_ORDER, SHADINGS};
use super::redirects::Redirects;
use super::slots::Slots;
use crate::{BlendMode, DrawRange, Instance, Shading, TextureHandle};

/// One bucket per (blend, shading) pair, in draw order.
pub(super) const BUCKETS: usize = BLEND_ORDER.len() * SHADINGS.len();
const _: () = assert!(BUCKETS <= 32);

/// The (blend, shading) pairs in draw order: blend modes outermost, shadings inside. The position of a
/// pair is its bucket index.
pub(super) fn combinations() -> impl Iterator<Item = (BlendMode, Shading)> {
    BLEND_ORDER.into_iter().flat_map(|b| SHADINGS.into_iter().map(move |s| (b, s)))
}

/// Whether a draw of shading `draw` belongs in the bucket of pipeline `bucket`: the draws that share it, and the
/// neighbours a view may borrow (the frame's draw rules pick among them).
pub(super) fn takes(draw: Shading, bucket: Shading) -> bool {
    draw.same_pipeline(bucket)
        || (bucket == Shading::WorldNoFog && matches!(draw, Shading::World | Shading::WorldNormal(_)))
        || (bucket == Shading::Prelit && matches!(draw, Shading::WorldNormal(_) | Shading::WorldWindow(_)))
}

/// Whether this view draws `d` in the pipeline `shading`. The bucket holds the draws that share a pipeline with
/// it; the rules here decide which of those (and which neighbours a view borrows) this view draws.
pub(super) fn drawn_here(d: &DrawRange, shading: Shading, projected: bool, original: bool) -> bool {
    let own = d.shading.same_pipeline(shading) && !(projected && d.shading == Shading::World);
    let projected_borrow =
        projected && shading == Shading::WorldNoFog && matches!(d.shading, Shading::World | Shading::WorldNormal(_));
    let prelit_borrow = !projected
        && !original
        && shading == Shading::Prelit
        && matches!(d.shading, Shading::WorldNormal(_) | Shading::WorldWindow(_));
    own || projected_borrow || prelit_borrow
}

/// One draw of one run of instances, ready to bind and draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Cmd {
    /// The mesh's slot. Its vertex and index buffers are bound for the draw.
    pub(super) mesh: usize,
    /// Index into the mesh's `draws`.
    pub(super) draw: u32,
    /// The texture's slot, after redirects and the white fallback.
    pub(super) texture: usize,
    /// The instances drawn: a range of the instance buffer.
    pub(super) instances: (u32, u32),
    /// The faces drawn, before the view's own mirroring (`Face::seen`).
    pub(super) face: Face,
}

/// One draw of a normal-map, window or sky material. The material is the draw's own shading, so it is read from
/// the mesh when the draw is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LayerCmd {
    /// The mesh's slot.
    pub(super) mesh: usize,
    /// Index into the mesh's `draws`.
    pub(super) draw: u32,
    /// The instances drawn: a range of the instance buffer.
    pub(super) instances: (u32, u32),
    /// The faces drawn, before the view's own mirroring (`Face::seen`).
    pub(super) face: Face,
}

/// The run of instances that starts at `start`: it ends before the next instance of another mesh, and when
/// `by_mirror` before the next one that mirrors otherwise ([`Instance::mirrored`]), since a single-sided draw keeps
/// the other face for those. Returns the run's end and whether it mirrors (always `false` without `by_mirror`).
pub(super) fn run(instances: &[Instance], start: usize, by_mirror: bool) -> (usize, bool) {
    let first = &instances[start];
    let mirrored = by_mirror && first.mirrored();
    let same = |i: &&Instance| i.mesh == first.mesh && (!by_mirror || i.mirrored() == mirrored);
    (start + instances[start..].iter().take_while(same).count(), mirrored)
}

/// The draws of one scene pass, sorted into lists. Kept on the renderer so the lists reuse their capacity from
/// frame to frame.
#[derive(Default)]
pub(super) struct DrawList {
    buckets: Vec<Vec<Cmd>>,
    /// Normal-map draws, one list per blend mode in `BLEND_ORDER`. A run of one mesh is one instanced draw.
    normal: [Vec<LayerCmd>; BLEND_ORDER.len()],
    /// Window draws, one list per blend mode in `BLEND_ORDER`.
    window: [Vec<LayerCmd>; BLEND_ORDER.len()],
    /// Sky layer draws of every layer kind, one instance at a time in instance order (the layers blend, so the
    /// order between instances is part of the picture).
    sky: Vec<LayerCmd>,
    /// Every instance's matrix at a persistent slot; the views upload slot lists into it.
    pub(super) store: instance_store::InstanceStore,
}

impl DrawList {
    /// The draws for the pipeline at bucket `slot`, in draw order. Empty before the first build.
    pub(super) fn bucket(&self, slot: usize) -> &[Cmd] {
        let Some(bucket) = self.buckets.get(slot) else { return &[] };
        bucket
    }

    /// The normal-map draws of blend mode number `blend` (in `BLEND_ORDER`), in draw order.
    pub(super) fn normal(&self, blend: usize) -> &[LayerCmd] {
        let Some(list) = self.normal.get(blend) else { return &[] };
        list
    }

    /// The window draws of blend mode number `blend` (in `BLEND_ORDER`), in draw order.
    pub(super) fn window(&self, blend: usize) -> &[LayerCmd] {
        let Some(list) = self.window.get(blend) else { return &[] };
        list
    }

    /// The sky layer draws of every kind, in instance order.
    pub(super) fn sky(&self) -> &[LayerCmd] {
        &self.sky
    }

    /// Sort the draws of `instances` into the lists, replacing what an earlier build put there.
    pub(super) fn build(
        &mut self,
        instances: &[Instance],
        meshes: &Slots<GpuMesh>,
        redirects: &Redirects,
        textures: &Slots<wgpu::BindGroup>,
    ) {
        let _trace = blackbox_trace::scope("Renderer CPU: build draw list");
        self.fill(
            instances,
            |slot| meshes.get(slot).map(|mesh| (mesh.draws.as_slice(), mesh.draw_masks.as_slice())),
            |texture| resolve_texture(texture, redirects, textures),
        );
    }

    /// The walk behind [`Self::build`], over plain draw lists so the tests need no device. `draws_of`
    /// gives a mesh's draws and their masks (`None` for a mesh that is not live); `texture_of` gives a draw's
    /// texture slot.
    fn fill<'a>(
        &mut self,
        instances: &[Instance],
        draws_of: impl Fn(usize) -> Option<(&'a [DrawRange], &'a [DrawMask])>,
        texture_of: impl Fn(Option<TextureHandle>) -> Option<usize>,
    ) {
        self.buckets.resize_with(BUCKETS, Vec::new);
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        for list in self.normal.iter_mut().chain(&mut self.window) {
            list.clear();
        }
        self.sky.clear();
        let mut start = 0;
        while start < instances.len() {
            let mesh = instances[start].mesh.raw();
            let Some((draws, masks)) = draws_of(mesh) else {
                start = run(instances, start, false).0;
                continue;
            };
            let (end, mirrored) = run(instances, start, draws.iter().any(|d| !d.two_sided));
            self.push_run(mesh, (draws, masks), ((start as u32, end as u32), mirrored), &texture_of);
            start = end;
        }
    }

    /// Add one run of `mesh`'s draws (with their masks), drawn over `instances` that all mirror or not, in order.
    /// Each draw goes into every list its mask names. A colour draw is skipped when it has no texture to bind.
    fn push_run(
        &mut self,
        mesh: usize,
        (draws, masks): (&[DrawRange], &[DrawMask]),
        (instances, mirrored): ((u32, u32), bool),
        texture_of: &impl Fn(Option<TextureHandle>) -> Option<usize>,
    ) {
        for (index, (d, mask)) in draws.iter().zip(masks).enumerate() {
            let draw = index as u32;
            let face = Face::of(d, mirrored);
            let layer = LayerCmd { mesh, draw, instances, face };
            for (blend, (normal, window)) in self.normal.iter_mut().zip(&mut self.window).enumerate() {
                if mask.normal(blend) {
                    normal.push(layer);
                }
                if mask.window(blend) {
                    window.push(layer);
                }
            }
            let mut slots = mask.colour_slots().peekable();
            if slots.peek().is_none() {
                continue;
            }
            let Some(texture) = texture_of(d.texture) else { continue };
            let cmd = Cmd { mesh, draw, texture, instances, face };
            for slot in slots {
                self.buckets[slot].push(cmd);
            }
        }
        // Sky layers blend without depth writes, so their order is the picture: every instance on its own, each with
        // all its sky draws in mesh order, as the old pass drew them.
        if !masks.iter().any(|mask| mask.sky()) {
            return;
        }
        for i in instances.0..instances.1 {
            let sky_draws = masks.iter().enumerate().filter(|(_, mask)| mask.sky());
            self.sky.extend(sky_draws.map(|(index, _)| LayerCmd {
                mesh,
                draw: index as u32,
                instances: (i, i + 1),
                face: Face::of(&draws[index], mirrored),
            }));
        }
    }
}

/// The texture slot a draw binds: its redirect when it has one (animated textures), else the white
/// fallback in slot 0. `None` when neither slot is live.
fn resolve_texture<T>(texture: Option<TextureHandle>, redirects: &Redirects, textures: &Slots<T>) -> Option<usize> {
    let wanted = texture.map(|t| redirects.resolve(t.raw()));
    [wanted, Some(0)].into_iter().flatten().find(|&slot| textures.get(slot).is_some())
}

#[cfg(test)]
#[path = "draw_list_tests.rs"]
mod tests;
