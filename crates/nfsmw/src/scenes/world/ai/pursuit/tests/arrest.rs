use super::*;
use crate::scenes::world::ai::PlayerView;

#[test]
fn a_bust_keeps_stopped_cops_until_payment_then_releases_the_terminal_state() {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut world = TrafficWorld::new(cop_road());
    world.add_cop_model(cop_model(&mut renderer, "copx", player()));
    world.start_pursuit(1, vec![wave()], player());
    let spot = CopSpot { at: Vec3::new(0.0, 0.0, 8.0), target: Vec3::ZERO, speed: 0.0, player: player() };
    assert!(world.spawn_cop_at("copx", spot, &collision, &surfaces));
    let identity = world.cars[0].audio_emitter().id;
    let view = PlayerView { position: Vec3::ZERO, forward: Vec3::X, speed: 0.0, nitrous: 0.0, throttle: 0.0 };
    for _ in 0..51 {
        world.seen_by_cops = true;
        world.update_police(0.1, &view, &|_, _| true);
    }
    assert!(world.arrest_active() && world.is_busted());
    assert!(!world.is_pursuing(), "normal pursuit/music feed ends on the bust");
    assert_eq!(world.cars.len(), 1, "the arrest must not erase the visible cop");
    assert_eq!(world.cars[0].audio_emitter().id, identity);
    assert!(!world.cars[0].emergency_lights());
    world.step(focus(), &collision, &mut crate::scenes::world::props::PropWorld::default(), &surfaces);
    let controls = world.cars[0].controls();
    assert_eq!((controls.gas, controls.brake, controls.handbrake, controls.steer), (0.0, 1.0, 1.0, 0.0));
    assert!(!controls.nos);
    assert!(!world.pay_bust(false), "payment cannot skip the hold");
    world.update_police(3.1, &view, &|_, _| true);
    assert!(world.take_bust_ticket().is_some(), "the no-NIS fallback reaches the ticket after3s");
    assert!(!world.pay_bust(true), "an unavailable marker keeps the ticket active");
    assert!(world.arrest_active());
    assert!(world.pay_bust(false));
    assert!(!world.arrest_active() && !world.is_busted());
    assert_eq!(world.busted_meter(), 0.0);
    assert!(world.cars.is_empty(), "the declared free-roam fallback clears its retained units");
    assert_eq!(world.vehicle_heat(), 1.0);
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.is_pursuing() && !world.is_busted());
}
