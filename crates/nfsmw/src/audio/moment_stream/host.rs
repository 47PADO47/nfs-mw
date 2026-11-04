//! The host side of the shared moment stream: ranks from the install, the breaker markers of the track, and the
//! voices a committed moment stops. Spec: `scenery-effect-audio.md` ("Shared moment stream").
use super::super::{Audio, AudioView, trace::Record};
use super::{Arbiter, Decision, Owner, Rank};
use glam::Vec3;
use std::collections::HashMap;

#[derive(Default)]
pub(in crate::audio) struct Stream {
    pub(in crate::audio) arbiter: Arbiter,
    /// `strmpriority` and `CanInterupt` by moment key; `None` when the install lacks the moment.
    ranks: HashMap<u32, Option<Rank>>,
    markers_loaded: bool,
}

fn players(view: &AudioView) -> Vec<Vec3> {
    view.car_position.is_finite().then_some(view.car_position).into_iter().collect()
}

impl Audio {
    pub(super) fn moment_rank(&mut self, key: u32) -> Option<Rank> {
        if let Some(rank) = self.moments.ranks.get(&key) {
            return *rank;
        }
        let rank = self
            .database()
            .ok()
            .and_then(|db| nfsmw_data::sound::MomentSound::by_key(&db, key))
            .map(|m| Rank { priority: m.priority, can_interrupt: m.can_interrupt });
        self.moments.ranks.insert(key, rank);
        rank
    }

    /// Asks the stream for moment `key` on behalf of `owner`, to play at `point` (`None`: at the listener). On
    /// success the caller starts its voice at the returned point (a released queued moment plays at its marker) and
    /// reports it with [`Audio::moment_started`]; the voice it replaces is already stopped.
    pub(in crate::audio) fn request_moment(
        &mut self,
        owner: Owner,
        key: u32,
        point: Option<Vec3>,
        view: &AudioView,
    ) -> Option<(Rank, Option<Vec3>)> {
        let rank = self.moment_rank(key)?;
        let decision = self.moments.arbiter.receive(key, rank, &players(view), self.nis_playing());
        let current = self.moments.arbiter.current().unwrap_or(0) as i32;
        match decision {
            Decision::Refuse => {
                self.trace.record(Record {
                    event: "moment_refused",
                    class: "MOMENT_STREAM",
                    params: &[key as i32, current],
                    success: Some(false),
                    ..Default::default()
                });
                None
            }
            Decision::Release => Some((rank, self.moments.arbiter.point().or(point))),
            Decision::Start { interrupt } => {
                if let Some(other) = interrupt.filter(|other| *other != owner) {
                    self.stop_moment_owner(other);
                }
                Some((rank, point))
            }
        }
    }

    pub(in crate::audio) fn moment_started(&mut self, owner: Owner, key: u32, rank: Rank, point: Option<Vec3>) {
        self.moments.arbiter.started(owner, key, rank, point);
    }

    /// `owner` stopped its moment on its own.
    pub(in crate::audio) fn moment_stopped(&mut self, owner: Owner) {
        self.moments.arbiter.stopped(owner);
    }

    fn stop_moment_owner(&mut self, owner: Owner) {
        match owner {
            Owner::Petrol => self.stop_explosion_voice(),
            Owner::Breaker => self.stop_breaker_moment_voice(),
            Owner::Thunder => self.stop_thunder_voice(),
            Owner::World => self.world_effects.sweetener.stop(),
        }
    }

    fn moment_owner_playing(&self, owner: Owner) -> bool {
        match owner {
            Owner::Petrol => self.explosion_playing(),
            Owner::Breaker => self.breaker_moment_playing(),
            Owner::Thunder => self.thunder_playing(),
            Owner::World => self.world_effects.sweetener.is_playing(),
        }
    }

    /// One frame of the stream: retrigger clock, finished and distant moments, and the breaker-marker queue.
    pub(in crate::audio) fn update_moment_stream(&mut self, view: AudioView, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.load_moment_markers();
        let playing = self.moments.arbiter.owner().is_some_and(|owner| self.moment_owner_playing(owner));
        let blocked = self.nis_playing();
        let ranks = &self.moments.ranks;
        let tick = self
            .moments
            .arbiter
            .update(dt, &players(&view), playing, blocked, |key| ranks.get(&key).copied().flatten());
        if let Some(owner) = tick.stop {
            self.stop_moment_owner(owner);
        }
        let Some(key) = tick.queued else { return };
        self.trace.record(Record {
            event: "moment_queue",
            class: "MOMENT_STREAM",
            params: &[key as i32],
            success: Some(true),
            ..Default::default()
        });
        // Decode ahead, as the original preloads the queued stream, so the release is immediate.
        let mut voice = std::mem::take(&mut self.world_effects.sweetener);
        voice.prepare(self, key);
        self.world_effects.sweetener = voice;
    }

    pub(super) fn load_moment_markers(&mut self) {
        if self.moments.markers_loaded {
            return;
        }
        self.moments.markers_loaded = true;
        let markers = match nfsmw_data::world::moment::load(&self.dir) {
            Ok(data) => data.queued_moments(),
            Err(error) => {
                log::warn!("breaker moment markers unavailable: {error:#}");
                return;
            }
        };
        for &(_, key) in &markers {
            self.moment_rank(key);
        }
        self.moments.arbiter.set_markers(markers);
    }

    /// The scene changed: nothing plays or waits in the stream.
    pub(in crate::audio) fn stop_moment_stream(&mut self) {
        self.moments.arbiter.clear();
    }

    /// `unpause` at `position`: the moment of the breaker marker it releases, and where that marker is.
    pub(in crate::audio) fn unpause_moment(&self, position: Vec3) -> Option<(u32, Vec3)> {
        self.moments.arbiter.unpause_target(position)
    }
}
