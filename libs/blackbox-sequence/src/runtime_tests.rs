use std::collections::BTreeMap;

use crate::*;

const TAGS: Tags = Tags { start: 100, end: 101, stop: 102 };

fn package() -> Package {
    let state = |stimulus, end_state, action| State {
        filter: None,
        responses: BTreeMap::from([(stimulus, Response { end_state, action })]),
    };
    let system =
        System { initial: 1, default: 1, states: BTreeMap::from([(1, state(7, 2, 10)), (2, state(8, 1, 11))]) };
    let action = |duration, times| Action {
        name_hash: 0,
        duration,
        tags: BTreeMap::from([(TAGS.start, 40), (TAGS.end, 41), (TAGS.stop, 42)]),
        times,
    };
    Package {
        engines: vec![Engine { name: "synthetic".into(), name_hash: 1, systems: BTreeMap::from([(5, system)]) }],
        actions: BTreeMap::from([(10, action(1.0, vec![(0.0, 20), (0.5, 21), (1.0, 22)])), (11, action(0.0, vec![]))]),
        ..Package::default()
    }
}

fn emissions(player: &mut Player) -> Vec<(u32, Option<u64>)> {
    std::iter::from_fn(|| player.pop()).map(|e| (e.list, e.context)).collect()
}

#[test]
fn idle_starts_busy_queues_against_the_projected_state() {
    let p = package();
    let mut player = Player::new(&p, 0, TAGS).unwrap();
    assert!(player.stimulate(&p, Some(5), 7, QueueMode::Allow, Some(99)).unwrap());
    assert_eq!(emissions(&mut player), [(40, Some(99)), (20, None)]);
    assert_eq!(player.state(5), Some(1));
    assert!(player.stimulate(&p, Some(5), 8, QueueMode::Allow, Some(77)).unwrap());
    player.advance(&p, 0.5).unwrap();
    assert_eq!(emissions(&mut player), [(21, None)]);
    player.advance(&p, 0.5).unwrap();
    assert_eq!(emissions(&mut player), [(22, None), (41, None), (40, None), (41, None)]);
    assert_eq!(player.state(5), Some(1));
    assert!(!player.busy(5));
    player.advance(&p, 10.0).unwrap();
    assert!(emissions(&mut player).is_empty());
}

#[test]
fn queue_capacity_filters_unknown_stimuli_and_obeys_modes() {
    let mut p = package();
    p.engines[0].systems.get_mut(&5).unwrap().states.get_mut(&1).unwrap().responses.get_mut(&7).unwrap().end_state = 1;
    let mut player = Player::new(&p, 0, TAGS).unwrap();
    player.stimulate(&p, None, 7, QueueMode::Disable, None).unwrap();
    assert!(!player.stimulate(&p, None, 7, QueueMode::Disable, None).unwrap());
    assert!(!player.stimulate(&p, None, 999, QueueMode::Allow, None).unwrap());
    assert!(player.stimulate(&p, None, 7, QueueMode::Shallow, None).unwrap());
    assert!(!player.stimulate(&p, None, 7, QueueMode::Shallow, None).unwrap());
    for _ in 1..4 {
        assert!(player.stimulate(&p, None, 7, QueueMode::Allow, None).unwrap());
    }
    assert!(!player.stimulate(&p, None, 7, QueueMode::Allow, None).unwrap());
    assert!(!player.stimulate(&p, None, 7, QueueMode::Flush, None).unwrap());
    player.advance(&p, 1.0).unwrap();
    assert!(!player.busy(5));
}

#[test]
fn abort_fires_stop_then_uses_the_end_state_and_clears_queue() {
    let p = package();
    let mut player = Player::new(&p, 0, TAGS).unwrap();
    player.stimulate(&p, None, 7, QueueMode::Allow, None).unwrap();
    player.stimulate(&p, None, 8, QueueMode::Allow, None).unwrap();
    emissions(&mut player);
    player.stimulate(&p, None, 8, QueueMode::Abort, Some(3)).unwrap();
    assert_eq!(emissions(&mut player), [(42, Some(3)), (40, Some(3)), (41, None)]);
    assert!(!player.busy(5));
    assert_eq!(player.state(5), Some(1));
}

#[test]
fn activation_order_and_invalid_time_are_explicit() {
    let mut p = package();
    let system = p.engines[0].systems[&5].clone();
    p.engines[0].systems.insert(3, system);
    let mut player = Player::new(&p, 0, TAGS).unwrap();
    player.stimulate(&p, Some(5), 7, QueueMode::Allow, None).unwrap();
    player.stimulate(&p, Some(3), 7, QueueMode::Allow, None).unwrap();
    emissions(&mut player);
    for dt in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(player.advance(&p, dt), Err(RuntimeError::Time));
    }
    player.advance(&p, 0.5).unwrap();
    assert_eq!(player.pop().unwrap().system, 5);
    assert_eq!(player.pop().unwrap().system, 3);
    p.engines[0].systems.get_mut(&5).unwrap().states.get_mut(&2).unwrap().filter = Some(666);
    assert_eq!(player.stimulate(&p, Some(5), 8, QueueMode::Allow, None), Err(RuntimeError::UnsupportedFilter(666)));
}

#[test]
fn fixed_tick_event_trace_is_identical_across_render_rates_and_stalls() {
    fn trace(frames: &[f64]) -> Vec<(usize, u32)> {
        let p = package();
        let mut player = Player::new(&p, 0, TAGS).unwrap();
        player.stimulate(&p, None, 7, QueueMode::Allow, None).unwrap();
        player.stimulate(&p, None, 8, QueueMode::Allow, None).unwrap();
        let mut out = Vec::new();
        let (mut accumulated, mut tick, mut frame) = (0.0, 0, 0);
        while tick < 180 {
            accumulated += frames[frame % frames.len()];
            frame += 1;
            while accumulated + 1e-10 >= 1.0 / 60.0 && tick < 180 {
                accumulated -= 1.0 / 60.0;
                while let Some(e) = player.pop() {
                    out.push((tick, e.list));
                }
                player.advance(&p, 1.0 / 60.0).unwrap();
                tick += 1;
            }
        }
        out
    }
    let expected = trace(&[1.0 / 60.0]);
    for fps in [5.0, 10.0, 30.0, 60.0, 120.0, 144.0, 240.0] {
        assert_eq!(trace(&[1.0 / fps]), expected);
    }
    assert_eq!(trace(&[0.003, 0.010, 0.300, 0.002]), expected);
    assert_eq!(expected.iter().map(|e| e.1).collect::<Vec<_>>(), [40, 20, 21, 22, 41, 40, 41]);
}
