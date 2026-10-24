//! What the server asks of the world each tick, apart from the scene's own frame (`step_bench_tests.rs`): the step with
//! its event collection, the entity export and every seat's own state. Diagnostic: `cargo test -p nfsmw
//! host_tick_costs -- --ignored --nocapture` with `NFSMW_GAME_DIR` set.

use std::time::Instant;

use nfsmw_sim::{PlayerId, SimEvent, WorldSim};

use super::installed::{CAR, inputs, join, open};

const PLAYERS: u8 = 16;
const WARM_UP: u32 = 900;
const MEASURED: u32 = 1200;

fn report(name: &str, times: &mut [f32]) {
    times.sort_by(f32::total_cmp);
    let at = |percent: usize| times[(times.len() - 1) * percent / 100];
    let mean = times.iter().sum::<f32>() / times.len() as f32;
    eprintln!("host-tick {name}: mean {mean:.3} ms, p50 {:.3}, p95 {:.3}, p99 {:.3}", at(50), at(95), at(99));
}

#[test]
#[ignore = "needs the game; diagnostic benchmark"]
fn host_tick_costs() {
    let mut host = open(1, Some((16, 20)));
    let seats: Vec<u8> = (0..PLAYERS).collect();
    for &seat in &seats[1..] {
        host.add_player(PlayerId(seat), &join(CAR)).unwrap();
    }
    let (mut step, mut export, mut own) = (Vec::new(), Vec::new(), Vec::new());
    let (mut events, mut views) = (Vec::new(), Vec::new());
    for tick in 0..WARM_UP + MEASURED {
        let input = inputs(tick, &seats);
        events.clear();
        let start = Instant::now();
        host.step(&input, &mut events);
        let stepped = start.elapsed();
        views.clear();
        let start = Instant::now();
        host.entities(&mut views);
        let exported = start.elapsed();
        let start = Instant::now();
        for &seat in &seats {
            std::hint::black_box(host.own_state(PlayerId(seat)));
        }
        let owned = start.elapsed();
        if tick >= WARM_UP {
            step.push(stepped.as_secs_f32() * 1000.0);
            export.push(exported.as_secs_f32() * 1000.0);
            own.push(owned.as_secs_f32() * 1000.0);
        }
    }
    let _: &[SimEvent] = &events;
    report("step", &mut step);
    report("entities", &mut export);
    report("own_state x16", &mut own);
}
