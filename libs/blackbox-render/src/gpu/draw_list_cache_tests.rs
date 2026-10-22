//! Structural reuse must keep current transforms and invalidate recycled resources/redirects.
use super::*;
use crate::gpu::test_support;
use blackbox_gfx::{PixelFormat, TextureDesc};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "needs a GPU"]
fn moving_transforms_reuse_selection_but_resource_and_order_changes_rebuild() {
    let _serial = test_support::serial();
    let mut r = test_support::headless((8, 8)).expect("GPU adapter");
    let mut builder = MeshBuilder::new();
    builder.state(None, BlendMode::Opaque, Shading::Prelit);
    builder.card(Vec3::ZERO, 1.0, 1.0, 0.0, prelit([1.0; 3], 1.0));
    let mesh = builder.finish().upload(&mut r, "cache lifetime fixture");
    let mut instances = vec![Instance::new(mesh, Mat4::IDENTITY)];
    let mut draws = DrawList::default();
    let build = |list: &mut DrawList, r: &crate::Renderer, instances: &[Instance]| {
        list.build(instances, r.draw_revision, &r.meshes, &r.redirects, &r.textures)
    };
    assert!(build(&mut draws, &r, &instances));
    let first: Vec<_> = (0..BUCKETS).flat_map(|i| draws.bucket(i)).copied().collect();
    assert!(!first.is_empty());
    instances[0].transform = Mat4::from_translation(Vec3::new(3.0, 1.0, 0.0));
    assert!(!build(&mut draws, &r, &instances), "transforms are uploaded independently every frame");
    instances.push(instances[0]);
    assert!(build(&mut draws, &r, &instances), "instance ranges changed");
    let tex = r.create_texture(&TextureDesc {
        label: "redirect target",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&[255, 0, 0, 255]],
    });
    assert!(build(&mut draws, &r, &instances));
    r.redirect_texture(crate::TextureHandle::from_raw(0), Some(tex));
    assert!(build(&mut draws, &r, &instances));
    r.redirect_texture(crate::TextureHandle::from_raw(0), Some(tex));
    assert!(!build(&mut draws, &r, &instances), "an unchanged redirect is not a resource change");
    r.destroy_texture(tex);
    assert!(build(&mut draws, &r, &instances));
    r.destroy_mesh(mesh);
    assert!(build(&mut draws, &r, &instances));
    assert!((0..BUCKETS).all(|i| draws.bucket(i).is_empty()));
}
