//! A scene pass's draw lists: every draw of one view's instances, sorted into a list per pipeline. The colour
//! pass walks each pipeline's bucket, and the normal-map, window and sky passes walk their own lists, instead
//! of each re-walking every instance and its draws. Each list keeps the order the old per-pipeline walk drew in,
//! so blended draws stay in the order they were given.

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
            let mesh = instances[start].mesh;
            let end = start + instances[start..].iter().take_while(|i| i.mesh == mesh).count();
            if let Some((draws, masks)) = draws_of(mesh.raw()) {
                self.push_run(mesh.raw(), draws, masks, (start as u32, end as u32), &texture_of);
            }
            start = end;
        }
    }

    /// Add one run of `mesh`'s draws, drawn over `instances`, in order. Each draw goes into every list its mask
    /// names. A colour draw is skipped when it has no texture to bind.
    fn push_run(
        &mut self,
        mesh: usize,
        draws: &[DrawRange],
        masks: &[DrawMask],
        instances: (u32, u32),
        texture_of: &impl Fn(Option<TextureHandle>) -> Option<usize>,
    ) {
        for (index, (d, mask)) in draws.iter().zip(masks).enumerate() {
            let draw = index as u32;
            let layer = LayerCmd { mesh, draw, instances };
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
            let cmd = Cmd { mesh, draw, texture, instances };
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
            self.sky.extend(sky_draws.map(|(index, _)| LayerCmd { mesh, draw: index as u32, instances: (i, i + 1) }));
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
mod tests {
    use super::*;
    use crate::{GlossyMaterialHandle, MeshHandle};
    use blackbox_gfx::{NormalMaterialHandle, SkyMaterialHandle, WindowMaterialHandle};
    use glam::Mat4;

    fn glossy(raw: usize) -> Shading {
        Shading::Glossy(GlossyMaterialHandle::from_raw(raw))
    }

    fn normal(raw: usize) -> Shading {
        Shading::WorldNormal(NormalMaterialHandle::from_raw(raw))
    }

    fn window(raw: usize) -> Shading {
        Shading::WorldWindow(WindowMaterialHandle::from_raw(raw))
    }

    fn sky_layer(raw: usize) -> Shading {
        Shading::SkyLayer(SkyMaterialHandle::from_raw(raw))
    }

    fn draw(blend: BlendMode, shading: Shading, texture: Option<usize>) -> DrawRange {
        DrawRange {
            first_index: 0,
            index_count: 3,
            base_vertex: 0,
            texture: texture.map(TextureHandle::from_raw),
            blend,
            shading,
        }
    }

    /// A mesh's draws and their masks, as `GpuMesh` keeps them.
    struct TestMesh {
        draws: Vec<DrawRange>,
        masks: Vec<DrawMask>,
    }

    impl TestMesh {
        fn new(draws: Vec<DrawRange>) -> Self {
            let masks = draws.iter().map(DrawMask::of).collect();
            Self { draws, masks }
        }
    }

    /// Mesh slot 3 is dead and slot 4 is past the end, so runs of them must be skipped.
    fn test_meshes() -> Vec<Option<TestMesh>> {
        vec![
            Some(TestMesh::new(vec![
                draw(BlendMode::Opaque, Shading::Lit, Some(1)),
                draw(BlendMode::AlphaBlend, Shading::Prelit, None),
                draw(BlendMode::Opaque, glossy(2), Some(2)),
                draw(BlendMode::Additive, Shading::Sky, Some(7)),
                draw(BlendMode::AlphaTest, Shading::Lit, Some(2)),
                draw(BlendMode::Opaque, normal(1), Some(1)),
                draw(BlendMode::AlphaBlend, window(1), None),
            ])),
            Some(TestMesh::new(vec![
                draw(BlendMode::Opaque, glossy(3), Some(1)),
                draw(BlendMode::Opaque, glossy(4), Some(1)),
                draw(BlendMode::Additive, glossy(5), None),
                draw(BlendMode::AlphaBlend, normal(2), Some(2)),
                draw(BlendMode::Opaque, normal(1), Some(1)),
                draw(BlendMode::Additive, window(2), Some(1)),
                draw(BlendMode::Opaque, Shading::World, Some(1)),
            ])),
            Some(TestMesh::new(vec![
                draw(BlendMode::AlphaBlend, Shading::Sky, Some(1)),
                draw(BlendMode::Opaque, Shading::Lit, Some(1)),
                draw(BlendMode::AlphaBlend, glossy(6), Some(2)),
                draw(BlendMode::Opaque, sky_layer(1), None),
                draw(BlendMode::AlphaBlend, sky_layer(2), Some(2)),
                draw(BlendMode::AlphaBlend, window(1), Some(2)),
            ])),
            None,
        ]
    }

    fn lookup<'a>(meshes: &'a [Option<TestMesh>]) -> impl Fn(usize) -> Option<(&'a [DrawRange], &'a [DrawMask])> {
        move |slot| meshes.get(slot)?.as_ref().map(|m| (m.draws.as_slice(), m.masks.as_slice()))
    }

    fn test_textures() -> Slots<u8> {
        let mut textures = Slots::new();
        textures.insert(0);
        textures.insert(1);
        textures.insert(2);
        textures
    }

    fn instances(meshes: &[usize]) -> Vec<Instance> {
        meshes.iter().map(|&m| Instance::new(MeshHandle::from_raw(m), Mat4::IDENTITY)).collect()
    }

    /// The runs of consecutive instances of one mesh, as `(mesh, start, end)`.
    fn runs(instances: &[Instance]) -> Vec<(usize, u32, u32)> {
        let mut out = Vec::new();
        let mut start = 0;
        while start < instances.len() {
            let mesh = instances[start].mesh;
            let end = start + instances[start..].iter().take_while(|i| i.mesh == mesh).count();
            out.push((mesh.raw(), start as u32, end as u32));
            start = end;
        }
        out
    }

    /// The old colour pass: for every pipeline in order, walk every run of instances and draw every
    /// matching draw of its mesh.
    fn reference(
        instances: &[Instance],
        meshes: &[Option<TestMesh>],
        texture_of: impl Fn(Option<TextureHandle>) -> Option<usize>,
    ) -> Vec<(usize, Cmd)> {
        let mut out = Vec::new();
        for (slot, (mode, shading)) in combinations().enumerate() {
            for (mesh, start, end) in runs(instances) {
                let draws = meshes.get(mesh).and_then(|m| m.as_ref()).map_or(&[][..], |m| m.draws.as_slice());
                for (index, d) in draws.iter().enumerate() {
                    if d.blend != mode || !takes(d.shading, shading) {
                        continue;
                    }
                    let Some(texture) = texture_of(d.texture) else { continue };
                    out.push((slot, Cmd { mesh, draw: index as u32, texture, instances: (start, end) }));
                }
            }
        }
        out
    }

    /// The old normal-map and window passes: per blend mode, every run's matching draws in order.
    fn reference_layers(
        instances: &[Instance],
        meshes: &[Option<TestMesh>],
        wanted: impl Fn(&DrawRange) -> bool,
        mode: BlendMode,
    ) -> Vec<LayerCmd> {
        let mut out = Vec::new();
        for (mesh, start, end) in runs(instances) {
            let draws = meshes.get(mesh).and_then(|m| m.as_ref()).map_or(&[][..], |m| m.draws.as_slice());
            for (index, d) in draws.iter().enumerate() {
                if d.blend == mode && wanted(d) {
                    out.push(LayerCmd { mesh, draw: index as u32, instances: (start, end) });
                }
            }
        }
        out
    }

    /// The old sky pass: every instance on its own, every sky draw of its mesh in order.
    fn reference_sky(instances: &[Instance], meshes: &[Option<TestMesh>]) -> Vec<LayerCmd> {
        let mut out = Vec::new();
        for (i, instance) in instances.iter().enumerate() {
            let mesh = instance.mesh.raw();
            let draws = meshes.get(mesh).and_then(|m| m.as_ref()).map_or(&[][..], |m| m.draws.as_slice());
            for (index, d) in draws.iter().enumerate() {
                if matches!(d.shading, Shading::SkyLayer(_)) {
                    out.push(LayerCmd { mesh, draw: index as u32, instances: (i as u32, i as u32 + 1) });
                }
            }
        }
        out
    }

    fn flatten(list: &DrawList) -> Vec<(usize, Cmd)> {
        (0..BUCKETS).flat_map(|slot| list.bucket(slot).iter().map(move |&cmd| (slot, cmd))).collect()
    }

    fn check_layers(list: &DrawList, instances: &[Instance], meshes: &[Option<TestMesh>]) {
        for (index, mode) in BLEND_ORDER.into_iter().enumerate() {
            let is_normal = |d: &DrawRange| matches!(d.shading, Shading::WorldNormal(_));
            let is_window = |d: &DrawRange| matches!(d.shading, Shading::WorldWindow(_));
            assert_eq!(list.normal(index), reference_layers(instances, meshes, is_normal, mode), "{mode:?}");
            assert_eq!(list.window(index), reference_layers(instances, meshes, is_window, mode), "{mode:?}");
        }
        assert_eq!(list.sky(), reference_sky(instances, meshes));
    }

    #[test]
    fn buckets_hold_the_draws_in_the_old_loop_order() {
        let meshes = test_meshes();
        let mut redirects = Redirects::default();
        redirects.insert(2, 1);
        let textures = test_textures();
        let texture_of = |t: Option<TextureHandle>| resolve_texture(t, &redirects, &textures);
        let run_list = instances(&[0, 0, 1, 2, 2, 0, 1, 1, 3, 4, 0, 2]);
        let mut list = DrawList::default();
        list.fill(&run_list, lookup(&meshes), texture_of);
        let got = flatten(&list);
        assert!(!got.is_empty());
        assert_eq!(got, reference(&run_list, &meshes, texture_of));
        check_layers(&list, &run_list, &meshes);
        assert!(!list.sky().is_empty() && !list.normal(0).is_empty() && !list.window(2).is_empty());

        // A second build replaces the first rather than adding to it.
        let fewer = instances(&[2, 1]);
        list.fill(&fewer, lookup(&meshes), texture_of);
        assert_eq!(flatten(&list), reference(&fewer, &meshes, texture_of));
        check_layers(&list, &fewer, &meshes);

        let none = instances(&[]);
        list.fill(&none, lookup(&meshes), texture_of);
        assert!(flatten(&list).is_empty() && list.sky().is_empty() && list.normal(0).is_empty());
    }

    #[test]
    fn sky_draws_stay_one_instance_each_and_normal_draws_share_a_run() {
        let meshes = test_meshes();
        let textures = test_textures();
        let redirects = Redirects::default();
        let texture_of = |t: Option<TextureHandle>| resolve_texture(t, &redirects, &textures);
        let mut list = DrawList::default();
        list.fill(&instances(&[2, 2, 2]), lookup(&meshes), texture_of);
        let sky: Vec<_> = list.sky().iter().map(|c| (c.draw, c.instances)).collect();
        assert_eq!(sky, [(3, (0, 1)), (4, (0, 1)), (3, (1, 2)), (4, (1, 2)), (3, (2, 3)), (4, (2, 3))]);
        list.fill(&instances(&[1, 1, 1]), lookup(&meshes), texture_of);
        assert_eq!(list.normal(0).iter().map(|c| c.instances).collect::<Vec<_>>(), [(0, 3)]);
    }

    #[test]
    fn texture_slots_follow_redirects_then_fall_back_to_white() {
        let textures = test_textures();
        let mut redirects = Redirects::default();
        redirects.insert(5, 1);
        redirects.insert(6, 9);
        assert_eq!(resolve_texture(None, &redirects, &textures), Some(0));
        assert_eq!(resolve_texture(Some(TextureHandle::from_raw(2)), &redirects, &textures), Some(2));
        assert_eq!(resolve_texture(Some(TextureHandle::from_raw(5)), &redirects, &textures), Some(1));
        // A redirect to a dead slot, and a dead slot with no redirect, both fall back to white.
        assert_eq!(resolve_texture(Some(TextureHandle::from_raw(6)), &redirects, &textures), Some(0));
        assert_eq!(resolve_texture(Some(TextureHandle::from_raw(7)), &redirects, &textures), Some(0));
    }

    #[test]
    fn no_texture_when_white_is_gone() {
        let mut textures: Slots<u8> = Slots::new();
        let white = textures.insert(0);
        textures.remove(white);
        assert_eq!(resolve_texture(None, &Redirects::default(), &textures), None);
    }
}
