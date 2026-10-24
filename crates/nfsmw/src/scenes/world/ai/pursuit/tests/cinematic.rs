use super::*;
use crate::scenes::world::ai::CopRequestRole;
use blackbox_driver::VehicleView;

fn body(at: Vec3, speed: f32) -> Body {
    Body { position: at, velocity: Vec3::Z * speed, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 }
}

#[test]
fn a_world_moment_holds_placements_without_locking_out_or_consuming_requests() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.add_cop_model(cop_model(&mut free_roam::renderer(), "copx", player()));
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![wave()], player());
    world.queue_cop_spawn("copx", Vec3::new(2.0, 0.0, 150.0), Vec3::Z, CopRequestRole::Chase);
    let collision = floor();
    let surfaces = SurfaceTable::default();
    world.hold_world_moment(true);
    for _ in 0..30 {
        world.service_spawn_requests(focus(), &collision, &surfaces);
        world.update_pursuit(0.1, focus(), &collision, &surfaces);
    }
    assert!(world.cars.is_empty());
    assert_eq!(world.cop_requests_waiting(), 1);
    assert_eq!(world.density(), 0.0);
    assert!(world.cops.spawning_enabled(), "the moment is separate from a session lockout");
    assert_eq!(world.pursuit.as_ref().unwrap().involved, 0);
    world.hold_world_moment(false);
    world.service_spawn_requests(focus(), &collision, &surfaces);
    assert_eq!(world.cop_requests_waiting(), 0);
    assert_eq!(cop_count(&world), 1, "the original request is placed after release");
}

#[test]
fn temporary_racer_controls_steer_down_the_road_without_player_buttons() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    let body = body(Vec3::new(2.0, 0.0, -30.0), 15.0);
    let view = VehicleView {
        position: body.position,
        forward: Vec3::Z,
        forward_speed: 15.0,
        planar_speed: 15.0,
        gear_is_reverse: false,
        max_steer: 0.5,
        in_shock: false,
        staging: false,
    };
    let mut state = None;
    for _ in 0..16 {
        let input = world.moment_input(&mut state, body, view, player(), AiVehicle::default(), 1500.0);
        assert!(input.steer.abs() < 0.1, "a straight road is ahead: {input:?}");
        assert!(!input.reset && !input.nos && !input.speedbreaker && !input.shift_up && !input.shift_down);
    }
    assert!(state.is_some(), "the physical player acquired a race driver");
}

#[test]
fn an_arrest_uses_the_road_boundary_midpoint_instead_of_the_players_lane() {
    let mut net = straight_road(1000.0);
    net.profiles[0] = RoadProfile {
        middle: 1,
        zones: vec![
            Zone { kind: zone::TRAFFIC, width: 6.0, offset: 3.0 },
            Zone { kind: zone::TRAFFIC, width: 2.0, offset: 1.0 },
        ],
    };
    let world = TrafficWorld::new(net);
    let player = body(Vec3::new(1.0, 0.0, 30.0), 0.0);
    let anchor = world.arrest_anchor(player).unwrap();
    let at = Vec3::from_array(space::to_physics(anchor.origin));
    assert!((at - Vec3::new(-2.0, 0.0, 30.0)).length() < 1e-3, "road midpoint {at:?}");
    assert!((at - player.position).length() > 2.9);
}

#[test]
fn no_road_means_no_fabricated_arrest_location() {
    let world = TrafficWorld::new(RoadNetwork { nodes: vec![], segments: vec![], profiles: vec![], roads: vec![] });
    assert!(world.arrest_anchor(body(Vec3::ZERO, 0.0)).is_none());
}
