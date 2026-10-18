//! The region unlock screen (`SafehouseRegionUnlock.fng`, `docs/specs/career-regions.md` section 7). It shows the rival
//! just beaten over the background picture with the region's text, and waits for Accept, a Continue click or nothing
//! else. The rule (stages, leave time, input) is in `nfsmw_career::regions::unlock`; this file only draws it and sends
//! the game the one command that closes it.
//!
//! The screen is opened in two places with the same `Args` (`safehouse: true`): by the world when a region gate
//! opens (`flow_career::poll_career`, `CareerRequest::RegionUnlock`, the world is paused), and by the Blacklist
//! stepper (step 2, spec 7.1). The two cannot be told apart from the arguments or the stage, and the stepper is not
//! wired to the front end yet, so every Accept or Continue sends `Command::Resume`, which closes the screen and
//! drives on as the world expects. When the stepper is wired, its screen will need its own way back to the stepper.

use blackbox_feng::ids::PAD_ACCEPT;
use blackbox_feng::{fe_hash_upper, resource_handle};
use glam::{Vec2, Vec3};
use nfsmw_career::regions::unlock::FENG_TICKS_PER_SECOND;
use nfsmw_career::regions::{RegionPicture, UnlockEvent, UnlockInput, UnlockScreen, region_unlock_screen};

use crate::career::CareerHandle;
use crate::frontend::ids::{self, screen};
use crate::frontend::logic::{Args, Command, Cx, ScreenLogic};

/// The background picture object: `H_BG` (`fe_hash_upper`, spec 7.2).
const BACKGROUND: u32 = 0x2CBE_1DD0;
/// The label string the region text is set on (`0xD6C0E097`, spec 7.2). Its black twin is not set (unconfirmed).
const LABEL: u32 = 0xD6C0_E097;
/// The message that runs every object's out script after Accept (spec 7.4, step 2).
const OUT_SCRIPTS: u32 = 0x97BD_B465;
/// The Continue button group, by name.
const CONTINUE: &str = "Continue";
/// Half the size of the Continue button in FEng units, around its centre. Not read from the package: a guess until
/// the button's size is decoded (unconfirmed).
const CONTINUE_HALF: Vec2 = Vec2::new(90.0, 24.0);

/// Makes the screen for `name` (lower case) when it is the region unlock screen.
pub fn make(name: &str, _args: Args) -> Option<Box<dyn ScreenLogic>> {
    if name != screen::SAFEHOUSE_REGION_UNLOCK.to_ascii_lowercase() {
        return None;
    }
    Some(Box::new(RegionUnlockLogic::default()))
}

/// The region unlock screen: the state machine of the spec, the picture and the text of the career's stage.
#[derive(Default)]
pub struct RegionUnlockLogic {
    screen: UnlockScreen,
}

impl ScreenLogic for RegionUnlockLogic {
    fn start(&mut self, cx: &mut Cx) {
        let picture = CareerHandle::shared().with(|career| region_unlock_screen(career.stage())).flatten();
        let Some(picture) = picture else {
            // No region opens at this stage: nothing to show, so the screen closes at once.
            cx.send(Command::Resume);
            return;
        };
        show(cx, picture);
        // The package starts with its button off; the intro keeps playing and input is live at once (spec 7.4).
        cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if message != PAD_ACCEPT {
            return;
        }
        self.press(cx, UnlockInput::Accept);
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        let event = self.screen.tick(ticks_for(dt));
        self.handle(cx, event);
    }

    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        let Some(centre) = continue_centre(cx) else { return false };
        if !is_continue_point(point, centre) {
            return false;
        }
        self.press(cx, UnlockInput::MouseReleaseOnContinue);
        true
    }
}

impl RegionUnlockLogic {
    fn press(&mut self, cx: &mut Cx, input: UnlockInput) {
        let event = self.screen.input(input);
        self.handle(cx, event);
    }

    /// Runs what the state machine did: the out scripts after Accept, the game's command when the leave is over.
    fn handle(&mut self, cx: &mut Cx, event: UnlockEvent) {
        if let UnlockEvent::Accepted { .. } = event {
            cx.rt.post_to_package(cx.package, OUT_SCRIPTS);
        }
        if let Some(command) = command_for(event) {
            cx.send(command);
        }
    }
}

/// Sets the region's text and background picture. The background is named `RIVAL_<key>_BG`; its texture comes from
/// `GLOBAL/RIVALS.BIN`, which the UI assets load (`ui::assets`, spec 8).
fn show(cx: &mut Cx, picture: RegionPicture) {
    if let Some(label) = cx.object(LABEL) {
        cx.label(label, picture.label_hash);
    }
    let Some(background) = cx.object(BACKGROUND) else { return };
    cx.rt.set_texture(background, resource_handle(&picture.texture_name()));
}

/// The centre of the Continue button on the screen, from its node in the package tree.
fn continue_centre(cx: &Cx) -> Option<Vec2> {
    let hash = fe_hash_upper(CONTINUE);
    let tree = cx.rt.tree(cx.package);
    let node = tree.nodes.iter().find(|n| n.name_hash == hash)?;
    Some(node.world.transform_point3(Vec3::ZERO).truncate())
}

/// The FEng ticks in `dt` seconds (1200 a second), rounded. A negative time counts as none.
pub(super) fn ticks_for(dt: f32) -> u32 {
    (dt.max(0.0) * FENG_TICKS_PER_SECOND as f32).round() as u32
}

/// Whether a point is on the Continue button around `centre`.
pub(super) fn is_continue_point(point: Vec2, centre: Vec2) -> bool {
    (point - centre).abs().cmple(CONTINUE_HALF).all()
}

/// The command the game gets for an event: the leave is over, so the screen closes and the world goes on driving.
pub(super) fn command_for(event: UnlockEvent) -> Option<Command> {
    match event {
        UnlockEvent::Done => Some(Command::Resume),
        UnlockEvent::None | UnlockEvent::Accepted { .. } => None,
    }
}

#[cfg(test)]
#[path = "region_unlock_tests.rs"]
mod tests;
