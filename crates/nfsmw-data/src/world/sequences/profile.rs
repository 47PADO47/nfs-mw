//! What an installed event engine does for a pursuit: the zones it opens for the cops, the cinematic moment it plays
//! and how far its area stimuli reach. Spec: `docs/specs/pursuit-breakers.md`.

use std::collections::BTreeSet;

use blackbox_sequence::Event;

use super::{Sequence, WorldEvent};

/// The pursuit-relevant events of one engine, read from every action it can run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BreakerProfile {
    /// `(radius m, duration s)` of each `EBreakerStopCops` event.
    pub stops: Vec<(f32, f32)>,
    /// The scene names of the cinematic moments (`EPlayRaceNIS` of type "Moment") the engine plays.
    pub moments: Vec<String>,
    /// The largest radius (metres) of an area stimulus the engine sends: how far a trigger part reaches.
    pub reach: f32,
    /// How many area stimuli the engine sends. A part that sends some is a trigger for the models near it.
    pub area_events: usize,
}

impl BreakerProfile {
    /// Whether the engine opens a cop zone itself: a breaker proper (a roof, a tower, a boat...).
    pub fn is_breaker(&self) -> bool {
        !self.stops.is_empty()
    }

    /// Whether the engine only passes the collapse on to other models (a support).
    pub fn is_trigger(&self) -> bool {
        self.stops.is_empty() && self.area_events > 0
    }

    /// The largest zone the engine opens: `(radius, duration)`.
    pub fn largest_stop(&self) -> Option<(f32, f32)> {
        self.stops.iter().copied().max_by(|a, b| a.0.total_cmp(&b.0))
    }
}

impl Sequence {
    /// Every event of every action the engine can run, once per action: the timed lists and the start, end and stop
    /// tags.
    pub fn events(&self) -> impl Iterator<Item = &Event> + '_ {
        let package = &*self.package;
        let engine = &package.engines[self.engine];
        let mut seen = BTreeSet::new();
        let actions: Vec<u32> = engine
            .systems
            .values()
            .flat_map(|system| system.states.values())
            .flat_map(|state| state.responses.values())
            .map(|response| response.action)
            .filter(|action| seen.insert(*action))
            .collect();
        actions
            .into_iter()
            .filter_map(move |action| package.actions.get(&action))
            .flat_map(|action| action.times.iter().map(|(_, list)| list).chain(action.tags.values()))
            .filter_map(move |list| package.event_lists.get(list))
            .flatten()
    }

    /// The zones, moments and reach of the engine.
    pub fn breaker_profile(&self) -> BreakerProfile {
        let mut profile = BreakerProfile::default();
        for event in self.events() {
            let Ok(decoded) = WorldEvent::decode(event) else { continue };
            match decoded {
                WorldEvent::StopCops { radius, duration } => profile.stops.push((radius, duration)),
                WorldEvent::AreaStimulus { radius, .. } => {
                    profile.reach = profile.reach.max(radius);
                    profile.area_events += 1;
                }
                WorldEvent::PlayNis { scene, kind } => {
                    let name = |hash: u32| self.package.strings.get(&hash).cloned();
                    if name(kind).as_deref() == Some("Moment")
                        && let Some(scene) = name(scene)
                    {
                        profile.moments.push(scene);
                    }
                }
                _ => {}
            }
        }
        profile
    }
}
