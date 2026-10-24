//! What the Custom HUD draws of the notifications this frame: the queue's visible notices and the feedback lane's chip,
//! with their text resolved in the current language from the catalogue's Fluent ids and the values their events carry.
//! The HUD plugin puts it in the Custom feed (`ui/theme/custom/feed.rs`) each frame.

use crate::hud::{HudEvent, LABEL_KMH, LABEL_MPH, game_word};
use crate::i18n::{Args, fmt, text};
use crate::settings::SpeedUnits;
use crate::ui::theme::custom::feed::{ChipView, NoticeView, NotificationView};

use super::HudNotifications;
use super::catalog::spec;
use super::feedback::{Chip, ChipKind};

/// Kilometres per hour in a mile per hour.
const KMH_PER_MPH: f32 = 1.609_344;

impl HudNotifications {
    /// The notices and the chip on the screen, the speeds in `units`. `count_up` animates the bounty delta (off with
    /// motion reduction, which shows the sum at once).
    pub fn view(&self, units: SpeedUnits, count_up: bool) -> NotificationView {
        let clock = self.queue.clock();
        let mut notices: Vec<NoticeView> = self
            .queue
            .visible()
            .iter()
            .filter_map(|active| {
                let entry = spec(&active.payload)?;
                let id = entry.text?;
                Some(NoticeView {
                    id: entry.id,
                    priority: entry.priority,
                    category: entry.category,
                    text: text(id, &args(&active.payload, units)),
                    secondary: entry.secondary.map(|id| text(id, &Args::new())),
                    icon: entry.icon,
                    age: (clock - active.started).max(0.0),
                    remaining: (active.expires - clock).max(0.0),
                    count: active.count,
                })
            })
            .collect();
        // The most urgent on top, then the oldest: the age runs the other way from the arrival.
        notices.sort_by(|a, b| a.priority.cmp(&b.priority).then(b.age.total_cmp(&a.age)));
        NotificationView {
            notices,
            chip: self.lane.chip().and_then(|chip| chip_view(chip, self.lane.clock(), units, count_up)),
            nitrous_age: self.lane.nitrous_age(),
        }
    }
}

/// The lane's chip as drawn at the lane's clock `now`.
fn chip_view(chip: &Chip, now: f32, units: SpeedUnits, count_up: bool) -> Option<ChipView> {
    let entry = spec(&chip.event)?;
    let id = entry.text?;
    let args = match chip.kind {
        ChipKind::Bounty => Args::new().with("amount", fmt::grouped(i64::from(chip.bounty_at(now, count_up)))),
        _ => args(&chip.event, units),
    };
    Some(ChipView {
        kind: chip.kind,
        text: text(id, &args),
        icon: entry.icon,
        age: (now - chip.started).max(0.0),
        remaining: (chip.expires - now).max(0.0),
        count: chip.count,
    })
}

/// The values of `event`'s text (the Fluent variables of its catalogue message).
fn args(event: &HudEvent, units: SpeedUnits) -> Args {
    match *event {
        HudEvent::HeatUp { level } => Args::new().with("level", level),
        HudEvent::Checkpoint { number, total } => Args::new().with("number", number).with("total", total),
        HudEvent::Lap { lap, laps } => Args::new().with("lap", lap).with("laps", laps),
        HudEvent::PositionChanged { to, .. } => Args::new().with("place", fmt::ordinal(to)),
        HudEvent::SpeedTrap { kmh, .. } => {
            let (speed, unit) = speed(kmh, units);
            Args::new().with("speed", speed).with("unit", unit)
        }
        HudEvent::BountyMilestone { bounty } => Args::new().with("bounty", fmt::grouped(i64::from(bounty))),
        HudEvent::BountyGained { amount } => Args::new().with("amount", fmt::grouped(i64::from(amount))),
        _ => Args::new(),
    }
}

/// A speed of `kmh` in whole `units`, and the game's word for the unit.
fn speed(kmh: f32, units: SpeedUnits) -> (u32, String) {
    let kmh = match kmh.is_finite() {
        true => kmh.max(0.0),
        false => 0.0,
    };
    match units {
        SpeedUnits::Metric => (kmh.round() as u32, game_word(LABEL_KMH, "KM/H")),
        SpeedUnits::Imperial => ((kmh / KMH_PER_MPH).round() as u32, game_word(LABEL_MPH, "MPH")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_speed_trap_reads_in_the_players_units() {
        assert_eq!(speed(212.4, SpeedUnits::Metric).0, 212);
        assert_eq!(speed(160.934, SpeedUnits::Imperial).0, 100);
        assert_eq!(speed(f32::NAN, SpeedUnits::Metric).0, 0);
    }
}
