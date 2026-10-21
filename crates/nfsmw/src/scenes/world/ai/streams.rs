//! The traffic world's random numbers: one seeded stream per system, so how often one system draws does not shift what
//! another one picks (`docs/specs/multiplayer/determinism.md`, D4). All streams come from one world seed.

use blackbox_roads::SplitMix;

/// The default world seed: the seed of the single stream the traffic world used before it was split.
pub(super) const WORLD_SEED: u64 = 0x6E66_736D_7721;

/// One random stream per system of the traffic world.
#[derive(Debug, Clone)]
pub(super) struct Streams {
    /// Lane and route choices of every car as it drives: traffic, patrols, cops and racers.
    pub nav: SplitMix,
    /// The traffic population: the type timers, the model picks, the spawn probes and the start staggers.
    pub population: SplitMix,
    /// The police: the sensing rolls, the bust, the chase's cop spawns and search cars, the budget and support rolls.
    pub police: SplitMix,
    /// Roadblocks: the request lottery, the wave pick, the spikes and the site walk.
    pub roadblock: SplitMix,
    /// The police helicopter.
    pub air: SplitMix,
}

impl Streams {
    /// The streams of a world started from `world_seed`.
    pub fn new(world_seed: u64) -> Self {
        Self {
            nav: stream(world_seed, 1),
            population: stream(world_seed, 2),
            police: stream(world_seed, 3),
            roadblock: stream(world_seed, 4),
            air: stream(world_seed, 5),
        }
    }
}

impl Default for Streams {
    fn default() -> Self {
        Self::new(WORLD_SEED)
    }
}

/// The stream of system `id`: the world seed mixed with the id by the SplitMix64 finaliser, so that neighbouring ids give
/// unrelated streams.
fn stream(world_seed: u64, id: u64) -> SplitMix {
    let mut z = world_seed ^ id.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    SplitMix(z ^ (z >> 31))
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_roads::RandomSource;

    fn draws(rng: &mut SplitMix) -> Vec<u32> {
        (0..16).map(|_| rng.next_f32().to_bits()).collect()
    }

    #[test]
    fn the_streams_of_one_seed_are_distinct_and_reproducible() {
        let mut a = Streams::default();
        let mut b = Streams::new(WORLD_SEED);
        let all = [
            draws(&mut a.nav),
            draws(&mut a.population),
            draws(&mut a.police),
            draws(&mut a.roadblock),
            draws(&mut a.air),
        ];
        for (i, x) in all.iter().enumerate() {
            for y in &all[i + 1..] {
                assert_ne!(x, y, "two systems share a stream");
            }
        }
        assert_eq!(all[2], draws(&mut b.police), "the same seed gives the same stream");
    }

    #[test]
    fn drawing_from_one_system_leaves_the_others_alone() {
        let mut busy = Streams::default();
        let mut quiet = Streams::default();
        for _ in 0..1000 {
            busy.police.next_f32();
        }
        assert_eq!(draws(&mut busy.nav), draws(&mut quiet.nav));
        assert_eq!(draws(&mut busy.population), draws(&mut quiet.population));
    }
}
