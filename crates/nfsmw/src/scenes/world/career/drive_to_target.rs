//! The drive to the first safe house in the world (`docs/specs/story-flow.md` section 2.5). After the car lot the front end
//! starts free roam with the car the player picked, and the rules of `nfsmw_career::story::drive_to_target` run on it:
//! the cops are kept off, the GPS navigates to the safe house, message 15 comes and message 121 after 120 s. Arriving at
//! the safe house respawn marker calls the safe house intro, which is a request to the front end
//! ([`CareerRequest::SafeHouse`], the same one a safe house entry sends).

use glam::Vec3;
use nfsmw_career::story::drive_to_target::{DriveEvent, DriveState, DriveToSafeHouse, skip_career_intro};
use nfsmw_data::gameplay::BoxTrigger;

use crate::career::CareerRequest;
use crate::scenes::world::{WorldScene, space};

/// The safe house's activity zone. Its `respawn` marker is where the drive ends (`DoSafeHouseIntro`, F-17).
const SAFE_HOUSE_ZONE: &str = "menu_gate_safehouse/zone_safehouse_1";
/// The end of the `gpstarget` trigger's name: the child of the drive's activity, `race_bin_16/drive_to_safe_house/gpstarget`.
const GPS_TARGET_SUFFIX: &str = "drive_to_safe_house/gpstarget";
/// How near the respawn marker the car must be to arrive, when the activity's trigger is not in the data (metres).
const ARRIVAL_RADIUS_M: f32 = 30.0;

/// The drive's state in the world: asked for and waiting for the car to stand, or running with its machine.
#[derive(Debug, Default)]
pub(in crate::scenes::world) struct SafeHouseDrive {
    /// The drive was asked for; it starts once the player's car stands on the road.
    pub waiting: bool,
    /// The drive while it runs. `None` before it starts and once it is done.
    pub machine: Option<DriveToSafeHouse>,
    /// The GPS route to the safe house is wanted and not set yet: the car may not stand on a road when the drive starts, so
    /// the route is tried again until it holds.
    pub gps_wanted: bool,
    /// Seconds until the next try at the route.
    pub gps_retry: f32,
}

/// Seconds between the tries at the GPS route.
const GPS_RETRY_SECS: f32 = 1.0;

/// Whether the player is at the safe house: inside the activity's `gpstarget` box when the data has it, else within
/// [`ARRIVAL_RADIUS_M`] of the respawn marker. Positions are in physics space. Pure, so the caller reads the data.
pub fn arrived(at: Vec3, gps_target: Option<&BoxTrigger>, respawn: Option<Vec3>) -> bool {
    match (gps_target, respawn) {
        (Some(zone), _) => zone.contains(at),
        (None, Some(marker)) => at.distance(marker) <= ARRIVAL_RADIUS_M,
        (None, None) => false,
    }
}

impl WorldScene {
    /// Asks for the drive to the first safe house. It starts once the player's car stands in the world, which the front end
    /// sets up after the car lot.
    pub fn start_drive_to_safe_house(&mut self) {
        self.career.safe_house_drive.waiting = true;
    }

    /// One frame of the drive: it starts when asked and the car stands, its timers run and its events are carried out. The
    /// arrival calls the safe house intro, and the drive is then done (`finish` runs with it, a simplification of the
    /// bin's activity ending later).
    pub(super) fn career_drive_to_target(&mut self, dt: f32) {
        if self.career.safe_house_drive.waiting && self.players.has_lead() {
            self.career.safe_house_drive.waiting = false;
            self.begin_safe_house_drive();
        }
        if self.career.safe_house_drive.machine.is_none() {
            return;
        }
        self.retry_gps(dt);
        let arrived = self.at_safe_house();
        let Some(machine) = self.career.safe_house_drive.machine.as_mut() else { return };
        let mut events = machine.update(dt);
        if arrived {
            events.extend(machine.arrive());
            events.extend(machine.finish());
        }
        let done = machine.state() == DriveState::Done;
        if done {
            self.career.safe_house_drive.machine = None;
        }
        self.apply_safe_house_drive(events);
    }

    /// Starts the drive's machine: cops off, the GPS to the safe house, and message 15 unless the career intro is skipped.
    /// The GPS is taken as set to auto (the setting is not read yet).
    fn begin_safe_house_drive(&mut self) {
        let skip = self.career.handle.with(|career| skip_career_intro(false, career.settings.flags)).unwrap_or(false);
        log::info!("drive: the drive to the first safe house starts (career intro skipped: {skip})");
        let (machine, events) = DriveToSafeHouse::start(true, skip);
        self.career.safe_house_drive.machine = Some(machine);
        self.apply_safe_house_drive(events);
    }

    /// Carries out the drive's events, in order.
    fn apply_safe_house_drive(&mut self, events: Vec<DriveEvent>) {
        for event in events {
            match event {
                DriveEvent::CopsOff => self.bar_drive_cops(true),
                DriveEvent::CopsOn => self.bar_drive_cops(false),
                DriveEvent::AddSms(id) => self.deliver_drive_sms(id),
                DriveEvent::GpsNavigate { .. } => {
                    self.career.safe_house_drive.gps_wanted = !self.route_to_safe_house();
                    self.career.safe_house_drive.gps_retry = GPS_RETRY_SECS;
                }
                // The hint at an offset from the target is not drawn yet; the route set at the start stays.
                DriveEvent::GpsShow { .. } => {}
                DriveEvent::GpsOff => {
                    self.career.safe_house_drive.gps_wanted = false;
                    self.route_to(None);
                }
                DriveEvent::SafeHouseIntro { .. } => {
                    log::info!("drive: arrived at the first safe house");
                    self.career.requests.push_back(CareerRequest::SafeHouse { zone: SAFE_HOUSE_ZONE.to_owned() });
                }
            }
        }
    }

    /// Keeps the cops out of the world (`barred`) or lets them back in.
    fn bar_drive_cops(&mut self, barred: bool) {
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.bar_cops(barred);
        }
    }

    /// `EAddSMS(id)`: the message goes to the inbox at once, or is queued when the phone holds it.
    fn deliver_drive_sms(&mut self, id: u32) {
        let phone = &mut self.career.phone;
        let outcome = self.career.handle.with(|career| phone.add_sms(&mut career.settings.sms, id));
        log::info!("drive: message {id} sent: {outcome:?}");
    }

    /// Sets the GPS route to the safe house's respawn marker. `false` when there is no route yet.
    fn route_to_safe_house(&mut self) -> bool {
        let Some(placement) = self.gameplay.as_ref().and_then(|data| data.zone_respawn(SAFE_HOUSE_ZONE)) else {
            log::warn!("drive: {SAFE_HOUSE_ZONE} has no respawn marker, so the GPS has no route");
            return true;
        };
        let marker = space::to_render(placement.position.to_array());
        self.route_to(Some([marker.x, marker.y]))
    }

    /// Tries the GPS route again while it is wanted and not set, once a second.
    fn retry_gps(&mut self, dt: f32) {
        let drive = &mut self.career.safe_house_drive;
        if !drive.gps_wanted {
            return;
        }
        drive.gps_retry -= dt;
        if drive.gps_retry > 0.0 {
            return;
        }
        drive.gps_retry = GPS_RETRY_SECS;
        let set = self.route_to_safe_house();
        self.career.safe_house_drive.gps_wanted = !set;
        if set {
            log::info!("drive: the GPS route to the safe house is set");
        }
    }

    /// Whether the car is at the safe house now (`false` without a car or the data).
    fn at_safe_house(&self) -> bool {
        let (Some(drive), Some(data)) = (self.players.lead(), self.gameplay.as_ref()) else {
            return false;
        };
        let at = Vec3::from(space::to_physics(drive.position()));
        let gps_target = data.triggers.iter().find(|zone| zone.name.ends_with(GPS_TARGET_SUFFIX));
        let respawn = data.zone_respawn(SAFE_HOUSE_ZONE).map(|placement| placement.position);
        arrived(at, gps_target, respawn)
    }
}

#[cfg(test)]
#[path = "drive_to_target_tests.rs"]
mod tests;
