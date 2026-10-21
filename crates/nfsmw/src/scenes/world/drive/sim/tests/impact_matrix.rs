//! Installed geometry/contact coverage, independent of speculative trajectory or stopping-distance tuning.
use super::*;
use crate::scenes::world::object_geometry::{BodyGeometryFrame, ObjectGeometry, object_shapes};

fn shapes(car: &CarSim) -> Vec<ObjectGeometry> {
    object_shapes(
        car.body(),
        car.geometry.as_ref().map(|g| BodyGeometryFrame { geometry: &g.body, previous: car.previous_contact_pose }),
    )
}

fn support(car: &CarSim, direction: Vec3) -> f32 {
    shapes(car)
        .iter()
        .map(|s| {
            let b = &s.bounds;
            (b.centre - car.body().position).dot(direction) + b.axes.transpose().mul_vec3(direction).abs().dot(b.half)
        })
        .fold(f32::NEG_INFINITY, f32::max)
}

#[test]
#[ignore = "requires installed cop physics and detailed collision geometry"]
fn installed_cop_impacts_cover_front_rear_sides_corners_types_rotations_and_pair_orders() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = nfsmw_data::car::physics::PhysicsData::load(&dir).unwrap();
    let types = [
        "copmidsize",
        "copghost",
        "copgto",
        "copgtoghost",
        "copsport",
        "copsportghost",
        "copsporthench",
        "copsuv",
        "copsuvl",
        "copsuvpatrol",
        "copcross",
    ];
    let directions = [
        Vec3::Z,
        Vec3::NEG_Z,
        Vec3::X,
        Vec3::NEG_X,
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(-1.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, -1.0),
        Vec3::new(-1.0, 0.0, -1.0),
    ];
    for name in types {
        let mut callbacks = 0;
        for local in directions.map(Vec3::normalize) {
            for yaw in [0.0, 0.61] {
                for player_first in [false, true] {
                    let mut cop = CarSim::new(data.car(name).unwrap(), [0.3; 4]);
                    let mut player = CarSim::new(data.car("bmwm3gtr").unwrap(), [0.3; 4]);
                    assert!(cop.geometry.is_some() && player.geometry.is_some());
                    cop.configure_ai();
                    let at = Vec3::Y * 10.0;
                    let rotation = Quat::from_rotation_y(yaw);
                    let outward = rotation * local;
                    let forward = -outward;
                    let heading = Quat::from_rotation_y(forward.x.atan2(forward.z));
                    cop.body_mut().place(at, rotation);
                    player.body_mut().place(at, heading);
                    let distance = support(&cop, outward) + support(&player, -outward) + 0.1;
                    let mut found = false;
                    // Locate first geometry overlap, without selecting a successful reaction or damage result.
                    for step in 0..512 {
                        player.body_mut().place(at + outward * (distance - step as f32 * 0.025), heading);
                        cop.previous_contact_pose = (at, rotation);
                        player.previous_contact_pose = (player.body().position + outward / 3.0, heading);
                        if shapes(&player).iter().any(|a| {
                            shapes(&cop).iter().any(|b| a.contact(b).is_some_and(|hit| hit.overlap > 0.000001))
                        }) {
                            found = true;
                            break;
                        }
                    }
                    assert!(found, "{name} {local:?} {yaw}: actual bodywork must meet");
                    player.body_mut().linear_velocity = -outward * 20.0;
                    let original = (cop.body().mass(), cop.body().inertia());
                    let mut hits = Vec::new();
                    let reacted = match player_first {
                        true => player.collide_with_each(&mut cop, |hit, _, _| hits.push(hit.damage[1])),
                        false => cop.collide_with_each(&mut player, |hit, _, _| hits.push(hit.damage[0])),
                    };
                    assert!(
                        reacted && !hits.is_empty(),
                        "{name} {local:?} {yaw} {player_first}: approaching bodywork reacts"
                    );
                    assert!(cop.is_finite() && player.is_finite());
                    assert_eq!((cop.body().mass(), cop.body().inertia()), original);
                    assert_eq!(cop.pending_sound_contacts.len(), hits.len());
                    assert_eq!(player.pending_sound_contacts.len(), hits.len());
                    for hit in &hits {
                        assert!(hit.force.is_finite() && hit.force > 0.0);
                        assert!(hit.arm.dot(local) > 0.0, "{name}: callback remains on the struck half: {hit:?}");
                        assert!(hit.normal.dot(local) < 0.0, "{name}: normal faces into the struck body: {hit:?}");
                        assert!(hit.other.player && hit.other.vehicle);
                    }
                    callbacks += hits.len();
                }
            }
        }
        eprintln!("{name}: 32 detailed-shape impact fixtures, {callbacks} physical callbacks");
    }
}
