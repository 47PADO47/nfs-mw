//! Installed marker/profile integration; original bytes remain in the user's install.

use super::*;
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

#[test]
#[ignore = "requires an installed game"]
fn installed_environment_lights_use_authored_markers_profiles_and_global_textures() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let db = nfsmw_data::shared::attributes(&dir).unwrap();
    let profiles = nfsmw_data::world::lights::profiles(&db);
    assert_eq!(profiles.len(), 6);
    let globals = nfsmw_data::world::load_global_textures(&dir).unwrap();
    for (_, p) in &profiles {
        assert!(
            globals.textures.iter().any(|t| t.name_hash == p.texture),
            "authored flare texture must be globally available"
        );
    }
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let i = index.by_name("X0").unwrap();
    let section = &index.sections[i];
    let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
    let data = nfsmw_data::world::parse_section(i, &bytes).unwrap();
    for (name, kind, count) in [
        ("XS_CHEVRCNRA_1_00", Kind::AmberBlink, 1),
        ("XS_MEDIANPOLER_1B_00", Kind::AmberBlink, 1),
        ("XS_MEDIANPOLEGREEN_1B_CK_00", Kind::GreenBlink, 1),
        ("XO_TRAFFICLIGHTC_1B_00", Kind::RedBlink, 1),
        ("XO_CATSEYEORANGE_1A_00", Kind::Orange, 1),
    ] {
        let solid = data.solids.iter().find(|s| s.name == name).unwrap();
        let markers = nfsmw_data::world::lights::markers(solid);
        if name == "XS_MEDIANPOLER_1B_00" {
            assert_eq!(solid.markers.len(), 2, "the stock solid contains the stray marker");
            assert!(markers.iter().all(|marker| marker.position.length() < 10.0));
        }
        assert_eq!(markers.len(), count, "{name}");
        assert!(markers.iter().all(|m| m.kind == kind));
        let mut renderer =
            crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
        let mut resources = super::super::resident::SectionResources::default();
        resources.upload_solid_list(&mut renderer, std::slice::from_ref(solid), &Default::default(), false);
        let mesh = resources.meshes[&solid.name_hash].0;
        assert_eq!(&*resources.flare_markers[&mesh], markers);
        // Verify the installed local point gets exactly one pivot subtraction before placement.
        let pivot = Vec3::from_slice(&solid.transform[12..15]);
        let expected = Vec3::from_array(solid.markers[0].translation()) - pivot;
        assert_eq!(markers[0].position, expected);
        let world = Mat4::from_translation(Vec3::new(20.0, 30.0, 5.0)) * Mat4::from_rotation_z(0.7);
        let at = world.transform_point3(expected);
        let forward = (at - world.w_axis.truncate()).with_z(0.0).normalize_or(Vec3::X);
        let eye = at + forward * 10.0;
        let sample = draw::sample(markers[0], world, eye, -forward);
        assert!(sample.is_some(), "{name} produces a finite sample");
        resources.release(&mut renderer);
    }
    assert!(profiles.iter().all(|(_, p)| p.texture == blackbox_hash::bstring_hash("HEADLIGHTGLOW")));
}
