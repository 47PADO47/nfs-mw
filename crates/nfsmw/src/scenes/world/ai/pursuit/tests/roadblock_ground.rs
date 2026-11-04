//! Roadblocks on a road that climbs or falls between the player and the site (spec `ai-pursuit-roadblocks.md`
//! sections 2.4 and 2.5): every element starts at the road's height at the site, not at the player's, each prop rests
//! the bottom of its collision box on the surface under it, and each car stands on its wheels there.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::spikes::CENTRE_LIFT;

use super::*;
use crate::scenes::world::ai::PlayerView;
use crate::scenes::world::ai::roadblock::Cause;

/// Vertex units per metre of the collision floor (`VERT_SCALE`).
const UNITS: f32 = 128.0;
/// The floor's half size, vertex units (200 m).
const EDGE: i16 = 25_600;

/// The road's height at `z` for a road falling `grade` metres per metre along +z.
fn height(grade: f32, z: f32) -> f32 {
    -grade * z
}

/// A straight road along z, 2 km long, falling `grade` metres per metre, with a lane either side.
fn sloped_road(grade: f32) -> RoadNetwork {
    let lane = |offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset };
    let node = |z: f32| RoadNode { position: Vec3::new(0.0, height(grade, z), z), profile: 0, segments: vec![0] };
    RoadNetwork {
        nodes: vec![node(-1000.0), node(1000.0)],
        segments: vec![RoadSegment {
            nodes: [0, 1],
            length: 2000.0,
            road: Some(0),
            flags: 0,
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        }],
        profiles: vec![RoadProfile { middle: 1, zones: vec![lane(2.0), lane(2.0)] }],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    }
}

/// A plane 200 m to each side of the origin under the road of [`sloped_road`].
fn sloped_floor(grade: f32) -> CollisionWorld {
    let corners = [(-EDGE, -EDGE), (-EDGE, EDGE), (EDGE, -EDGE), (EDGE, EDGE)];
    let y = |z: i16| (height(grade, f32::from(z) / UNITS) * UNITS) as i16;
    let strip = Strip {
        center: [0.0; 3],
        radius: 300.0,
        flags: STRIP_UP_FACING,
        verts: corners.map(|(x, z)| PackedVert { x, y: y(z), z, surface: 0, flags: 0 }).to_vec(),
    };
    let mut pack = CollisionPack {
        section: 1,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 200.0,
            half_length: 200.0,
            half_height: 50.0,
            radius: 300.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
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

/// The player on the road at `z`, driving along +z.
fn view_on(grade: f32, z: f32) -> PlayerView {
    PlayerView {
        nos_input: false,
        nitrous_boost: 0.0,
        velocity: space::to_render(Vec3::Z.to_array()) * 20.0,
        position: space::to_render([0.0, height(grade, z), z]),
        forward: space::to_render(Vec3::Z.to_array()),
        speed: 20.0,
        nitrous: 0.0,
        throttle: 0.5,
    }
}

/// A heat-9 chase (spike layouts every time) on the sloped road, with a roadblock built 250 m ahead of z = -150.
fn roadblock_on(grade: f32) -> (TrafficWorld, Result<usize, String>) {
    let mut world = TrafficWorld::new(sloped_road(grade));
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    world.add_cop_model(cop_model(&mut renderer, "copx", player()));
    world.police.set_forced_pursuit(true);
    world.start_pursuit(9, vec![wave()], player());
    let built = world.build_roadblock(
        Cause::Console,
        view_on(grade, -150.0),
        &sloped_floor(grade),
        None,
        &SurfaceTable::default(),
    );
    (world, built)
}

fn assert_seated(grade: f32) {
    let (world, built) = roadblock_on(grade);
    let cars = built.unwrap_or_else(|e| panic!("grade {grade}: the roadblock is built: {e}"));
    assert!(cars >= 1);
    let strips = world.roadblocks.strips();
    assert!(!strips.is_empty(), "heat 9 takes the spike layouts");
    for strip in &strips {
        let ground = height(grade, strip.position.y);
        let bottom = strip.elevation + CENTRE_LIFT - strip.half[2];
        assert!((bottom - ground).abs() < 0.02, "grade {grade}: strip box bottom {bottom} on ground {ground}");
    }
    for car in world.cars.iter().filter(|car| car.roadblock_group().is_some()) {
        let at = car.physics_position();
        let above = at.y - height(grade, at.z);
        assert!((0.0..2.5).contains(&above), "grade {grade}: a roadblock car stands {above} m above the road");
    }
}

#[test]
fn a_roadblock_below_the_player_lies_on_its_own_road() {
    // The site, about 250 m on, is 25 m lower than the player: the props no longer hang at the player's height.
    assert_seated(0.1);
}

#[test]
fn a_roadblock_above_the_player_lies_on_its_own_road() {
    // The site is 25 m higher than the player: its cars and props are found on the road there, not under it.
    assert_seated(-0.1);
}

#[test]
fn a_level_roadblock_rests_its_strip_boxes_on_the_road() {
    assert_seated(0.0);
}
