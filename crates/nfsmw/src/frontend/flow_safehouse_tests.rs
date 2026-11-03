use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::*;
use crate::career::{Career, CareerData, CareerHandle};
use crate::frontend::flow_movie_tests::{Rig, install_rig};
use crate::frontend::ids::screen;
use crate::frontend::logic::{Command, QuitTo};
use crate::viewer::Scene;

struct WorldProbe {
    race: bool,
    pursuit: bool,
    prepared: Arc<AtomicUsize>,
}
impl Scene for WorldProbe {
    fn title(&self) -> String {
        "synthetic world".into()
    }
    fn init(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (blackbox_gfx::FrameParams, &[blackbox_gfx::Instance]) {
        unreachable!()
    }
    fn race_active(&self) -> bool {
        self.race
    }
    fn pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        self.pursuit.then_some(crate::audio::PursuitFeed { heat: 3, ..Default::default() })
    }
    fn prepare_safe_house(&mut self) -> bool {
        self.prepared.fetch_add(1, Ordering::Relaxed);
        true
    }
}

fn rig() -> Rig {
    let mut rig = install_rig().expect("installed menus");
    let data = CareerData::load(&rig.fe.dir).unwrap();
    let mut career = Career::new("synthetic", Arc::new(data), &Default::default()).unwrap();
    career.settings.stage = 15;
    career.blacklist = nfsmw_career::blacklist::Blacklist::new(career.data.bins.clone(), 15).unwrap();
    assert_eq!(career.stage(), 15, "the shortcut fixture must be past the prologue");
    rig.fe.career = CareerHandle::shared();
    rig.fe.career.set(career);
    rig.fe.career.begin_play();
    rig.fe.origin = QuitTo::Career;
    rig.fe.save = false;
    rig.fe.profiles = None;
    rig
}

#[test]
#[ignore = "requires installed menu/career tables; set NFSMW_GAME_DIR"]
fn batch12_pause_safe_house_rechecks_context_then_enters_the_room() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    for pursuit in [false, true] {
        let mut rig = rig();
        let prepared = Arc::new(AtomicUsize::new(0));
        rig.with(|fe, host, env| {
            assert!(fe.replace_scene(
                host,
                Box::new(WorldProbe { race: true, pursuit, prepared: prepared.clone() }),
                false
            ));
            fe.stage = Stage::Paused;
            assert_eq!(fe.pause_safe_house(host), None, "races do not offer the free-roam shortcut");
            fe.apply(vec![Command::JumpToSafeHouse], host, env);
            assert_eq!(prepared.load(Ordering::Relaxed), 0);
            assert!(fe.replace_scene(
                host,
                Box::new(WorldProbe { race: false, pursuit, prepared: prepared.clone() }),
                false
            ));
            assert_eq!(fe.pause_safe_house(host), Some(true));
            fe.apply(vec![Command::JumpToSafeHouse], host, env);
            assert_eq!(fe.stage, Stage::Menus);
            assert!(fe.in_safe_house && fe.career.is_prelude() && fe.career.is_active());
            assert_eq!(fe.screens.top(), Some(screen::MAIN_MENU_SUB));
            assert_eq!(prepared.load(Ordering::Relaxed), 1, "the old world's pending chase was handled once");
            fe.apply(vec![Command::JumpToSafeHouse], host, env);
            assert_eq!(prepared.load(Ordering::Relaxed), 1, "repeated commands cannot repeat the jump");
        });
    }
    CareerHandle::shared().clear();
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn batch12_garage_loading_stays_visible_and_back_requires_release() {
    let mut rig = install_rig().expect("installed menus");
    let mut overlay = crate::garage::Overlay::default();
    rig.with(|fe, host, env| {
        fe.menu_garage = true;
        fe.menu_garage_requested = true;
        fe.latch = true;
        fe.garage_loading_frame(0.2, blackbox_feng::runtime::pad::BACK, host, env, &mut overlay);
        assert_eq!(fe.screens.top(), Some(screen::LOADING));
        assert!(fe.menu_garage_requested && host.hold_capture);
        fe.garage_loading_frame(0.2, 0, host, env, &mut overlay);
        fe.garage_loading_frame(0.2, blackbox_feng::runtime::pad::BACK, host, env, &mut overlay);
        assert!(!fe.menu_garage_requested && !overlay.menu_requested);
        assert_eq!(fe.screens.top(), Some(screen::MAIN_MENU));
    });
}
