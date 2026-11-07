//! Installed pace probe. An external, asset-free player trace controls catch-up and world residency.
use super::installed_tests::loaded_world;
use crate::frontend::custom_race::{Mode, settings::Settings};
use crate::input::ActionState;
use crate::scenes::world::{drive::STEP, road::Spawn, space};
use crate::viewer::Scene;
use glam::Vec3;

#[derive(serde::Deserialize)]
struct Trace {
    event: String,
    #[serde(default)]
    catch_up: Option<bool>,
    #[serde(default)]
    finish_time: Option<f32>,
    #[serde(default)]
    grid: Vec<RecordedRacer>,
    #[serde(default)]
    physics_rate_hz: Option<u32>,
    // Race seconds, physics position, physics forward, speed in m/s.
    samples: Vec<(f32, [f32; 3], [f32; 3], f32)>,
}

#[derive(serde::Deserialize)]
struct RecordedRacer {
    id: usize,
    model: String,
    position: [f32; 3],
    forward: [f32; 3],
}

fn spawn(position: [f32; 3], forward: [f32; 3]) -> Spawn {
    Spawn { position: space::to_render(position), heading: (-forward[0]).atan2(forward[2]) }
}

#[test]
#[ignore = "needs installed world and NFSMW_RACER_TRACE player trajectory JSON"]
fn installed_stock_racer_pace_against_a_recorded_player() {
    let path = std::env::var_os("NFSMW_RACER_TRACE").expect("NFSMW_RACER_TRACE");
    let trace: Trace = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert!(trace.samples.len() > 100);
    assert!(trace.samples.windows(2).all(|pair| pair[1].0 > pair[0].0));
    let (mut scene, mut renderer) = loaded_world();
    if let Some(rate) = trace.physics_rate_hz {
        assert!(matches!(rate, 60 | 144));
        scene.tuning.physics_rate_hz = rate;
    }
    eprintln!("PACE_CADENCE physics_hz={}", scene.tuning.physics_rate_hz);
    scene.set_session_rules(crate::scenes::world::session_rules::SessionRules {
        seed: Some(0x6E66_736D_7721),
        ..Default::default()
    });
    let settings = Settings {
        opponents: 6,
        difficulty: 2,
        traffic: 0,
        heat: 0,
        catch_up: trace.catch_up.unwrap_or(true),
        ..Default::default()
    };
    crate::i18n::start(&scene.dir, "en".parse().ok());
    let race = scene
        .gameplay
        .as_ref()
        .unwrap()
        .races
        .iter()
        .find(|race| {
            let event = crate::career::race_event_id(race);
            let label = blackbox_text::label_hash(&format!("TRACKNAME_{}", event.replace('.', "_")));
            race.race_type == Some(nfsmw_data::gameplay::RaceType::Sprint)
                && crate::i18n::game(label).is_some_and(|label| label.eq_ignore_ascii_case(&trace.event))
        })
        .expect("recorded track title");
    let event = crate::career::race_event_id(race);
    let label = blackbox_text::label_hash(&format!("TRACKNAME_{}", event.replace('.', "_")));
    eprintln!(
        "PACE_EVENT {event} name={:?} length={} label={:?}",
        race.name,
        race.params.route_length,
        crate::ui::Text::Game(label).resolve()
    );
    let name = race.name.clone();
    scene.start_race(&name, settings.race_options(Mode::Sprint)).unwrap();
    if !trace.grid.is_empty() {
        assert_eq!(trace.grid.len(), 7);
        let session = scene.race.as_mut().unwrap();
        assert_eq!(session.plan.len(), 7);
        for recorded in &trace.grid {
            let plan = &mut session.plan[recorded.id];
            plan.spawn = spawn(recorded.position, recorded.forward);
            if recorded.id == 0 {
                assert_eq!(recorded.model, "CARRERAGT");
                continue;
            }
            plan.model = Some(recorded.model.clone());
            plan.tried.clear();
            scene.traffic.as_mut().unwrap().request_racer_model(&recorded.model);
        }
        scene.players.lead_mut().unwrap().place_exactly(session.plan[0].spawn, 0.0);
    }
    let input = ActionState::default();
    let mut started = false;
    for _ in 0..20_000 {
        scene.update(&mut renderer, &input, STEP);
        if scene
            .race
            .as_ref()
            .and_then(|r| r.engine.as_ref())
            .is_some_and(|r| r.phase() == blackbox_race::Phase::Racing)
        {
            started = true;
            break;
        }
    }
    assert!(started);
    let player = scene.players.lead().unwrap().ai_body().position;
    assert!(player.distance(Vec3::from(trace.samples[0].1)) < 50.0, "trace must start on this course");
    eprintln!("PACE_GRID {:?}", scene.race.as_ref().unwrap().plan);
    eprintln!("PACE_PLAYER_RATING {}", scene.race.as_ref().unwrap().player_top_speed_rating);
    for id in 1..=6 {
        let racer = scene.traffic.as_mut().unwrap().racer_mut(id).unwrap();
        eprintln!(
            "PACE_LIMITS id={id} own={:?} player={:?} limits={:?}",
            racer.performance, racer.player, racer.limits
        );
    }
    let mut cursor = 0;
    let mut paused_updates = 0;
    let mut pending_placements = 0;
    let mut advanced_updates = 0;
    let mut severe_losses = [0_u32; 6];
    let mut previous_speeds = [0.0_f32; 6];
    let mut physics_frames = [0_u32; 6];
    let mut stand_in_frames = [0_u32; 6];
    let mut last_logged = -1;
    let end = match trace.finish_time {
        Some(_) => trace.samples.last().unwrap().0,
        None => trace.samples.last().unwrap().0.min(120.0),
    };
    for _ in 0..30_000 {
        let before = scene.race.as_ref().unwrap().engine.as_ref().unwrap().elapsed();
        let time = before;
        if time >= end || scene.race.as_ref().unwrap().engine.as_ref().unwrap().racers()[0].done() {
            break;
        }
        while cursor + 2 < trace.samples.len() && trace.samples[cursor + 1].0 < time {
            cursor += 1;
        }
        let (a, b) = (&trace.samples[cursor], &trace.samples[cursor + 1]);
        let fraction = ((time - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
        let position = Vec3::from(a.1).lerp(Vec3::from(b.1), fraction);
        let forward = Vec3::from(a.2).lerp(Vec3::from(b.2), fraction);
        scene
            .players
            .lead_mut()
            .unwrap()
            .place_exactly(spawn(position.to_array(), forward.to_array()), a.3 + (b.3 - a.3) * fraction);
        scene.update(&mut renderer, &input, STEP);
        let engine = scene.race.as_ref().unwrap().engine.as_ref().unwrap();
        paused_updates += usize::from(engine.elapsed() == before);
        advanced_updates += usize::from(engine.elapsed() > before);
        pending_placements += usize::from(scene.players.lead().unwrap().waiting_for_road().is_some());
        for car in &scene.traffic.as_ref().unwrap().cars {
            let Some(id) = car.racer_id() else { continue };
            let speed = car.race_speed();
            severe_losses[id - 1] +=
                u32::from(previous_speeds[id - 1] > 15.0 && previous_speeds[id - 1] - speed > 10.0);
            previous_speeds[id - 1] = speed;
            match car.race_uses_physics() {
                true => physics_frames[id - 1] += 1,
                false => stand_in_frames[id - 1] += 1,
            }
        }
        if time as i32 == last_logged {
            continue;
        }
        last_logged = time as i32;
        let engine = scene.race.as_ref().unwrap().engine.as_ref().unwrap();
        for car in &scene.traffic.as_ref().unwrap().cars {
            let Some(id) = car.racer_id() else { continue };
            eprintln!("PACE t={time:.2} id={id} progress={} {}", engine.racers()[id].percent, car.race_diagnostic());
            eprintln!("PACE_POWER t={time:.2} id={id} {}", car.race_power_diagnostic());
        }
        for id in 1..=6 {
            let state = scene.traffic.as_mut().unwrap().racer_mut(id).unwrap();
            eprintln!(
                "PACE_GLUE t={time:.2} id={id} base={} glue={} cheat={}",
                state.base_skill,
                state.glue,
                state.cheat()
            );
        }
    }
    let engine = scene.race.as_ref().unwrap().engine.as_ref().unwrap();
    let external = advanced_updates as f32 * STEP;
    eprintln!(
        "PACE_CLOCK external={external} elapsed={} paused_updates={paused_updates} pending_placements={pending_placements}",
        engine.elapsed()
    );
    eprintln!("PACE_FINAL {:?}", engine.racers());
    eprintln!(
        "PACE_RESULT {}",
        serde_json::json!({
            "catch_up": settings.catch_up,
            "elapsed": engine.elapsed(), "native_human_finish": trace.finish_time,
            "human_finished": engine.racers()[0].done(),
            "severe_speed_losses": severe_losses, "physics_frames": physics_frames, "stand_in_frames": stand_in_frames,
            "racers": engine.racers().iter().enumerate().map(|(id, racer)| serde_json::json!({
                "id": id, "percent": racer.percent, "finished_at": racer.finished_at,
                "checkpoints": racer.checkpoint_times,
                "projected": (racer.percent > 0.0).then(|| engine.elapsed() * 100.0 / racer.percent)
            })).collect::<Vec<_>>()
        })
    );
    assert!((engine.elapsed() - external).abs() < 0.1, "trace must follow the simulated race clock");
    if let Some(finish_time) = trace.finish_time {
        assert!(engine.racers()[0].done(), "the recorded player must cross the actual finish gate");
        assert!((engine.elapsed() - finish_time).abs() < 0.5, "finish continuation must stay bounded");
    }
}
