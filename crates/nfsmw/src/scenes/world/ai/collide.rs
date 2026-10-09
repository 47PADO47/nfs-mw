//! Cars hitting each other and the player's car.

use super::TrafficWorld;
use crate::scenes::world::drive::CarSim;

/// One body that can be hit: a car or the trailer it tows.
struct Part<'a> {
    /// Index of the car it belongs to.
    owner: usize,
    /// Its car has a hitched trailer: the car and the trailer never collide with each other.
    coupled: bool,
    sim: &'a mut CarSim,
}

/// A hit, remembered until the cars can be told: the car, the impulse (N s) and whether the player did it.
struct Hit {
    owner: usize,
    impulse: f32,
    by_player: bool,
}

/// Whether the bounding spheres of two bodies touch, a cheap test before the box test.
fn near(a: &CarSim, b: &CarSim) -> bool {
    let (a, b) = (a.collision_box(), b.collision_box());
    a.centre.distance(b.centre) < a.half.length() + b.half.length()
}

impl TrafficWorld {
    /// Lets the cars hit each other and the player's car (`player`). A tractor and the trailer hitched to it do
    /// not hit each other; a trailer that came loose is a body like any other.
    pub fn collide(&mut self, mut player: Option<&mut CarSim>) {
        let mut parts: Vec<Part<'_>> = Vec::new();
        for (owner, car) in self.cars.iter_mut().enumerate() {
            let coupled = car.is_coupled();
            let (tractor, trailer) = car.sims_mut();
            parts.push(Part { owner, coupled, sim: tractor });
            if let Some(sim) = trailer {
                parts.push(Part { owner, coupled, sim });
            }
        }
        let mut hits = Vec::new();
        for i in 0..parts.len() {
            let (head, tail) = parts.split_at_mut(i + 1);
            let part = &mut head[i];
            if let Some(player) = player.as_deref_mut()
                && near(player, part.sim)
                && let Some(hit) = player.collide_with(part.sim)
            {
                hits.push(Hit { owner: part.owner, impulse: hit.impulse, by_player: true });
            }
            for other in tail {
                if part.coupled && part.owner == other.owner {
                    continue;
                }
                if !near(part.sim, other.sim) {
                    continue;
                }
                if let Some(hit) = part.sim.collide_with(other.sim) {
                    hits.push(Hit { owner: part.owner, impulse: hit.impulse, by_player: false });
                    hits.push(Hit { owner: other.owner, impulse: hit.impulse, by_player: false });
                }
            }
        }
        drop(parts);
        for hit in hits {
            self.cars[hit.owner].on_hit(hit.impulse, hit.by_player);
        }
    }
}
