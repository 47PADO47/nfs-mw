//! Exercise real zone transitions and retirement without a GPU or installed assets.

use super::*;
use crate::app::test_backend::NullBackend;
use blackbox_gfx::{MeshHandle, RenderExperiments, ShadowProjection};
use blackbox_scene::Aabb;
use blackbox_streaming::visible::{Boundary, DrivableSection};
use glam::{Mat4, Vec3};

#[test]
fn custom_casters_survive_a_real_zone_seam_without_expanding_colour_draws() {
    let (mut residency, mut renderer) = fixture();
    renderer.set_render_experiments(custom());
    cross_seam(&mut residency, &mut renderer);
    assert!(!residency.drawn.contains(&0));
    assert!(residency.shadow_placed().any(|p| p.key == keys::tile(0, 0)));
    assert!(!residency.placed().any(|p| p.key == keys::tile(0, 0)), "retention must not expand colour draws");
    assert!(!residency.tiles.contains_key(&2), "distant previous-zone tiles must retire");
    assert!(residency.tiles.contains_key(&3), "new off-PVS shadow support must be requested");
    residency.update(&mut renderer, 2.0, 0.0);
    assert!(residency.tiles.contains_key(&0), "small movement cannot immediately remove the retained caster");
}

#[test]
fn retained_tiles_expire_on_distance_without_another_zone_change() {
    let (mut residency, mut renderer) = fixture();
    renderer.set_render_experiments(custom());
    cross_seam(&mut residency, &mut renderer);
    let zone = residency.zone();
    residency.update(&mut renderer, 600.0, 0.0);
    assert_eq!(residency.zone(), zone);
    assert!(!residency.tiles.contains_key(&0), "the retention region must remain bounded");
}

#[test]
fn disabling_shadows_releases_retained_tiles_in_both_map_modes() {
    for projection in [None, Some(projected())] {
        let (mut residency, mut renderer) = fixture();
        renderer.set_render_experiments(RenderExperiments { shadow_projection: projection, ..custom() });
        cross_seam(&mut residency, &mut renderer);
        assert!(residency.tiles.contains_key(&0));
        renderer.set_render_experiments(RenderExperiments {
            sun_shadows: false,
            shadow_projection: projection,
            ..custom()
        });
        residency.update(&mut renderer, 1.0, 0.0);
        assert!(!residency.tiles.contains_key(&0));
    }
}

#[test]
fn far_coverage_changes_the_bounded_retention_region() {
    for far_shadows in [false, true] {
        let (mut residency, mut renderer) = fixture();
        residency.sections[0].center = [-500.0, 0.0];
        renderer.set_render_experiments(RenderExperiments { far_shadows, ..custom() });
        cross_seam(&mut residency, &mut renderer);
        assert_eq!(residency.tiles.contains_key(&0), far_shadows);
    }
}

#[test]
fn nonfinite_shadow_distance_cannot_retain_unbounded_scenery() {
    let (mut residency, mut renderer) = fixture();
    renderer.set_render_experiments(RenderExperiments { shadow_distance: f32::INFINITY, ..custom() });
    cross_seam(&mut residency, &mut renderer);
    assert!(residency.tiles.contains_key(&0));
    assert!(!residency.tiles.contains_key(&2));
}

#[test]
fn shadow_support_loads_new_tiles_without_blocking_colour_and_limits_in_flight() {
    for projection in [None, Some(projected())] {
        let (mut residency, mut renderer) = fixture();
        for x in [40.0, 50.0, 60.0] {
            let mut section = residency.sections[3].clone();
            section.center = [x, 0.0];
            residency.sections.push(section);
        }
        renderer.set_render_experiments(RenderExperiments { shadow_projection: projection, ..custom() });
        residency.update(&mut renderer, -1.0, 0.0);
        assert_eq!(residency.zone().as_deref(), Some("A1"));
        assert!(residency.complete(), "shadow requests must not gate ready colour geometry");
        let requested: Vec<_> =
            residency.tiles.iter().filter(|(_, tile)| matches!(tile, TileState::Requested)).map(|(&i, _)| i).collect();
        assert_eq!(requested.len(), 2, "shadow-only loading reserves the other six request slots");
        assert!(requested.iter().all(|i| !residency.wanted.contains(i)));
        assert!(residency.shadow_wanted.contains(&3), "off-PVS scenery must no longer depend on visit history");
        assert!(!residency.drawn.contains(&3));
    }
}

#[test]
fn extra_resident_budget_never_evicts_core_and_holds_through_small_camera_moves() {
    let (mut residency, mut renderer) = fixture();
    for i in 0..80 {
        let mut section = residency.sections[3].clone();
        section.number = 300 + i;
        section.center = [80.0 + i as f32, 0.0];
        section.radius = 2.0;
        let index = residency.sections.len();
        residency.sections.push(section);
        residency
            .tiles
            .insert(index, TileState::Resident { resources: Default::default(), placed: vec![], packs: vec![] });
    }
    renderer.set_render_experiments(RenderExperiments { far_shadows: true, shadow_distance: 320.0, ..custom() });
    residency.set_shadow_sun(glam::Vec3::new(1.0, 0.0, 0.2));
    residency.update(&mut renderer, -1.0, 0.0);
    let (eligible, selected) = residency.shadow_support_counts();
    assert!(eligible > selected && selected == shadow_support::MAX_EXTRA_TILES);
    assert!(
        residency.wanted.iter().all(|i| residency.tiles.contains_key(i)),
        "core scenery is outside the extra budget"
    );
    let extras = residency.tiles.keys().filter(|i| !residency.wanted.contains(i)).count();
    assert!(extras <= shadow_support::MAX_EXTRA_TILES, "past visits cannot accumulate all eligible tiles");
    let before = residency.shadow_wanted.clone();
    for x in [-1.01, -0.99] {
        residency.update(&mut renderer, x, 0.0);
        assert_eq!(residency.shadow_wanted, before, "submetre motion cannot replace budget-boundary scenery");
    }
}

#[test]
fn upstream_support_preserves_the_entire_receiver_disk_and_rejects_downstream_only_tiles() {
    let (mut residency, mut renderer) = fixture();
    let mut indices = Vec::new();
    for center in [[200.0, 0.0], [-100.0, 0.0], [0.0, 100.0], [0.0, -100.0], [-200.0, 0.0]] {
        let mut section = residency.sections[3].clone();
        section.center = center;
        section.radius = 1.0;
        indices.push(residency.sections.len());
        residency.sections.push(section);
    }
    renderer.set_render_experiments(RenderExperiments { shadow_distance: 120.0, ..custom() });
    residency.set_shadow_sun(glam::Vec3::new(1.0, 0.0, 0.2));
    residency.update(&mut renderer, -1.0, 0.0);
    for i in &indices[..4] {
        assert!(residency.shadow_wanted.contains(i), "receiver disk and upstream sweep must both be covered");
    }
    assert!(
        !residency.shadow_wanted.contains(&indices[4]),
        "the upstream margin must not expand behind the receiver domain"
    );
}

#[test]
fn receiver_extent_admits_far_corners_and_rejects_nonfinite_metadata() {
    for (scale, offset, expected) in [(1.0, 0.0, false), (1.8, 5.0, true), (f32::NAN, f32::INFINITY, false)] {
        let (mut residency, mut renderer) = fixture();
        let mut section = residency.sections[3].clone();
        section.center = [-200.0, 0.0];
        section.radius = 1.0;
        let index = residency.sections.len();
        residency.sections.push(section);
        residency.set_shadow_sun(glam::Vec3::new(1.0, 0.0, 0.2));
        residency.set_shadow_receiver_extent(scale, offset);
        renderer.set_render_experiments(RenderExperiments { shadow_distance: 120.0, ..custom() });
        residency.update(&mut renderer, -1.0, 0.0);
        assert_eq!(residency.shadow_wanted.contains(&index), expected);
    }
}

fn custom() -> RenderExperiments {
    RenderExperiments { sun_shadows: true, shadow_distance: 160.0, ..Default::default() }
}

fn cross_seam(residency: &mut Residency, renderer: &mut NullBackend) {
    residency.update(renderer, -1.0, 0.0);
    assert_eq!(residency.zone().as_deref(), Some("A1"), "fixture must enter a valid drivable zone");
    assert!(residency.drawn.contains(&0));
    residency.update(renderer, 1.0, 0.0);
    assert_eq!(residency.zone().as_deref(), Some("A2"), "fixture must cross a real zone boundary");
    assert!(residency.complete(), "new colour tiles are already resident");
    assert!(residency.previous.is_empty(), "this exercises retirement after the loading handoff");
}

fn projected() -> ShadowProjection {
    ShadowProjection {
        frustum: [0.1, 50.0],
        depth_bias: [1.0, 1.0],
        near: 0.1,
        width_scale: 1.0,
        height_scale: 1.0,
        far_scale: 1.0,
        offset: [0.0; 2],
        post_transform: Mat4::IDENTITY.to_cols_array_2d(),
    }
}

fn fixture() -> (Residency, NullBackend) {
    let sections: Vec<_> = [-20.0, 20.0, 500.0, 30.0]
        .into_iter()
        .enumerate()
        .map(|(i, x)| StreamingSection {
            name: format!("tile{i}"),
            number: 101 + i as i16,
            file_type: 0,
            file_offset: 0,
            size: 0,
            compressed_size: 0,
            perm_size: 0,
            priority: 0,
            center: [x, 0.0],
            radius: 10.0,
            checksum: 0,
        })
        .collect();
    let visible = VisibleSections {
        lod_offset: 40,
        region: vec![101, 102],
        boundaries: vec![boundary(101, -1000.0, 0.0), boundary(102, 0.0, 1000.0)],
        drivable: vec![
            DrivableSection { section: 101, visible: vec![101, 102, 103] },
            DrivableSection { section: 102, visible: vec![102] },
        ],
        loading: vec![],
    };
    // All requested tiles are preseeded below. The worker opens this existing file read-only,
    // then waits idle; no installed assets, synthetic disk files or asynchronous loads are needed.
    let streamer = Streamer::spawn(std::env::current_exe().unwrap(), sections.clone(), 1);
    let mut residency = Residency::new(
        sections,
        visible,
        streamer,
        GlobalTextures::default(),
        None,
        PropCatalog::new(vec![], &blackbox_attrib::Database::new()),
        Vec::new(),
    );
    residency.shared_ready = true;
    for i in 0..3 {
        let at = Vec3::from_array([residency.sections[i].center[0], 0.0, 0.0]);
        let placed = Placed {
            signal: None,
            lods: [Some(MeshHandle::from_raw(i)); 4],
            detailed: None,
            transform: Mat4::from_translation(at),
            bounds: Aabb::new((at - 1.0).to_array(), (at + 1.0).to_array()),
            radius: 1.0,
            flags: 0,
            prop: None,
            prop_id: 0,
            key: keys::tile(i, 0),
            original_shadow: false,
            casts_sun_shadow: true,
            spawner: None,
            parts: None,
            curve_lamp: None,
        };
        residency
            .tiles
            .insert(i, TileState::Resident { resources: Default::default(), placed: vec![placed], packs: vec![] });
    }
    (residency, NullBackend::new(crate::settings::test_caps::native()))
}

fn boundary(section: i16, min_x: f32, max_x: f32) -> Boundary {
    Boundary {
        section,
        panorama: false,
        bbox_min: [min_x, -1000.0],
        bbox_max: [max_x, 1000.0],
        centre: [(min_x + max_x) * 0.5, 0.0],
        points: vec![[min_x, -1000.0], [max_x, -1000.0], [max_x, 1000.0], [min_x, 1000.0]],
    }
}
