//! The story fields of a race: the intro and outro cutscenes and movies, the intro message, the quick race cutscene
//! and the car a race forces on the player. Spec: `docs/specs/career-scripts-events.md` sections 1.1 and 2.1;
//! `docs/formats/gameplay-races.md` section 3.2.

use blackbox_attrib::CollectionRef;

use super::text;

/// What a race shows and plays around its run: the intro, the outro and the forced player car. Empty texts and an
/// unset message id (`-1`) read as `None`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RaceStory {
    /// `IntroMessageID`: the message screen the intro shows first, when set (`career-scripts-events.md` section 1.1).
    pub intro_message_id: Option<i32>,
    /// `IntroMovie`: the movie played before the race; it plays only when no `IntroNIS` is set
    /// (`career-scripts-events.md` section 1.1).
    pub intro_movie: Option<String>,
    /// `IntroNIS`: the intro cutscene played at the start marker (`career-scripts-events.md` section 1.1).
    pub intro_nis: Option<String>,
    /// `QuickRaceNIS`: the cutscene a boss race plays instead of the intro when it starts from the quick race menu
    /// (`career-scripts-events.md` section 1.1).
    pub quick_race_nis: Option<String>,
    /// `OutroNIS`: the cutscene played after the race, when the race has an outro marker (`career-scripts-events.md`
    /// section 1.1). Only one race of the install sets it.
    pub outro_nis: Option<String>,
    /// `OutroMessageID`: the outro message screen, shown after the outro cutscene when set (`career-scripts-events.md`
    /// section 1.1). The field exists in the schema; no race of the install sets it.
    pub outro_message_id: Option<i32>,
    /// `OutroMovie`: the movie played last, before the race is done (`career-scripts-events.md` section 1.1).
    pub outro_movie: Option<String>,
    /// `PlayerCarType`: the car the race forces on the player, for example `M3GTRCAREERSTART` in the prologue
    /// (`gameplay-races.md` section 3.2; `career-scripts-events.md` section 2.1).
    pub player_car_type: Option<String>,
    /// `PlayerCarPerformance`: the performance of the forced car, 0..1 (`gameplay-races.md` section 3.2). A
    /// negative value reads as `None`.
    pub player_car_performance: Option<f32>,
}

impl RaceStory {
    pub(super) fn read(c: CollectionRef<'_>) -> Self {
        Self {
            intro_message_id: c.get_i32("IntroMessageID").filter(|id| *id != -1),
            intro_movie: text(c, "IntroMovie"),
            intro_nis: text(c, "IntroNIS"),
            quick_race_nis: text(c, "QuickRaceNIS"),
            outro_nis: text(c, "OutroNIS"),
            outro_message_id: c.get_i32("OutroMessageID").filter(|id| *id != -1),
            outro_movie: text(c, "OutroMovie"),
            player_car_type: text(c, "PlayerCarType"),
            player_car_performance: c.get_f32("PlayerCarPerformance").filter(|p| *p >= 0.0),
        }
    }
}
