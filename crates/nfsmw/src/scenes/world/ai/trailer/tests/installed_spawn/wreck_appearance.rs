//! A destroyed actor retains its installed hood selection while clocks and detail change.
use super::*;
use crate::scenes::world::ai::{
    car::{AiCar, Role, Start},
    traffic::Cruise,
};
use crate::scenes::world::drive::DetailView;
use crate::scenes::world::events::{DamageHit, OtherBody};
use blackbox_roads::RoadNav;
use blackbox_scenery::LodView;
mod hood_dynamics;

fn hood(
    car: &AiCar,
    renderer: &NullBackend,
    detail: Option<&DetailView>,
    names: &[String],
) -> Vec<(String, glam::Mat4)> {
    let mut out = Vec::new();
    car.instances_with_detail(1.0, detail, &mut out);
    out.into_iter()
        .filter_map(|part| {
            let name = renderer.mesh_labels.get(&part.mesh)?;
            names.contains(name).then(|| (name.clone(), part.transform))
        })
        .collect()
}

#[test]
#[ignore = "needs installed police driving LODs, sequences and highway ground"]
fn installed_destroyed_police_hood_persists_across_contact_ticks_lods_and_reshading() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let (mut spawn, _, _) = highway(index.road_network.as_ref().unwrap());
    let collision = collision(&dir, &index, spawn);
    spawn.position.z =
        road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap().height;
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan)).with_mesh_tracking();
    let installed =
        nfsmw_data::car::load_driving(&dir, "COPMIDSIZE", &LoadOptions { lod: 'A', all_parts: false, preset: None })
            .unwrap();
    let hood_slot = nfsmw_data::car::LAYOUT.slots.iter().position(|s| s.name == "DAMAGE_HOOD").unwrap();
    let mut names = Vec::new();
    for (lod, placements, damaged) in std::iter::once((0, installed.placements.as_slice(), false))
        .chain(installed.driving_lods.levels.iter().map(|l| (l.level, l.placements.as_slice(), false)))
        .chain(installed.driving_lods.damage.levels.iter().map(|l| (l.level, l.placements.as_slice(), true)))
    {
        for placement in placements.iter().filter(|p| p.slot == hood_slot) {
            let solid = &installed.solids[&placement.solid];
            eprintln!(
                "hood lod={lod} damaged={damaged} name={} bounds={:?}..{:?}",
                solid.name, solid.bounds_min, solid.bounds_max
            );
            names.push(solid.name.clone());
        }
    }
    names.sort();
    names.dedup();
    let mut template = model(&dir, &data, "copmidsize", &mut renderer);
    template.rig = Rc::new(CarRig::upload(&mut renderer, 9876, installed, CarShading::default(), sun::to_sun()));
    let mut car = AiCar::place(
        &template,
        Role::Traffic,
        RoadNav::traffic(),
        spawn,
        Start { speed: 0.0, stagger: 0, cruise: Cruise::default(), patrol: true },
        &ground,
    )
    .unwrap();
    let pristine = hood(&car, &renderer, None, &names);
    assert!(!pristine.is_empty());
    assert!(car.destroy_by_breaker());
    let initial = hood(&car, &renderer, None, &names);
    assert!(!initial.is_empty());
    assert_ne!(initial, pristine, "the installed damaged hood must actually replace the pristine geometry");
    let far = DetailView {
        view: LodView {
            position: (spawn.position - Vec3::Y * 1_000_000.0).to_array(),
            forward: Vec3::Y.to_array(),
            pixel_scale: 1.0,
        },
        settings: crate::settings::DetailSettings { car: crate::settings::CarGeometry::High, ..Default::default() },
    };
    let mut props = PropWorld::default();
    for tick in 0..1800 {
        if tick % 60 == 0 {
            car.sims_mut().0.events.as_mut().unwrap().hit(DamageHit {
                arm: Vec3::Z * 3.0,
                normal: Vec3::NEG_Z,
                half: Vec3::ONE,
                closing_speed: 10.0,
                force: 20_000.0,
                shock_force: 0.0,
                before_velocity: Vec3::ZERO,
                other: OtherBody::default(),
            });
        }
        car.update_events(&mut props, &data, spawn.position);
        props.step(&ground, STEP);
        assert_eq!(car.sims_mut().0.events.as_ref().unwrap().source_levels(), &[6; 10]);
        if tick % 60 != 0 {
            continue;
        }
        let _ = hood(&car, &renderer, Some(&far), &names);
        assert_eq!(
            hood(&car, &renderer, None, &names),
            initial,
            "hood reverted after event tick {tick} and LOD round trip"
        );
    }
    assert!(!template.rig.reshade(&mut renderer, CarShading::Simple, sun::to_sun()).is_empty());
    assert_eq!(hood(&car, &renderer, None, &names), initial);
}

#[test]
#[ignore = "needs installed CopCar hood action, driving geometry, fragment physics and highway ground"]
fn installed_twenty_second_hood_detachment_keeps_the_damaged_mesh_and_parent_hidden_after_cleanup() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let (mut spawn, _, _) = highway(index.road_network.as_ref().unwrap());
    let collision = collision(&dir, &index, spawn);
    spawn.position.z =
        road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap().height;
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan)).with_mesh_tracking();
    let installed =
        nfsmw_data::car::load_driving(&dir, "COPMIDSIZE", &LoadOptions { lod: 'A', all_parts: false, preset: None })
            .unwrap();
    let slot = nfsmw_data::car::LAYOUT.slots.iter().position(|s| s.name == "DAMAGE_HOOD").unwrap();
    let pristine = installed.placements.iter().find(|p| p.slot == slot).unwrap();
    let successor = installed
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
    let pristine_name = installed.solids[&pristine.solid].name.clone();
    let damaged_name = installed.solids[&successor.solid].name.clone();
    assert_ne!(pristine_name, damaged_name);
    let mut template = model(&dir, &data, "copmidsize", &mut renderer);
    template.rig = Rc::new(CarRig::upload(&mut renderer, 9877, installed, CarShading::default(), sun::to_sun()));
    let spec = template.physics.events.as_ref().unwrap();
    assert!(spec.strength > 0.0 && spec.visual_scale[0] > 0.0);
    let mut car = AiCar::place(
        &template,
        Role::Traffic,
        RoadNav::traffic(),
        spawn,
        Start { speed: 0.0, stagger: 0, cruise: Cruise::default(), patrol: true },
        &ground,
    )
    .unwrap();
    let id = car.key();
    let mut props = PropWorld::default();
    car.sims_mut().0.events.as_mut().unwrap().hit(DamageHit {
        arm: Vec3::Z * 3.0,
        normal: Vec3::NEG_Z,
        half: Vec3::new(1.0, 1.0, 4.0),
        closing_speed: 10.0,
        force: 1.1 * spec.strength / spec.visual_scale[0],
        shock_force: 0.0,
        before_velocity: Vec3::ZERO,
        other: OtherBody::default(),
    });
    // This real zone hit starts the installed 20-second action before destruction.
    car.update_events(&mut props, &data, spawn.position);
    assert_eq!(car.sims_mut().0.events.as_ref().unwrap().source_levels()[0], 1);
    let names = [damaged_name.clone(), pristine_name.clone()];
    let before = hood(&car, &renderer, None, &names);
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].0, damaged_name);
    assert!(car.destroy_by_breaker());
    let mut release = None;
    for tick in 1..=1220 {
        props.step(&ground, STEP);
        car.update_events(&mut props, &data, spawn.position);
        assert_eq!(car.key(), id, "this must remain the same actor, not a recycled replacement");
        let hidden = car.sims_mut().0.events.as_ref().unwrap().hidden_slots.contains(&slot);
        if !hidden {
            continue;
        }
        let mut fragments = Vec::new();
        props.fragment_instances(&mut fragments);
        let detached: Vec<_> =
            fragments.iter().filter(|part| renderer.mesh_labels.get(&part.mesh) == Some(&damaged_name)).collect();
        assert_eq!(
            detached.len(),
            1,
            "delayed release must retain the damaged hood; a pristine A fragment reproduces the snap"
        );
        assert!(fragments.iter().all(|part| renderer.mesh_labels.get(&part.mesh) != Some(&pristine_name)));
        assert!(detached[0].transform.abs_diff_eq(before[0].1, 0.001), "handover begins at the selected parent's pose");
        assert!(hood(&car, &renderer, None, &names).is_empty());
        release = Some(tick);
        break;
    }
    assert!(
        release.is_some_and(|tick| (1198..=1204).contains(&tick)),
        "release tick {release:?} must follow the installed 20-second action"
    );
    assert!(props.fragment_count() > 0);
    // Exercise actual fragment lifetime removal with no supporting world, while retaining the actor.
    let no_world = blackbox_vehicle::FlatGround::new(-20_000.0);
    for _ in 0..70 {
        props.update_lifetimes(&no_world, glam::Mat4::IDENTITY, STEP);
    }
    assert_eq!(props.fragment_count(), 0);
    assert!(car.sims_mut().0.events.as_ref().unwrap().hidden_slots.contains(&slot));
    assert!(hood(&car, &renderer, None, &names).is_empty(), "cleanup must not reveal a repaired-looking parent hood");
}
