//! UI-060: Performance Tuning that cannot open must hand the pause menu back. The pause menu plays its leave before the
//! request arrives, so a quiet refusal leaves it faded with its input off.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use super::*;
use crate::frontend::flow_movie_tests::install_rig;
use crate::frontend::ids::screen;
use crate::viewer::Scene;
use blackbox_feng::runtime::pad;

const FRAME: f32 = 1.0 / 60.0;

/// A driving scene whose tuning availability and session start the test controls.
struct Tunable {
    available: Arc<AtomicBool>,
    begin_fails: bool,
    cancelled: Arc<AtomicUsize>,
}

impl Scene for Tunable {
    fn title(&self) -> String {
        "synthetic driving".into()
    }
    fn init(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (blackbox_gfx::FrameParams, &[blackbox_gfx::Instance]) {
        unreachable!()
    }
    fn tuning_available(&self) -> Option<bool> {
        Some(self.available.load(Ordering::Relaxed))
    }
    fn begin_tuning(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
        if self.begin_fails {
            anyhow::bail!("A garage session is already open");
        }
        Ok(())
    }
    fn cancel_garage(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
        self.cancelled.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

/// Runs the screens for `seconds` with `mask` held, applying their commands as the plugin does.
fn frames(fe: &mut Frontend, host: &mut Host, env: &mut Env, mask: u32, seconds: f32) {
    for _ in 0..(seconds / FRAME).ceil() as usize {
        let commands = fe.screens.update(FRAME, mask, env);
        fe.apply(commands, host, env);
    }
}

fn press(fe: &mut Frontend, host: &mut Host, env: &mut Env, button: u32) {
    frames(fe, host, env, button, 2.0 * FRAME);
    frames(fe, host, env, 0, 1.0);
}

#[test]
#[ignore = "requires installed pause/dialog packages; set NFSMW_GAME_DIR"]
fn ui060_refused_tuning_returns_a_usable_pause_menu() {
    let mut rig = install_rig().expect("installed menus");
    let available = Arc::new(AtomicBool::new(true));
    let cancelled = Arc::new(AtomicUsize::new(0));
    let scene = Tunable { available: available.clone(), begin_fails: false, cancelled };
    rig.with(|fe, host, env| {
        assert!(fe.replace_scene(host, Box::new(scene), false));
        fe.pause(host, env);
        frames(fe, host, env, 0, 1.5);
        // Resume, Options, Performance Tuning: the entry was offered when the menu opened.
        press(fe, host, env, pad::RIGHT);
        press(fe, host, env, pad::RIGHT);
        available.store(false, Ordering::Relaxed);
        press(fe, host, env, pad::ACCEPT);
        assert_eq!(fe.screens.top(), Some(screen::DIALOG), "a refused request explains itself");
        press(fe, host, env, pad::ACCEPT);
        assert_eq!(fe.screens.top(), Some(screen::PAUSE_MENU));
        assert_eq!(fe.stage, Stage::Paused);
        frames(fe, host, env, 0, 1.0);
        press(fe, host, env, pad::BACK);
        frames(fe, host, env, 0, 1.0);
        assert_ne!(fe.stage, Stage::Paused, "the pause menu takes input again: Back resumes the game");
    });
}

#[test]
#[ignore = "requires installed pause/dialog packages; set NFSMW_GAME_DIR"]
fn ui060_a_garage_session_without_screens_is_released_before_tuning() {
    let mut rig = install_rig().expect("installed menus");
    let cancelled = Arc::new(AtomicUsize::new(0));
    let scene = Tunable { available: Arc::new(AtomicBool::new(true)), begin_fails: true, cancelled: cancelled.clone() };
    rig.with(|fe, host, env| {
        assert!(fe.replace_scene(host, Box::new(scene), false));
        fe.pause(host, env);
        frames(fe, host, env, 0, 1.5);
        // A session whose pages all closed without `close_garage`.
        fe.garage_ui = Some(GarageUi::new());
        fe.garage_tuning = true;
        fe.apply(vec![Command::OpenTuning], host, env);
        assert!(fe.garage_ui.is_none() && !fe.garage_tuning, "the stale session must not block tuning");
        assert_eq!(cancelled.load(Ordering::Relaxed), 1, "its unconfirmed selection is rolled back once");
        assert_eq!(fe.screens.top(), Some(screen::DIALOG), "the failed start is reported, not ignored");
    });
}

#[test]
fn ui060_a_garage_session_is_detached_once_no_screen_shares_it() {
    let ui = GarageUi::new();
    assert!(ui.is_detached());
    let page = ui.logic(Page::Tuning);
    assert!(!ui.is_detached());
    drop(page);
    assert!(ui.is_detached());
}
