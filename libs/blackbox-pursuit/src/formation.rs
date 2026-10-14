//! Choosing the cop formation: the timer that says when the next one starts, the alternation between stagger follow and
//! the weighted aggressive formations of the heat level, and the reset when the heat level changes. Spec:
//! `docs/specs/ai-pursuit-formations.md` sections 1 and 2. The formations' slots and their tightening are not here.

pub mod collapse;
pub mod finisher;
pub mod slots;

/// The formations a chase can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formation {
    Pit,
    BoxIn,
    RollingBlock,
    Follow,
    Herd,
    StaggerFollow,
}

/// One weighted record of a heat level: the formation, how long it lasts (seconds) and its weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    pub formation: Formation,
    pub duration: f32,
    pub frequency: f32,
}

/// The records of heat levels 1 to 10 (`Num_CopFormations`, Duration and Frequency; install-verified in the spec).
pub fn records(level: u32) -> &'static [Record] {
    use Formation::*;
    const fn r(formation: Formation, duration: f32, frequency: f32) -> Record {
        Record { formation, duration, frequency }
    }
    const HEAT_1: [Record; 1] = [r(RollingBlock, 40.0, 100.0)];
    const HEAT_2: [Record; 1] = [r(BoxIn, 40.0, 100.0)];
    const HEAT_3: [Record; 4] =
        [r(BoxIn, 40.0, 25.0), r(Pit, 40.0, 60.0), r(RollingBlock, 40.0, 20.0), r(Herd, 40.0, 40.0)];
    const HEAT_4: [Record; 4] =
        [r(Pit, 40.0, 50.0), r(BoxIn, 40.0, 50.0), r(Herd, 40.0, 30.0), r(RollingBlock, 40.0, 50.0)];
    const HEAT_5: [Record; 4] =
        [r(RollingBlock, 40.0, 50.0), r(BoxIn, 40.0, 50.0), r(Pit, 40.0, 50.0), r(Herd, 40.0, 50.0)];
    const HEAT_6: [Record; 3] = [r(RollingBlock, 40.0, 60.0), r(BoxIn, 40.0, 50.0), r(Herd, 40.0, 50.0)];
    const HEAT_7: [Record; 2] = [r(BoxIn, 40.0, 10.0), r(RollingBlock, 40.0, 60.0)];
    const HEAT_8: [Record; 4] =
        [r(BoxIn, 30.0, 60.0), r(RollingBlock, 20.0, 40.0), r(Pit, 30.0, 70.0), r(Herd, 30.0, 50.0)];
    const HEAT_9: [Record; 1] = [r(Pit, 40.0, 100.0)];
    const HEAT_10: [Record; 1] = [r(Follow, 10.0, 100.0)];
    match level.clamp(1, 10) {
        1 => &HEAT_1,
        2 => &HEAT_2,
        3 => &HEAT_3,
        4 => &HEAT_4,
        5 => &HEAT_5,
        6 => &HEAT_6,
        7 => &HEAT_7,
        8 => &HEAT_8,
        9 => &HEAT_9,
        _ => &HEAT_10,
    }
}

/// How long a stagger follow lasts at a heat level (`StaggerFormationTime`). The spec gives heats 1 to 7 and 9; heat 8
/// and heat 10 are not given there, so they take 4 s **[unconfirmed]**.
pub fn stagger_time(level: u32) -> f32 {
    const TIMES: [f32; 10] = [7.0, 6.0, 5.0, 4.0, 3.0, 5.0, 4.0, 4.0, 7.0, 4.0];
    TIMES[(level as usize).clamp(1, 10) - 1]
}

/// What the picker decided this tick: the formation to run and for how long.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Choice {
    pub formation: Formation,
    pub duration: f32,
}

/// The formation timer and the attempt count of one chase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picker {
    timer: f32,
    attempts: u32,
}

impl Default for Picker {
    fn default() -> Self {
        Self::new()
    }
}

impl Picker {
    pub fn new() -> Self {
        Self { timer: 0.0, attempts: 0 }
    }

    /// Heat changed level: the next formation is chosen now (`formation_time = 0`).
    pub fn heat_changed(&mut self) {
        self.timer = 0.0;
    }

    /// Advances the timer by `dt` seconds and, when it runs out and `allowed` holds, chooses the next formation.
    /// `allowed` is false while a finisher runs, the player is busted or bailed, or the cops' speech is still going.
    /// `follow_forced` is true with a roadblock standing that has not been dodged. `random` holds two uniform
    /// numbers in `0..=1`: the first picks the weighted record, the second the random part of the next timer.
    pub fn tick(
        &mut self,
        dt: f32,
        level: u32,
        allowed: bool,
        follow_forced: bool,
        random: [f32; 2],
    ) -> Option<Choice> {
        self.timer -= dt;
        if self.timer > 0.0 || !allowed {
            return None;
        }
        let stagger = stagger_time(level);
        let (formation, duration) = if follow_forced {
            (Formation::Follow, stagger)
        } else if self.attempts % 2 == 1 {
            let record = pick(records(level), random[0]);
            (record.formation, record.duration)
        } else {
            (Formation::StaggerFollow, stagger)
        };
        self.attempts += 1;
        self.timer = random[1].clamp(0.0, 1.0) * 3.0 + duration;
        Some(Choice { formation, duration })
    }

    /// Attempts made so far in this chase.
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

/// The record picked with probability proportional to its frequency; `unit` in `0..=1`.
fn pick(records: &[Record], unit: f32) -> Record {
    let total: f32 = records.iter().map(|r| r.frequency).sum();
    let mut at = unit.clamp(0.0, 1.0) * total;
    for record in records {
        if at < record.frequency {
            return *record;
        }
        at -= record.frequency;
    }
    records[records.len() - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_attempt_is_stagger_follow_then_the_weighted_one() {
        let mut picker = Picker::new();
        let first = picker.tick(0.0, 3, true, false, [0.0, 0.0]).expect("first");
        assert_eq!(first.formation, Formation::StaggerFollow);
        assert_eq!(first.duration, stagger_time(3), "5 s at heat 3");
        // The next timer is the stagger time plus a random 0 to 3 s; with a random of 0 it is exactly 5 s.
        assert!(picker.tick(4.9, 3, true, false, [0.0, 0.0]).is_none(), "still stagger");
        let second = picker.tick(0.2, 3, true, false, [0.99, 0.0]).expect("second");
        assert_eq!(second.formation, Formation::Herd, "the last record of heat 3 is picked at 0.99");
        assert_eq!(second.duration, 40.0);
    }

    #[test]
    fn records_are_weighted_by_frequency() {
        // Heat 3 weights are 25, 60, 20, 40 (total 145): 0.1 is box-in, 0.5 is pit, 0.6 is rolling block.
        let heat3 = records(3);
        assert_eq!(pick(heat3, 0.1).formation, Formation::BoxIn);
        assert_eq!(pick(heat3, 0.5).formation, Formation::Pit);
        assert_eq!(pick(heat3, 0.6).formation, Formation::RollingBlock);
    }

    #[test]
    fn nothing_starts_while_it_is_not_allowed() {
        let mut picker = Picker::new();
        assert!(picker.tick(1.0, 1, false, false, [0.5, 0.5]).is_none());
        assert_eq!(picker.attempts(), 0);
    }

    #[test]
    fn a_standing_roadblock_forces_follow() {
        let mut picker = Picker::new();
        let choice = picker.tick(0.0, 5, true, true, [0.5, 0.0]).expect("chosen");
        assert_eq!(choice.formation, Formation::Follow);
    }

    #[test]
    fn a_heat_change_chooses_at_once() {
        let mut picker = Picker::new();
        picker.tick(0.0, 2, true, false, [0.0, 100.0]);
        assert!(picker.tick(1.0, 2, true, false, [0.0, 0.0]).is_none(), "the timer is still running");
        picker.heat_changed();
        assert!(picker.tick(0.0, 2, true, false, [0.0, 0.0]).is_some());
    }

    #[test]
    fn the_stagger_times_match_the_spec_where_it_gives_them() {
        assert_eq!(stagger_time(1), 7.0);
        assert_eq!(stagger_time(7), 4.0);
        assert_eq!(stagger_time(9), 7.0);
    }
}
