//! Empty fleet -> automatic placement -> formation -> physical road interception.
//! Behavioral reference: ai-pursuit-reinforcement-driving.md; no recorded game data is embedded.

use super::*;
use crate::scenes::world::ai::PlayerView;
use crate::scenes::world::drive::STEP;
use crate::scenes::world::props::PropWorld;
use crate::viewer::camera::Lens;
use game_install::GameDir;
use nfsmw_data::car::physics::PhysicsData;

/// Tile synthetic ground along a long straight corridor without changing installed collision data.
pub(super) fn corridor() -> CollisionWorld {
    let strip = Strip {
        center: [0.0; 3],
        radius: FLOOR_RADIUS,
        flags: STRIP_UP_FACING,
        verts: [(-2560, -FLOOR_EDGE), (-2560, FLOOR_EDGE), (2560, -FLOOR_EDGE), (2560, FLOOR_EDGE)]
            .map(|(x, z)| PackedVert { x, y: 0, z, surface: 0, flags: 0 })
            .to_vec(),
    };
    let instances = (-5..=12)
        .map(|tile| Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0, 0.0, tile as f32 * 400.0],
            half_width: 20.0,
            half_length: 200.0,
            half_height: 1.0,
            radius: FLOOR_RADIUS,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        })
        .collect();
    let mut pack = CollisionPack {
        section: 1,
        instances,
        articles: vec![Article {
            strips: vec![strip],
            barriers: vec![],
            surfaces: vec![0],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: vec![],
    };
    pack.apply_group_exclusion();
    let mut world = CollisionWorld::new(None);
    world.insert(pack);
    world
}

fn moving_target(seconds: f32) -> (Focus, PlayerView, Lens) {
    let speed = 60.0;
    let body = Body {
        position: Vec3::new(2.0, 0.7, seconds * speed),
        velocity: Vec3::Z * speed,
        forward: Vec2::Y,
        half_width: 1.0,
        half_length: 2.3,
    };
    let position = space::to_render(body.position.to_array());
    let forward = space::to_render([0.0, 0.0, 1.0]);
    let focus = Focus { position: [position.x, position.y], heading: 0.0, speed, body: Some(body) };
    let view = PlayerView {
        position,
        velocity: forward * speed,
        forward,
        speed,
        throttle: 1.0,
        nitrous: 0.0,
        nos_input: false,
        nitrous_boost: 0.0,
    };
    let camera = Lens::look_at(position - forward * 8.0 + Vec3::Z * 3.0, position, 60.0, 0.1, 1.0);
    (focus, view, camera)
}

#[test]
#[ignore = "needs installed police physics and heat tables; full automatic-spawn driving on synthetic ground"]
fn installed_automatic_reinforcements_keep_a_forward_road_target_and_accelerate() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let player_physics = data.car("bmwm3gtr").unwrap();
    let performance = nfsmw_data::car::physics::measured::ai_performance_of_spec(
        &player_physics.spec,
        player_physics.tire_integration,
    );
    let collision = corridor();
    for heat in [1, 3, 5] {
        let mut renderer = free_roam::renderer();
        let mut model = cop_model(&mut renderer, "copmidsize", performance);
        model.model.physics = data.car("copmidsize").unwrap();
        model.car.performance = nfsmw_data::car::physics::measured::ai_performance_of_spec(
            &model.model.physics.spec,
            model.model.physics.tire_integration,
        );
        model.car.ai = nfsmw_data::pursuit::ai_vehicle(data.database(), "copmidsize");
        let mut world = TrafficWorld::new(straight_road(3000.0));
        world.load_data(data.database(), None);
        world.configure(0, 0);
        world.add_cop_model(model);
        let mut rows = nfsmw_data::pursuit::heat_rows(data.database());
        for row in &mut rows {
            // Hold model/count fixed to isolate each heat's driving, not fleet composition or support.
            row.cops = vec![CopRecord { name: "copmidsize".into(), count: 1, chance: 100 }];
            row.full_engagement_cops = 1;
            row.patrol_cars = 0;
        }
        world.set_career_bin(Some(16));
        world.police.set_forced_pursuit(true);
        world.start_pursuit(heat, rows, performance);
        let mut props = PropWorld::default();
        let mut original = None;
        let mut birth_tick = 0;
        let mut samples = 0;
        let mut maximum_offset = 0.0_f32;
        for tick in 0..1200 {
            let (focus, view, camera) = moving_target(tick as f32 * STEP);
            world.player_position = focus.body.map(|body| body.position);
            world.step(focus, &collision, &mut props, &data.surfaces);
            world.collide(None);
            world.finish_step(focus, &camera, &camera, &props, &collision, &data.surfaces);
            world.player_position = focus.body.map(|body| body.position);
            world.player_speed = focus.speed;
            let cast = crate::scenes::world::drive::sight::sight_cast(&collision);
            world.update_sight(STEP, &cast);
            world.update_police(STEP, &view, &|from, to| cast(from, to).is_none());
            if original.is_none() {
                let Some(car) = world.cars.iter().find(|car| car.is_cop()) else { continue };
                assert!(car.physics_forward().dot(Vec3::Z) > 0.9, "straight-road entry faces along travel");
                original = Some(car.audio_emitter().id);
                birth_tick = tick;
                eprintln!(
                    "AUTO_ENTRY heat={heat} tick={tick} position={:?} speed={}",
                    car.body().position,
                    car.speed()
                );
            }
            let car = world
                .cars
                .iter()
                .find(|car| Some(car.audio_emitter().id) == original)
                .expect("the first reinforcement must remain through its entry sequence");
            let age_ticks = tick - birth_tick;
            maximum_offset = maximum_offset.max((car.body().position.x - 2.0).abs());
            assert!(car.is_finite(), "automatic reinforcement body must remain finite");
            if ![60, 180, 300, 600].contains(&age_ticks) {
                continue;
            }
            let body = car.body();
            let (target, requested) = car.drive_request();
            let forward_target = (target - body.position).dot(car.physics_forward());
            eprintln!(
                "AUTO_DRIVE heat={heat} age={:.1} actual={} requested={requested} forward_target={forward_target} body={body:?} controls={:?}; {}",
                age_ticks as f32 * STEP,
                body.velocity.length(),
                car.controls(),
                car.debug()
            );
            // Engineering safety conditions from the recorded forward encounters, not fitted OG speed constants.
            assert!(
                car.physics_forward().dot(Vec3::Z) > 0.5,
                "entry must not reverse on an unobstructed straight road"
            );
            assert!(forward_target > 0.0, "road steering target must stay in front during forward interception");
            assert!(maximum_offset < 6.0, "entry must stay in the road corridor: {maximum_offset}");
            assert!(
                requested <= 5.0 || car.speed() > requested * 0.25,
                "a high forward request must reach physical movement; leading-car braking is permitted"
            );
            samples += 1;
            if age_ticks >= 600 {
                break;
            }
        }
        assert_eq!(samples, 4, "automatic manager placement must reach all entry checkpoints at heat {heat}");
    }
}
