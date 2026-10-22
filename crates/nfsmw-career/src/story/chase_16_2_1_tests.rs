use super::*;

/// Frame `frame` (1-based) of a run with a frame time of one second, the call trigger entered in frame 1.
fn trigger_clock(frame: usize) -> ChaseInput {
    ChaseInput { secs_since_call_trigger: Some((frame - 1) as f32), dt: 1.0, ..Default::default() }
}

/// Runs the frames and returns each event with the 1-based frame it came in.
fn run(chase: &mut Chase, inputs: impl IntoIterator<Item = ChaseInput>) -> Vec<(usize, ChaseEvent)> {
    inputs
        .into_iter()
        .enumerate()
        .flat_map(|(i, input)| chase.update(input).into_iter().map(move |event| (i + 1, event)))
        .collect()
}

#[test]
fn the_call_trigger_requests_the_call_and_the_backup_sabotages_at_15_s() {
    let mut chase = Chase::new();
    let events = run(&mut chase, (1..=20).map(trigger_clock));
    assert_eq!(events, vec![(1, ChaseEvent::PlaceCall { id: 10 }), (16, ChaseEvent::SabotageEngine { level: 5 })]);
    assert!(!chase.is_finished(), "no end without a call start or an engine blow");
}

#[test]
fn kill_engine_sabotages_at_once_and_the_backup_does_not_repeat() {
    let mut chase = Chase::new();
    let frames = (1..=20).map(|f| ChaseInput { at_kill_engine: f == 3, ..trigger_clock(f) });
    let events = run(&mut chase, frames);
    assert_eq!(events, vec![(1, ChaseEvent::PlaceCall { id: 10 }), (3, ChaseEvent::SabotageEngine { level: 5 })]);
}

#[test]
fn kill_engine_before_the_call_trigger_sabotages_and_the_call_is_placed_later() {
    let mut chase = Chase::new();
    let frames = (1..=3).map(|f| ChaseInput {
        at_kill_engine: f == 1,
        secs_since_call_trigger: (f >= 2).then(|| (f - 2) as f32),
        dt: 1.0,
        ..Default::default()
    });
    let events = run(&mut chase, frames);
    assert_eq!(events, vec![(1, ChaseEvent::SabotageEngine { level: 5 }), (2, ChaseEvent::PlaceCall { id: 10 })]);
}

#[test]
fn the_call_path_fades_on_at_8_s_and_ends_at_11_s_after_the_call_starts() {
    let mut chase = Chase::new();
    let frames = (1..=15).map(|f| ChaseInput { call_started: f == 2, ..trigger_clock(f) });
    let events = run(&mut chase, frames);
    assert_eq!(
        events,
        vec![
            (1, ChaseEvent::PlaceCall { id: 10 }),
            (10, ChaseEvent::FadeOn),
            (13, ChaseEvent::FadeOut),
            (13, ChaseEvent::PostRace),
        ]
    );
    assert!(chase.is_finished());
}

#[test]
fn nothing_is_emitted_after_postrace() {
    let mut chase = Chase::new();
    let frames = (1..=13).map(|f| ChaseInput { call_started: f == 2, ..trigger_clock(f) });
    run(&mut chase, frames);
    assert!(chase.update(trigger_clock(30)).is_empty());
    let blown = ChaseInput { engine_blown: true, dt: 1.0, ..Default::default() };
    assert!(chase.update(blown).is_empty());
}

#[test]
fn a_call_start_counts_only_while_the_call_is_waited_for() {
    let mut chase = Chase::new();
    let frames = (1..=20).map(|f| ChaseInput { dt: 1.0, call_started: f == 2, ..Default::default() });
    assert!(run(&mut chase, frames).is_empty());
    assert!(!chase.is_finished());
}

#[test]
fn an_engine_blow_ends_the_race_1_s_after_it_happens() {
    let mut chase = Chase::new();
    let frames = (1..=6).map(|f| ChaseInput { at_kill_engine: f == 2, engine_blown: f == 3, ..trigger_clock(f) });
    let events = run(&mut chase, frames);
    assert_eq!(
        events,
        vec![
            (1, ChaseEvent::PlaceCall { id: 10 }),
            (2, ChaseEvent::SabotageEngine { level: 5 }),
            (4, ChaseEvent::FadeOut),
            (4, ChaseEvent::PostRace),
        ]
    );
}

#[test]
fn an_engine_blow_before_the_fade_cancels_the_call_path() {
    let mut chase = Chase::new();
    let frames = (1..=15).map(|f| ChaseInput { call_started: f == 2, engine_blown: f == 5, ..trigger_clock(f) });
    let events = run(&mut chase, frames);
    assert_eq!(
        events,
        vec![(1, ChaseEvent::PlaceCall { id: 10 }), (6, ChaseEvent::FadeOut), (6, ChaseEvent::PostRace)],
        "the fade (at 10) and the kill timer (at 13) never fire"
    );
}

#[test]
fn an_engine_blow_after_the_fade_ends_the_race_once() {
    let mut chase = Chase::new();
    let frames = (1..=15).map(|f| ChaseInput { call_started: f == 2, engine_blown: f == 11, ..trigger_clock(f) });
    let events = run(&mut chase, frames);
    assert_eq!(
        events,
        vec![
            (1, ChaseEvent::PlaceCall { id: 10 }),
            (10, ChaseEvent::FadeOn),
            (12, ChaseEvent::FadeOut),
            (12, ChaseEvent::PostRace),
        ]
    );
}

#[test]
fn the_timers_count_the_frame_time() {
    // Frame time 0.5 s: the call starts in frame 1, so the fade is due at clock 8 s (frame 17) and the end at clock
    // 11 s (frame 23). The backup stays below 15 s over these frames.
    let mut chase = Chase::new();
    let frames = (1..=24).map(|f| ChaseInput {
        secs_since_call_trigger: Some((f - 1) as f32 * 0.5),
        call_started: f == 1,
        dt: 0.5,
        ..Default::default()
    });
    let events = run(&mut chase, frames);
    assert_eq!(
        events,
        vec![
            (1, ChaseEvent::PlaceCall { id: 10 }),
            (17, ChaseEvent::FadeOn),
            (23, ChaseEvent::FadeOut),
            (23, ChaseEvent::PostRace),
        ]
    );
}

#[test]
fn the_cop_trigger_turns_cops_on_and_spawns_two_mid_size_cops_once() {
    let mut chase = Chase::new();
    let frames = (1..=6).map(|f| ChaseInput { at_cop_spawn: f == 2 || f == 5, dt: 1.0, ..Default::default() });
    let events = run(&mut chase, frames);
    assert_eq!(events, vec![(2, ChaseEvent::CopsOn), (2, ChaseEvent::SpawnCops { count: 2, kind: CopKind::MidSize }),]);
}

#[test]
fn the_bail_trigger_runs_its_three_calls_once() {
    let mut chase = Chase::new();
    let frames = (1..=6).map(|f| ChaseInput { at_bail: f == 4 || f == 6, dt: 1.0, ..Default::default() });
    let events = run(&mut chase, frames);
    assert_eq!(
        events,
        vec![
            (4, ChaseEvent::DispIntroRace),
            (4, ChaseEvent::BailPursuit { a: 1.0, b: 1.0 }),
            (4, ChaseEvent::NoNewPursuits),
        ]
    );
}
