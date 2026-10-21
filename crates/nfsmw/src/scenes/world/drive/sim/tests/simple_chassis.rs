//! NPC configuration reaches the chassis without changing the player's physical car.

use super::*;

#[test]
fn configured_npc_uses_original_simple_chassis_cog_and_live_steering_range() {
    let (mut player, mut npc) = (sim(), sim());
    npc.configure_ai();
    let (mass, inertia) = (npc.body().mass(), npc.body().inertia());
    assert!((npc.driver_view().max_steer.to_degrees() - 45.0).abs() < 1e-5);
    player.step(&DriveInput::default(), &NoGround, None);
    npc.step(&DriveInput::default(), &NoGround, None);
    assert!((npc.body().cog().y - player.body().cog().y - 10.0 * 0.0254).abs() < 1e-6);
    assert!((npc.driver_view().max_steer.to_degrees() - 60.0).abs() < 1e-5);
    assert!((player.driver_view().max_steer.to_degrees() - 45.0).abs() < 1e-5);
    assert_eq!((npc.body().mass(), npc.body().inertia()), (mass, inertia));
}
