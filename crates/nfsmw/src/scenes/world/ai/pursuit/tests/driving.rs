//! Integrated regressions: the goal's lead target must reach the physical driver's steering input.

use super::*;
use crate::scenes::world::ai::car::Ctx;
use crate::scenes::world::drive::STEP as PHYSICS_STEP;
use crate::scenes::world::props::PropWorld;
use blackbox_pursuit::goals::{GoalCommand, GoalKind, Step};
use blackbox_roads::SplitMix;

#[test]
fn a_physical_cop_converges_to_either_lane_without_steering_out_of_the_road() {
    for destination in [-2.0, 2.0] {
        let collision = floor();
        let surfaces = SurfaceTable::default();
        let mut world = TrafficWorld::new(straight_road(180.0));
        let mut renderer = free_roam::renderer();
        world.add_cop_model(cop_model(&mut renderer, "copx", player()));
        assert!(world.spawn_cop_at(
            "copx",
            CopSpot {
                at: Vec3::new(-destination, 0.0, -60.0),
                target: Vec3::Z * 100.0,
                speed: 15.0,
                player: player(),
                band: None,
            },
            &collision,
            &surfaces
        ));
        let car = &mut world.cars[0];
        let mut rng = SplitMix(7);
        let mut props = PropWorld::default();
        let mut max_offset = 0.0_f32;
        for tick in 0..300 {
            let at = car.body().position;
            car.set_goal_drive(
                Some(Step {
                    limit: 20.0,
                    command: GoalCommand {
                        target_point: Some(Vec2::new(destination, at.z + 30.0)),
                        desired_speed: 20.0,
                        arrived: false,
                    },
                }),
                Some(GoalKind::RollingBlock),
            );
            let target = Body {
                position: Vec3::new(destination, at.y, at.z + 60.0),
                velocity: Vec3::Z * 20.0,
                forward: Vec2::Y,
                half_width: 1.0,
                half_length: 2.0,
            };
            let mut ctx = Ctx {
                net: &world.network,
                index: &world.index,
                rng: &mut rng,
                bodies: &[],
                me: 0,
                player: Some(target),
                aims: &[None],
                lights: None,
            };
            car.step(&mut ctx, &collision, &mut props, &surfaces);
            max_offset = max_offset.max(car.body().position.x.abs());
            if tick % 60 == 0 {
                eprintln!(
                    "lane {destination}: t{:.1} body={:?} controls={:?}",
                    tick as f32 * PHYSICS_STEP,
                    car.body(),
                    car.controls()
                );
            }
        }
        let at = car.body().position;
        assert!(max_offset < 4.0, "lane {destination}: drove outside corridor to {max_offset}; {at:?}");
        assert!((at.x - destination).abs() < 1.0, "lane {destination}: failed to converge: {at:?}");
        assert!(car.speed() > 15.0, "a clear lane must retain useful speed: {}", car.speed());
    }
}

#[test]
fn a_ram_goal_steers_at_its_command_instead_of_the_raw_road_slot() {
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut world = TrafficWorld::new(straight_road(180.0));
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    world.add_cop_model(cop_model(&mut renderer, "copx", player()));
    assert!(world.spawn_cop_at(
        "copx",
        CopSpot { at: Vec3::new(2.0, 0.0, -20.0), target: Vec3::Z * 100.0, speed: 10.0, player: player(), band: None },
        &collision,
        &surfaces
    ));
    let car = &mut world.cars[0];
    let position = car.body().position;
    let goal = Vec2::new(position.x + 12.0, position.z + 25.0);
    car.set_goal_drive(
        Some(Step {
            limit: 30.0,
            command: GoalCommand { target_point: Some(goal), desired_speed: 30.0, arrived: false },
        }),
        Some(GoalKind::Pit),
    );
    let raw_slot = Some(position + Vec3::Z * 80.0);
    let bodies = [(car.body(), true)];
    let target = Body {
        position: position + Vec3::Z * 45.0,
        velocity: Vec3::Z * 20.0,
        forward: Vec2::Y,
        half_width: 1.0,
        half_length: 2.0,
    };
    let mut rng = SplitMix(7);
    let mut props = PropWorld::default();
    for _ in 0..8 {
        let mut ctx = Ctx {
            net: &world.network,
            index: &world.index,
            rng: &mut rng,
            bodies: &bodies,
            me: 0,
            player: Some(target),
            aims: &[raw_slot],
            lights: None,
        };
        car.step(&mut ctx, &collision, &mut props, &surfaces);
    }
    assert!(car.controls().steer > 0.05, "the rightward goal command must reach steering: {:?}", car.controls());
    assert!(!car.controls().nos, "ram goals force nitrous off");
}
