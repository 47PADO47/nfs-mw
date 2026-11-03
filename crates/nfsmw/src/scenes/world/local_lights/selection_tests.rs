use super::*;
use blackbox_gfx::{Instance, MeshHandle};
use glam::Vec2;

fn switches() -> Switches {
    Switches {
        lamps: true,
        street: true,
        street_night: 1.0,
        street_strength: 1.5,
        street_range: 35.0,
        lamp_strength: 1.0,
        street_glow: 1.0,
    }
}

fn fixture(street: bool) -> Fixture {
    Fixture {
        position: Vec3::new(0.0, 0.0, 8.0),
        direction: Vec3::NEG_Z,
        street,
        lens_half: Vec2::splat(0.3),
        glow_position: Vec3::new(0.0, 0.0, 8.0),
        lantern_half_height: 0.0,
        glass: false,
    }
}

/// A street candidate `distance` metres along X from the camera at the origin.
fn at(distance: f32, owner: u32) -> Candidate {
    let transform = Mat4::from_translation(Vec3::new(distance, 0.0, -8.0));
    candidate(&fixture(true), transform, InstanceKey::new(1, owner), 0, Vec3::ZERO, &switches())
}

#[test]
fn unpowered_or_tilted_street_fixtures_do_not_emit() {
    let s = switches();
    assert!(admits(&fixture(true), &s, || true, Vec3::Z));
    assert!(!admits(&fixture(true), &s, || false, Vec3::Z), "a hidden or moved placement is unpowered");
    let tilted = Vec3::new(1.0, 0.0, 0.6);
    assert!(!admits(&fixture(true), &s, || true, tilted), "a knocked-over pole is off");
    assert!(admits(&fixture(true), &s, || true, Vec3::new(0.0, 0.5, 1.0)), "a slight lean stays on");
    assert!(!admits(&fixture(true), &Switches { street: false, ..s }, || true, Vec3::Z));
    let mut asked = false;
    assert!(admits(&fixture(false), &s, || std::mem::replace(&mut asked, true), Vec3::X));
    assert!(!asked, "tunnel fixtures are not powered props and ignore tilt");
    assert!(!admits(&fixture(false), &Switches { lamps: false, ..s }, || true, Vec3::Z));
}

#[test]
fn glow_budget_keeps_the_nearest_256_and_fades_towards_the_boundary() {
    let mut candidates: Vec<_> = (0..300).rev().map(|i| at(10.0 + i as f32, i)).collect();
    candidates.push(candidate(&fixture(false), Mat4::IDENTITY, InstanceKey::new(2, 0), 0, Vec3::ZERO, &switches()));
    sort(&mut candidates);
    assert_eq!(candidates[1].distance, 10.0, "nearest first after the tunnel fixture");
    let mut glows = Vec::new();
    push_glows(&candidates, &switches(), &mut glows);
    assert_eq!(glows.len(), GLOW_BUDGET, "tunnel fixtures and street bulbs beyond the budget draw no bulb");
    // The 257th street bulb sits at 266 m, so the last kept ones fade over the 32 m before it.
    assert_eq!(glows[0].1, 1.0);
    let last = glows[GLOW_BUDGET - 1];
    assert!(last.1 > 0.0 && last.1 < 0.05, "the last bulb almost faded: {}", last.1);
    assert!(glows.windows(2).all(|pair| pair[0].1 >= pair[1].1));
}

#[test]
fn glows_without_a_full_budget_fade_at_the_glow_distance() {
    let candidates = [at(100.0, 0), at(GLOW_DISTANCE - FADE_METRES * 0.5, 1)];
    let mut glows = Vec::new();
    push_glows(&candidates, &switches(), &mut glows);
    assert_eq!(glows[0].1, 1.0);
    assert!((glows[1].1 - 0.5).abs() < 1e-5, "half way through the fade: {}", glows[1].1);
    assert_eq!(fade(100.0, 100.0), 0.0);
    assert_eq!(fade(100.0, 100.0 - FADE_METRES), 1.0);
    assert_eq!(glow_distance(600.0), 600.0);
    assert_eq!(glow_distance(160.0), GLOW_DISTANCE);
}

#[test]
fn surface_lights_share_the_limit_and_fade_at_the_budget() {
    assert_eq!(remaining_slots(128, 6), 122);
    assert_eq!(remaining_slots(8, 12), 0, "player and traffic lights may already fill the limit");
    let candidates: Vec<_> = (0..6).map(|i| at(20.0 * i as f32, i)).collect();
    let mut lights = Vec::new();
    push_lights(&candidates, 90.0, 3, &mut lights);
    assert_eq!(lights.len(), 3);
    // The fourth candidate (60 m) is the budget boundary: 20 m stays bright and 40 m is fading.
    assert_eq!(lights[1].color, candidates[1].light.color);
    let faded = Vec3::from_array(lights[2].color).x / Vec3::from_array(candidates[2].light.color).x;
    assert!((faded - fade(60.0, 40.0)).abs() < 1e-5 && faded < 1.0);
    lights.clear();
    push_lights(&candidates, 90.0, 10, &mut lights);
    assert_eq!(lights.len(), 5, "only candidates within the light distance");
    let faded = Vec3::from_array(lights[4].color).x / Vec3::from_array(candidates[4].light.color).x;
    assert!((faded - fade(90.0, 80.0)).abs() < 1e-5, "the last light fades towards the distance");
    lights.clear();
    push_lights(&candidates, 90.0, 0, &mut lights);
    assert!(lights.is_empty());
}

#[test]
fn lanterns_without_glass_keep_a_glow_and_lose_the_lens() {
    let mut lantern = fixture(true);
    lantern.lantern_half_height = 0.4;
    let plain = candidate(&lantern, Mat4::IDENTITY, InstanceKey::new(3, 1), 0, Vec3::ZERO, &switches());
    assert_eq!((plain.glow.2, plain.glow.3), (Vec3::ZERO, Vec3::ZERO), "no lens panel is guessed");
    assert_eq!(plain.light.shadow_exclude, Some(InstanceKey::new(3, 1)));
    lantern.glass = true;
    let glass = candidate(&lantern, Mat4::IDENTITY, InstanceKey::new(3, 1), 0, Vec3::ZERO, &switches());
    let mut glows = Vec::new();
    push_glows(&[plain, glass], &switches(), &mut glows);
    assert_eq!(glows[0].4, Vec3::ZERO, "without glass the camera-facing glow stays");
    assert_eq!(glows[1].4, Vec3::Z * 0.4, "emissive glass replaces the sprite");
    let road = candidate(&fixture(true), Mat4::IDENTITY, InstanceKey::new(3, 2), 0, Vec3::ZERO, &switches());
    assert_ne!(road.glow.2, Vec3::ZERO);
    assert_eq!(road.light.shadow_exclude, None);
    assert_eq!(road.light.range, 35.0);
    assert_eq!(candidate(&fixture(false), Mat4::IDENTITY, road.owner, 0, Vec3::ZERO, &switches()).light.range, 18.0);
}

#[test]
fn mixed_lod_lanterns_follow_the_drawn_mesh() {
    let near = MeshHandle::from_raw(10);
    let far = MeshHandle::from_raw(11);
    let mut lantern = fixture(true);
    lantern.lantern_half_height = 0.4;
    let mut candidates: Vec<_> = (0..3)
        .map(|i| {
            let mut c = candidate(&lantern, Mat4::IDENTITY, InstanceKey::new(4, i), 0, Vec3::ZERO, &switches());
            c.glass_meshes = [Some(near), None, None, None];
            c
        })
        .collect();
    let drawn = [
        Instance::keyed(far, Mat4::IDENTITY, InstanceKey::new(4, 1)),
        Instance::keyed(near, Mat4::IDENTITY, InstanceKey::new(4, 0)),
        Instance::keyed(near, Mat4::IDENTITY, InstanceKey::new(9, 9)),
    ];
    let (mut owners, mut seen) = (Vec::new(), Vec::new());
    resolve_drawn_glass(&mut candidates, &drawn, &mut owners, &mut seen);
    let glass: Vec<_> = candidates.iter().map(|c| c.glass).collect();
    assert_eq!(glass, [true, false, false], "near LOD glows; far LOD and undrawn placements keep the sprite");
    assert_eq!(seen.len(), 2, "only the candidates' owners are collected");
}
