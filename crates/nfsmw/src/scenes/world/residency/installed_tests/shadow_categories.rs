//! Installed backdrop classification must preserve its independently visible colour geometry.
use crate::app::test_backend::NullBackend;
use crate::scenes::world::resident::{SectionResources, place};

#[test]
#[ignore = "needs installed city scenery selected by NFSMW_GAME_DIR"]
fn installed_panorama_retains_colour_geometry_without_becoming_a_sun_caster() {
    let (dir, scene) = super::open();
    let index = nfsmw_data::world::WorldIndex::open(&dir, &scene.track).unwrap();
    let section = index.by_name("O93").expect("the measured panorama section must exist");
    let data = nfsmw_data::world::load_section(
        &mut dir.open_file(&index.stream_file).unwrap(),
        section,
        &index.sections[section],
    )
    .unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
    let mut own = SectionResources::default();
    let shared = SectionResources::default();
    own.upload_meshes(&mut renderer, &data, &shared);
    let (placed, _) = place(&data, &own, &shared, &scene.residency.catalog);
    let panorama = data
        .scenery
        .iter()
        .flat_map(|s| s.instances.iter().filter_map(|i| Some((i, s.info_of(i)?))))
        .find_map(|(i, info)| info.name.eq_ignore_ascii_case("PAN_ISLAND_1A_08").then_some(i.position))
        .expect("the independently measured backdrop must resolve");
    let p = placed
        .iter()
        .find(|p| p.transform.w_axis.truncate().to_array() == panorama)
        .expect("colour placement must survive classification");
    assert!(p.lods.iter().any(Option::is_some));
    assert!(!p.original_shadow, "a panorama is not an authored shadow overlay");
    assert!(!p.casts_sun_shadow);
    own.release(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
}
