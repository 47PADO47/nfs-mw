//! The pursuit breaker through the streamed world: the scenery that residency placed collapses under a hit, its sequence
//! opens a cop zone, and the police cars inside it are wrecked and counted while the others are not.

use super::*;
use crate::input::{ActionState, Bindings};
use crate::scenes::world::ai::installed_support::patrol_at;
use crate::scenes::world::props::scenery::ram;
use crate::scenes::world::{DriveOptions, Options, WorldScene, drive::STEP};
use crate::viewer::Scene;
use bevy_input::keyboard::KeyCode;
use game_install::GameDir;

/// A gas station of the installed city: physics position of its roof (`docs/specs/pursuit-breakers.md`).
const GAS_STATION: [f32; 3] = [-2147.85, 157.14, 1156.97];

fn flat(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

/// The streamed city around the gas station, a chase running, one patrol car inside the zone and one far outside it.
struct Fixture {
    scene: WorldScene,
    renderer: NullBackend,
    pad: ActionState,
    roof: Vec3,
    inside: u32,
    outside: u32,
}

impl Fixture {
    fn open() -> Self {
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
        let roof = Vec3::from_array(GAS_STATION);
        let map = space::to_render(roof.to_array());
        let mut scene = WorldScene::open(
            &dir,
            Options {
                at: Some([map.x, map.y]),
                height: 40.0,
                heading: 45.0,
                pitch: -20.0,
                fog_distance: 1500.0,
                wait_for_load: true,
                drive: Some(DriveOptions { car: "ELISE".into(), script: None }),
            },
        )
        .unwrap();
        let mut renderer = NullBackend::new(crate::settings::test_caps::native());
        scene.init(&mut renderer).unwrap();
        let mut pad = ActionState::default();
        pad.update_for_test(&Bindings::default(), &[KeyCode::Space], &[], false);
        for _ in 0..2000 {
            scene.update(&mut renderer, &pad, STEP);
            if scene.ready() && scene.residency.complete() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(scene.ready() && scene.residency.complete(), "the streamed tiles must be resident");
        for _ in 0..60 {
            scene.update(&mut renderer, &pad, STEP);
        }
        let mut fixture = Self { scene, renderer, pad, roof, inside: 0, outside: 0 };
        fixture.start_chase();
        fixture
    }

    /// Two patrol cars: one beside the station, one far down the road. A chase starts against the parked player.
    fn start_chase(&mut self) {
        let (scene, renderer) = (&mut self.scene, &mut self.renderer);
        let site = scene.residency.props().breaker_sites(&scene.physics, flat(self.roof))[0].clone();
        crate::scenes::world::ai::models::load_cop_models(scene, renderer, &["copmidsize".into()]).unwrap();
        crate::scenes::world::ai::models::load_requested(scene, renderer, false, true);
        let performance = scene.drive.as_ref().unwrap().performance();
        scene.traffic.as_mut().unwrap().cars.clear();
        scene.traffic.as_mut().unwrap().configure(0, 0);
        let inside = patrol_at(scene, self.roof).expect("a cop stands by the station");
        let inside_at = inside.physics_position();
        assert!(
            flat(inside_at).distance(flat(site.position)) < site.zone.0 - 2.0,
            "the near cop must start inside the zone: {inside_at:?} against {:?}",
            site.position
        );
        let mut outside = None;
        for offset in [80.0, -80.0, 140.0, -140.0, 220.0, -220.0] {
            let candidate = patrol_at(scene, self.roof + Vec3::new(0.0, 0.0, offset));
            if candidate
                .as_ref()
                .is_some_and(|c| flat(c.physics_position()).distance(flat(self.roof)) > site.zone.0 + 30.0)
            {
                outside = candidate;
                break;
            }
        }
        let outside = outside.expect("a cop stands well outside the zone");
        (self.inside, self.outside) = (inside.audio_emitter().id, outside.audio_emitter().id);
        let player = scene.drive.as_ref().unwrap().ai_body().position;
        let world = scene.traffic.as_mut().unwrap();
        world.cars.push(inside);
        world.cars.push(outside);
        let mut rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
        // The wave wants both patrol cars, so the manager keeps them: cops over the wave's count are sent away.
        for row in &mut rows {
            row.cops = vec![nfsmw_data::pursuit::CopRecord { name: "copmidsize".into(), count: 2, chance: 100 }];
        }
        world.player_position = Some(player);
        world.start_pursuit(1, rows, performance);
        assert_eq!(world.cops_destroyed(), 0);
    }

    /// Runs the world for `seconds`; true when a zone was open at any tick.
    fn run(&mut self, seconds: f32) -> bool {
        let mut zone_seen = false;
        for _ in 0..(seconds / STEP) as usize {
            self.scene.update(&mut self.renderer, &self.pad, STEP);
            zone_seen |= !self.scene.traffic.as_ref().unwrap().breaker_zone_list().is_empty();
        }
        zone_seen
    }

    fn fired(&self) -> bool {
        self.scene.residency.props().breaker_sites(&self.scene.physics, flat(self.roof))[0].fired
    }

    fn wrecked(&self, id: u32) -> Option<bool> {
        let traffic = self.scene.traffic.as_ref().unwrap();
        traffic.cars.iter().find(|c| c.audio_emitter().id == id).map(|c| c.is_wrecked())
    }

    fn hit(&mut self, engine: &str) {
        let props = self.scene.residency.world_parts().1;
        let id = ram::nearest(props, engine, self.roof).unwrap_or_else(|| panic!("a {engine} is resident"));
        ram::strike(props, id);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR): streams the real city around a gas station"]
fn a_streamed_gas_station_is_a_resident_breaker_with_its_zone_and_scene() {
    let fixture = Fixture::open();
    let sites = fixture.scene.residency.props().breaker_sites(&fixture.scene.physics, flat(fixture.roof));
    let site = &sites[0];
    assert_eq!(site.engine, "gasstationroof_seq", "{sites:?}");
    assert!(flat(site.position).distance(flat(fixture.roof)) < 3.0, "{site:?}");
    assert!(site.supports >= 2, "the pillars that trigger it are resident: {site:?}");
    assert_eq!(site.zone, (16.0, 5.0));
    assert_eq!(site.moments, vec!["WMGasStation".to_owned()]);
    assert!(!site.fired);
    let report = crate::scenes::world::ai::inspect::breakers(&fixture.scene).unwrap();
    assert!(report.contains("gasstationroof_seq") && report.contains("WMGasStation"), "{report}");
    assert!(report.contains("no zone is open"), "{report}");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR): streams the real city around a gas station"]
fn hitting_a_gas_pump_collapses_the_roof_and_wrecks_the_cops_in_its_zone_only() {
    let mut fixture = Fixture::open();
    fixture.hit("gaspumpexplosion_seq");
    assert!(fixture.run(10.0), "the traffic took the zone from the scenery");
    let history: Vec<_> = fixture.scene.residency.props().breaker_history().iter().cloned().collect();
    assert!(history.iter().any(|l| l.contains("gasstationroof_seq opened a cop zone: 16 m for 5 s")), "{history:?}");
    assert!(history.iter().any(|l| l.contains("WMGasStation")), "{history:?}");
    assert_eq!(fixture.wrecked(fixture.inside), Some(true), "the cop inside the zone is destroyed");
    assert_eq!(fixture.wrecked(fixture.outside), Some(false), "the cop outside the zone is not");
    let traffic = fixture.scene.traffic.as_ref().unwrap();
    assert_eq!(traffic.cops_destroyed(), 1, "the wreck counts once in the chase");
    assert!(traffic.breaker_zone_list().is_empty(), "the zone lasts five seconds");
    assert!(fixture.fired());
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR): streams the real city around a gas station"]
fn the_gas_station_pillars_are_permanent_and_only_a_pump_brings_the_roof_down() {
    let mut fixture = Fixture::open();
    let props = fixture.scene.residency.props();
    for id in ram::all(props, "gasstationsupport_seq") {
        assert!(
            ram::describe(props, id).starts_with("detach -1 "),
            "a pillar never detaches: {}",
            ram::describe(props, id)
        );
    }
    for id in ram::all(props, "gaspumpexplosion_seq") {
        assert!(ram::describe(props, id).starts_with("detach 10000 "), "a pump detaches: {}", ram::describe(props, id));
    }
    // Rammed hard, a pillar stands: nothing falls and no zone opens.
    fixture.hit("gasstationsupport_seq");
    assert!(!fixture.run(3.0));
    assert!(!fixture.fired(), "a pillar alone does not bring the roof down");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR): streams the real city around a gas station"]
fn the_players_car_driving_into_a_gas_pump_opens_the_breaker() {
    use crate::scenes::world::{drive::WorldGround, road::Spawn};
    let mut fixture = Fixture::open();
    let pump = {
        let props = fixture.scene.residency.props();
        let id = ram::nearest(props, "gaspumpexplosion_seq", fixture.roof).unwrap();
        ram::position(props, id)
    };
    let mut throttle = ActionState::default();
    throttle.update_for_test(&Bindings::default(), &[KeyCode::ArrowUp], &[], false);
    let mut results = Vec::new();
    for degrees in (0..360).step_by(30) {
        let angle = (degrees as f32).to_radians();
        let start = pump + Vec3::new(angle.cos() * 14.0, 0.0, angle.sin() * 14.0);
        let render = space::to_render(start.to_array());
        let towards = space::to_render((pump - start).to_array());
        let spawn = Spawn { position: render, heading: towards.y.atan2(towards.x) };
        {
            let scene = &mut fixture.scene;
            let ground = WorldGround { collision: scene.residency.collision(), surfaces: &scene.physics.surfaces };
            let sim = scene.drive.as_mut().unwrap().sim_mut().unwrap();
            if !sim.place_moving(&ground, spawn, 14.0) {
                results.push(format!("{degrees}: no ground"));
                continue;
            }
        }
        for _ in 0..180 {
            fixture.scene.update(&mut fixture.renderer, &throttle, STEP);
        }
        let speed = fixture.scene.drive.as_ref().unwrap().speed();
        results.push(format!("{degrees}: fired {} speed now {speed:.1}", fixture.fired()));
        if fixture.fired() {
            break;
        }
    }
    eprintln!("{results:#?}");
    assert!(fixture.fired(), "the player's Elise opens the gas station breaker by driving into a pump: {results:?}");
}
