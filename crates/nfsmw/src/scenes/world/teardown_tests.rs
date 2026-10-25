//! The world's GPU teardown, counted on the test backend's live meshes and textures.

use blackbox_gfx::{BlendMode, Capabilities, GraphicsApi, MeshDesc, PixelFormat, RenderBackend, TextureDesc};
use blackbox_scenery::LodModel;

use crate::app::test_backend::NullBackend;
use crate::scenes::world::ai::LampMeshes;
use crate::scenes::world::drive::MarkerMeshes;
use crate::scenes::world::resident::SectionResources;

const WHITE: [u8; 4] = [255; 4];

fn tracking_renderer() -> NullBackend {
    NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan)).with_mesh_tracking()
}

#[test]
fn a_section_frees_the_meshes_and_textures_it_uploaded() {
    let mut renderer = tracking_renderer();
    let texture = renderer.create_texture(&TextureDesc {
        label: "section texture",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&WHITE[..]],
    });
    let mesh = renderer.create_mesh(&MeshDesc {
        label: "section",
        vertices: &[],
        indices: &[],
        draws: Vec::new(),
        tangents: None,
        secondary_uvs: None,
    });
    let mut section = SectionResources::default();
    section.materials.insert(1, (texture, BlendMode::Opaque));
    section.meshes.insert(2, (mesh, LodModel { num_polys: 4, density: 1.0 }, Default::default()));
    assert_eq!((renderer.mesh_labels.len(), renderer.texture_labels.len()), (1, 1));
    section.release(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
    assert!(renderer.texture_labels.is_empty());
}

#[test]
fn the_contact_markers_and_the_traffic_lights_free_their_meshes() {
    let mut renderer = tracking_renderer();
    let markers = MarkerMeshes::upload(&mut renderer);
    let lamps = LampMeshes::upload(&mut renderer);
    // Five contact markers and three lamp colours; no synthetic posts or stop bars.
    assert_eq!(renderer.mesh_labels.len(), 8);
    markers.release(&mut renderer);
    assert_eq!(renderer.mesh_labels.len(), 3);
    lamps.release(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
}
