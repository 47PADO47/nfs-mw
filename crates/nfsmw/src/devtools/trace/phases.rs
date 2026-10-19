//! Wall-time boundaries around the application's ordered Update sets, including uninstrumented work.
use crate::app::FrameSet;
use bevy_app::{App, Update};
use bevy_ecs::prelude::*;

#[derive(Default)]
struct Phase(Option<blackbox_trace::Scope<'static>>);

fn enter(mut phase: NonSendMut<Phase>, name: &'static str) {
    phase.0.take();
    phase.0 = Some(blackbox_trace::scope(name));
}

pub(super) fn build(app: &mut App) {
    use FrameSet::*;
    app.insert_non_send(Phase::default())
        .add_systems(Update, (|phase: NonSendMut<Phase>| enter(phase, "App phase: prepare")).before(Prepare))
        .add_systems(
            Update,
            (|phase: NonSendMut<Phase>| enter(phase, "App phase: commands")).after(Prepare).before(Commands),
        )
        .add_systems(
            Update,
            (|phase: NonSendMut<Phase>| enter(phase, "App phase: mods pre")).after(Commands).before(ModsPre),
        )
        .add_systems(
            Update,
            (|phase: NonSendMut<Phase>| enter(phase, "App phase: frontend")).after(ModsPre).before(Frontend),
        )
        .add_systems(
            Update,
            (|phase: NonSendMut<Phase>| enter(phase, "App phase: scene update")).after(Frontend).before(SceneUpdate),
        )
        .add_systems(
            Update,
            (|phase: NonSendMut<Phase>| enter(phase, "App phase: mods post")).after(SceneUpdate).before(ModsPost),
        )
        .add_systems(
            Update,
            (|phase: NonSendMut<Phase>| enter(phase, "App phase: UI / audio")).after(ModsPost).before(Ui),
        )
        .add_systems(Update, (|phase: NonSendMut<Phase>| enter(phase, "App phase: HUD")).after(Ui).before(Hud))
        .add_systems(Update, (|phase: NonSendMut<Phase>| enter(phase, "App phase: draw")).after(Hud).before(Draw))
        .add_systems(
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
