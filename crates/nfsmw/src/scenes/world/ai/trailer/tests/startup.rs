//! Measures the first visible second of an installed rig; later settling can hide a spawn impulse.

use super::*;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_truck_startup_motion_is_measured_before_suspension_settling() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = game_install::GameDir::open(std::path::PathBuf::from(path)).unwrap();
    let data = nfsmw_data::car::physics::PhysicsData::load(&dir).unwrap();
    let ground = FlatGround::new(5.0);
    let speed = crate::scenes::world::ai::traffic::STREET_SPEED * 0.75;
    for name in ["semia", "semib", "semicon", "semicrate", "semilog"] {
        let physics = data.car(name).unwrap();
        let trailer_name = data.trailer_of(name).unwrap();
        let trailer_physics = data.car(&trailer_name).unwrap();
        let pivots = (physics.bounds.pivot, trailer_physics.bounds.pivot);
        let mut a = CarSim::new(physics, [0.5; 4]);
        let mut b = CarSim::new(trailer_physics, [0.5; 4]);
        a.configure_ai();
        b.configure_trailer();
        assert!(a.place_moving(&ground, spawn(), speed));
        let mut coupling = Coupling::new(&a, &b).unwrap();
        let back = coupling.joint.anchor_b.z - coupling.joint.anchor_a.z;
        assert!(b.place_moving(&ground, Spawn { position: spawn().position - Vec3::X * back, heading: 0.0 }, speed));
        coupling.align(&a, &mut b);
        let anchors = coupling.joint.world_anchors(a.body(), b.body());
        let initial_gap = coupling.gap(&a, &b);
        let initial_y = [a.body().position.y, b.body().position.y];
        let mut max_vertical_travel = [0.0_f32; 2];
        let mut max_vertical_speed = [0.0_f32; 2];
        let mut max_angular_speed = [0.0_f32; 2];
        let mut min_up = [1.0_f32; 2];
        for tick in 0..60 {
            a.step(&DriveInput::default(), &ground, None);
            b.step(&DriveInput::default(), &ground, None);
            coupling.hold(&mut a, &mut b);
            for (i, sim) in [&a, &b].into_iter().enumerate() {
                let body = sim.body();
                assert!(sim.is_finite(), "{name} body {i} tick {tick}");
                max_vertical_travel[i] = max_vertical_travel[i].max((body.position.y - initial_y[i]).abs());
                max_vertical_speed[i] = max_vertical_speed[i].max(body.linear_velocity.y.abs());
                max_angular_speed[i] = max_angular_speed[i].max(body.angular_velocity.length());
                min_up[i] = min_up[i].min(sim.up().y);
            }
            if tick == 0 {
                println!(
                    "{name}: first_tick gap={:.6} y={:?} up={:?} velocity_y={:?}",
                    coupling.gap(&a, &b),
                    [a.body().position.y, b.body().position.y],
                    [a.up().y, b.up().y],
                    [a.body().linear_velocity.y, b.body().linear_velocity.y]
                );
            }
        }
        println!(
            "{name}/{trailer_name}: anchors={anchors:?} pivots={pivots:?} initial_gap={initial_gap:.6} initial_y={initial_y:?} vertical_travel={max_vertical_travel:?} vertical_speed={max_vertical_speed:?} angular_speed={max_angular_speed:?} min_up={min_up:?} hitched={}",
            coupling.is_hitched()
        );
        assert!(coupling.is_hitched(), "{name} released during flat-road startup");
    }
}
