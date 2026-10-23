//! The buffers of every remote entity and the display time.

use std::collections::BTreeMap;

use nfsmw_sim::{EntityId, EntityView, Tick};

use super::track::Track;
use super::{AdaptiveDelay, Drawn, InterpConfig};

/// The client's view of every remote entity. Feed it each snapshot's records with [`Interpolator::push`] (the full
/// record of every entity the snapshot holds, baseline included), removals with [`Interpolator::remove`], and the
/// snapshot's age with [`Interpolator::on_snapshot`]; each frame call [`Interpolator::advance`], then sample at
/// [`Interpolator::display_tick`].
#[derive(Debug, Clone)]
pub struct Interpolator {
    config: InterpConfig,
    tracks: BTreeMap<EntityId, Track>,
    delay: AdaptiveDelay,
    refused: u64,
}

impl Default for Interpolator {
    fn default() -> Self {
        Self::new(InterpConfig::default())
    }
}

impl Interpolator {
    pub fn new(config: InterpConfig) -> Self {
        Self { config, tracks: BTreeMap::new(), delay: AdaptiveDelay::new(&config), refused: 0 }
    }

    pub fn config(&self) -> &InterpConfig {
        &self.config
    }

    pub fn delay(&self) -> &AdaptiveDelay {
        &self.delay
    }

    /// Store `view` as the entity's record at server tick `tick`. A new entity beyond
    /// [`InterpConfig::max_entities`] is refused and counted.
    pub fn push(&mut self, tick: Tick, view: &EntityView) {
        let cap = self.config.buffer_len.max(2);
        if let Some(track) = self.tracks.get_mut(&view.id) {
            track.push(tick, *view, cap);
            return;
        }
        if self.tracks.len() >= self.config.max_entities {
            self.refused += 1;
            return;
        }
        self.tracks.insert(view.id, Track::new(tick, *view));
    }

    /// The entity was removed at server tick `tick`: it is drawn until the display time reaches it.
    pub fn remove(&mut self, id: EntityId, tick: Tick) {
        if let Some(track) = self.tracks.get_mut(&id) {
            track.remove_at(tick);
        }
    }

    /// A snapshot of `tick` arrived when the estimated server tick was `server_tick`.
    pub fn on_snapshot(&mut self, tick: Tick, server_tick: f64) {
        self.delay.on_snapshot(server_tick - f64::from(tick));
    }

    /// Let `dt_ms` of frame time pass (the delay slews).
    pub fn advance(&mut self, dt_ms: f64) {
        self.delay.advance(dt_ms);
    }

    /// The display tick `R` for the estimated server tick `server_tick`.
    pub fn display_tick(&self, server_tick: f64) -> f64 {
        server_tick - self.delay.delay_ticks()
    }

    /// The drawn record of one entity at display tick `r`.
    pub fn sample(&mut self, id: EntityId, r: f64) -> Option<Drawn> {
        let config = self.config;
        self.tracks.get_mut(&id)?.sample(r, &config)
    }

    /// Every entity drawn at display tick `r`, in id order, appended to `out`.
    pub fn sample_all(&mut self, r: f64, out: &mut Vec<(EntityId, Drawn)>) {
        let config = self.config;
        for (id, track) in self.tracks.iter_mut() {
            if let Some(drawn) = track.sample(r, &config) {
                out.push((*id, drawn));
            }
        }
    }

    /// Forget the entities that are gone at display tick `r` (removed, or silent past the forget time). Call once a
    /// frame after sampling.
    pub fn prune(&mut self, r: f64) {
        let config = self.config;
        self.tracks.retain(|_, track| !track.is_gone(r, &config));
    }

    /// Entities tracked.
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Records held across every entity.
    pub fn records(&self) -> usize {
        self.tracks.values().map(Track::len).sum()
    }

    /// New entities refused because the table was full.
    pub fn refused(&self) -> u64 {
        self.refused
    }

    /// The newest record tick of an entity.
    pub fn newest_tick(&self, id: EntityId) -> Option<Tick> {
        self.tracks.get(&id)?.newest_tick()
    }
}
