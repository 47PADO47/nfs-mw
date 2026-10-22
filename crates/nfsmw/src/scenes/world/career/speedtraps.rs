//! The career's speed traps in free roam (`docs/specs/career-milestones.md` section 3.7,
//! `docs/specs/career-scripts-events.md` section 7.2). A trap is a place of the road: driving into it above the
//! trap's threshold completes it (bounty to the car, a challenge passed in its bin); driving into it slower, or into a
//! trap that is not armed, says "too slow". An armed trap is on the minimap.

use nfsmw_career::economy::cash::award_player_bounty;
use nfsmw_career::milestones::speedtrap::Crossing;
use nfsmw_career::phone::{BinMoment, bin_messages};

use crate::hud::{MapBlip, ModMessageTone, RACE_SPEEDTRAP, set_mod_message};
use crate::scenes::world::players::{LEAD, PlayerId};
use crate::scenes::world::{WorldScene, space};

/// How close the car has to come to a speed trap's place to be inside it when the record states no `Radius`. It does on
/// all 39: 20 m, 25 m on one and 30 m on two (`docs/formats/gameplay-races.md`).
const DEFAULT_RADIUS: f32 = 20.0;
/// Seconds a trap's message stays on the screen.
const MESSAGE_SECS: f32 = 3.0;

/// The traps each player's car is inside, by key: a trap fires when a car enters it, not while it stays.
#[derive(Default)]
pub(in crate::scenes::world) struct TrapWatch {
    /// Per player, the keys of the traps that car was near last frame.
    inside: Vec<(PlayerId, Vec<u32>)>,
}

impl TrapWatch {
    /// The keys among `near` that `who`'s car was not near last frame; remembers `near` for its next frame.
    fn entered(&mut self, who: PlayerId, near: Vec<u32>) -> Vec<u32> {
        let was = self.inside_of(who);
        let entered = near.iter().copied().filter(|key| !was.contains(key)).collect();
        *was = near;
        entered
    }

    /// The keys `who`'s car was near last frame, empty on its first frame.
    fn inside_of(&mut self, who: PlayerId) -> &mut Vec<u32> {
        let at = match self.inside.iter().position(|(id, _)| *id == who) {
            Some(at) => at,
            None => {
                self.inside.push((who, Vec::new()));
                self.inside.len() - 1
            }
        };
        &mut self.inside[at].1
    }

    /// Forgets the cars that are no longer in the world (a guest that left co-op).
    fn keep(&mut self, present: &[PlayerId]) {
        self.inside.retain(|(id, _)| present.contains(id));
    }

    fn clear(&mut self) {
        self.inside.clear();
    }
}

/// The career record a trap's bounty goes to: the car that tripped it. The lead's car is the career's car; a guest's car
/// is its own record. `None` when that car has no record.
fn trap_credit(lead_handle: u32, guest_handle: Option<u32>, who: PlayerId) -> Option<u32> {
    if who == LEAD {
        return Some(lead_handle);
    }
    guest_handle
}

impl WorldScene {
    /// One frame of the speed traps: a trap a car has just driven into is judged at that car's speed.
    pub(super) fn career_speed_traps(&mut self) {
        let Some(data) = self.gameplay.clone() else { return };
        if self.race.is_some() {
            self.career.traps.clear();
            return;
        }
        let present: Vec<PlayerId> = self.players.iter().map(|player| player.id).collect();
        self.career.traps.keep(&present);
        let cars: Vec<(PlayerId, glam::Vec3, f32)> = self
            .players
            .iter()
            .filter(|player| player.drive.has_sim())
            .map(|player| {
                let at = glam::Vec3::from(space::to_physics(player.drive.position()));
                (player.id, at, player.drive.speed().abs() * 3.6)
            })
            .collect();
        for (who, at, kmh) in cars {
            let near: Vec<u32> = data
                .speed_traps
                .iter()
                .filter(|trap| {
                    let radius = trap.radius.unwrap_or(DEFAULT_RADIUS);
                    trap.position.is_some_and(|p| p.with_y(0.0).distance(at.with_y(0.0)) < radius)
                })
                .map(|trap| trap.key)
                .collect();
            for key in self.career.traps.entered(who, near) {
                self.drive_into_trap(who, key, kmh);
            }
        }
    }

    /// The car of `who` drove into the trap `key` at `kmh`.
    fn drive_into_trap(&mut self, who: PlayerId, key: u32, kmh: f32) {
        let guest_handle = self.coop.as_ref().and_then(|coop| coop.guest_handle);
        let phone = &mut self.career.phone;
        let outcome = self.career.handle.with(|career| {
            let trap = career.traps.iter_mut().find(|trap| trap.key() == key)?;
            let bin = trap.bin();
            let crossing = trap.crossing(kmh);
            let Crossing::Passed { bounty, .. } = crossing else { return Some((crossing, 0)) };
            let credit = trap_credit(career.settings.car_handle, guest_handle, who);
            if let Some(record) = credit.and_then(|handle| career.stable.career_for_handle_mut(handle)) {
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
        assert_eq!(watch.entered(LEAD, vec![7]), vec![7]);
        assert!(watch.entered(LEAD, vec![7]).is_empty(), "still inside");
        assert!(watch.entered(LEAD, Vec::new()).is_empty(), "left");
        assert_eq!(watch.entered(LEAD, vec![7, 9]), vec![7, 9], "both entered again");
    }

    #[test]
    fn each_car_watches_its_own_traps() {
        let guest = PlayerId(1);
        let mut watch = TrapWatch::default();
        assert_eq!(watch.entered(LEAD, vec![7]), vec![7]);
        assert_eq!(watch.entered(guest, vec![7]), vec![7], "the guest enters the same trap on its own");
        assert!(watch.entered(LEAD, vec![7]).is_empty());
        assert!(watch.entered(guest, vec![7]).is_empty());
    }

    #[test]
    fn a_car_that_left_the_world_is_forgotten() {
        let guest = PlayerId(1);
        let mut watch = TrapWatch::default();
        watch.entered(guest, vec![7]);
        watch.keep(&[LEAD]);
        assert_eq!(watch.entered(guest, vec![7]), vec![7], "entered again after rejoining");
    }

    #[test]
    fn lead_trap_credits_the_lead_car() {
        assert_eq!(trap_credit(7, Some(9), LEAD), Some(7));
        assert_eq!(trap_credit(7, None, LEAD), Some(7));
    }

    #[test]
    fn guest_trap_credits_the_guest_car() {
        assert_eq!(trap_credit(7, Some(9), PlayerId(1)), Some(9));
    }

    #[test]
    fn guest_without_a_record_credits_nothing() {
        assert_eq!(trap_credit(7, None, PlayerId(1)), None);
    }
}
