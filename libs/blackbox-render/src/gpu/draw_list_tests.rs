//! Tests of the draw lists: the buckets keep the old per-pipeline walk order.

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
        two_sided: true,
    }
}

/// A mesh's draws and their masks, as `GpuMesh` keeps them.
struct TestMesh {
    draws: Vec<DrawRange>,
    masks: Vec<DrawMask>,
    single_sided: bool,
}

impl TestMesh {
    fn new(draws: Vec<DrawRange>) -> Self {
        let masks = draws.iter().map(DrawMask::of).collect();
        let single_sided = draws.iter().any(|d| !d.two_sided);
        Self { draws, masks, single_sided }
    }
}

/// Mesh slot 3 is dead and slot 4 is past the end, so runs of them must be skipped.
fn test_meshes() -> Vec<Option<TestMesh>> {
    vec![
        Some(TestMesh::new(vec![
            draw(BlendMode::Opaque, Shading::Lit, Some(1)),
            draw(BlendMode::AlphaBlend, Shading::Prelit, None),
            draw(BlendMode::Subtractive, Shading::Prelit, Some(1)),
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

fn lookup<'a>(meshes: &'a [Option<TestMesh>]) -> impl Fn(usize) -> Option<(&'a [DrawRange], &'a [DrawMask], bool)> {
    move |slot| meshes.get(slot)?.as_ref().map(|m| (m.draws.as_slice(), m.masks.as_slice(), m.single_sided))
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
                out.push((slot, Cmd { mesh, draw: index as u32, texture, instances: (start, end), face: Face::Both }));
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
                out.push(LayerCmd { mesh, draw: index as u32, instances: (start, end), face: Face::Both });
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
                out.push(LayerCmd { mesh, draw: index as u32, instances: (i as u32, i as u32 + 1), face: Face::Both });
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
fn water_lists_hold_only_the_projected_materials_draws() {
    let meshes = test_meshes();
    let textures = test_textures();
    let redirects = Redirects::default();
    let texture_of = |t: Option<TextureHandle>| resolve_texture(t, &redirects, &textures);
    let run_list = instances(&[0, 1, 1, 2, 0]);
    let mut list = DrawList::default();
    list.set_projected([2usize].into_iter());
    list.fill(&run_list, lookup(&meshes), texture_of);
    for index in 0..BLEND_ORDER.len() {
        let projected = |c: &LayerCmd| {
            let mesh = meshes[c.mesh].as_ref();
            mesh.is_some_and(|m| m.draws[c.draw as usize].shading == normal(2))
        };
        let expected: Vec<LayerCmd> = list.normal(index).iter().copied().filter(projected).collect();
        assert_eq!(list.water(index), Some(expected.as_slice()), "blend {index}");
    }
    assert_eq!(list.water(2).map(<[LayerCmd]>::len), Some(1));
    assert_eq!(list.water(0).map(<[LayerCmd]>::len), Some(0));

    // A changed set leaves the old lists unusable until the next build.
    list.set_projected([1usize, 2].into_iter());
    assert_eq!(list.water(0), None);
    list.fill(&run_list, lookup(&meshes), texture_of);
    assert_eq!(list.water(0).map(<[LayerCmd]>::len), Some(list.normal(0).len()));

    // The same set again changes nothing.
    list.set_projected([1usize, 2].into_iter());
    assert!(list.water(0).is_some());
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

#[test]
fn single_sided_meshes_split_their_runs_where_the_mirroring_changes() {
    let single = |shading| DrawRange { two_sided: false, ..draw(BlendMode::Opaque, shading, Some(1)) };
    let meshes = vec![
        Some(TestMesh::new(vec![single(Shading::Lit), draw(BlendMode::AlphaTest, Shading::Lit, Some(1))])),
        Some(TestMesh::new(vec![draw(BlendMode::Opaque, Shading::Lit, Some(1))])),
    ];
    let textures = test_textures();
    let redirects = Redirects::default();
    let texture_of = |t: Option<TextureHandle>| resolve_texture(t, &redirects, &textures);
    let mirror = Mat4::from_scale(glam::Vec3::new(1.0, -1.0, 1.0));
    let mut run_list = instances(&[0, 0, 0, 0, 1, 1]);
    run_list[1].transform = mirror;
    run_list[2].transform = mirror;
    // Data that bakes a mirror into its vertices flips the faces; a mirroring transform on top of it cancels that.
    run_list[3].transform = mirror;
    run_list[3] = run_list[3].with_flipped_winding(true);
    run_list[5].transform = mirror;
    let mut list = DrawList::default();
    list.fill(&run_list, lookup(&meshes), texture_of);
    let opaque_lit = 0;
    let got: Vec<_> = list.bucket(opaque_lit).iter().map(|c| (c.mesh, c.instances, c.face)).collect();
    assert_eq!(
        got,
        [(0, (0, 1), Face::Front), (0, (1, 3), Face::Back), (0, (3, 4), Face::Front), (1, (4, 6), Face::Both)],
        "mesh 1 draws both faces, so its mirrored instance stays in the run",
    );
    let alpha_test_lit = SHADINGS.len();
    let cutouts: Vec<_> = list.bucket(alpha_test_lit).iter().map(|c| (c.instances, c.face)).collect();
    assert_eq!(cutouts, [((0, 1), Face::Both), ((1, 3), Face::Both), ((3, 4), Face::Both)]);
}
