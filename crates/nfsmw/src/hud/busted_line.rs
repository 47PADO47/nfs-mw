//! The busted overlay's suspect line: the crawl of `BUSTED_OVERLAY.fng` (`docs/specs/busted-sequence.md` section 6.1).
//! When an arrest scene puts the overlay up, the line takes one of five language strings, picked by the chase's
//! roadblocks, spike strips, cops destroyed and cost to state, and filled with the chase's numbers and its duration.

use crate::i18n::fmt::grouped;

/// The crawl's string object (its sample text reads "SUSPECT: WHITE MALE 185LBS. ...").
pub(super) const LINE: u32 = 0xF1ED_545A;

/// The five language strings **[exe]**, `BUSTED_OVERLAY_CRAWL_TEXT_00` to `_04`: cops damaged and immobilized,
/// roadblocks, cost to state and duration; duration and infractions; duration, cops damaged and immobilized and
/// infractions; duration, spike strips, roadblocks and infractions; duration, cost to state and infractions.
const ROADBLOCKS: u32 = 0x626A_C042;
const SPEEDING: u32 = 0x626A_C043;
const DANGEROUS: u32 = 0x626A_C044;
const CUSTODY: u32 = 0x626A_C045;
const RECKLESS: u32 = 0x626A_C046;
/// Above this cost to state a chase with too few roadblocks and wrecks reads as reckless driving.
const RECKLESS_COST: u32 = 10_000;

/// The chase an arrest closes, as the suspect line reads it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ChaseSummary {
    /// Seconds of the chase.
    pub secs: f32,
    pub cops_damaged: u32,
    pub cops_destroyed: u32,
    pub roadblocks_dodged: u32,
    pub spike_strips_dodged: u32,
    pub cost_to_state: u32,
    /// The different infractions of the chase.
    pub infractions: u32,
}

impl ChaseSummary {
    /// The language string of the line: two roadblocks or more pick the roadblocks line, or the custody line with two
    /// spike strips or more; else two cops destroyed or more the dangerous-driving line; else a cost to state above
    /// 10,000 the reckless-driving line; else the speeding line.
    pub fn label(&self) -> u32 {
        if self.roadblocks_dodged >= 2 {
            return match self.spike_strips_dodged >= 2 {
                true => CUSTODY,
                false => ROADBLOCKS,
            };
        }
        if self.cops_destroyed >= 2 {
            return DANGEROUS;
        }
        if self.cost_to_state > RECKLESS_COST {
            return RECKLESS;
        }
        SPEEDING
    }

    /// The line from the text of its language string: the numbers with the thousands separator (`%$d`) and the
    /// duration (`%s`), in the string's order.
    pub fn text(&self, template: &str) -> String {
        let n = |value: u32| grouped(i64::from(value));
        let duration = duration_text(self.secs);
        let args = match self.label() {
            ROADBLOCKS => vec![
                n(self.cops_damaged),
                n(self.cops_destroyed),
                n(self.roadblocks_dodged),
                n(self.cost_to_state),
                duration,
            ],
            DANGEROUS => vec![duration, n(self.cops_damaged), n(self.cops_destroyed), n(self.infractions)],
            CUSTODY => vec![duration, n(self.spike_strips_dodged), n(self.roadblocks_dodged), n(self.infractions)],
            RECKLESS => vec![duration, n(self.cost_to_state), n(self.infractions)],
            _ => vec![duration, n(self.infractions)],
        };
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        blackbox_text::printf::fill(template, &args)
    }
}

/// The chase's duration as the original's timer prints it with no flags **[exe]**: counted in 1/4000 s, then
/// `h:mm:ss` from an hour, `m:ss.cc` from a minute, `s.cc` from a second and `0.cc` below, the first number padded with a
/// space to two places. The original's other languages swap the separators; the rewrite keeps this one (times keep the
/// original's fixed formats, `i18n/fmt.rs`).
pub fn duration_text(secs: f32) -> String {
    let ticks = (secs.max(0.0) * 4000.0) as u64;
    let (whole, hundredths) = (ticks / 4000, ticks % 4000 / 40);
    let (hours, minutes, seconds) = (whole / 3600, whole / 60 % 60, whole % 60);
    if hours > 0 {
        return format!("{hours:>2}:{minutes:02}:{seconds:02}");
    }
    if minutes > 0 {
        return format!("{minutes:>2}:{seconds:02}.{hundredths:02}");
    }
    if seconds > 0 {
        return format!("{seconds:>2}.{hundredths:02}");
    }
    format!("0.{hundredths:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(roadblocks: u32, spikes: u32, destroyed: u32, cost: u32) -> ChaseSummary {
        ChaseSummary {
            roadblocks_dodged: roadblocks,
            spike_strips_dodged: spikes,
            cops_destroyed: destroyed,
            cost_to_state: cost,
            ..ChaseSummary::default()
        }
    }

    #[test]
    fn the_line_is_picked_by_roadblocks_then_wrecks_then_cost() {
        assert_eq!(summary(0, 0, 0, 0).label(), SPEEDING);
        assert_eq!(summary(2, 1, 5, 50_000).label(), ROADBLOCKS, "two roadblocks win over wrecks and cost");
        assert_eq!(summary(2, 2, 0, 0).label(), CUSTODY, "and two spike strips with them");
        assert_eq!(summary(1, 3, 0, 0).label(), SPEEDING, "spike strips count only with two roadblocks");
        assert_eq!(summary(1, 0, 2, 50_000).label(), DANGEROUS, "two wrecks win over the cost");
        assert_eq!(summary(0, 0, 1, 10_001).label(), RECKLESS);
        assert_eq!(summary(0, 0, 1, 10_000).label(), SPEEDING, "the cost has to be above 10,000");
    }

    #[test]
    fn the_duration_reads_as_the_original_timer() {
        assert_eq!(duration_text(0.0), "0.00");
        assert_eq!(duration_text(0.5), "0.50");
        assert_eq!(duration_text(5.25), " 5.25");
        assert_eq!(duration_text(42.999), "42.99", "cut, not rounded");
        assert_eq!(duration_text(95.5), " 1:35.50");
        assert_eq!(duration_text(725.0), "12:05.00");
        assert_eq!(duration_text(3_930.0), " 1:05:30", "from an hour: no hundredths");
    }

    #[test]
    fn the_numbers_fill_the_string_in_its_own_order() {
        let chase = ChaseSummary { secs: 95.5, infractions: 3, cost_to_state: 25_500, ..summary(0, 0, 1, 25_500) };
        assert_eq!(
            chase.text("RECKLESS - DURATION %s - COST %$d - INFRACTIONS %$d"),
            "RECKLESS - DURATION  1:35.50 - COST 25,500 - INFRACTIONS 3"
        );
        let roadblocks = ChaseSummary { secs: 30.0, cops_damaged: 4, ..summary(3, 0, 2, 1_200) };
        assert_eq!(
            roadblocks.text("DAMAGED %$d - IMMOBILIZED %$d - ROADBLOCKS %$d - COST %$d - DURATION %s"),
            "DAMAGED 4 - IMMOBILIZED 2 - ROADBLOCKS 3 - COST 1,200 - DURATION 30.00"
        );
    }
}
