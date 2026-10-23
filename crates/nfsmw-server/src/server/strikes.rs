//! The suspicion score of `security-gameplay.md` section 5.2: each violation adds a weight, the score decays by 1 per
//! 10 s, a score of [`WARN_SCORE`] is logged, [`RESYNC_SCORE`] resends a full baseline and [`KICK_SCORE`] kicks.

/// Logged and noted.
pub const WARN_SCORE: u32 = 10;
/// The next snapshot goes out without a baseline.
pub const RESYNC_SCORE: u32 = 30;
/// The player is kicked for violations.
pub const KICK_SCORE: u32 = 60;
/// Milliseconds per point of decay.
const DECAY_MS: u64 = 10_000;

/// What a violation is worth. Small for what lag causes, large for what only a modified client sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Violation {
    /// Ten input frames dropped over the input rate.
    InputFlood,
    /// Inputs more than 12 ticks ahead of the server.
    EarlyInput,
    /// A reliable message over the message rate.
    MessageFlood,
    /// A message that does not decode, or a frame that does not.
    Malformed,
    /// A message the sender may not send now (a lobby command before joining, a second `Join`).
    OutOfOrder,
    /// A command the sender's rank does not allow (a spoofed host command).
    Spoof,
}

impl Violation {
    /// The weight of the violation.
    pub fn weight(self) -> u32 {
        match self {
            Self::InputFlood => 1,
            Self::EarlyInput => 1,
            Self::MessageFlood => 2,
            Self::Malformed => 5,
            Self::OutOfOrder => 5,
            Self::Spoof => 10,
        }
    }
}

/// What the score asks the server to do after a violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    Nothing,
    Warn,
    Resync,
    Kick,
}

/// One connection's score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Suspicion {
    score: u32,
    since_ms: u64,
}

impl Suspicion {
    /// The score at `now_ms`, after decay.
    pub fn score(&mut self, now_ms: u64) -> u32 {
        let points = now_ms.saturating_sub(self.since_ms) / DECAY_MS;
        if points > 0 {
            self.score = self.score.saturating_sub(u32::try_from(points).unwrap_or(u32::MAX));
            self.since_ms += points * DECAY_MS;
        }
        self.score
    }

    /// Adds `violation` at `now_ms` and says what to do: the highest threshold the score crossed by this violation.
    pub fn add(&mut self, violation: Violation, now_ms: u64) -> Verdict {
        let before = self.score(now_ms);
        if before == 0 {
            self.since_ms = now_ms;
        }
        let after = before.saturating_add(violation.weight());
        self.score = after;
        let crossed = |threshold: u32| before < threshold && after >= threshold;
        if after >= KICK_SCORE {
            return Verdict::Kick;
        }
        if crossed(RESYNC_SCORE) {
            return Verdict::Resync;
        }
        if crossed(WARN_SCORE) {
            return Verdict::Warn;
        }
        Verdict::Nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_score_warns_resyncs_and_kicks() {
        let mut score = Suspicion::default();
        let verdicts: Vec<Verdict> = (0..12).map(|_| score.add(Violation::Malformed, 0)).collect();
        assert_eq!(verdicts[1], Verdict::Warn, "10 points");
        assert_eq!(verdicts[5], Verdict::Resync, "30 points");
        assert_eq!(verdicts[11], Verdict::Kick, "60 points");
    }

    #[test]
    fn the_score_decays_one_point_per_ten_seconds() {
        let mut score = Suspicion::default();
        score.add(Violation::Spoof, 1_000);
        assert_eq!(score.score(1_000), 10);
        assert_eq!(score.score(31_000), 7);
        assert_eq!(score.score(200_000), 0);
    }
}
