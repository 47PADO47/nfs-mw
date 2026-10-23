//! The AI cars' integrations on several threads at once. The result must not depend on it: the netcode relies on a
//! bit-identical step (`docs/physics-timing.md`).
//!
//! A traffic step runs each car's start in car order, then the cars' integrations, then each car's end in car order
//! (`car/stepping.rs`, `simulation.rs`). An integration reads only its own car's body and terrain caches, the static
//! collision (each car casting through its own candidate window) and the surface table; it reads no other car, draws
//! no random number and queues nothing. So it gives the same body wherever it runs, and moving every car's start ahead
//! of the earlier cars' ends changes nothing either: a start reads the bodies of the others as the step began
//! (`TrafficWorld::bodies`) and its own car, and the explosion waves it may push the body with are a part of the props
//! that the ends never touch. Everything with side effects (props, contacts, sounds, effects, damage, the random
//! streams) stays in the serial parts, in car order.

use std::sync::OnceLock;

use blackbox_collision::CollisionWorld;
use nfsmw_data::car::physics::SurfaceTable;

use super::car::AiCar;
use crate::scenes::world::drive::{Integration, StepStart};

/// The machine's threads a step uses at most: past this many the cars of one tick are too few to share out.
const MAX_MACHINE_THREADS: usize = 8;

/// When a traffic step integrates its cars on several threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Parallel {
    /// With fewer cars than this the step runs on the simulation thread alone: starting a thread costs more than a
    /// few cars' integrations.
    pub min_cars: usize,
    /// The fewest cars a thread is given.
    pub per_thread: usize,
    /// The most threads, the simulation thread included; 0 for the machine's (at most [`MAX_MACHINE_THREADS`]).
    pub max_threads: usize,
}

impl Default for Parallel {
    fn default() -> Self {
        Self { min_cars: 8, per_thread: 4, max_threads: 0 }
    }
}

impl Parallel {
    /// Every step on the simulation thread, in the plain order (each car's whole step before the next car's).
    #[cfg(test)]
    pub const SERIAL: Self = Self { min_cars: usize::MAX, per_thread: 1, max_threads: 1 };

    /// The threads a step of `cars` cars uses; 1 is the plain serial step.
    pub fn threads(&self, cars: usize) -> usize {
        if cars < self.min_cars {
            return 1;
        }
        let most = match self.max_threads {
            0 => machine_threads(),
            n => n,
        };
        most.min(cars / self.per_thread.max(1)).max(1)
    }
}

fn machine_threads() -> usize {
    static THREADS: OnceLock<usize> = OnceLock::new();
    *THREADS.get_or_init(|| std::thread::available_parallelism().map_or(1, |n| n.get()).min(MAX_MACHINE_THREADS))
}

/// One car's integration and the start of its step.
struct Job<'a> {
    integration: Integration<'a>,
    start: StepStart,
}

/// Integrates the cars whose step started (`starts`, by car index; `None` for a car that does not step this tick) on
/// up to `threads` threads, the calling thread taking the first share.
pub(super) fn integrate(
    cars: &mut [AiCar],
    starts: &[Option<StepStart>],
    collision: &CollisionWorld,
    surfaces: &SurfaceTable,
    threads: usize,
) {
    let mut jobs: Vec<Job<'_>> = cars
        .iter_mut()
        .zip(starts)
        .filter_map(|(car, start)| Some(Job { start: (*start)?, integration: car.integration() }))
        .collect();
    let share = jobs.len().div_ceil(threads.max(1)).max(1);
    std::thread::scope(|scope| {
        let mut shares = jobs.chunks_mut(share);
        let own = shares.next();
        for part in shares {
            scope.spawn(move || run(part, collision, surfaces));
        }
        if let Some(own) = own {
            run(own, collision, surfaces);
        }
    });
}

fn run(jobs: &mut [Job<'_>], collision: &CollisionWorld, surfaces: &SurfaceTable) {
    for job in jobs {
        job.integration.run_windowed(&job.start, collision, surfaces);
    }
}

#[cfg(test)]
mod tests {
    use super::Parallel;

    #[test]
    fn small_steps_stay_on_one_thread_and_shares_keep_their_minimum() {
        let parallel = Parallel { min_cars: 8, per_thread: 4, max_threads: 6 };
        assert_eq!(parallel.threads(7), 1);
        assert_eq!(parallel.threads(8), 2);
        assert_eq!(parallel.threads(13), 3);
        assert_eq!(parallel.threads(100), 6);
        assert_eq!(Parallel::SERIAL.threads(100), 1);
        assert_eq!(Parallel::default().threads(1), 1);
    }
}
