//! The speech actors: one per cop car that talks, with a voice, a battalion and a call sign, and the pools the voices and
//! call signs are drawn from. The data side only; the flows that use it are in [`sync`](super::sync).

use super::config::Layout;
use super::line::Speaker;
use super::params::Battalion;
use super::state::Rng;

/// Which cops a random pick may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Any,
    Primary,
    Secondary,
}

/// What kind of voice a new actor takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceKind {
    /// Any cop voice but the cross car's.
    Any,
    Secondary,
    Cross,
}

/// A cop (or the helicopter) as the speech knows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Actor {
    pub id: u32,
    pub voice: Speaker,
    pub battalion: Battalion,
    pub call_sign: u8,
    pub heli: bool,
    pub active: bool,
    pub los: bool,
    pub in_view: bool,
    pub ahead: bool,
    pub in_formation: bool,
    pub in_position: bool,
    pub offset: (f32, f32),
    pub distance: f32,
    pub position: glam::Vec3,
    pub speed: f32,
    pub health: f32,
    pub destroyed: bool,
    pub roadblock: bool,
    /// Fuel left, for the helicopter.
    pub fuel_secs: Option<f32>,
    pub traffic_hits: u32,
    pub times_rammed: u32,
    pub last_rammed: f32,
    /// The last time the cop was not falling behind (the distance did not grow).
    pub last_closing: f32,
    pub last_activity: f32,
    /// The helicopter has said it must leave.
    pub bail_said: bool,
}

impl Actor {
    pub fn new(id: u32, voice: Speaker, battalion: Battalion, call_sign: u8, now: f32) -> Self {
        Self {
            id,
            voice,
            battalion,
            call_sign,
            heli: false,
            active: false,
            los: false,
            in_view: false,
            ahead: false,
            in_formation: false,
            in_position: false,
            offset: (0.0, 0.0),
            distance: 0.0,
            position: glam::Vec3::ZERO,
            speed: 0.0,
            health: 1.0,
            destroyed: false,
            roadblock: false,
            fuel_secs: None,
            traffic_hits: 0,
            times_rammed: 0,
            last_rammed: f32::NEG_INFINITY,
            last_closing: now,
            last_activity: f32::NEG_INFINITY,
            bail_said: false,
        }
    }

    /// Seconds since the cop was last rammed.
    pub fn since_rammed(&self, now: f32) -> f32 {
        now - self.last_rammed
    }
}

/// The actors and the pools of voices and call signs.
#[derive(Debug, Clone, Default)]
pub struct Roster {
    pub actors: Vec<Actor>,
    voices: Vec<Speaker>,
    call_signs: Vec<(Battalion, Vec<u8>)>,
    /// The cop that speaks for the pursuit.
    pub leader: Option<u32>,
    /// The cop that joined last.
    pub latest: Option<u32>,
    /// The cops in a formation (the helicopter never is).
    pub formation: Vec<u32>,
    pub last_in_formation: Option<u32>,
}

impl Roster {
    pub fn new(layout: &Layout, rng: &mut Rng) -> Self {
        let mut voices = layout.cop_voices();
        rng.shuffle(&mut voices);
        Self { voices, ..Self::default() }
    }

    pub fn find(&self, id: u32) -> Option<&Actor> {
        self.actors.iter().find(|a| a.id == id)
    }

    pub fn find_mut(&mut self, id: u32) -> Option<&mut Actor> {
        self.actors.iter_mut().find(|a| a.id == id)
    }

    pub fn by_voice(&self, voice: Speaker) -> Option<&Actor> {
        self.actors.iter().find(|a| a.voice == voice)
    }

    pub fn is_primary(&self, layout: &Layout, id: u32) -> bool {
        self.find(id).is_some_and(|a| layout.is_primary(a.voice))
    }

    pub fn voices_free(&self) -> usize {
        self.voices.len()
    }

    /// A voice of `kind` from the free pool, taken out of it. `None` when there is none.
    pub fn take_voice(&mut self, layout: &Layout, kind: VoiceKind) -> Option<Speaker> {
        let wanted = |v: Speaker| match kind {
            VoiceKind::Any => v != layout.cross,
            VoiceKind::Secondary => layout.is_secondary(v),
            VoiceKind::Cross => v == layout.cross,
        };
        let at = self.voices.iter().position(|&v| wanted(v))?;
        Some(self.voices.remove(at))
    }

    /// Takes `voice` out of the free pool if it is there.
    pub fn claim_voice(&mut self, voice: Speaker) {
        self.voices.retain(|&v| v != voice);
    }

    pub fn free_voice(&mut self, voice: Speaker) {
        if !self.voices.contains(&voice) {
            self.voices.push(voice);
        }
    }

    /// The next call sign number of `battalion`'s pool, refilling (and shuffling) the pool when it is empty. 0 for a
    /// battalion with no pool.
    pub fn take_call_sign(&mut self, layout: &Layout, rng: &mut Rng, battalion: Battalion) -> u8 {
        let Some(size) = layout.pool_size(battalion) else { return 0 };
        let at = match self.call_signs.iter().position(|(b, _)| *b == battalion) {
            Some(at) => at,
            None => {
                self.call_signs.push((battalion, Vec::new()));
                self.call_signs.len() - 1
            }
        };
        let pool = &mut self.call_signs[at].1;
        if pool.is_empty() {
            *pool = (1..=size).collect();
            rng.shuffle(pool);
        }
        pool.remove(0)
    }

    /// The cop that is closest to the player, optionally only among those with a line of sight, optionally with the
    /// helicopter.
    pub fn closest(&self, enforce_los: bool, include_heli: bool) -> Option<u32> {
        let eligible = |a: &&Actor| (!enforce_los || a.los) && (include_heli || !a.heli);
        self.actors.iter().filter(eligible).min_by(|a, b| a.distance.total_cmp(&b.distance)).map(|a| a.id)
    }

    /// The cop that is farthest from the player.
    pub fn furthest(&self, include_heli: bool) -> Option<u32> {
        let eligible = |a: &&Actor| include_heli || !a.heli;
        self.actors.iter().filter(eligible).max_by(|a, b| a.distance.total_cmp(&b.distance)).map(|a| a.id)
    }

    fn pick_from(&self, layout: &Layout, pool: Vec<u32>, pick: Pick, rng: &mut Rng) -> Option<u32> {
        let fits = |id: &u32| match pick {
            Pick::Any => true,
            Pick::Primary => self.is_primary(layout, *id),
            Pick::Secondary => !self.is_primary(layout, *id),
        };
        let pool: Vec<u32> = pool.into_iter().filter(fits).collect();
        if pool.is_empty() {
            return None;
        }
        Some(pool[rng.below(pool.len())])
    }

    /// A random active cop (with a line of sight if asked) of the kind asked for.
    pub fn random_active(&self, layout: &Layout, rng: &mut Rng, pick: Pick, need_los: bool) -> Option<u32> {
        let pool = self.actors.iter().filter(|a| a.active && (!need_los || a.los)).map(|a| a.id).collect();
        self.pick_from(layout, pool, pick, rng)
    }

    /// A random cop of the kind asked for, active or not.
    pub fn random(&self, layout: &Layout, rng: &mut Rng, pick: Pick) -> Option<u32> {
        self.pick_from(layout, self.actors.iter().map(|a| a.id).collect(), pick, rng)
    }

    /// The active cops with a line of sight to the player.
    pub fn with_los(&self) -> Vec<u32> {
        self.actors.iter().filter(|a| a.active && a.los).map(|a| a.id).collect()
    }

    pub fn active(&self) -> Vec<u32> {
        self.actors.iter().filter(|a| a.active).map(|a| a.id).collect()
    }

    /// Removes an actor, returning its voice to the pool and clearing every reference to it.
    pub fn remove(&mut self, id: u32) -> Option<Actor> {
        let at = self.actors.iter().position(|a| a.id == id)?;
        let gone = self.actors.remove(at);
        self.free_voice(gone.voice);
        self.formation.retain(|&f| f != id);
        for slot in [&mut self.leader, &mut self.latest, &mut self.last_in_formation] {
            if *slot == Some(id) {
                *slot = None;
            }
        }
        Some(gone)
    }

    /// Swaps the voices, battalions and call signs of two actors.
    pub fn swap_identity(&mut self, a: u32, b: u32) {
        let (Some(first), Some(second)) = (self.find(a).copied(), self.find(b).copied()) else { return };
        if let Some(actor) = self.find_mut(a) {
            (actor.voice, actor.battalion, actor.call_sign) = (second.voice, second.battalion, second.call_sign);
        }
        if let Some(actor) = self.find_mut(b) {
            (actor.voice, actor.battalion, actor.call_sign) = (first.voice, first.battalion, first.call_sign);
        }
    }

    /// Moves the actor `old` to the car `new` (the voice and call sign stay with the actor).
    pub fn reassign(&mut self, old: u32, new: u32) {
        if let Some(actor) = self.find_mut(old) {
            actor.id = new;
        }
        let swap = |slot: &mut Option<u32>| {
            if *slot == Some(old) {
                *slot = Some(new);
            }
        };
        swap(&mut self.leader);
        swap(&mut self.latest);
        swap(&mut self.last_in_formation);
        for member in &mut self.formation {
            if *member == old {
                *member = new;
            }
        }
    }

    /// Drops every actor (a reset): all voices free, all call sign pools full again.
    pub fn clear(&mut self) {
        let voices: Vec<Speaker> = self.actors.iter().map(|a| a.voice).collect();
        for voice in voices {
            self.free_voice(voice);
        }
        self.actors.clear();
        self.formation.clear();
        self.leader = None;
        self.latest = None;
        self.last_in_formation = None;
    }

    /// Seconds since a cop was rammed last (a large number when none was).
    pub fn since_last_rammed(&self, now: f32) -> f32 {
        let rammed = self.actors.iter().filter(|a| a.times_rammed > 0).map(|a| a.since_rammed(now));
        rammed.fold(65535.0, f32::min)
    }
}
