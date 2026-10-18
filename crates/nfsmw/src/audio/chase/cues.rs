//! The mid-chase cues: edges of the chase as the sound reads it each frame. A cue fires once per edge, so the
//! speech is asked once where the chase changes, not every frame it stays changed. Pure: no sound, no game.
//!
//! Sight is read from the feed's `intensity`, which the world sets to exactly 1 while the cops see the player and
//! below 1 once the unseen progress has begun (see `chase_intensity` in the world's police code). So "seen" is
//! `intensity >= 1`. The one blur: an unseen spell that has not yet progressed reads as seen for that frame.
//!
//! Every edge of the feed's air-support state (rain, roadblock, been hit, cop down) is a rising edge: the first frame
//! of a chase only records the state, so a chase that opens already raining or already hit says nothing.

use blackbox_pursuit::EndReason;

use crate::audio::PursuitFeed;
use crate::audio::speech::random::Rng;

/// Seconds without sight before the cops call the visual lost: the original's `NoLOSCommentaryTime` (2 s).
pub const NO_CONTACT_SECS: f32 = 2.0;
/// Seconds into a chase before a heat jump may be said: the original's "after 5 s of chase".
pub const HEAT_JUMP_AFTER_SECS: f32 = 5.0;
/// The heat levels whose crossing is a heat jump (`heatjump` says the radio's reaction to them).
const JUMP_LEVELS: [u32; 4] = [2, 3, 4, 5];
/// The chance that a heat jump is said at all: the original's coin flip.
const HEAT_JUMP_CHANCE: f32 = 0.5;
/// The rain, 0 (dry) to 1 (heavy), at which the weather is reported. **[guess]**: the feed does not say where the
/// original sets this; half the scale is the middle of a rain that a line would mention.
pub const RAIN_THRESHOLD: f32 = 0.5;

/// A change of the chase that the speech answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// The cops have not seen the player for `NO_CONTACT_SECS`.
    Lost,
    /// The cops see the player again after a `Lost`.
    Regained,
    /// The heat rose across one of `JUMP_LEVELS`.
    HeatJump,
    /// The search cooled down while the cops still had no sight: the feed's `secs_to_evade` ran out in a lost spell.
    /// Once per lost spell (the original's "time expired" once the cool-down is complete).
    TimeExpired,
    /// The rain rose to `RAIN_THRESHOLD` during the chase.
    Rain,
    /// A roadblock of the chase appeared (the feed's `roadblock_active` rose).
    RoadblockCalled,
    /// The player hit traffic (the feed's `player_hit` rose).
    BeenHit,
    /// The helicopter lost the player: a `Lost` while the helicopter is in the chase.
    HeliLost,
    /// The helicopter sees the player again: a `Regained` while the helicopter is in the chase.
    HeliSpotted,
    /// A cop car was wrecked: the feed's `cops_destroyed` went up.
    CopDown,
    /// The chase ended evaded: the suspect got away (`lostsuspect`). Said once per end, not from the feed.
    Escaped,
}

/// The rising edge of a flag. `None` before the chase's first frame, which only records the flag.
#[derive(Default)]
struct Rising {
    before: Option<bool>,
}

impl Rising {
    /// Records `now` and tells whether the flag rose since the last frame.
    fn rose(&mut self, now: bool) -> bool {
        let before = self.before.replace(now);
        matches!(before, Some(false)) && now
    }

    fn reset(&mut self) {
        self.before = None;
    }
}

/// The state of the chase the edges are found from. Kept across frames by the audio.
pub struct Cues {
    /// The heat of the last frame of the chase.
    heat: Option<u32>,
    /// The chase time when the sight was lost, while it stays lost.
    unseen_since: Option<f32>,
    /// A `Lost` was said and no `Regained` has been since.
    lost_said: bool,
    /// A `TimeExpired` was said in this lost spell.
    expired_said: bool,
    rain: Rising,
    roadblock: Rising,
    hit: Rising,
    /// The cops wrecked on the last frame of the chase.
    cops_down: Option<u32>,
    /// The escape's line was said for the chase that just ended.
    escape_said: bool,
    rng: Rng,
}

impl Cues {
    pub fn new(rng: Rng) -> Self {
        Self {
            heat: None,
            unseen_since: None,
            lost_said: false,
            expired_said: false,
            rain: Rising::default(),
            roadblock: Rising::default(),
            hit: Rising::default(),
            cops_down: None,
            escape_said: false,
            rng,
        }
    }

    /// The escape's line, once per chase that ends evaded: the suspect got away. While a chase runs it re-arms, so the
    /// next end is said too.
    pub fn escape(&mut self, chase_running: bool, ended: EndReason) -> Option<Cue> {
        if chase_running {
            self.escape_said = false;
            return None;
        }
        if ended != EndReason::Evaded || self.escape_said {
            return None;
        }
        self.escape_said = true;
        Some(Cue::Escaped)
    }

    /// Once per frame: the cues of this frame. `None` (no chase) forgets the chase, so the next one starts fresh.
    pub fn step(&mut self, chase: Option<&PursuitFeed>) -> Vec<Cue> {
        let Some(chase) = chase else {
            self.forget();
            return Vec::new();
        };
        let mut cues = Vec::new();
        let sight = self.sight(chase);
        cues.extend(sight);
        cues.extend(sight.and_then(|cue| heli_sight(cue, chase)));
        cues.extend(self.time_expired(chase));
        cues.extend(self.heat_jump(chase));
        cues.extend(self.air_edges(chase));
        cues
    }

    fn forget(&mut self) {
        self.heat = None;
        self.unseen_since = None;
        self.lost_said = false;
        self.expired_said = false;
        self.rain.reset();
        self.roadblock.reset();
        self.hit.reset();
        self.cops_down = None;
    }

    /// The search's end: once a lost spell has run its evade time out (`secs_to_evade` reaches 0), said once.
    fn time_expired(&mut self, chase: &PursuitFeed) -> Option<Cue> {
        if !self.lost_said || self.expired_said || chase.secs_to_evade > 0.0 {
            return None;
        }
        self.expired_said = true;
        Some(Cue::TimeExpired)
    }

    /// The sight edges: `Lost` once the player has been unseen for `NO_CONTACT_SECS`, `Regained` when seen again
    /// after that. A short unseen spell says nothing.
    fn sight(&mut self, chase: &PursuitFeed) -> Option<Cue> {
        if chase.intensity >= 1.0 {
            self.unseen_since = None;
            return self.regained();
        }
        let since = *self.unseen_since.get_or_insert(chase.secs);
        if self.lost_said || chase.secs - since < NO_CONTACT_SECS {
            return None;
        }
        self.lost_said = true;
        Some(Cue::Lost)
    }

    fn regained(&mut self) -> Option<Cue> {
        if !self.lost_said {
            return None;
        }
        self.lost_said = false;
        self.expired_said = false;
        Some(Cue::Regained)
    }

    /// The heat edge: the heat rising across one of `JUMP_LEVELS`, after `HEAT_JUMP_AFTER_SECS` of chase, with the
    /// coin flip. The first frame of a chase only records the heat.
    fn heat_jump(&mut self, chase: &PursuitFeed) -> Option<Cue> {
        let before = self.heat.replace(chase.heat)?;
        let crossed = JUMP_LEVELS.iter().any(|&level| before < level && level <= chase.heat);
        if !crossed || chase.secs < HEAT_JUMP_AFTER_SECS {
            return None;
        }
        if self.rng.next_f32() >= HEAT_JUMP_CHANCE {
            return None;
        }
        Some(Cue::HeatJump)
    }

    /// The edges of the air-support state: the rain, a roadblock, a hit on traffic and a cop wrecked. Each frame all
    /// of them record their state, so none misses an edge because another fired first.
    fn air_edges(&mut self, chase: &PursuitFeed) -> Vec<Cue> {
        let edges = [
            (self.rain.rose(chase.rain_intensity >= RAIN_THRESHOLD), Cue::Rain),
            (self.roadblock.rose(chase.roadblock_active), Cue::RoadblockCalled),
            (self.hit.rose(chase.player_hit), Cue::BeenHit),
            (self.cop_down(chase.cops_destroyed), Cue::CopDown),
        ];
        edges.into_iter().filter(|(rose, _)| *rose).map(|(_, cue)| cue).collect()
    }

    /// The cop-down edge: the count of wrecked cops went up since the last frame. The first frame only records it.
    fn cop_down(&mut self, now: u32) -> bool {
        let before = self.cops_down.replace(now);
        matches!(before, Some(b) if now > b)
    }
}

/// The helicopter's line for a sight edge: while the helicopter is in the chase, the cops' losing the player is also
/// the helicopter's (`HeliLost`), and their regaining it is the helicopter's spotting (`HeliSpotted`). No helicopter,
/// no line.
fn heli_sight(cue: Cue, chase: &PursuitFeed) -> Option<Cue> {
    if !chase.heli_present {
        return None;
    }
    match cue {
        Cue::Lost => Some(Cue::HeliLost),
        Cue::Regained => Some(Cue::HeliSpotted),
        _ => None,
    }
}
