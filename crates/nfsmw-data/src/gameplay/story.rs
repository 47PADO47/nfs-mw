//! The story fields of a race: the intro and outro cutscenes and movies, the intro message, the quick race cutscene,
//! the car a race forces on the player, and the flags and reward list the career reads when the race is won. Spec:
//! `docs/specs/career-scripts-events.md` sections 1.1 and 2.1; `docs/specs/story-flow.md` sections 2 and 4;
//! `docs/specs/story-flow-2.md` sections 2.2, 4.5 and 5; `docs/formats/gameplay-races.md` section 3.2.

use blackbox_attrib::{CollectionRef, Database, Value};

use super::rewards::{self, RewardRecord};
use super::{child_key, collection, text};

/// Attribute `0x4393F69B` (flag): the race runs the special-setup hook, which forces the E3 demo BMW while settings
/// bit 20 is clear (`story-flow-2.md` F-14). The name is not known, so the key is read.
const SPECIAL_SETUP_FIELD: u32 = 0x4393_F69B;
/// Attribute `0xF2FE50D7` (flag): the race defers its rewards to the safe house markers screen, instead of applying
/// them at once (`story-flow-2.md` F-25). The name is not known, so the key is read.
const DEFER_REWARDS_FIELD: u32 = 0xF2FE_50D7;
/// The child whose presence marks the race's outro (`story-flow.md` section 2.7 and F-12).
const OUTRO_MARKER: &str = "outro_nis_marker";
/// Attribute `0xFF5EE5D6` (flag): the race is a boss race, which a quick race plays with `QuickRaceNIS` (`story-flow.md`
/// section 1.2). The name is not known, so the key is read (`docs/formats/gameplay-races.md`).
const BOSS_RACE_FIELD: u32 = 0xFF5E_E5D6;

/// What a race shows and plays around its run: the intro, the outro, the forced player car, the flags the career
/// reads when the race starts and is won, and its reward list. Empty texts and an unset message id (`-1`) read as
/// `None`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RaceStory {
    /// `IntroMessageID`: the message screen the intro shows first, when set (`career-scripts-events.md` section 1.1).
    pub intro_message_id: Option<i32>,
    /// `IntroMovie`: the movie played before the race; it plays only when no `IntroNIS` is set
    /// (`career-scripts-events.md` section 1.1).
    pub intro_movie: Option<String>,
    /// `IntroNIS`: the intro cutscene played at the start marker (`career-scripts-events.md` section 1.1).
    pub intro_nis: Option<String>,
    /// `IntroCameraTrack`: the camera track the intro cutscene passes to `NIS.Play` (`story-flow-2.md` F-01). 0 when
    /// unset.
    pub intro_camera_track: i32,
    /// `QuickRaceNIS`: the cutscene a boss race plays instead of the intro when it starts from the quick race menu
    /// (`career-scripts-events.md` section 1.1).
    pub quick_race_nis: Option<String>,
    /// `NISShell`: when set, the intro cutscene's completion does not lead to prerace (`story-flow.md` section 1.1,
    /// intronis).
    pub nis_shell: bool,
    /// `DDayRace`: a D-Day (prologue) race. Its tutorial state is skipped (`story-flow.md` section 1.1).
    pub dday: bool,
    /// `BossRace` (attribute `0xFF5EE5D6`, see [`BOSS_RACE_FIELD`]): a boss race. A quick race of one plays
    /// `QuickRaceNIS` and no movie (`story-flow.md` section 1.2).
    pub boss_race: bool,
    /// The race runs the special-setup hook (attribute `0x4393F69B`, see [`SPECIAL_SETUP_FIELD`]).
    pub special_setup: bool,
    /// The race defers its rewards to the safe house markers screen (attribute `0xF2FE50D7`, see
    /// [`DEFER_REWARDS_FIELD`]).
    pub defer_rewards: bool,
    /// `OutroNIS`: the cutscene played after the race, when the race has an outro marker (`career-scripts-events.md`
    /// section 1.1). Only one race of the install sets it.
    pub outro_nis: Option<String>,
    /// Whether the race has an `outro_nis_marker` child. The outro cutscene plays when `outro_nis` is set and this is
    /// too (`story-flow.md` section 1.1, outronis). `16_1_1_circuit` has the marker and no `OutroNIS` (F-12).
    pub has_outro_marker: bool,
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
    /// The reward list (attribute `0x50104D90`): what a first completion grants, in order. Empty for the races with no
    /// rewards; the earlier races of a gauntlet have none (`story-flow-2.md` section 4.5).
    pub rewards: Vec<RewardRecord>,
}

impl RaceStory {
    pub(super) fn read(db: &Database, c: CollectionRef<'_>) -> Self {
        Self {
            intro_message_id: c.get_i32("IntroMessageID").filter(|id| *id != -1),
            intro_movie: text(c, "IntroMovie"),
            intro_nis: text(c, "IntroNIS"),
            intro_camera_track: c.get_i32("IntroCameraTrack").unwrap_or(0),
            quick_race_nis: text(c, "QuickRaceNIS"),
            nis_shell: c.get_bool("NISShell").unwrap_or(false),
            dday: c.get_bool("DDayRace").unwrap_or(false),
            boss_race: flag_at(c, BOSS_RACE_FIELD),
            special_setup: flag_at(c, SPECIAL_SETUP_FIELD),
            defer_rewards: flag_at(c, DEFER_REWARDS_FIELD),
            outro_nis: text(c, "OutroNIS"),
            has_outro_marker: collection(db, child_key(c, OUTRO_MARKER)).is_some(),
            outro_message_id: c.get_i32("OutroMessageID").filter(|id| *id != -1),
            outro_movie: text(c, "OutroMovie"),
            player_car_type: text(c, "PlayerCarType"),
            player_car_performance: c.get_f32("PlayerCarPerformance").filter(|p| *p >= 0.0),
            rewards: rewards::read_list(db, c),
        }
    }
}

/// A boolean attribute read by its key, through the inheritance chain; `false` when unset.
fn flag_at(c: CollectionRef<'_>, field_key: u32) -> bool {
    c.get_by_key(field_key).and_then(Value::as_bool).unwrap_or(false)
}
