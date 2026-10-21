use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::events::{DamageHit, OtherBody, VehicleEvents};
use nfsmw_data::car::physics::CarEventSpec;
use nfsmw_data::world::sequences::SequenceCatalog;
use std::{collections::BTreeSet, sync::Arc};

#[test]
#[ignore = "requires an installed game"]
fn installed_heat_one_collision_selects_real_damaged_meshes_per_actor_and_lod() {
    let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
    let model = nfsmw_data::car::load_driving(
        &dir,
        "COPMIDSIZE",
        &nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None },
    )
    .unwrap();
    assert!(model.driving_lods.damage.enabled);
    let cached: Vec<_> = std::iter::once(0).chain(model.driving_lods.levels.iter().map(|l| l.level)).collect();
    assert_eq!(cached, [0, 1, 2, 3, 4], "the pristine heat-one body has authored A-E levels");
    for assembly in &model.driving_lods.levels {
        assert!(
            assembly.placements.iter().any(|p| p.slot == nfsmw_data::car::slot::DAMAGE_BODY),
            "police body must survive the production lower-LOD body guard"
        );
    }
    assert!(
        model.driving_lods.damage.levels.iter().find(|l| l.level == 4).unwrap().placements.is_empty(),
        "the installed successor parts likewise have no E geometry"
    );
    let db = nfsmw_data::shared::attributes(&dir).unwrap();
    let spec =
        CarEventSpec::read(db.collection("pvehicle", "copmidsize").unwrap(), &SequenceCatalog::default()).unwrap();
    let mut events = VehicleEvents::new(Arc::new(spec)).unwrap();
    events.hit(DamageHit {
        arm: Vec3::Z * 3.0,
        normal: Vec3::NEG_Z,
        half: Vec3::ONE,
        closing_speed: 20.0,
        force: 160_000.0,
        shock_force: 80_000.0,
        before_velocity: Vec3::Z * 20.0,
        other: OtherBody { vehicle: true, player: true, ..Default::default() },
    });
    events.tick(0.0);
    assert_eq!(events.source_levels()[0], 2);
    let mask = nfsmw_data::car::damage_parts::damaged_slots(events.source_levels());
    for slot in [2, 10, 12] {
        let original = model.placements.iter().find(|p| p.slot == slot).unwrap();
        let successor = model
            .driving_lods
            .damage
            .levels
            .iter()
            .find(|l| l.level == 0)
            .unwrap()
            .placements
            .iter()
            .find(|p| p.slot == slot)
            .unwrap();
        assert_ne!(original.solid, successor.solid, "slot {slot} needs its authored successor");
        assert!(model.solids.contains_key(&successor.solid));
    }
    let mut backend =
        NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking().with_texture_tracking();
    let rig = CarRig::upload(&mut backend, keys::next_car(), model, CarShading::Original, Vec3::Z);
    let glass = nfsmw_data::car::damage_parts::glass_mask(events.source_levels());
    rig.prepare_glass(&mut backend, &BTreeSet::from([0, glass]), Vec3::Z);
    let pose = CarPose { position: Vec3::ZERO, rotation: Quat::IDENTITY, wheels: [Default::default(); 4] };
    let draw = |car, damage, glass| {
        let mut out = Vec::new();
        rig.instances_detail(
            &pose,
            None,
            &BTreeSet::new(),
            super::super::DrawOptions { car, damage, glass, ..Default::default() },
            &mut out,
        );
        out
    };
    let pristine = draw(7, None, 0);
    let damaged = draw(7, Some(events.source_levels()), glass);
    assert_ne!(pristine.iter().map(|i| i.mesh).collect::<Vec<_>>(), damaged.iter().map(|i| i.mesh).collect::<Vec<_>>());
    for part in &rig.appearance.borrow().parts {
        if part.lod != 0 || part.placement.slot > 22 || mask & (1 << part.placement.slot) == 0 {
            continue;
        }
        let key = keys::car_part(7, part.key);
        if !part.damaged {
            assert!(
                !damaged.iter().any(|i| i.key == key && i.mesh == part.mesh),
                "pristine part must leave the draw list"
            );
        }
    }
    assert_eq!(
        pristine.iter().map(|i| i.mesh).collect::<Vec<_>>(),
        draw(8, None, 0).iter().map(|i| i.mesh).collect::<Vec<_>>(),
        "a second actor sharing the rig stays pristine"
    );
    for lod in 0..=3 {
        let uploaded = rig.appearance.borrow();
        assert!(uploaded.parts.iter().any(|p| p.lod == lod && p.damaged), "damaged LOD {lod}");
    }
    let far = super::super::DetailView {
        view: blackbox_scenery::LodView { position: [0.0, -10_000.0, 0.0], forward: [0.0, 1.0, 0.0], pixel_scale: 1.0 },
        settings: crate::settings::DetailSettings { car: crate::settings::CarGeometry::High, ..Default::default() },
    };
    let at_d = super::super::DetailView {
        view: blackbox_scenery::LodView {
            pixel_scale: 14.0 * (10_000.0 - rig.detail_radius) / rig.detail_radius,
            ..far.view
        },
        ..far
    };
    let draw_detail = |view, damage| {
        let mut out = Vec::new();
        rig.instances_detail(
            &pose,
            None,
            &BTreeSet::new(),
            super::super::DrawOptions { car: 7, detail: Some(view), damage, ..Default::default() },
            &mut out,
        );
        out
    };
    for (view, lod) in [(&at_d, 3), (&far, 4)] {
        assert_eq!(super::super::car_detail::choose(&rig.model, Vec3::ZERO, rig.detail_radius, view, false), lod);
        let before = draw_detail(view, None);
        let after = draw_detail(view, Some(events.source_levels()));
        let appearance = rig.appearance.borrow();
        let body = appearance
            .parts
            .iter()
            .find(|part| part.lod == lod && !part.damaged && part.placement.slot == nfsmw_data::car::slot::DAMAGE_BODY)
            .unwrap();
        let body_key = keys::car_part(7, body.key);
        assert!(before.iter().any(|part| part.key == body_key && part.mesh == body.mesh));
        if lod == 3 {
            let damaged = appearance
                .parts
                .iter()
                .find(|part| {
                    part.lod == lod && part.damaged && part.placement.slot == nfsmw_data::car::slot::DAMAGE_BODY
                })
                .unwrap();
            assert!(
                after.iter().any(|part| part.key == body_key && part.mesh == damaged.mesh),
                "D must draw the actual damaged body"
            );
            continue;
        }
        assert!(!after.iter().any(|part| part.key == body_key), "missing damaged E body is skipped, as in the source");
        for survivor in &after {
            assert!(
                before.iter().any(|part| part.key == survivor.key && part.mesh == survivor.mesh),
                "E can retain unaffected source parts without inventing replacement geometry"
            );
        }
    }
    rig.release(&mut backend);
    let resources = backend.resources.lock().unwrap();
    assert_eq!((resources.meshes, resources.textures), (0, 0));
}
