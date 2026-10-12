//! The point each cop drives at this think: the player, a collapse ring slot, a formation slot, or a block slot or the
//! rear quarter for the tactics the chase allows. The rules are pure (`blackbox_pursuit::tactics::plan`,
//! `formation::slots::assign`, `formation::collapse::ring_slots`); this file reads the cars, the heat, the formation and
//! the player's speed. Spec: `docs/specs/ai-pursuit-tactics.md`, `docs/specs/police-tactics.md`,
//! `docs/specs/ai-pursuit-formations.md`.

use blackbox_pursuit::formation::collapse::{self, ring_slots};
use blackbox_pursuit::formation::slots::{assign, assign_slots, in_position_offsets};
use blackbox_pursuit::tactics::plan;
use blackbox_roads::Body;
use glam::{Vec2, Vec3};

/// From this heat level cops block the road (the heat tables' roadblock rows; the level is unconfirmed).
const BLOCK_FROM_LEVEL: u32 = 3;
/// From this heat level cops pit the player (unconfirmed).
const PIT_FROM_LEVEL: u32 = 4;
/// The tightness the in-position offsets use: the install's level tightness is not read yet, so the middle value (0.5).
const IN_POSITION_TIGHTNESS: f32 = 0.5;
/// Cops this far from the player (metres) can join a collapse ring (`ai-pursuit-formations.md` §5.1).
const COLLAPSE_RANGE: f32 = 60.0;

impl super::TrafficWorld {
    /// The aim of every car this think, by car index. Only cops in a chase have one; `None` is the player.
    pub(super) fn cop_aims(&self, player: &Body) -> Vec<Option<Vec3>> {
        let mut aims = vec![None; self.cars.len()];
        let Some(level) = self.police.level() else { return aims };
        let cops: Vec<usize> = (0..self.cars.len()).filter(|&i| self.cars[i].is_cop()).collect();
        let positions: Vec<Vec2> = cops
            .iter()
            .map(|&i| {
                let p = self.cars[i].avoidable().position;
                Vec2::new(p.x, p.z)
            })
            .collect();
        let player_xz = Vec2::new(player.position.x, player.position.z);
        let forward = player.forward.normalize_or_zero();
        let right = Vec2::new(forward.y, -forward.x);
        let speed_kmh = Vec2::new(player.velocity.x, player.velocity.z).length() * 3.6;
        let collapse = collapse::level(level);
        // The finisher: each cop drives to its in-position offset of the formation, when the formation has one (pit,
        // box-in, rolling block). Other cops drive at the player, which is the ram.
        if self.police.finisher_running() {
            if let Some(formation) = self.police.last_formation() {
                if let Some(offsets) = in_position_offsets(formation, IN_POSITION_TIGHTNESS) {
                    let slot_of = assign_slots(formation, &positions, player_xz);
                    let planned: Vec<Option<Vec2>> = slot_of
                        .iter()
                        .map(|slot| slot.and_then(|s| offsets.get(s)).map(|o| player_xz + right * o.x + forward * o.y))
                        .collect();
                    for (&car, aim) in cops.iter().zip(planned) {
                        aims[car] = aim.map(|a| Vec3::new(a.x, player.position.y, a.y));
                    }
                    return aims;
                }
            }
            return aims;
        }
        let planned = if self.police.last_formation().is_some() && speed_kmh < collapse.speed_kmh {
            self.collapse_aims(&cops, &positions, player_xz, forward, right, collapse)
        } else {
            match self.police.last_formation() {
                Some(formation) => assign(formation, &positions, player_xz, forward),
                None => plan(
                    &positions,
                    player_xz,
                    forward,
                    level >= BLOCK_FROM_LEVEL,
                    level >= PIT_FROM_LEVEL,
                ),
            }
        };
        for (&car, aim) in cops.iter().zip(planned) {
            aims[car] = aim.map(|a| Vec3::new(a.x, player.position.y, a.y));
        }
        aims
    }

    /// The collapse: the cops within range take the ring slots around the player; the others keep the chase.
    fn collapse_aims(
        &self,
        cops: &[usize],
        positions: &[Vec2],
        player: Vec2,
        forward: Vec2,
        right: Vec2,
        level: collapse::Level,
    ) -> Vec<Option<Vec2>> {
        let mut planned = vec![None; cops.len()];
        let near: Vec<usize> = (0..cops.len()).filter(|&k| (positions[k] - player).length() <= COLLAPSE_RANGE).collect();
        let offsets: Vec<Vec2> = near
            .iter()
            .map(|&k| {
                let rel = positions[k] - player;
                Vec2::new(rel.dot(right), rel.dot(forward))
            })
            .collect();
        for (slot, offset) in ring_slots(&offsets, level) {
            planned[near[slot]] = Some(player + right * offset.x + forward * offset.y);
        }
        planned
    }
}
