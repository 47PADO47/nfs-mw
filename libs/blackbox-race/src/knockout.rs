//! The lap knockout rule (`docs/specs/race-mode.md` section 7).

/// Whether the racers who have not completed `lap` are knocked out now: when the number of racers that have
/// completed the lap equals `racers - knockouts_per_lap * lap` (`racers` counts the player). The check runs when a
/// racer finishes the lap, so it fires once, at the moment the last allowed racer crosses.
pub fn knockout_due(racers: usize, knockouts_per_lap: u32, lap: u32, completed: usize) -> bool {
    if knockouts_per_lap == 0 {
        return false;
    }
    let allowed = (racers as i64) - i64::from(knockouts_per_lap) * i64::from(lap);
    allowed > 0 && completed as i64 == allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_of_four_is_out_when_three_finish_lap_one() {
        assert!(!knockout_due(4, 1, 1, 2));
        assert!(knockout_due(4, 1, 1, 3));
        assert!(!knockout_due(4, 1, 1, 4));
    }

    #[test]
    fn the_next_lap_drops_one_more() {
        assert!(knockout_due(4, 1, 2, 2));
        assert!(!knockout_due(4, 1, 2, 3));
    }

    #[test]
    fn no_knockouts_per_lap_never_fires() {
        assert!(!knockout_due(4, 0, 1, 3));
    }

    #[test]
    fn the_rule_stops_before_the_field_is_empty() {
        assert!(!knockout_due(2, 1, 2, 0), "nobody is left to allow");
        assert!(!knockout_due(2, 1, 5, 0));
    }
}
