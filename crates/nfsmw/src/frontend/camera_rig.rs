//! The front end's camera, shared by the backdrops the menus swap between (spec: `docs/specs/menu-backdrop-looks.md`
//! section 4.4). The original has one camera mover that outlives every screen: it plays an intro when a stage comes up and
//! then eases to the pose of each screen that opens, or that is backed out to. A `MenuScene` is rebuilt when the look
//! changes (the main menu to the career menu), so the mover lives here and both ends hold a handle to it.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use game_install::GameDir;
use nfsmw_data::frontend_camera::PoseBook;

use super::camera_move::CameraMove;
use super::camera_pose::CameraPose;
use super::stage::Stage;

/// Seconds that end any intro.
const INTRO_OVER: f32 = 1.0e6;

#[derive(Default)]
struct State {
    /// The install's poses, read when the first intro starts.
    book: Option<PoseBook>,
    /// The stage whose intro the camera played last; another stage plays its own.
    stage: Option<Stage>,
    motion: Option<CameraMove>,
    /// The intro has just started: the first screen's pose is only noted, as the original does while its intro flag is
    /// set, so the screen does not cut the intro short.
    fresh: bool,
    /// The pose collection of the screen the camera last went to.
    shown: Option<&'static str>,
    /// An intro starts at its end (`CameraRig::settled`).
    settled: bool,
}

/// A handle on the camera. Clones share one camera.
#[derive(Clone, Default)]
pub struct CameraRig(Arc<Mutex<State>>);

impl CameraRig {
    fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A camera whose intros are over as they begin, for a picture of the screen at rest (`view-screen --screenshot`).
    pub fn settled() -> Self {
        let rig = Self::default();
        rig.state().settled = true;
        rig
    }

    /// Forgets where the camera was, so the next stage plays its intro even when it is the one played last.
    pub fn reset(&self) {
        let mut state = self.state();
        (state.stage, state.motion, state.shown) = (None, None, None);
    }

    /// A backdrop of `stage` is up. A stage that did not have the camera before plays its intro: the camera is put on
    /// `angle_entry_<intro>` and eases onto `angle_final_<intro>`. One that had it keeps its camera. An install whose poses
    /// cannot be read leaves the camera empty, and the backdrop uses its own.
    pub fn begin(&self, dir: &GameDir, stage: Stage, intro: &str) {
        let mut state = self.state();
        if state.stage == Some(stage) {
            return;
        }
        if state.book.is_none() {
            state.book = PoseBook::load(dir).inspect_err(|e| log::warn!("the menu camera has no poses: {e:#}")).ok();
        }
        let Some(book) = &state.book else { return };
        let (entry, goal) = (book.angle(&format!("angle_entry_{intro}")), book.angle(&format!("angle_final_{intro}")));
        let (Some(entry), Some(goal)) = (entry, goal) else {
            return log::warn!("the menu camera has no intro poses for {intro}");
        };
        let mut motion = CameraMove::between(&entry, &goal);
        if state.settled {
            motion.advance(INTRO_OVER);
        }
        state.motion = Some(motion);
        (state.stage, state.fresh, state.shown) = (Some(stage), true, None);
    }

    /// The screen on top has the pose collection `screen`: the camera eases to its pose from where it is. Asking again for
    /// the same collection does nothing.
    pub fn show(&self, screen: &'static str) {
        let mut state = self.state();
        if state.shown == Some(screen) || state.motion.is_none() {
            return;
        }
        state.shown = Some(screen);
        let first = std::mem::take(&mut state.fresh);
        if first && !state.settled {
            return;
        }
        let Some(goal) = state.book.as_ref().and_then(|book| book.screen(screen)) else { return };
        let settle = first && state.settled;
        if let Some(motion) = &mut state.motion {
            motion.retarget(&goal);
            // A direct screenshot of a submenu needs that screen's rest pose, not the stage intro's final pose.
            if settle {
                motion.advance(INTRO_OVER);
            }
        }
    }

    /// Resume the selected screen's authored move from the player's inspection pose.
    pub(super) fn resume_from(&self, pose: CameraPose) -> bool {
        let mut state = self.state();
        let Some(goal) = state.shown.and_then(|screen| state.book.as_ref()?.screen(screen)) else { return false };
        state.motion = Some(CameraMove::from_pose(pose, &goal));
        true
    }

    /// Inspection belongs to a screen's Bool policy, not to every screen sharing the garage room.
    pub(super) fn user_rotate(&self) -> bool {
        let state = self.state();
        state
            .shown
            .and_then(|screen| state.book.as_ref().map(|book| book.screen_options(screen).user_rotate))
            .unwrap_or(false)
    }

    pub(super) fn moving(&self) -> bool {
        self.state().motion.as_ref().is_some_and(|motion| !motion.finished())
    }

    /// The selected angle also supplies the damping and periods of the transition into inspection.
    pub(super) fn screen_angle(&self) -> Option<nfsmw_data::frontend_camera::CameraAngle> {
        let state = self.state();
        state.shown.and_then(|screen| state.book.as_ref()?.screen(screen))
    }

    /// Lets `dt` seconds go by. Whether the camera has a pose to show.
    pub fn advance(&self, dt: f32) -> bool {
        let mut state = self.state();
        let Some(motion) = &mut state.motion else { return false };
        motion.advance(dt);
        true
    }

    /// Where the camera is, or `None` when no stage has given it a pose.
    pub fn pose(&self) -> Option<CameraPose> {
        self.state().motion.as_ref().map(CameraMove::pose)
    }
}
