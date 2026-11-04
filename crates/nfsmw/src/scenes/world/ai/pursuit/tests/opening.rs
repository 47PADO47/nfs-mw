//! The forced start's first cop (spec `ai-pursuit-cops.md` section 4.1): 340 m ahead along the road, facing on, for a
//! chase started where the player drives; 80 m ahead, facing back at the player, at rest, for one started after a
//! jump to a bounty place.

use super::free_roam::renderer;
use super::*;

fn row() -> HeatRow {
    let mut row = wave();
    row.cops[0].count = 4;
    row.full_engagement_cops = 20;
    row.time_between_first_four_spawns = 10.0;
    row.time_between_cop_spawns = 2.0;
    row
}

/// A chase on a straight road along +Z with the player standing at z = -200 facing +Z; the first cop's position and
/// physics forward once it stands.
fn first_cop(jumped: bool) -> (Vec3, Vec3, f32) {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    world.configure(20, 5);
    let body = Body {
        position: Vec3::new(0.0, 0.0, -200.0),
        velocity: Vec3::ZERO,
        forward: Vec2::Y,
        half_width: 1.0,
        half_length: 2.0,
    };
    let focus = Focus { position: [-200.0, 0.0], heading: 0.0, speed: 0.0, body: Some(body) };
    world.player_position = Some(body.position);
    world.police.set_forced_pursuit(true);
    world.set_jump_start(jumped);
    world.start_pursuit(1, vec![row(); 5], player());
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    world.update_pursuit(0.25, focus, &collision, &surfaces);
    let cop = world.cars.first().expect("the forced start places its first cop at once");
    (cop.physics_position(), cop.physics_forward(), cop.speed())
}

#[test]
fn a_chase_started_where_the_player_drives_puts_its_first_cop_340_m_ahead_facing_on() {
    let (at, forward, speed) = first_cop(false);
    assert!((at.z - 140.0).abs() < 5.0, "{at:?}");
    assert!(forward.z > 0.9, "{forward:?}");
    assert!(speed.abs() < 0.5, "{speed}");
}

#[test]
fn a_chase_started_after_a_bounty_jump_puts_its_first_cop_80_m_ahead_facing_the_player() {
    let (at, forward, speed) = first_cop(true);
    assert!((at.z + 120.0).abs() < 5.0, "{at:?}");
    assert!(forward.z < -0.9, "{forward:?}");
    assert!(speed.abs() < 0.5, "{speed}");
}

#[test]
fn the_jump_mark_is_taken_by_one_chase_only() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.set_jump_start(true);
    world.start_pursuit(1, vec![row(); 5], player());
    assert_eq!(world.pursuit.as_ref().unwrap().opening, Some(Opening::AfterJump));
    world.stop_pursuit();
    world.start_pursuit(1, vec![row(); 5], player());
    assert_eq!(world.pursuit.as_ref().unwrap().opening, Some(Opening::Ahead));
    world.pursuit.as_mut().unwrap().joined(false);
    assert_eq!(world.pursuit.as_ref().unwrap().opening, None, "a joined unit ends the forced start");
}

#[test]
#[ignore = "needs the installed city road network; set NFSMW_GAME_DIR"]
fn installed_riverfront_jump_puts_the_first_cop_where_the_original_does() {
    // Riverfront Stadium (bounty place 11): the car stands on the marker facing its Rotation. The stock game's first
    // cop stands at physics (648.4, 111.1, 978.9), facing (-0.326, -0.025, -0.945) (spec `ai-pursuit-cops.md` 4.1).
    let (_scene, _renderer, mut world) = super::installed_route::open_at([939.84, -595.59]);
    let forward = Vec2::new(0.9745, -0.2196).normalize();
    let body = Body {
        position: Vec3::new(595.59, 110.84, 939.84),
        velocity: Vec3::ZERO,
        forward,
        half_width: 1.0,
        half_length: 2.4,
    };
    let nav = world.opening_cursor(&body, Opening::AfterJump).expect("a road ahead of the stadium marker");
    let original = Vec3::new(648.36, 111.11, 978.85);
    let off = Vec2::new(nav.position.x - original.x, nav.position.z - original.z).length();
    let facing = Vec2::new(nav.forward.x, nav.forward.z).normalize().dot(Vec2::new(-0.326, -0.945).normalize());
    eprintln!("RIVERFRONT_OPENING at={:?} forward={:?} off={off:.2} facing={facing:.3}", nav.position, nav.forward);
    // The walk takes the original's road (the junction link north onto the stadium road) and lands about 10 m
    // farther along it: the lane the walk runs in is not the original's.
    assert!(off < 12.0, "{off} m from the original's first cop");
    assert!(facing > 0.95, "facing dot {facing}");
}
