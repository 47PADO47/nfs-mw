//! The race's shortcuts: each shortcut marker of the race gives the route's nearest leg a road through the marker,
//! kept when it is at most 1.25 times the leg (`ai/race_route.rs`). Each racer rolls once per shortcut on the grid, with
//! the chance its skill gives, and drives its own route with the shortcuts it rolled for. The rolls come from the
//! race and the racer, so every run and every machine of a network game draws the same.
//! Spec: `docs/specs/ai-racers-route.md` section 2.

use std::sync::Arc;

use blackbox_roads::{Goal, LegPath, RandomSource, Route, SplitMix};
use nfsmw_data::gameplay::{ShortcutMarker, world_heading};

use crate::scenes::world::ai::TrafficWorld;

/// A shortcut a racer may take: the legs it replaces (the same leg of every lap) and their road through the marker.
#[derive(Debug, Clone)]
pub(in crate::scenes::world) struct Shortcut {
    pub marker: ShortcutMarker,
    pub legs: Vec<usize>,
    pub path: LegPath,
}

/// The shortcuts of `markers` on `route`. A marker with no road through it, or one too long, is left out.
pub(super) fn shortcuts_of(traffic: &TrafficWorld, route: &Route, markers: &[ShortcutMarker]) -> Vec<Shortcut> {
    let mut found = Vec::new();
    for marker in markers {
        let via = Goal { position: marker.position, forward: world_heading(marker.heading_deg) };
        let Some((leg, path)) = traffic.route_through(route, via) else { continue };
        let (from, to) = (route.legs[leg].from, route.legs[leg].to);
        let legs = route.legs.iter().enumerate().filter(|(_, l)| l.from == from && l.to == to).map(|(i, _)| i);
        found.push(Shortcut { marker: *marker, legs: legs.collect(), path });
    }
    log::info!("race: {} of {} shortcut(s) have a road", found.len(), markers.len());
    found
}

/// Which of `shortcuts` a racer of `skill` takes, drawn from `seed`: one roll per shortcut, in order.
pub(super) fn rolls(shortcuts: &[Shortcut], skill: f32, seed: u64) -> Vec<bool> {
    let mut rng = SplitMix(seed);
    shortcuts.iter().map(|s| rng.next_f32() < s.marker.chance(skill)).collect()
}

/// The seed of racer `id`'s rolls in the race whose key is `race`.
pub(super) fn seed_of(race: u32, id: usize) -> u64 {
    (u64::from(race) << 16) ^ id as u64 ^ 0x5A17_C0DE
}

/// The route of a racer that rolled `taken`: `base`, with the legs of each shortcut it takes replaced.
pub(super) fn racer_route(
    traffic: &TrafficWorld,
    base: &Arc<Route>,
    shortcuts: &[Shortcut],
    taken: &[bool],
) -> Arc<Route> {
    let replaced: Vec<(usize, LegPath)> = shortcuts
        .iter()
        .zip(taken)
        .filter(|(_, take)| **take)
        .flat_map(|(s, _)| s.legs.iter().map(|&leg| (leg, s.path.clone())))
        .collect();
    match replaced.is_empty() {
        true => Arc::clone(base),
        false => Arc::new(traffic.splice_route(base, &replaced)),
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    fn shortcut(min: f32, max: f32) -> Shortcut {
        let marker = ShortcutMarker { key: 1, position: Vec3::ZERO, heading_deg: 0.0, min, max };
        Shortcut { marker, legs: vec![0], path: LegPath { segments: vec![0], from_t: 0.0, goal: (0, 1.0) } }
    }

    #[test]
    fn the_rolls_are_the_same_for_the_same_race_and_racer() {
        let shortcuts: Vec<Shortcut> = (0..16).map(|_| shortcut(0.4, 0.6)).collect();
        assert_eq!(rolls(&shortcuts, 0.5, seed_of(7, 2)), rolls(&shortcuts, 0.5, seed_of(7, 2)));
        assert_ne!(rolls(&shortcuts, 0.5, seed_of(7, 2)), rolls(&shortcuts, 0.5, seed_of(7, 3)), "racers differ");
    }

    #[test]
    fn a_shortcut_racers_never_take_is_never_taken_and_a_sure_one_always() {
        for id in 0..32 {
            assert_eq!(rolls(&[shortcut(0.0, 0.0), shortcut(1.0, 1.0)], 1.0, seed_of(9, id)), vec![false, true]);
        }
    }

    #[test]
    fn the_skill_raises_the_share_of_racers_that_cut() {
        let shortcuts = [shortcut(0.1, 0.9)];
        let count = |skill: f32| (0..400).filter(|&id| rolls(&shortcuts, skill, seed_of(3, id))[0]).count();
        let (weak, strong) = (count(0.0), count(1.0));
        assert!(weak < 100 && strong > 300, "{weak} weak, {strong} strong racers cut");
    }
}
