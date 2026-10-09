//! Whether a request may be said now: the checks of the original's `PostValidate`, in its order.
//! Spec: `docs/specs/speech.md` §3.

use nfsmw_data::speech::SpeechEvent;

use super::history::History;

/// The answer for one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Say it.
    Keep,
    /// Not yet: ask again next frame.
    Defer,
    /// Never: drop the request.
    Ditch,
}

/// The unit that would speak: what the distance and visibility checks look at. A request without one skips them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Actor {
    /// Distance to the player, in meters.
    pub distance: f32,
    /// Whether the unit is on screen (for events that need it).
    pub on_screen: bool,
    /// Whether the unit has a line of sight to the player (for events that need it).
    pub line_of_sight: bool,
}

/// The state of the game the checks look at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Context {
    /// The wanted level of the pursuit (0 outside one).
    pub heat: i32,
    /// The player's speed in miles per hour.
    pub player_speed: f32,
}

impl Default for Context {
    fn default() -> Self {
        Self { heat: 0, player_speed: 0.0 }
    }
}

/// What the checks need to know besides the event.
pub struct Env<'a> {
    pub now: f32,
    /// Seconds since a line last sounded (0 while one does).
    pub dead_air: f32,
    pub history: &'a History,
    /// The event being said right now.
    pub playing: Option<u32>,
    pub context: Context,
}

/// One request, as the checks see it.
pub struct Asked {
    /// Seconds on the clock when the request was made (or last refreshed, or its delay ended).
    pub entry: f32,
    pub actor: Option<Actor>,
}

/// Judge a request of `event`.
pub fn judge(event: &SpeechEvent, asked: &Asked, env: &Env) -> Verdict {
    if env.now - asked.entry > event.expiry {
        return Verdict::Ditch;
    }
    // The original lets an event be said once more than its limit (it compares with `>`).
    if event.max_playback > -1 && env.history.count(event.id) as i64 > i64::from(event.max_playback) {
        return Verdict::Ditch;
    }
    if event.dead_air > 0.0 && env.dead_air < event.dead_air {
        return Verdict::Defer;
    }
    let heard = env.history.last_time(event.id);
    if heard.is_some_and(|t| env.now - t < event.interval) {
        return Verdict::Defer;
    }
    if asked.actor.is_some_and(|a| a.distance > event.culling_range) {
        return Verdict::Defer;
    }
    if !event.dep_follow.is_empty() && !follows(event, env) {
        return Verdict::Defer;
    }
    if event.on_screen_only && asked.actor.is_some_and(|a| !a.on_screen) {
        return Verdict::Defer;
    }
    if event.require_line_of_sight && asked.actor.is_some_and(|a| !a.line_of_sight) {
        return Verdict::Defer;
    }
    let ctx = env.context;
    if ctx.heat > event.max_heat || ctx.heat < event.min_heat {
        return Verdict::Defer;
    }
    let speed = ctx.player_speed.abs();
    if speed < event.min_player_speed || speed > event.max_player_speed {
        return Verdict::Defer;
    }
    Verdict::Keep
}

/// The `DepFollow` check: one of the events it names was said just before, or is being said.
fn follows(event: &SpeechEvent, env: &Env) -> bool {
    let playing = |dep: &u32| env.playing == Some(*dep);
    if event.back_time > 0.0 {
        let recent = |dep: &u32| {
            env.history.count(*dep) > 0 && env.history.last_time(*dep).is_some_and(|t| env.now - t < event.back_time)
        };
        return event.dep_follow.iter().any(|dep| recent(dep) || playing(dep));
    }
    let Some(last) = env.history.last_event() else { return false };
    event.dep_follow.iter().any(|dep| *dep == last || playing(dep))
}
