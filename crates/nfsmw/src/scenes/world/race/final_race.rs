//! The final race 1.8.1, "epic pursuit" (`docs/specs/story-flow-2.md` section 5). It runs as a pursuit challenge: the chase
//! and the cops are the challenge's, and the rules are `nfsmw_career::story::FinalRace`. Each frame this module feeds the
//! rules the player's car, plays their events ([`final_race_events`]) and settles the race: a totalled racer loses it,
//! and a win waits for its outro and then wins. The chain after the win is the front end's (`frontend/flow_endgame.rs`).

use glam::Vec3;
use nfsmw_career::blacklist::FINAL_RACE_EVENT;
use nfsmw_career::settings::Flags;
use nfsmw_career::story::{FinalRace, FinalRaceInput};
use nfsmw_data::gameplay::{FinalRaceMarkers, Placement, Race, world_heading};

use super::RaceSession;
use super::final_race_events::{FinalEffects, Settled, apply_events};
use crate::scenes::world::moment::heading_angle;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::scene_space::Anchor;
use crate::scenes::world::scenery_groups::DOOR_GROUP;
use crate::scenes::world::{WorldScene, space};

#[cfg(test)]
#[path = "final_race_tests.rs"]
mod tests;

/// The car the special setup gives the player: the E3 demo BMW, a stable car (`story-flow.md` section 2.2, F-14).
const SPECIAL_CAR: &str = "e3_demo_bmw";
/// Kilometres per hour in one metre per second.
const KMH_PER_MPS: f32 = 3.6;
/// The win counts a crossing of the win trigger's plane within this distance of its centre, metres. The spec gives the
/// plane only (F-31 gives a planet of 50 units, unconfirmed), so this reach is `[ours]`.
const WIN_REACH_M: f32 = 50.0;
/// The reach of `barrier_area` when its marker stores no dimensions, metres (`[ours]`).
const BARRIER_REACH_M: f32 = 30.0;

/// Where the race's car goes on the grid: the spawn and the rolling speed the race set (`start_challenge`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Place {
    pub spawn: Spawn,
    pub speed: f32,
}

/// The final race's run: its rules, its markers and what is still to do.
#[derive(Debug, Clone)]
pub(super) struct FinalRun {
    rules: FinalRace,
    markers: FinalRaceMarkers,
    /// The special car to swap in on the first frame, when the race gives one.
    car: Option<&'static str>,
    place: Place,
    /// The race time, in simulated seconds since the start: it stops while a scene plays.
    clock: f32,
    /// The player's signed distance from the win trigger's plane, last frame.
    last_side: Option<f32>,
    /// The win is settled and its outro plays; the race is won when the outro is over.
    awaiting_win: bool,
}

impl FinalRun {
    fn new(markers: FinalRaceMarkers, facing: Vec3, car: Option<&'static str>, place: Place) -> Self {
        Self {
            rules: FinalRace::new(facing.to_array()),
            markers,
            car,
            place,
            clock: 0.0,
            last_side: None,
            awaiting_win: false,
        }
    }

    /// The special car still to give, with the place it goes to. Once only.
    pub(super) fn take_car(&mut self) -> Option<(&'static str, Place)> {
        let car = self.car.take()?;
        Some((car, self.place))
    }

    /// The rules' input for one frame: the player's physics-space position, forward and speed, and the triggers it is in.
    fn input(&mut self, at: Vec3, forward: Vec3, speed_mps: f32) -> FinalRaceInput {
        FinalRaceInput {
            elapsed_secs: self.clock,
            player_pos: map_point(at),
            player_speed_kmh: speed_mps.abs() * KMH_PER_MPS,
            player_forward: forward.to_array(),
            in_barrier_area: self.in_barrier(at),
            crossed_win_plane: self.crossed_win(at),
            in_lose_box: self.markers.lose_trigger.as_ref().is_some_and(|lose| lose.contains(at)),
        }
    }

    /// Whether the player is in `barrier_area`: within half its largest dimension, or [`BARRIER_REACH_M`] when the marker
    /// stores none.
    fn in_barrier(&self, at: Vec3) -> bool {
        let Some(area) = self.markers.barrier_area.as_ref() else { return false };
        let reach = area
            .dimensions
            .map(|d| 0.5 * d.into_iter().fold(0.0, f32::max))
            .filter(|reach| *reach > 0.0)
            .unwrap_or(BARRIER_REACH_M);
        at.distance(area.placement.position) <= reach
    }

    /// Whether the player crossed the win trigger's plane from behind it to in front, within [`WIN_REACH_M`] of its centre.
    /// The plane's normal is the trigger's facing (physics space). The first frame has no previous side, so no crossing.
    fn crossed_win(&mut self, at: Vec3) -> bool {
        let Some(win) = self.markers.win_trigger.as_ref() else { return false };
        let side = (at - win.position).dot(world_heading(win.heading_deg));
        let before = self.last_side.replace(side);
        let passed = before.is_some_and(|before| before < 0.0 && side >= 0.0);
        passed && at.distance(win.position) <= WIN_REACH_M
    }
}

/// A physics-space point as map coordinates, the inverse of `nfsmw_data::gameplay::to_world` (`world = (-y, z, x)`). The
/// rules measure in map units (`BARRIER_AREA_POS`).
fn map_point(p: Vec3) -> [f32; 3] {
    [p.z, -p.x, p.y]
}

/// Whether `race` is the final race: its career event id is 1.8.1.
pub(super) fn is_final_race(race: &Race) -> bool {
    crate::career::race_event_id(race) == FINAL_RACE_EVENT
}

impl WorldScene {
    /// The final race's run for `race`, from the install's markers. The car is the special one while the career has not
    /// given it (settings bit 20, `story-flow.md` section 2.2; the bit is not set here).
    pub(super) fn final_run_for(&self, race: &Race, place: Place) -> Result<FinalRun, String> {
        let data = self.gameplay.as_ref().ok_or("the race data could not be read from the install")?;
        let markers = data.final_race.clone().ok_or("the install has no markers for the final race")?;
        let win = markers.win_trigger.as_ref().ok_or("the final race has no win trigger")?;
        let facing = world_heading(win.heading_deg);
        let given =
            self.career.handle.with(|career| career.settings.flags.contains(Flags::E3_CAR_GIVEN)).unwrap_or(false);
        let car = (race.story.special_setup && !given).then_some(SPECIAL_CAR);
        Ok(FinalRun::new(markers, facing, car, place))
    }

    /// Swaps the race's special car in, then puts it on the grid where the race set it. A car that cannot be loaded leaves
    /// the player's own car in place, and the race goes on with it.
    pub(super) fn give_final_car(&mut self, renderer: &mut dyn blackbox_gfx::RenderBackend, car: &str, place: Place) {
        let (name, model) = match crate::scenes::world::car_loader::load_named_car(&self.dir, car) {
            Ok(loaded) => loaded,
            Err(why) => {
                log::warn!("final race: the special car {car} is not given: {why:#}");
                return;
            }
        };
        if let Err(why) = self.start_driving(renderer, name, model, None) {
            log::warn!("final race: the special car {car} does not start: {why:#}");
            return;
        }
        if let Some(drive) = self.drive.as_mut() {
            drive.place_exactly(place.spawn, place.speed);
        }
    }

    /// One frame of the final race once the chase runs: the rules run on the car, their events are played and the race
    /// settles. Returns whether the race goes on.
    pub(super) fn final_race_frame(&mut self, session: &mut RaceSession, run: &mut FinalRun, sim_dt: f32) -> bool {
        if run.awaiting_win {
            return self.final_win_done(session, run);
        }
        run.clock += sim_dt;
        let Some(input) = self.final_input(run) else { return true };
        let events = run.rules.update(input);
        let settled = apply_events(&mut WorldFinal { world: self, markers: &run.markers }, &events);
        match settled {
            Settled::Running => {}
            Settled::Won => run.awaiting_win = true,
            Settled::Lost => self.finish_challenge(session, false, run.clock),
        }
        true
    }

    /// The win's outro is over: the race is won and the career hears of it. The end game waits for the results' answer
    /// (`report.rs`).
    fn final_win_done(&mut self, session: &mut RaceSession, run: &FinalRun) -> bool {
        if self.nis.is_playing() {
            return true;
        }
        self.finish_challenge(session, true, run.clock);
        true
    }

    /// The rules' input from the player's car, or `None` while the car is not on the road.
    fn final_input(&self, run: &mut FinalRun) -> Option<FinalRaceInput> {
        let drive = self.drive.as_ref().filter(|d| d.has_sim() && d.waiting_for_road().is_none())?;
        let pose = drive.physics_pose();
        let at = Vec3::from_array(space::to_physics(pose.position));
        let forward = Vec3::from_array(space::to_physics(pose.rotation * Vec3::X));
        Some(run.input(at, forward, drive.speed()))
    }

    /// Places phone call `id`: the phone takes the immediate id, and the call is processed with the others (`career_phone`).
    fn final_call(&mut self, id: u16) {
        let phone = &mut self.career.phone;
        let outcome = self.career.handle.with(|career| phone.add_sms(&mut career.settings.sms, u32::from(id)));
        log::info!("phone: the final race calls {id}: {outcome:?}");
    }

    /// Opens the door group: it is on after a clear and shut at the start, and this switches it off.
    fn open_final_door(&mut self) {
        let Some(door) = self.scenery_groups.named(DOOR_GROUP).map(|group| group.number) else {
            log::warn!("final race: the scenery group {DOOR_GROUP} is not on this track");
            return;
        };
        self.switch_scenery_group(door, None);
        self.refresh_road_barriers();
    }

    /// The anchor a marker's scene stands at: the marker on the ground, facing its heading.
    fn marker_anchor(&mut self, placement: &Placement) -> Anchor {
        let at = space::to_render(placement.position.to_array());
        let forward = space::to_render(world_heading(placement.heading_deg).to_array());
        let (collision, _) = self.residency.world_parts();
        let ground = space::ground_below(collision, at.x, at.y, at.z + 10.0, at.z - 10.0).unwrap_or(at.z);
        Anchor::facing(Vec3::new(at.x, at.y, ground), heading_angle(forward))
    }
}

/// The world's side of the final race's effects for one frame (`final_race_events.rs`).
struct WorldFinal<'a> {
    world: &'a mut WorldScene,
    markers: &'a FinalRaceMarkers,
}

impl FinalEffects for WorldFinal<'_> {
    fn place_call(&mut self, id: u16) {
        self.world.final_call(id);
    }

    fn open_door(&mut self) {
        self.world.open_final_door();
    }

    fn gps_to_barrier(&mut self) {
        let Some(area) = self.markers.barrier_area.as_ref() else {
            log::warn!("final race: there is no barrier_area to route to");
            return;
        };
        let [x, y, _] = space::to_render(area.placement.position.to_array()).to_array();
        if !self.world.route_to(Some([x, y])) {
            log::warn!("final race: the GPS to the barrier area is not set");
        }
    }

    fn hide_gps(&mut self) {
        self.world.route_to(None);
    }

    fn play_outro(&mut self, scene: &str) {
        let played = match self.markers.win_nis_marker.as_ref() {
            Some(marker) => {
                let anchor = self.world.marker_anchor(&marker.placement);
                self.world.play_nis_at(scene, 0, anchor)
            }
            None => self.world.play_nis(scene, 0),
        };
        if let Err(why) = played {
            log::warn!("final race: the win scene {scene} does not play: {why}");
        }
    }

    fn bail_pursuit(&mut self) {
        if let Some(traffic) = self.world.traffic.as_mut() {
            traffic.stop_pursuit();
        }
    }

    fn total_racer(&mut self) {
        log::info!("final race: the racer is totalled");
    }
}
