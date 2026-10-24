//! Wall-time boundaries around the application's ordered Update sets, including uninstrumented work.
use crate::app::FrameSet;
use bevy_app::{App, Update};
use bevy_ecs::prelude::*;

#[derive(Default)]
struct Phase(Option<blackbox_trace::Scope<'static>>);

/// The boundaries only mean something in a capture, so they stay out of the schedule while none is recording.
fn recording() -> bool {
    blackbox_trace::global().recording()
}

fn enter(mut phase: NonSendMut<Phase>, name: &'static str) {
    phase.0.take();
    phase.0 = Some(blackbox_trace::scope(name));
}

/// Opens the phase `name` right before the set `next` (and right after the set `previous`, when there is one).
fn boundary(app: &mut App, name: &'static str, previous: Option<FrameSet>, next: FrameSet) {
    let mut config = (move |phase: NonSendMut<Phase>| enter(phase, name)).before(next).run_if(recording);
    if let Some(previous) = previous {
        config = config.after(previous);
    }
    app.add_systems(Update, config);
}

pub(super) fn build(app: &mut App) {
    use FrameSet::*;
    app.insert_non_send(Phase::default());
    boundary(app, "App phase: prepare", None, Prepare);
    boundary(app, "App phase: commands", Some(Prepare), Commands);
    boundary(app, "App phase: mods pre", Some(Commands), ModsPre);
    boundary(app, "App phase: frontend", Some(ModsPre), Frontend);
    boundary(app, "App phase: scene update", Some(Frontend), SceneUpdate);
    boundary(app, "App phase: mods post", Some(SceneUpdate), ModsPost);
    boundary(app, "App phase: UI / audio", Some(ModsPost), Ui);
    boundary(app, "App phase: HUD", Some(Ui), Hud);
    boundary(app, "App phase: draw", Some(Hud), Draw);
    // Always runs, so a capture that stops mid-frame does not leave a scope open for the next one.
    app.add_systems(
        Update,
        (|mut phase: NonSendMut<Phase>| {
            phase.0.take();
        })
        .after(Draw),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phase_boundaries_enclose_work_in_the_existing_set_order() {
        let _lock = super::super::tests::LOCK.lock().unwrap();
        let mut app = App::new();
        app.insert_non_send(Vec::<u8>::new());
        build(&mut app);
        for (set, name) in [
            (FrameSet::Prepare, "fixture prepare"),
            (FrameSet::SceneUpdate, "fixture scene"),
            (FrameSet::Draw, "fixture draw"),
        ] {
            app.add_systems(
                Update,
                (move |mut visits: NonSendMut<Vec<u8>>| {
                    let _trace = blackbox_trace::scope(name);
                    visits.push(1);
                })
                .in_set(set),
            );
        }
        assert!(blackbox_trace::global().start(100));
        app.update();
        let capture = blackbox_trace::global().stop().unwrap();
        let mut json = Vec::new();
        capture.write_json(&mut json).unwrap();
        let json = String::from_utf8(json).unwrap();
        assert_eq!(app.world().non_send::<Vec<u8>>().len(), 3);
        let positions: Vec<_> = [
            "fixture prepare",
            "App phase: prepare",
            "fixture scene",
            "App phase: scene update",
            "fixture draw",
            "App phase: draw",
        ]
        .map(|name| json.find(&format!("\"name\":\"{name}\"")).expect(name))
        .into();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
