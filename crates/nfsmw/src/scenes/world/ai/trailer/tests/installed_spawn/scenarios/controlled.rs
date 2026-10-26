//! Actual tractor/trailer runtime with installed models and controlled braking, grade, turns and cone contacts.

use super::*;
use crate::scenes::world::drive::FixedClock;
use blackbox_collision::{Article, CollisionPack, Instance, PackedVert, STRIP_UP_FACING, Strip};
use std::fmt::Write as _;

fn floor(grade: f32) -> CollisionWorld {
    let edge = 25_600;
    let corners = [(-edge, -edge), (-edge, edge), (edge, -edge), (edge, edge)];
    let strip = Strip {
        center: [0.0; 3],
        radius: 300.0,
        flags: STRIP_UP_FACING,
        verts: corners
            .into_iter()
            .map(|(x, z)| PackedVert { x, y: (-f32::from(z) * grade) as i16, z, surface: 0, flags: 0 })
            .collect(),
    };
    let mut pack = CollisionPack {
        section: 1,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 200.0,
            half_length: 200.0,
            half_height: 20.0,
            radius: 300.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![strip],
            barriers: Vec::new(),
            surfaces: vec![0],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: Vec::new(),
    };
    pack.apply_group_exclusion();
    let mut collision = CollisionWorld::new(None);
    collision.insert(pack);
    collision
}

fn cone(props: &mut PropWorld, data: &PhysicsData, at: Vec3, owner: &Rc<()>) {
    let bounds = Bounds {
        orientation: Quat::IDENTITY.to_array(),
        position: [0.0; 3],
        flags: flags::BOX | flags::PRIM_VS_OBJECTS | flags::PRIM_VS_GROUND,
        half_dimensions: [0.5; 3],
        child_count: 0,
        point_cloud: None,
        pivot: [0.0; 3],
        first_child: 0,
        radius: 1.0,
        surface: 0,
        name_hash: 0,
    };
    let spec = data.fragment_dynamics(vlt_hash("cone")).expect("installed cone physics");
    props
        .spawn_placed(crate::scenes::world::props::PlacedSpawn {
            bounds: Arc::new(BoundsSet { name_hash: 1, nodes: vec![bounds], point_clouds: vec![] }),
            spec,
            model: glam::Mat4::from_translation(space::to_render(at.to_array())),
            instances: Vec::new(),
            owner: Rc::downgrade(owner),
            parts: None,
        })
        .expect("controlled cone placement");
}

fn trace(
    model: &TrafficModel,
    trailer_model: &TrafficModel,
    data: &PhysicsData,
    scenario: &str,
    fps: usize,
) -> Vec<Vec<f32>> {
    let grade = match scenario {
        "downhill" => 0.08,
        _ => 0.0,
    };
    let collision = floor(grade);
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let spawn = Spawn { position: Vec3::ZERO, heading: 0.0 };
    let mut tractor = CarSim::for_rig(model.physics.clone(), &model.rig);
    tractor.configure_traffic();
    assert!(tractor.place_traffic_moving(&ground, spawn, 18.0));
    let mut trailer = TrailerCar::place(trailer_model, &tractor, spawn, 18.0, &ground).unwrap();
    let mut props = PropWorld::default();
    let owner = Rc::new(());
    let mut clock = FixedClock::default();
    let mut samples = Vec::new();
    let mut contacts = 0;
    let mut max_gap = 0.0_f32;
    let mut max_lateral = 0.0_f32;
    let mut max_angle = 0.0_f32;
    let mut brake_start = 0.0;
    for _ in 0..fps * 8 {
        for _ in 0..clock.advance(1.0 / fps as f32) {
            let tick = samples.len();
            if tick == 120 {
                brake_start = tractor.telemetry().speed_mps.abs();
            }
            if scenario == "cone" && tick == 60 {
                let nose =
                    tractor.body().position + tractor.body().rotation().z_axis * (tractor.half_dimensions().z + 0.7);
                cone(&mut props, data, Vec3::new(nose.x, 0.5, nose.z), &owner);
            }
            let braking = tick >= 120;
            let input = DriveInput {
                throttle: match braking {
                    true => 0.0,
                    false => 0.1,
                },
                brake: match braking {
                    true => 0.65,
                    false => 0.0,
                },
                steer: match scenario == "corner" && tick < 240 {
                    true => 0.015,
                    false => 0.0,
                },
                ..DriveInput::default()
            };
            props.step(&ground, STEP);
            let impact = tractor.step(&input, &ground, Some((&collision, &mut props)));
            contacts += impact
                .contacts
                .iter()
                .filter(|contact| {
                    matches!(
                        contact.kind,
                        crate::scenes::world::drive::walls::ContactKind::PropRigid
                            | crate::scenes::world::drive::walls::ContactKind::PropMovable
                    )
                })
                .count();
            trailer.step(
                &mut tractor,
                Brakes { brake: input.brake, handbrake: false },
                &collision,
                &mut props,
                &data.surfaces,
            );
            trailer.sync_contacts();
            assert!(tractor.is_finite() && trailer.is_finite(), "{} {scenario} tick {tick}", model.name);
            assert!(
                trailer.is_hitched(),
                "{} {scenario} tick {tick}: controlled mild maneuver released hitch",
                model.name
            );
            let gap = trailer.coupling.gap(&tractor, &trailer.sim);
            max_gap = max_gap.max(gap);
            assert!(gap < 0.1, "{} {scenario} tick {tick}: constraint gap {gap}", model.name);
            let a = tractor.body();
            let b = trailer.sim.body();
            let local = a.rotation().transpose() * a.linear_velocity;
            let trailer_local = b.rotation().transpose() * b.linear_velocity;
            let angle = a.rotation().z_axis.angle_between(b.rotation().z_axis);
            max_lateral = max_lateral.max(local.x.abs()).max(trailer_local.x.abs());
            max_angle = max_angle.max(angle);
            let mut sample = vec![input.throttle, input.brake, input.steer, 0.0, local.x, trailer_local.x, angle, gap];
            for sim in [&tractor, &trailer.sim] {
                let body = sim.body();
                sample.extend(body.position.to_array());
                sample.extend(body.orientation.to_array());
                sample.extend(body.linear_velocity.to_array());
                sample.extend(body.angular_velocity.to_array());
                sample.extend(body.cog().to_array());
                for wheel in sim.physics_wheel_states() {
                    sample.extend([
                        wheel.simulated_angular_velocity,
                        f32::from(u8::from(wheel.locked)),
                        f32::from(u8::from(wheel.on_ground)),
                        wheel.load,
                    ]);
                    if braking && wheel.on_ground {
                        assert!(!wheel.locked, "regular braking must not use the racer lock policy");
                    }
                }
            }
            samples.push(sample);
        }
    }
    assert_eq!(samples.len(), 480);
    assert!(
        tractor.telemetry().speed_mps.abs() < brake_start,
        "{} {scenario}: sustained braking did not reduce speed",
        model.name
    );
    if scenario == "cone" {
        assert!(contacts > 0, "{}: cone scenario must actually touch the prop", model.name);
    }
    eprintln!(
        "truck scenario {} {scenario} {fps}fps: max hitch gap {max_gap:.6}, angle {max_angle:.4}, lateral {max_lateral:.3}, initial brake speed {brake_start:.3}, final {:.3}, contacts {contacts}",
        model.name,
        tractor.telemetry().speed_mps
    );
    if let Some(path) = std::env::var_os("NFSMW_SCENARIO_REPORT_DIR") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        let mut columns: Vec<String> = [
            "throttle",
            "brake",
            "steer",
            "handbrake",
            "tractor_lateral",
            "trailer_lateral",
            "hitch_angle",
            "hitch_gap",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        for body in ["tractor", "trailer"] {
            for field in
                ["px", "py", "pz", "qx", "qy", "qz", "qw", "vx", "vy", "vz", "wx", "wy", "wz", "cgx", "cgy", "cgz"]
            {
                columns.push(format!("{body}_{field}"));
            }
            for wheel in 0..4 {
                for field in ["av", "locked", "supported", "load"] {
                    columns.push(format!("{body}_wheel{wheel}_{field}"));
                }
            }
        }
        let mut report = columns.join(",");
        report.push('\n');
        for sample in &samples {
            for (index, value) in sample.iter().enumerate() {
                if index > 0 {
                    report.push(',');
                }
                write!(&mut report, "{value}").unwrap();
            }
            report.push('\n');
        }
        std::fs::write(path.join(format!("{}-{scenario}-{fps}.csv", model.name)), report).unwrap();
    }
    samples
}

#[test]
#[ignore = "needs installed truck models; CPU-only controlled physics scenarios"]
fn installed_trucks_brake_downhill_turn_and_hit_props_on_one_fixed_timeline() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    for name in ["semia", "semib", "semicon", "semicrate", "semilog"] {
        let tractor = model(&dir, &data, name, &mut renderer);
        let trailer = model(&dir, &data, &data.trailer_of(name).unwrap(), &mut renderer);
        for scenario in ["brake", "downhill", "corner", "cone"] {
            let expected = trace(&tractor, &trailer, &data, scenario, 60);
            for fps in [30, 144] {
                assert_eq!(trace(&tractor, &trailer, &data, scenario, fps), expected, "{name} {scenario} {fps}fps");
            }
        }
    }
}
