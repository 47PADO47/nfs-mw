//! The career's speed traps in free roam (`docs/specs/career-milestones.md` section 3.7,
//! `docs/specs/career-scripts-events.md` section 7.2). A trap is a place of the road: driving into it above the
//! trap's threshold completes it (bounty to the car, a challenge passed in its bin); driving into it slower, or into a
//! trap that is not armed, says "too slow". An armed trap is on the minimap.

use nfsmw_career::economy::cash::award_player_bounty;
use nfsmw_career::milestones::speedtrap::Crossing;
use nfsmw_career::phone::{BinMoment, bin_messages};

use crate::hud::{MapBlip, ModMessageTone, RACE_SPEEDTRAP, set_mod_message};
use crate::scenes::world::{WorldScene, space};

/// How close the car has to come to a speed trap's place to be inside it when the record states no `Radius`. It does on
/// all 39: 20 m, 25 m on one and 30 m on two (`docs/formats/gameplay-races.md`).
const DEFAULT_RADIUS: f32 = 20.0;
/// Seconds a trap's message stays on the screen.
const MESSAGE_SECS: f32 = 3.0;

/// The traps the car is inside, by key: a trap fires when the car enters it, not while it stays.
#[derive(Default)]
pub(in crate::scenes::world) struct TrapWatch {
    inside: Vec<u32>,
}

impl TrapWatch {
    /// The keys among `near` that were not near last frame; remembers `near` for the next.
    fn entered(&mut self, near: Vec<u32>) -> Vec<u32> {
        let entered = near.iter().copied().filter(|key| !self.inside.contains(key)).collect();
        self.inside = near;
        entered
    }
}

impl WorldScene {
    /// One frame of the speed traps: a trap the car has just driven into is judged at the car's speed.
    pub(super) fn career_speed_traps(&mut self) {
        let Some(data) = self.gameplay.clone() else { return };
        if self.race.is_some() {
            self.career.traps.inside.clear();
            return;
        }
        let Some(drive) = self.drive.as_ref().filter(|d| d.has_sim()) else { return };
        let at = glam::Vec3::from(space::to_physics(drive.position()));
        let kmh = drive.speed().abs() * 3.6;
        let near: Vec<u32> = data
            .speed_traps
            .iter()
            .filter(|trap| {
                let radius = trap.radius.unwrap_or(DEFAULT_RADIUS);
                trap.position.is_some_and(|p| p.with_y(0.0).distance(at.with_y(0.0)) < radius)
            })
            .map(|trap| trap.key)
            .collect();
        for key in self.career.traps.entered(near) {
            self.drive_into_trap(key, kmh);
        }
    }

    /// The car drove into the trap `key` at `kmh`.
    fn drive_into_trap(&mut self, key: u32, kmh: f32) {
        let phone = &mut self.career.phone;
        let outcome = self.career.handle.with(|career| {
            let trap = career.traps.iter_mut().find(|trap| trap.key() == key)?;
            let bin = trap.bin();
            let crossing = trap.crossing(kmh);
            let Crossing::Passed { bounty, .. } = crossing else { return Some((crossing, 0)) };
            let handle = career.settings.car_handle;
            if let Some(record) = career.stable.career_for_handle_mut(handle) {
                award_player_bounty(record, bounty);
            }
            career.blacklist.challenge_passed(bin, &key.to_string());
            let total = u32::try_from(career.stable.total_bounty()).unwrap_or(u32::MAX);
            career.blacklist.bounty_changed(total);
            let count = career.blacklist.bin(bin).map_or(0, |record| record.challenges.len());
            let count = u8::try_from(count).unwrap_or(u8::MAX);
            phone.queue_bin_messages(&mut career.settings.sms, bin_messages(bin, BinMoment::Challenges(count)));
            Some((crossing, bounty))
        });
        match outcome.flatten() {
            Some((Crossing::Passed { speed_kmh, .. }, bounty)) => {
                log::info!("career: speed trap {key:#x} passed at {speed_kmh:.0} km/h, bounty {bounty}");
                self.career.trap_photo = Some(crate::viewer::PhotoReport {
                    kind: crate::viewer::PhotoKind::SpeedTrap,
                    picture: None,
                    cash: 0,
                    time: 0.0,
                    speed: speed_kmh / 3.6,
                    time_left: 0.0,
                    bonus_secs: 0.0,
                    bounty,
                });
                set_mod_message(
                    &format!("Speed trap: {speed_kmh:.0} km/h  +{bounty} bounty"),
                    ModMessageTone::Big,
                    MESSAGE_SECS,
                );
            }
            Some((Crossing::TooSlow, _)) => {
                set_mod_message("Too slow", ModMessageTone::Warning, MESSAGE_SECS);
            }
            None => {}
        }
    }

    /// Appends the minimap icons of the traps that can be completed now (armed and not done).
    pub(in crate::scenes::world) fn speed_trap_blips(&self, out: &mut Vec<MapBlip>) {
        let Some(data) = self.gameplay.as_ref() else { return };
        let active: Vec<u32> = self
            .career
            .handle
            .with(|career| career.traps.iter().filter(|t| t.is_active()).map(|t| t.key()).collect())
            .unwrap_or_default();
        for trap in data.speed_traps.iter().filter(|trap| active.contains(&trap.key)) {
            let Some(position) = trap.position else { continue };
            let at = space::to_render(position.to_array());
            out.push(MapBlip { style: RACE_SPEEDTRAP, position: [at.x, at.y], active: false, heading: None });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trap_fires_when_entered_and_not_while_the_car_stays() {
        let mut watch = TrapWatch::default();
        assert_eq!(watch.entered(vec![7]), vec![7]);
        assert!(watch.entered(vec![7]).is_empty(), "still inside");
        assert!(watch.entered(Vec::new()).is_empty(), "left");
        assert_eq!(watch.entered(vec![7, 9]), vec![7, 9], "both entered again");
    }
}
