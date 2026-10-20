//! The working context of one step: the inputs, the machine's state and the output, with the small questions the flows
//! ask about them ("is the chase hot", "is the heading valid", "how long since the player stopped").

use super::actors::{Pick, Roster};
use super::config::{Layout, Tune};
use super::input::{Channel, Snapshot};
use super::line::{Line, Speaker};
use super::output::{Effect, Output, Request, Talker};
use super::params::{Direction, Intensity, Params, Suspects};
use super::state::{Core, Focus, PursuitState, Rng};

/// Everything a flow needs in one step.
pub struct Cx<'a> {
    pub tune: &'a Tune,
    pub layout: &'a Layout,
    pub snap: &'a Snapshot,
    pub chan: &'a dyn Channel,
    pub now: f32,
    pub core: &'a mut Core,
    pub roster: &'a mut Roster,
    pub rng: &'a mut Rng,
    pub out: &'a mut Output,
}

impl Cx<'_> {
    // The chase's character.

    /// The chase is hot: the heat is at the tuning's mark or the pursuit is long.
    pub fn high_intensity(&self) -> bool {
        self.snap.pursuit.heat.floor() >= self.tune.high_intensity_heat
            || self.core.duration >= self.tune.duration_high_intensity
    }

    pub fn intensity(&self) -> Intensity {
        match self.high_intensity() {
            true => Intensity::High,
            false => Intensity::Normal,
        }
    }

    pub fn suspects(&self) -> Suspects {
        match self.snap.racers_near {
            true => Suspects::Multiple,
            false => Suspects::One,
        }
    }

    /// The reported heading is usable: a direction held for a while on a road the cops can name.
    pub fn heading_valid(&self) -> bool {
        let now = self.core.current[0];
        now.direction.is_some() && self.layout.road_is_placeable(now.road)
    }

    /// Seconds the player has been (nearly) stopped.
    pub fn player_stop_time(&self) -> f32 {
        self.now - self.core.t_low_speed
    }

    /// Seconds the cops have not seen the player.
    pub fn perp_lost_time(&self) -> f32 {
        self.now - self.core.t_no_los
    }

    /// Seconds a cop has had the player in view.
    pub fn time_in_view(&self) -> f32 {
        self.now - self.core.t_los
    }

    /// Seconds since a cop was rammed by the player.
    pub fn time_last_nailed_cop(&self) -> f32 {
        self.roster.since_last_rammed(self.now)
    }

    pub fn state_is(&self, state: PursuitState) -> bool {
        self.core.state == state
    }

    pub fn focus_is(&self, focus: Focus) -> bool {
        self.core.focus == focus
    }

    // The channel.

    pub fn busy(&self) -> bool {
        self.chan.busy()
    }

    pub fn has_been_said(&self, line: Line) -> bool {
        self.chan.said(line) > 0
    }

    pub fn said_or_playing(&self, line: Line) -> bool {
        self.has_been_said(line) || self.chan.is_playing(line)
    }

    // Random picks.

    pub fn random_active(&mut self, pick: Pick, need_los: bool) -> Option<u32> {
        self.roster.random_active(self.layout, self.rng, pick, need_los)
    }

    pub fn random_cop(&mut self, pick: Pick) -> Option<u32> {
        self.roster.random(self.layout, self.rng, pick)
    }

    // Output.

    pub fn effect(&mut self, effect: Effect) {
        self.out.effects.push(effect);
    }

    /// Stops the line that sounds and drops the queue.
    pub fn clear_queue(&mut self) {
        self.effect(Effect::ClearQueue);
    }

    pub fn emit(&mut self, line: Line, speaker: Speaker, params: Params, talker: Option<Talker>) {
        self.out.requests.push(Request { line, speaker, params, talker });
    }

    /// Dispatch says `line`.
    pub fn say_dispatch(&mut self, line: Line, params: Params) {
        self.emit(line, self.layout.dispatch, params, None);
    }

    /// A line with no voice.
    pub fn say_voiceless(&mut self, line: Line, params: Params) {
        self.emit(line, 0, params, None);
    }

    /// The actor `id` says `line`. Does nothing when there is no such actor.
    pub fn say(&mut self, line: Line, id: u32, params: Params) {
        let Some(actor) = self.roster.find(id) else { return };
        let talker = Talker { distance: actor.distance, los: actor.los, in_view: actor.in_view };
        let voice = actor.voice;
        self.emit(line, voice, params, Some(talker));
    }

    /// A line of a random voice (the original's helpers that pick a speaker number 3 to 8 themselves).
    pub fn say_random_voice(&mut self, line: Line, params: Params) {
        let voices = self.layout.cop_voices();
        let voice = voices[self.rng.below(voices.len())];
        self.emit(line, voice, params, None);
    }

    /// Where the player is heading: the direction held for three seconds and the road, as the dispatch reads them,
    /// falling back from the last known heading to the current ones and finally to a random direction.
    pub fn heading_for_dispatch(&mut self) -> (Option<Direction>, Option<u16>) {
        let known = [self.core.last_known, self.core.current[0], self.core.current[1]];
        let road = known
            .iter()
            .map(|h| h.road)
            .find(|r| *r >= self.layout.locatable_roads.0 && *r <= self.layout.locatable_roads.1);
        let direction = known.iter().find_map(|h| h.direction).or_else(|| Some(random_direction(self.rng)));
        (direction, road)
    }
}

/// A uniformly random direction.
pub fn random_direction(rng: &mut Rng) -> Direction {
    [Direction::North, Direction::South, Direction::East, Direction::West][rng.below(4)]
}
