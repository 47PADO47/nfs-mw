//! Co-op race credit (docs/specs/coop-career.md). [ours]: the original has no co-op career.

/// The finishing place credited to the co-op team: the best (lowest, 1-based) place among the human racers.
/// `humans` holds one entry per human racer: `Some(place)` when that racer finished, `None` when not.
/// Returns `None` when no human finished, so nothing is credited.
pub fn credited_place(humans: &[Option<u8>]) -> Option<u8> {
    humans.iter().flatten().copied().min()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_human_place_is_credited() {
        assert_eq!(credited_place(&[Some(3), None, Some(2)]), Some(2));
        assert_eq!(credited_place(&[Some(5)]), Some(5));
    }

    #[test]
    fn no_finisher_credits_nothing() {
        assert_eq!(credited_place(&[None, None]), None);
        assert_eq!(credited_place(&[]), None);
    }
}
