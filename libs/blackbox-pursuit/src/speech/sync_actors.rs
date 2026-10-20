//! Keeping the speech actors in step with the cars: the actor of each cop, its voice, the leader, the formation list and
//! the lines a cop says when it joins or leaves the chase (`SyncCarsToActors`, `ShuffleActors`, `MakeLeader` of the
//! original).

use super::actors::{Actor, Pick, VoiceKind};
use super::cx::Cx;
use super::input::{CopInfo, HeliInfo};
use super::params::Battalion;
use super::state::{Focus, PursuitState};

/// A cop is "closing" while its distance does not grow.
impl Cx<'_> {
    /// Brings the actors in line with the snapshot's cars: counts, sight, new cops, cops that left.
    pub fn sync_cars(&mut self) {
        self.sync_racers();
        let snapshot = self.snap;
        let mut fresh: Vec<&CopInfo> = Vec::new();
        let (mut in_view, mut with_los, mut ahead, mut active) = (0usize, 0usize, 0usize, 0usize);
        let mut nearest = 65535.0f32;
        for cop in &snapshot.cops {
            if cop.in_view {
                in_view += 1;
            }
            if cop.active && cop.los {
                with_los += 1;
            }
            if cop.active {
                active += 1;
                nearest = nearest.min(cop.distance);
                ahead += usize::from(cop.ahead);
            }
            if self.roster.find(cop.id).is_none() {
                fresh.push(cop);
                continue;
            }
            self.update_actor(cop);
        }
        if let Some(heli) = snapshot.heli {
            self.sync_heli(&heli);
        }
        self.core.cops_in_view = in_view;
        self.core.los_count = with_los;
        self.core.pursuit_dist = nearest;
        self.core.active_cops = active;
        self.core.flags.cops_ahead = active == ahead;
        for cop in fresh {
            self.add_new_cop(cop);
        }
        // A car that is no longer listed has gone from the world (unspawned): its actor goes with it, and its voice is free.
        let gone: Vec<u32> = self
            .roster
            .actors
            .iter()
            .filter(|a| !a.heli)
            .map(|a| a.id)
            .filter(|id| !snapshot.cops.iter().any(|c| c.id == *id))
            .collect();
        for id in gone {
            self.remove_cop(id);
        }
        let heli_gone = snapshot.heli.is_none();
        let stale: Vec<u32> = self.roster.actors.iter().filter(|a| a.heli && heli_gone).map(|a| a.id).collect();
        for id in stale {
            self.remove_cop(id);
        }
    }

    fn sync_racers(&mut self) {
        let near = self.snap.racers_near;
        let was_near = self.core.flags.racers_near;
        self.core.flags.racers_near = near;
        if !was_near || near {
            return;
        }
        if let Some(cop) = self.random_active(Pick::Any, false) {
            self.focus_change(cop);
        }
    }

    /// One cop's state, updated from the car. The activity change comes first (it may say a line from the old state).
    fn update_actor(&mut self, cop: &CopInfo) {
        let was_active = self.roster.find(cop.id).is_some_and(|a| a.active);
        if cop.active != was_active {
            self.set_active(cop.id, cop.active);
        }
        let now = self.now;
        let player_speed = self.snap.player.speed_mph;
        let blowby_interval = self.tune.blowby_interval;
        let Some(actor) = self.roster.find_mut(cop.id) else { return };
        actor.los = cop.active && cop.los;
        actor.ahead = cop.ahead;
        actor.in_view = cop.in_view;
        actor.roadblock = cop.roadblock;
        actor.in_formation = cop.active && cop.in_formation;
        actor.in_position = cop.active && cop.in_position;
        actor.offset = match cop.active {
            true => cop.offset,
            false => (0.0, 0.0),
        };
        if cop.active {
            if actor.los && cop.distance <= actor.distance {
                actor.last_closing = now;
            }
            actor.distance = cop.distance;
            actor.position = cop.position;
            actor.speed = cop.speed_mph;
        }
        // The cop's damage is read whether it is active or not.
        actor.health = cop.health;
        actor.destroyed = cop.destroyed;
        let actor = *actor;
        let blowby = &mut self.core.blowby;
        if now - blowby.at > blowby_interval && player_speed > actor.speed && !actor.ahead {
            *blowby = super::state::Blowby { distance: actor.distance, speed: player_speed - actor.speed, at: now };
        }
        let faster = player_speed - actor.speed;
        if actor.distance < blowby.distance && faster > 0.0 && !actor.ahead && faster > player_speed * 0.75 {
            *blowby = super::state::Blowby { distance: actor.distance, speed: faster, at: now };
        }
    }

    fn sync_heli(&mut self, heli: &HeliInfo) {
        if self.roster.find(heli.id).is_none() {
            self.add_new_heli(heli);
        }
        let was_active = self.roster.find(heli.id).is_some_and(|a| a.active);
        if heli.active != was_active {
            self.set_active(heli.id, heli.active);
        }
        let Some(actor) = self.roster.find_mut(heli.id) else { return };
        actor.distance = heli.distance;
        actor.health = heli.health;
        actor.fuel_secs = heli.fuel_secs;
        actor.in_view = heli.active;
        if heli.active {
            actor.los = heli.los;
        }
        let (id, fuel, bailed) = (actor.id, actor.fuel_secs, actor.bail_said);
        let leaving = self.focus_is(Focus::Pursuit) || !heli.active || bailed;
        if !leaving && fuel.is_some_and(|f| f < HELI_FUEL_CRITICAL_SECS) {
            self.heli_intent_to_bail(id);
        }
    }

    fn add_new_heli(&mut self, heli: &HeliInfo) {
        let (battalion, call_sign) = self.layout.heli_unit;
        let mut actor = Actor::new(heli.id, self.layout.heli, battalion, call_sign, self.now);
        actor.heli = true;
        self.roster.actors.push(actor);
        let focus = self.core.focus;
        if focus == Focus::Pursuit {
            self.core.flags.heli_intro_requested = true;
            return;
        }
        if focus != Focus::Terminal {
            self.backup_arrives(heli.id);
        }
    }

    /// The battalion a new cop calls itself: its model's, its road's area, or a random one.
    fn battalion_for(&mut self, cop: &CopInfo) -> Battalion {
        if let Some(battalion) = cop.vehicle_battalion.or(cop.road_battalion) {
            return battalion;
        }
        let pool = self.layout.random_battalions;
        pool[self.rng.below(pool.len())]
    }

    /// A cop with no actor yet. A closer cop takes over the identity of the farthest actor that is farther than it
    /// (the car gets that actor's voice); else it takes a voice from the pool. The cross car only takes its own voice.
    fn add_new_cop(&mut self, cop: &CopInfo) {
        let battalion = self.battalion_for(cop);
        let take_over = self.roster.voices_free() == 0 || !cop.cross;
        let existing = take_over
            .then(|| {
                self.roster.actors.iter().find(|a| !a.heli && !cop.roadblock && cop.distance < a.distance).map(|a| a.id)
            })
            .flatten();
        let id = match existing {
            Some(old) => {
                self.roster.reassign(old, cop.id);
                let call_sign = self.roster.take_call_sign(self.layout, self.rng, battalion);
                if let Some(actor) = self.roster.find_mut(cop.id) {
                    actor.battalion = battalion;
                    actor.call_sign = call_sign;
                }
                cop.id
            }
            None => {
                let kind = match (cop.cross, cop.roadblock) {
                    (true, _) => VoiceKind::Cross,
                    (false, true) => VoiceKind::Secondary,
                    _ => VoiceKind::Any,
                };
                if self.roster.voices_free() == 0 && !cop.cross {
                    return;
                }
                let Some(voice) = self.roster.take_voice(self.layout, kind) else { return };
                let call_sign = self.roster.take_call_sign(self.layout, self.rng, battalion);
                self.roster.actors.push(Actor::new(cop.id, voice, battalion, call_sign, self.now));
                cop.id
            }
        };
        self.update_actor(cop);
        self.roster.latest = Some(id);
        let searching = matches!(self.core.state, PursuitState::Active | PursuitState::Searching);
        if searching && !cop.roadblock && self.focus_is(Focus::Strategy) {
            match self.rng.coin() {
                true => self.backup_arrives(id),
                false => self.unit_backup_reply(id),
            }
        }
    }

    /// The actor of a car that has gone: its voice returns to the pool.
    pub fn remove_cop(&mut self, id: u32) {
        let Some(gone) = self.roster.remove(id) else { return };
        if gone.heli {}
    }

    /// A cop becomes active or inactive, and may say a line for it.
    pub fn set_active(&mut self, id: u32, active: bool) {
        let Some(before) = self.roster.find(id).copied() else { return };
        if before.active == active {
            return;
        }
        if let Some(actor) = self.roster.find_mut(id) {
            actor.active = active;
        }
        match active {
            true => self.on_activated(before),
            false => self.on_deactivated(before),
        }
    }

    fn on_activated(&mut self, actor: Actor) {
        let focus = self.core.focus;
        if focus != Focus::Lost && focus != Focus::Strategy {
            return;
        }
        let fresh = self.snap.cops.iter().find(|c| c.id == actor.id);
        let sees = fresh.map_or(actor.los, |c| c.los);
        if sees {
            match self.rng.coin() {
                true => self.spotted(actor.id),
                false => self.regain_visual(actor.id),
            }
            return;
        }
        let in_roadblock = fresh.is_some_and(|c| c.roadblock);
        if focus != Focus::Strategy
            || self.core.state == PursuitState::Inactive
            || self.snap.roadblock.is_none()
            || in_roadblock
        {
            return;
        }
        match self.rng.coin() {
            true => self.unit_backup_reply(actor.id),
            false => self.backup_arrives(actor.id),
        }
    }

    fn on_deactivated(&mut self, actor: Actor) {
        if self.now - actor.last_activity < 2.0 {
            return;
        }
        if !actor.in_formation && !actor.in_position {
            return;
        }
        if let Some(a) = self.roster.find_mut(actor.id) {
            a.last_activity = self.now;
        }
        let engaged = matches!(self.core.state, PursuitState::Active | PursuitState::Searching);
        if actor.traffic_hits >= 2 {
            self.bailout_traffic(actor.id);
            if let Some(a) = self.roster.find_mut(actor.id) {
                a.traffic_hits = 0;
            }
            if engaged {
                self.random_bailout_deny(actor.id);
            }
            return;
        }
        if actor.heli {
            self.heli_bailout(actor.id);
            return;
        }
        if actor.destroyed {
            match self.roster.closest(true, true) {
                Some(other) => {
                    let different = self.roster.find(other).is_some_and(|o| o.voice != actor.voice);
                    self.unit_disabled(other, different);
                }
                None => self.unit_disabled(actor.id, false),
            }
            return;
        }
        if actor.health < self.tune.min_health_commentary {
            self.bailout_high(actor.id);
            return;
        }
        self.bailout_low(actor.id);
        if engaged {
            self.random_bailout_deny(actor.id);
        }
    }

    /// Another primary cop objects when a cop in formation bails out.
    pub fn random_bailout_deny(&mut self, wimp: u32) {
        let Some(wimp) = self.roster.find(wimp).copied() else { return };
        if !wimp.in_formation || self.roster.actors.len() <= 1 || self.core.state == PursuitState::Inactive {
            return;
        }
        let deniers: Vec<u32> = self
            .roster
            .actors
            .iter()
            .filter(|a| self.layout.is_primary(a.voice) && a.voice != wimp.voice)
            .map(|a| a.id)
            .collect();
        for id in deniers {
            self.deny_bailout(id);
        }
    }

    /// The cops in formation, and the one that was alone in it last.
    pub fn sync_formations(&mut self) {
        let members: Vec<(u32, bool)> = self.roster.actors.iter().map(|a| (a.id, a.in_formation && !a.heli)).collect();
        for (id, member) in members {
            let listed = self.roster.formation.contains(&id);
            match (member, listed) {
                (true, false) => self.roster.formation.push(id),
                (false, true) => self.roster.formation.retain(|&f| f != id),
                _ => {}
            }
        }
        if self.roster.formation.len() == 1 {
            self.roster.last_in_formation = self.roster.formation.first().copied();
        }
    }

    /// Two actors never share a voice; the leader is the first primary with a line of sight when there is none.
    pub fn shuffle_actors(&mut self) {
        if self.roster.actors.is_empty() {
            return;
        }
        let (active, inactive): (Vec<Actor>, Vec<Actor>) = self.roster.actors.iter().copied().partition(|a| a.active);
        for idle in &inactive {
            if active.iter().any(|a| a.voice == idle.voice) {
                self.remove_cop(idle.id);
            }
        }
        self.dedupe_voices();
        let active: Vec<Actor> = self.roster.actors.iter().copied().filter(|a| a.active).collect();
        if active.is_empty() {
            return;
        }
        let leader = self.roster.leader.and_then(|id| self.roster.find(id)).copied();
        let needs_leader = leader.is_none_or(|l| !l.active || l.heli);
        if needs_leader {
            let pick = active.iter().find(|a| self.layout.is_primary(a.voice) && a.los).map(|a| a.id);
            if let Some(id) = pick {
                self.roster.leader = Some(id);
                let heli = self.is_heli(id);
                if self.focus_is(Focus::Strategy) && !heli {
                    self.primary_engage(id);
                }
            }
        }
        let leader = self.roster.leader.and_then(|id| self.roster.find(id)).copied();
        if let Some(l) = leader.filter(|l| !l.active) {
            self.make_leader(l.id);
        }
    }

    /// When two active cars hold one voice, the farther one loses its actor.
    fn dedupe_voices(&mut self) {
        loop {
            let active: Vec<Actor> = self.roster.actors.iter().copied().filter(|a| a.active).collect();
            let clash = active.iter().enumerate().find_map(|(i, a)| {
                active.iter().skip(i + 1).find(|b| b.voice == a.voice && b.id != a.id).map(|b| (*a, *b))
            });
            let Some((a, b)) = clash else { return };
            let loser = if a.distance < b.distance { b.id } else { a.id };
            self.remove_cop(loser);
        }
    }

    /// Makes `id` the voice of the pursuit: a primary voice is taken, from the pool or by swapping with a primary.
    pub fn make_leader(&mut self, id: u32) -> bool {
        let Some(newcomer) = self.roster.find(id).copied() else { return false };
        let leader = self.roster.leader.and_then(|l| self.roster.find(l)).copied();
        if leader.is_some_and(|l| l.voice == newcomer.voice) {
            return true;
        }
        if newcomer.active && self.layout.is_primary(newcomer.voice) {
            self.set_leader(id);
            return true;
        }
        let holder = self
            .roster
            .actors
            .iter()
            .find(|a| !a.heli && (self.layout.primary.contains(&a.voice) || a.voice == self.layout.cross))
            .map(|a| a.id);
        let Some(holder) = holder else {
            let voice = self.layout.primary[self.rng.below(self.layout.primary.len())];
            if let Some(actor) = self.roster.find_mut(id) {
                actor.voice = voice;
            }
            self.roster.claim_voice(voice);
            self.set_leader(id);
            return true;
        };
        let holder_voice = self.roster.find(holder).map(|a| a.voice);
        if let (Some(theirs), Some(actor)) = (holder_voice, self.roster.find_mut(id)) {
            let mine = std::mem::replace(&mut actor.voice, theirs);
            if let Some(other) = self.roster.find_mut(holder) {
                other.voice = mine;
            }
        }
        self.set_leader(id);
        true
    }

    fn set_leader(&mut self, id: u32) {
        if self.roster.leader == Some(id) {
            return;
        }
        self.roster.leader = Some(id);
        if self.focus_is(Focus::Strategy) {
            self.primary_engage(id);
        }
    }
}

/// Seconds of fuel at which the helicopter says it must leave.
const HELI_FUEL_CRITICAL_SECS: f32 = 8.0;
