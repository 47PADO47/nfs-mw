//! The colour pass's draw list: every draw of one pass's instances, sorted into a bucket per pipeline.
//! The pass then walks each pipeline's bucket instead of every instance run, and each bucket keeps
//! the order the old per-pipeline walk drew in.

use std::collections::HashMap;

use super::meshes::GpuMesh;
use super::pipelines::{BLEND_ORDER, SHADINGS};
use super::slots::Slots;
use crate::{BlendMode, DrawRange, Instance, Shading, TextureHandle};

/// One bucket per (blend, shading) pair, in draw order.
const BUCKETS: usize = BLEND_ORDER.len() * SHADINGS.len();

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

/// The draws of one colour pass, bucketed by pipeline. Kept on the renderer so the buckets reuse their
/// capacity from frame to frame.
#[derive(Default)]
pub(super) struct DrawList {
    buckets: Vec<Vec<Cmd>>,
}

impl DrawList {
    /// The draws for the pipeline at bucket `slot`, in draw order. Empty before the first build.
    pub(super) fn bucket(&self, slot: usize) -> &[Cmd] {
        let Some(bucket) = self.buckets.get(slot) else { return &[] };
        bucket
    }

    /// Sort the draws of `instances` into the buckets, replacing what an earlier build put there.
    pub(super) fn build(
        &mut self,
        instances: &[Instance],
        meshes: &Slots<GpuMesh>,
        redirects: &HashMap<usize, usize>,
        textures: &Slots<wgpu::BindGroup>,
    ) {
        self.fill(
            instances,
            |slot| meshes.get(slot).map(|mesh| mesh.draws.as_slice()),
            |texture| resolve_texture(texture, redirects, textures),
        );
    }

    /// The walk behind [`Self::build`], over plain draw lists so the tests need no device. `draws_of`
    /// gives a mesh's draws (`None` for a mesh that is not live); `texture_of` gives a draw's texture slot.
    fn fill<'a>(
        &mut self,
        instances: &[Instance],
        draws_of: impl Fn(usize) -> Option<&'a [DrawRange]>,
        texture_of: impl Fn(Option<TextureHandle>) -> Option<usize>,
    ) {
        self.buckets.resize_with(BUCKETS, Vec::new);
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        let mut start = 0;
        while start < instances.len() {
            let mesh = instances[start].mesh;
            let end = start + instances[start..].iter().take_while(|i| i.mesh == mesh).count();
            if let Some(draws) = draws_of(mesh.raw()) {
                self.push_run(mesh.raw(), draws, (start as u32, end as u32), &texture_of);
            }
            start = end;
        }
    }

    /// Add one run of `mesh`'s draws, drawn over `instances`, in order. Each draw goes into every bucket
    /// whose blend and shading it matches, and is skipped when it has no texture to bind.
    fn push_run(
        &mut self,
        mesh: usize,
        draws: &[DrawRange],
        instances: (u32, u32),
        texture_of: &impl Fn(Option<TextureHandle>) -> Option<usize>,
    ) {
        for (index, d) in draws.iter().enumerate() {
            let Some(texture) = texture_of(d.texture) else { continue };
            let cmd = Cmd { mesh, draw: index as u32, texture, instances };
            for (bucket, (blend, shading)) in self.buckets.iter_mut().zip(combinations()) {
                if d.blend == blend && takes(d.shading, shading) {
                    bucket.push(cmd);
                }
            }
        }
    }
}

/// The texture slot a draw binds: its redirect when it has one (animated textures), else the white
/// fallback in slot 0. `None` when neither slot is live.
fn resolve_texture<T>(
    texture: Option<TextureHandle>,
    redirects: &HashMap<usize, usize>,
    textures: &Slots<T>,
) -> Option<usize> {
    let wanted = texture.map(|t| redirects.get(&t.raw()).copied().unwrap_or(t.raw()));
    [wanted, Some(0)].into_iter().flatten().find(|&slot| textures.get(slot).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GlossyMaterialHandle, MeshHandle};
    use glam::Mat4;

    fn glossy(raw: usize) -> Shading {
        Shading::Glossy(GlossyMaterialHandle::from_raw(raw))
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

    /// Mesh slot 3 is dead and slot 4 is past the end, so runs of them must be skipped.
    fn test_meshes() -> Vec<Option<Vec<DrawRange>>> {
        vec![
            Some(vec![
                draw(BlendMode::Opaque, Shading::Lit, Some(1)),
                draw(BlendMode::AlphaBlend, Shading::Prelit, None),
                draw(BlendMode::Opaque, glossy(2), Some(2)),
                draw(BlendMode::Additive, Shading::Sky, Some(7)),
                draw(BlendMode::AlphaTest, Shading::Lit, Some(2)),
            ]),
            Some(vec![
                draw(BlendMode::Opaque, glossy(3), Some(1)),
                draw(BlendMode::Opaque, glossy(4), Some(1)),
                draw(BlendMode::Additive, glossy(5), None),
            ]),
            Some(vec![
                draw(BlendMode::AlphaBlend, Shading::Sky, Some(1)),
                draw(BlendMode::Opaque, Shading::Lit, Some(1)),
                draw(BlendMode::AlphaBlend, glossy(6), Some(2)),
            ]),
            None,
        ]
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

    /// The old colour pass: for every pipeline in order, walk every run of instances and draw every
    /// matching draw of its mesh.
    fn reference(
        instances: &[Instance],
        meshes: &[Option<Vec<DrawRange>>],
        texture_of: impl Fn(Option<TextureHandle>) -> Option<usize>,
    ) -> Vec<(usize, Cmd)> {
        let mut out = Vec::new();
        let pipelines = BLEND_ORDER.into_iter().flat_map(|b| SHADINGS.into_iter().map(move |s| (b, s)));
        for (slot, (mode, shading)) in pipelines.enumerate() {
            let mut start = 0;
            while start < instances.len() {
                let mesh = instances[start].mesh;
                let end = start + instances[start..].iter().take_while(|i| i.mesh == mesh).count();
                let draws = meshes.get(mesh.raw()).and_then(|m| m.as_deref()).unwrap_or(&[]);
                for (index, d) in draws.iter().enumerate() {
                    if d.blend != mode || !takes(d.shading, shading) {
                        continue;
                    }
                    let Some(texture) = texture_of(d.texture) else { continue };
                    let cmd =
                        Cmd { mesh: mesh.raw(), draw: index as u32, texture, instances: (start as u32, end as u32) };
                    out.push((slot, cmd));
                }
                start = end;
            }
        }
        out
    }

    fn flatten(list: &DrawList) -> Vec<(usize, Cmd)> {
        (0..BUCKETS).flat_map(|slot| list.bucket(slot).iter().map(move |&cmd| (slot, cmd))).collect()
    }

    #[test]
    fn buckets_hold_the_draws_in_the_old_loop_order() {
        let meshes = test_meshes();
        let redirects = HashMap::from([(2usize, 1usize)]);
        let textures = test_textures();
        let texture_of = |t: Option<TextureHandle>| resolve_texture(t, &redirects, &textures);
        let runs = instances(&[0, 0, 1, 2, 2, 0, 1, 1, 3, 4, 0, 2]);
        let mut list = DrawList::default();
        list.fill(&runs, |slot| meshes.get(slot).and_then(|m| m.as_deref()), texture_of);
        let got = flatten(&list);
        assert!(!got.is_empty());
        assert_eq!(got, reference(&runs, &meshes, texture_of));

        // A second build replaces the first rather than adding to it.
        let fewer = instances(&[2, 1]);
        list.fill(&fewer, |slot| meshes.get(slot).and_then(|m| m.as_deref()), texture_of);
        assert_eq!(flatten(&list), reference(&fewer, &meshes, texture_of));
    }

    #[test]
    fn texture_slots_follow_redirects_then_fall_back_to_white() {
        let textures = test_textures();
        let redirects = HashMap::from([(5usize, 1usize), (6usize, 9usize)]);
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
        assert_eq!(resolve_texture(None, &HashMap::new(), &textures), None);
    }
}
