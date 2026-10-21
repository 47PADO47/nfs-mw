//! The world's events as the machine's notices (`SoundAI::OnCollision` and the message handlers): the speech tuning
//! decides what is hard enough to remark on, the cops' own visibility what they can see.

use blackbox_pursuit::speech::impact::{closing_enough, crashed, intensity};
use blackbox_pursuit::speech::{Notice, Tune};
use glam::Vec3;

use crate::audio::chase::events::{Crash, EventLog, WorldEvent};
use crate::audio::chase::feed::SpeechFeed;

/// A collision the radio takes notice of: it closes fast enough (the original's squared test, see `impact::CLOSING_GATE`).
fn heard(closing: f32) -> bool {
    closing_enough(Vec3::new(closing, 0.0, 0.0))
}

fn hard(crash: &Crash, tune: &Tune) -> f32 {
    intensity(crash.delta_v, tune.smash_range)
}

fn lost_speed(crash: &Crash, tune: &Tune) -> bool {
    crashed(crash.speed_before, crash.speed_after, tune.crash_slowdown)
}

/// The notices for the events of `log` that carry a number above `seen`, which moves up to the newest.
pub fn notices(seen: &mut u32, log: &EventLog, tune: &Tune, speech: &SpeechFeed) -> Vec<Notice> {
    let mut out = Vec::new();
    for event in log.since(*seen) {
        collect(event, tune, speech, &mut out);
    }
    *seen = log.latest();
    out
}

fn collect(event: &WorldEvent, tune: &Tune, speech: &SpeechFeed, out: &mut Vec<Notice>) {
    match *event {
        WorldEvent::PlayerHitWorld(crash) => {
            if !heard(crash.closing) {
                return;
            }
            out.push(Notice::PlayerHitWorld { intensity: hard(&crash, tune), crash: lost_speed(&crash, tune) });
        }
        WorldEvent::PlayerHitProp(crash) => {
            if heard(crash.closing) && lost_speed(&crash, tune) {
                out.push(Notice::Crashed);
            }
        }
        WorldEvent::PlayerHitTraffic { crash, semi } => {
            if !heard(crash.closing) {
                return;
            }
            if lost_speed(&crash, tune) {
                out.push(Notice::Crashed);
            }
            out.push(Notice::PlayerHitTraffic { intensity: hard(&crash, tune), semi });
        }
        WorldEvent::PlayerHitCop { cop, crash, impact, deliberate } => {
            if !heard(crash.closing) {
                return;
            }
            out.push(Notice::PlayerHitCop { cop, impact, intensity: hard(&crash, tune), deliberate });
        }
        WorldEvent::CopHitTraffic { cop, closing } => {
            if heard(closing) {
                out.push(Notice::CopHitTraffic { cop });
            }
        }
        WorldEvent::CopHitWorld { cop, delta_v, closing } => {
            if !heard(closing) {
                return;
            }
            let visible = speech.cops().iter().any(|c| c.id == cop && c.in_view);
            let intensity = intensity(delta_v, tune.smash_range);
            out.push(Notice::CopHitWorld { cop, intensity, visible });
        }
        WorldEvent::Airborne { altitude, secs } => out.push(Notice::Airborne { altitude, secs }),
        WorldEvent::BlewByCop { cop } => out.push(Notice::BlewByCop(cop)),
        WorldEvent::Tunnel { inside } => out.push(Notice::Tunnel(inside)),
        WorldEvent::GasStation { by_player, at } => {
            out.push(Notice::GasStationExploded { by_player, at: Vec3::from_array(at) })
        }
        WorldEvent::RoadblockBuilt { spike_offset } => {
            if let Some(offset) = spike_offset {
                out.push(Notice::RoadblockPosition(offset));
            }
        }
        WorldEvent::RoadblockDodged => out.push(Notice::RoadblockDodged),
        WorldEvent::RoadblockHit(hit) => out.push(Notice::RoadblockHit(hit)),
        WorldEvent::TireBlown => out.push(Notice::TireBlown),
        WorldEvent::BackupRequested(backup) => out.push(Notice::BackupRequested(backup)),
        WorldEvent::BackupDenied(backup) => out.push(Notice::BackupDenied(backup)),
    }
}

#[cfg(test)]
mod tests;
