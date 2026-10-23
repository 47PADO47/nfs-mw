//! The recycling rules of `population.rs`: which unseen cars go and which stay.

use blackbox_roads::Body;
use glam::Vec2;

use super::*;

/// The bounding radius a test car uses (metres): about a sedan's half-diagonal.
const CAR_RADIUS: f32 = 3.0;

/// The free camera: eye at the origin, looking along +Y (render space, Z up), no car driven.
fn camera_north() -> Lens {
    Lens::look_at(Vec3::ZERO, Vec3::Y, 90.0, 0.1, 1.0)
}

fn free_focus() -> Focus {
    Focus { position: [0.0, 0.0], heading: 0.0, speed: 0.0, body: None }
}

fn car_at(position: Vec3, offscreen: f32, airborne: f32) -> Vitals {
    Vitals { position, finite: true, airborne, idle: 0.0, offscreen, detached_seen: false, stuck: false }
}

fn limits(density: f32) -> Limits {
    Limits { time: manager::offscreen_time(density), distance: manager::offscreen_distance(density) }
}

#[test]
fn a_car_unseen_for_five_seconds_off_the_minimap_is_removed() {
    let view = Viewing::new(&camera_north(), free_focus());
    // Behind the camera, so out of view, and beyond the minimap. At density 1 the limits are 4 s and 40 m.
    let behind = Vec3::new(0.0, -450.0, 0.0);
    assert!(!view.sees(behind, CAR_RADIUS));
    assert!(!view.on_minimap(behind));
    assert!(!car_at(behind, 5.0, 0.0).valid(&view, limits(1.0)));
    assert!(car_at(behind, 3.0, 0.0).valid(&view, limits(1.0)), "not yet 4 s");
}

#[test]
fn turning_round_finds_the_cars_that_were_behind() {
    let view = Viewing::new(&camera_north(), free_focus());
    // Unseen for a long time but still on the minimap: the original's 40 m rule would have removed them.
    for distance in [50.0, 150.0, 390.0] {
        let behind = Vec3::new(0.0, -distance, 0.0);
        assert!(!view.sees(behind, CAR_RADIUS));
        assert!(car_at(behind, 30.0, 0.0).valid(&view, limits(1.0)), "{distance} m behind stays");
    }
}

#[test]
fn a_visible_car_at_400_metres_is_not_removed() {
    let view = Viewing::new(&camera_north(), free_focus());
    let ahead = Vec3::new(0.0, 400.0, 0.0);
    assert!(view.sees(ahead, CAR_RADIUS));
    // The timer is 0 while the car is seen (note_views resets it), so the rewrite's 350 m rule does not apply.
    assert!(car_at(ahead, 0.0, 0.0).valid(&view, limits(1.0)));
    assert!(car_at(ahead, 0.0, 10.0).valid(&view, limits(1.0)), "airborne but seen");
}

#[test]
fn the_rewrite_rules_apply_only_while_unseen() {
    let view = Viewing::new(&camera_north(), free_focus());
    let behind_far = Vec3::new(0.0, -450.0, 0.0);
    assert!(car_at(behind_far, 0.5, 0.0).valid(&view, limits(1.0)), "new spawn range must survive a short look away");
    assert!(!car_at(behind_far, 5.0, 0.0).valid(&view, limits(1.0)), "normal unseen time still recycles it");
    assert!(!car_at(behind_far, 0.5, 4.0).valid(&view, limits(1.0)), "unseen and airborne");
}

#[test]
fn a_detached_part_in_view_keeps_an_otherwise_removable_car() {
    let view = Viewing::new(&camera_north(), free_focus());
    // The car is behind the camera, unseen 5 s and off the minimap: removable without a visible part.
    let behind = Vec3::new(0.0, -500.0, 0.0);
    let bare = car_at(behind, 5.0, 0.0);
    let with_part = Vitals { detached_seen: true, ..car_at(behind, 5.0, 0.0) };
    assert!(!bare.valid(&view, limits(1.0)), "removed when no part is in view");
    assert!(with_part.valid(&view, limits(1.0)), "a visible detached part keeps it");
    // The part is a sphere (render space) in front of the camera, which is what `parts_seen` tests.
    assert!(view.sees(Vec3::new(0.0, 20.0, 0.0), 1.0), "a part ahead is in view");
    assert!(!view.sees(Vec3::new(0.0, -50.0, 0.0), 1.0), "a part behind is not");
}

#[test]
fn a_stuck_car_goes_once_unseen_and_past_the_distance_only() {
    let view = Viewing::new(&camera_north(), free_focus());
    let stuck = |position: Vec3, offscreen: f32| Vitals { stuck: true, ..car_at(position, offscreen, 0.0) };
    let behind = Vec3::new(0.0, -500.0, 0.0);
    assert!(!view.sees(behind, CAR_RADIUS));
    assert!(stuck(behind, 0.5).valid(&view, limits(1.0)), "stuck cars still receive offscreen grace");
    assert!(!stuck(behind, 5.0).valid(&view, limits(1.0)), "stuck cars retire after grace");
    let near = Vec3::new(0.0, -10.0, 0.0);
    assert!(stuck(near, 0.5).valid(&view, limits(1.0)), "inside the distance rule it stays");
    assert!(stuck(Vec3::new(0.0, -50.0, 0.0), 5.0).valid(&view, limits(1.0)), "on the minimap it stays");
    let seen = Vec3::new(0.0, 50.0, 0.0);
    assert!(view.sees(seen, CAR_RADIUS));
    assert!(Vitals { stuck: true, ..car_at(seen, 0.0, 0.0) }.valid(&view, limits(1.0)), "never removed in view");
    assert!(car_at(behind, 0.5, 0.0).valid(&view, limits(1.0)), "not stuck: the time rule still applies");
}

#[test]
fn a_car_behind_the_player_stays_while_the_chase_camera_sees_it() {
    // The player drives east at (100, 0, 0). The chase camera sits 8 m behind it and 3 m up, looking at it.
    let player = Vec3::new(100.0, 0.0, 0.0);
    let camera = Lens::look_at(Vec3::new(92.0, 0.0, 3.0), player, 60.0, 0.1, 1.0);
    let focus = Focus {
        position: [100.0, 0.0],
        heading: 0.0,
        speed: 20.0,
        body: Some(Body {
            position: Vec3::from(space::to_physics(player)),
            velocity: Vec3::new(0.0, 0.0, 20.0),
            forward: Vec2::X,
            half_width: 1.0,
            half_length: 2.5,
        }),
    };
    let view = Viewing::new(&camera, focus);
    // 4 m behind the player, between the camera and the car: behind the player, in the chase frustum.
    let behind_player = Vec3::new(96.0, 0.0, 0.0);
    assert!(view.sees(behind_player, CAR_RADIUS));
    assert!(car_at(behind_player, 0.0, 0.0).valid(&view, limits(1.0)));
}

#[test]
fn patrol_uses_normal_traffic_speeds_and_search_keeps_its_installed_speeds() {
    let plan = PatrolPlan::new(vec!["copmidsize".into()], 50.0, 71.0);
    assert!((plan.cruise.street - 35.0 * MPH).abs() < 1e-5);
    assert!((plan.cruise.highway - 55.0 * MPH).abs() < 1e-5);
    assert!((plan.search_cruise.street - 22.352).abs() < 1e-2);
    assert!(plan.cruise.highway > plan.cruise.street);
    assert!(!plan.cruise.limit_acceleration);
}
