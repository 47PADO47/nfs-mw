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

// The same bust in a career: the ticket settles the car's record, then the career car select, the safe house menu and
// the drive (`docs/specs/busted-aftermath.md`).

use super::ids::screen::{CAR_SELECT, DIALOG, MAIN_MENU_SUB};
use super::logic::Command;
use crate::career::{Career, CareerData, CareerHandle};
use nfsmw_career::settings::NewCareer;
use nfsmw_career::stable::{CarRecord, CareerRecord, ImpoundState};

/// Two owned cars of the install's lot (handles 1 and 2), the first being driven, with `cash` and `strikes` on it.
fn career_of(dir: &GameDir, cash: u32, strikes: i8) -> Career {
    let data = CareerData::load(dir).expect("the career tables load");
    assert!(data.cars.len() >= 2, "the lot has cars");
    let (first, second) = (data.cars[0].frontend_key, data.cars[1].frontend_key);
    let mut career = Career::new("bust", Arc::new(data), &NewCareer::default()).unwrap();
    for (slot, (handle, key)) in [(1u32, first), (2, second)].into_iter().enumerate() {
        career.stable.cars[slot] = CarRecord {
            handle,
            frontend_key: key,
            flags: 0xF_0000 | CarRecord::FLAG_OWNED,
            career_index: slot as u8,
            customization_index: slot as u8,
            ..CarRecord::free()
        };
        career.stable.careers[slot] = CareerRecord::new(slot as u8);
    }
    career.stable.careers[0].strikes = strikes;
    career.stable.careers[0].heat = 3.5;
    career.settings.cash = cash;
    career.settings.car_handle = 1;
    career
}

fn installed_dir() -> GameDir {
    GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap()
}

/// A rig whose world and front end play `career`, stopped by the cop, with the ticket up.
fn career_rig_at_the_ticket(career: Career) -> Rig {
    let Some(mut fixture) = open_arrest_fixture() else { panic!("the arrest fixture needs the installed game") };
    let shared = CareerHandle::shared();
    shared.set(career);
    let stopped = handbrake();
    assert!(fixture.run_to_arrest(600), "the cop busts the stopped player");
    let ArrestFixture { mut scene, renderer, .. } = fixture;
    scene.set_career(shared.clone(), None);
    let mut rig = rig_around(scene, installed_dir());
    rig.renderer = renderer;
    rig.fe.career = shared;
    let mut opened = false;
    for _ in 0..600 {
        rig.play_frame(&stopped);
        opened = rig.fe.stage == Stage::Paused;
        if opened {
            break;
        }
    }
    assert!(opened, "the ticket opens after the arrest hold");
    assert_eq!(rig.fe.screens.top(), Some(INFRACTIONS));
    rig
}

impl Rig {
    /// Frames of the top screen with nothing pressed, then `mask` pressed for three frames and released.
    fn press(&mut self, mask: u32) {
        for _ in 0..90 {
            self.screen_frame(0);
        }
        for _ in 0..3 {
            self.screen_frame(mask);
        }
        for _ in 0..3 {
            self.screen_frame(0);
        }
    }

    /// Runs commands as the front end does when a screen sends them.
    fn run(&mut self, commands: Vec<Command>) {
        let (fe, host, mut env) = self.env();
        fe.apply(commands, host, &mut env);
    }

    /// The driven car's record.
    fn record(&self) -> CareerRecord {
        let record =
            CareerHandle::shared().with(|career| career.stable.career_for_handle(career.settings.car_handle).copied());
        record.flatten().expect("the driven car has a record")
    }

    /// Drives for three seconds: no second ticket, and the car moves.
    fn drives_on(&mut self) {
        let mut accelerate = ActionState::default();
        accelerate.update_for_test(&Bindings::default(), &[KeyCode::ArrowUp], &[], false);
        let start = self.host.scene.player_car().expect("the player car").position;
        let mut fastest = 0.0f32;
        for _ in 0..180 {
            self.play_frame(&accelerate);
            assert_eq!(self.fe.stage, Stage::Playing, "no second ticket");
            fastest = fastest.max(self.host.scene.player_car().map_or(0.0, |car| car.speed));
        }
        let end = self.host.scene.player_car().expect("the player car").position;
        let moved = glam::Vec3::from_array(end).distance(glam::Vec3::from_array(start));
        assert!(fastest > 3.0 && moved > 5.0, "the player drives off: {fastest} m/s, {moved} m");
    }
}

#[test]
#[ignore = "needs the installed world, the career tables, the infractions screen and the car select"]
fn installed_career_bust_cash_payment_leads_to_the_car_select_the_safe_house_and_driving() {
    let mut rig = career_rig_at_the_ticket(career_of(&installed_dir(), 5_000, 0));
    rig.press(pad::ACCEPT);

    // Cash paid: one strike, the heat back to 1, the car kept; the career car select is up over the frozen world.
    let record = rig.record();
    assert_eq!((record.strikes, record.heat, record.state), (1, 1.0, 0), "{record:?}");
    assert_eq!(record.busted, 1);
    assert_eq!(rig.fe.screens.top(), Some(CAR_SELECT), "the ticket leads to the car select");
    assert_eq!(rig.fe.stage, Stage::Paused);
    assert!(rig.fe.paused.get(), "the world is still held");

    // Accept on the free car goes to the safe house menu, whose resume drives again.
    rig.press(pad::ACCEPT);
    assert_eq!(rig.fe.screens.top(), Some(MAIN_MENU_SUB), "a free car leads to the safe house menu");
    rig.run(vec![Command::Resume]);
    assert_eq!(rig.fe.stage, Stage::Playing);
    assert!(!rig.fe.paused.get());
    assert!(rig.host.scene.take_bust_ticket().is_none());
    rig.drives_on();
    CareerHandle::shared().clear();
}

#[test]
#[ignore = "needs the installed world, the career tables, the infractions screen and the car select"]
fn installed_career_bust_third_strike_impounds_the_car_and_the_player_must_choose_another() {
    let mut rig = career_rig_at_the_ticket(career_of(&installed_dir(), 5_000, 2));
    rig.press(pad::ACCEPT);

    // The limit was reached: the car is impounded and the notice comes before the car select.
    let record = rig.record();
    assert_eq!((record.state, record.strikes, record.countdown), (ImpoundState::LimitReached.byte(), 3, 5));
    assert_eq!(rig.fe.screens.top(), Some(DIALOG), "the impound notice");
    rig.press(pad::ACCEPT);
    assert_eq!(rig.fe.screens.top(), Some(CAR_SELECT), "closing the notice opens the car select");

    // Back is refused while the car the player came with is impounded.
    rig.press(pad::BACK);
    assert_eq!(rig.fe.screens.top(), Some(DIALOG), "the must-choose-a-new-car notice");
    rig.press(pad::ACCEPT);
    assert_eq!(rig.fe.screens.top(), Some(CAR_SELECT));

    // Accept on the impounded car cannot release it (no cash for the fee, no marker): a notice, and the car select stays.
    rig.press(pad::ACCEPT);
    assert_eq!(rig.fe.screens.top(), Some(DIALOG), "the car cannot be released");
    rig.press(pad::ACCEPT);
    assert_eq!(rig.fe.screens.top(), Some(CAR_SELECT));

    // The second car is free: moving to it makes it the current car, and accepting it leaves for the safe house.
    rig.press(pad::RIGHT);
    assert_eq!(CareerHandle::shared().with(|career| career.settings.car_handle), Some(2));
    rig.press(pad::ACCEPT);
    assert_eq!(rig.fe.screens.top(), Some(MAIN_MENU_SUB));
    rig.run(vec![Command::Resume]);
    assert_eq!(rig.fe.stage, Stage::Playing);
    rig.drives_on();
    let flags = CareerHandle::shared().with(|career| career.settings.flags).unwrap();
    assert!(!flags.contains(nfsmw_career::settings::Flags::GAME_OVER), "another car is free");
    CareerHandle::shared().clear();
}
