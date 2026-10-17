use super::*;
use blackbox_gfx::RenderBackend;

#[test]
fn collision_survives_colour_retirement_then_expires_without_expanding_colour() {
    let (mut residency, mut renderer) = super::super::shadow_tests::fixture();
    renderer.set_render_experiments(Default::default());
    residency.collision.insert(CollisionPack { section: 77, instances: vec![], articles: vec![], objects: vec![] });
    let Some(TileState::Resident { packs, .. }) = residency.tiles.get_mut(&0) else { panic!("fixture") };
    packs.push(77);
    residency.set_traffic_support([-20.0, 0.0], 100.0, vec![]);
    residency.update(&mut renderer, -1.0, 0.0);
    residency.update(&mut renderer, 1.0, 0.0);
    assert!(!residency.tiles.contains_key(&0), "traffic cannot retain a tile's GPU allocations");
    assert!(!residency.drawn.contains(&0));
    assert!(residency.collision.pack(77).is_some(), "traffic owns the collision after colour retirement");
    residency.set_traffic_support([2000.0, 0.0], 100.0, vec![]);
    residency.update(&mut renderer, 1.0, 0.0);
    assert!(residency.collision.pack(77).is_none(), "traffic-only collision is bounded");
}

#[test]
fn retiring_auxiliary_owner_cannot_remove_a_colour_owned_pack() {
    let (mut residency, _) = super::super::shadow_tests::fixture();
    residency.collision.insert(CollisionPack { section: 77, instances: vec![], articles: vec![], objects: vec![] });
    let Some(TileState::Resident { packs, .. }) = residency.tiles.get_mut(&0) else { panic!("fixture") };
    packs.push(77);
    residency.traffic_support.resident.insert(0, vec![77]);
    residency.release_traffic_collision(0);
    assert!(residency.collision.pack(77).is_some());
}

#[test]
fn live_actor_support_wins_over_distant_focus_and_limits_requests() {
    let (mut residency, _) = super::super::shadow_tests::fixture();
    residency.set_traffic_support([2000.0, 0.0], 100.0, vec![(Vec2::new(-20.0, 0.0), 60.0)]);
    residency.update_traffic_support();
    assert!(residency.traffic_support.wanted.contains(&0));
    assert!(!residency.traffic_support.wanted.contains(&2));
    assert!(residency.traffic_support.pending.len() <= MAX_PENDING);
    assert!(residency.traffic_support.wanted.len() <= MAX_EXTRA);
}
