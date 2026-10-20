//! The bust's whole return to driving through the front end and the paused scene wrapper: the arrest hold, the ticket
//! screen opening over the frozen world, the cash payment pressed on the real `Infractions.fng`, and the world driving
//! again afterwards (`docs/specs/busted-sequence.md` sections 6 and 7). Skipped without `NFSMW_GAME_DIR`.

use std::sync::Arc;

use blackbox_feng::runtime::pad;
use game_install::GameDir;

use super::flow::{Frontend, Stage, Start};
use super::ids::screen::INFRACTIONS;
use super::logic::Env;
use super::scene::Pausable;
use super::screens::Screens;
use crate::app::{Host, test_backend::NullBackend};
use crate::input::{ActionState, Bindings};
use crate::scenes::world::{ArrestFixture, handbrake, open_arrest_fixture};

use crate::settings::{Partial, Settings, test_caps};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};
use bevy_input::keyboard::KeyCode;

const FRAME: f32 = 1.0 / 60.0;
const STEP: f32 = FRAME;

/// The front end over a paused-capable scene, the settings it reads and the renderer the scene updates with.
struct Rig {
    fe: Frontend,
    host: Host,
    settings: Settings,
    changed: Partial,
    renderer: NullBackend,
}

impl Rig {
    fn env(&mut self) -> (&mut Frontend, &mut Host, Env<'_>) {
        let env =
            Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
        (&mut self.fe, &mut self.host, env)
    }

    /// One frame as the plugin runs it while playing: the world updates (the paused wrapper ignores that while a
    /// screen holds the game), then the front end polls the bust.
    fn play_frame(&mut self, input: &ActionState) {
        self.host.scene.update(&mut self.renderer, input, STEP);
        let (fe, host, mut env) = self.env();
        fe.poll_bust(host, &mut env);
    }

    /// One frame of the ticket screen with the pad `mask` held.
    fn screen_frame(&mut self, mask: u32) {
        let (fe, host, mut env) = self.env();
        let commands = fe.screens.update(FRAME, mask, &mut env);
        fe.apply(commands, host, &mut env);
    }
}

fn rig_around(scene: crate::scenes::world::WorldScene, dir: GameDir) -> Rig {
    let assets = Arc::new(UiAssets::load(&dir).unwrap());
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let mut fe = Frontend::new(Screens::new(catalog, assets), dir, Start::Drive, None);
    fe.stage = Stage::Playing;
    fe.latch = false;
    let wrapped = Pausable::new(Box::new(scene), fe.paused.clone());
    let settings = Settings::from(Partial::default());
    let mut host = Host::new(Box::new(wrapped), &settings, None);
    host.renderer = Some(Box::new(NullBackend::new(test_caps::native())));
    Rig { fe, host, settings, changed: Partial::default(), renderer: NullBackend::new(test_caps::native()) }
}

#[test]
#[ignore = "needs the installed world, the infractions screen and the player/cop physics"]
fn installed_bust_ticket_and_payment_return_to_driving_through_the_front_end() {
    let Some(mut fixture) = open_arrest_fixture() else { return };
    let stopped = handbrake();
    assert!(fixture.run_to_arrest(600), "the cop busts the stopped player");
    let ArrestFixture { scene, renderer, .. } = fixture;
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut rig = rig_around(scene, dir);
    rig.renderer = renderer;

    // The hold runs, then the front end opens the ticket over the frozen world.
    let mut opened = false;
    for _ in 0..600 {
        rig.play_frame(&stopped);
        opened = rig.fe.stage == Stage::Paused;
        if opened {
            break;
        }
    }
    assert!(opened, "the ticket opens after the arrest hold");
    assert!(rig.fe.paused.get(), "the world is held under the ticket");
    assert_eq!(rig.fe.screens.top(), Some(INFRACTIONS));

    // Let the screen finish its entrance, then press the focused (cash) button.
    for _ in 0..90 {
        rig.screen_frame(0);
    }
    assert_eq!(rig.fe.stage, Stage::Paused, "nothing pays by itself");
    for _ in 0..3 {
        rig.screen_frame(pad::ACCEPT);
    }
    for _ in 0..3 {
        rig.screen_frame(0);
    }
    assert_eq!(rig.fe.stage, Stage::Playing, "the payment resumes the game");
    assert!(!rig.fe.paused.get(), "the world runs again");
    assert_eq!(rig.fe.screens.top(), None, "the ticket is gone");

    // The player drives off, the police and the chase are gone, and nothing re-arrests or re-opens the ticket.
    let mut accelerate = ActionState::default();
    accelerate.update_for_test(&Bindings::default(), &[KeyCode::ArrowUp], &[], false);
    let start = rig.host.scene.player_car().expect("the player car").position;
    let mut fastest = 0.0f32;
    for _ in 0..180 {
        rig.play_frame(&accelerate);
        assert_eq!(rig.fe.stage, Stage::Playing, "no second ticket");
        fastest = fastest.max(rig.host.scene.player_car().map_or(0.0, |car| car.speed));
    }
    let state = rig.host.scene.hud_state().expect("the HUD is back");
    assert!(state.visible, "the HUD shows again");
    assert!(state.pursuit.is_none(), "no pursuit board after the payment");
    assert!(rig.host.scene.pursuit_feed().is_none(), "no chase for the audio");
    assert!(rig.host.scene.take_bust_ticket().is_none());
    let end = rig.host.scene.player_car().expect("the player car").position;
    let moved = glam::Vec3::from_array(end).distance(glam::Vec3::from_array(start));
    assert!(fastest > 3.0 && moved > 5.0, "the player drives off: {fastest} m/s, {moved} m");
}
