//! Screen logic callbacks used by the front-end host.

use blackbox_feng::{PackageId, Runtime};
use glam::Vec2;

use super::{Cx, Footer, Pick, UiAssets};
use crate::ui::theme::MenuList;

pub trait ScreenLogic: Send + Sync {
    /// Whether a transition currently permits new logical input.
    fn accepts_input(&self) -> bool {
        true
    }

    /// Filters or replaces a package sound's candidate trigger.
    fn sound_trigger(
        &mut self,
        _runtime: &Runtime,
        _package: PackageId,
        _message: u32,
        cue: Option<i32>,
    ) -> Option<i32> {
        cue
    }
    /// The screen logic's type, naming its sound gate in the audio trace.
    fn sound_gate_name(&self) -> &'static str {
        let name = std::any::type_name::<Self>();
        name.rsplit("::").next().unwrap_or(name)
    }
    fn holds_photo_audio_pause(&self) -> bool {
        false
    }

    /// Full pages keep their parent for Back without drawing it underneath; dialogs leave it visible.
    fn covers_screens(&self) -> bool {
        false
    }

    /// Called once after the package is loaded.
    fn start(&mut self, _cx: &mut Cx) {}

    /// Called when the screen above it closes and this one is on top again.
    fn resume(&mut self, _cx: &mut Cx) {}

    /// A message the package sent to the game.
    fn message(&mut self, cx: &mut Cx, message: u32);

    /// A button of the package was pressed (`button` is its GUID). Only packages whose buttons answer
    /// `BUTTON_PRESSED` report this, and it comes whether or not the package also posts a message to the game.
    fn pressed(&mut self, _cx: &mut Cx, _button: u32) {}

    /// The pad buttons newly pressed this frame, as `blackbox_feng::runtime::pad` bits, whether or not the package
    /// answers them with a game message (the post-race pages' second button has no game response).
    fn pad_pressed(&mut self, _cx: &mut Cx, _bits: u32) {}

    /// The device whose Start or accept key went down this frame (the split-screen Press Start screens seat a player with it).
    fn start_pressed(&mut self, _cx: &mut Cx, _device: crate::input::PlayerDevice) {}

    /// Every frame after the runtime updated, `dt` in seconds.
    fn tick(&mut self, _cx: &mut Cx, _dt: f32) {}

    /// A click at a point of the screen (FEng units, origin at the centre). Returns whether the screen took it.
    fn click(&mut self, _cx: &mut Cx, _point: Vec2) -> bool {
        false
    }

    /// The pointer moved over a point of the screen (FEng units). A screen that shows focus follows it.
    fn hover(&mut self, _cx: &mut Cx, _point: Vec2) {}

    /// The mouse wheel turned `steps` notches, positive is up.
    fn scroll(&mut self, _cx: &mut Cx, _steps: i32) {}

    /// The screen's entries as a list, for a theme that draws them its own way (the Remastered vertical menus). Only the
    /// icon menus have one. The hints and the aspect are filled in by [`Screens`](crate::frontend::screens::Screens).
    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        None
    }

    /// Whether the screen is laid out as a vertical list now. Up and Down then move the selection, and a click or the
    /// pointer picks a row ([`ScreenLogic::pick_row`]) instead of an icon of the package.
    fn vertical(&self) -> bool {
        false
    }

    /// Whether Up and Down are turned into Left and Right for the package: the icon row's package knows no Up and Down.
    fn rotate_pad(&self) -> bool {
        false
    }

    /// Whether the screen takes typed text (`typed`): the keyboard then feeds it characters, and the pad's letter and
    /// Backspace bindings stay out of the way.
    fn wants_text(&self) -> bool {
        false
    }

    /// A key typed while the screen takes text.
    fn typed(&mut self, _cx: &mut Cx, _key: crate::frontend::name_entry::TextKey) {}

    /// The footer hints of the screen (`prompts::hint_set`).
    fn footer(&self) -> Footer {
        Footer::Standard
    }

    /// The current backdrop, or `None` to use the package's look (`factory::look_for`).
    fn look(&self) -> Option<crate::frontend::look::Look> {
        None
    }

    /// Override the package's camera collection when this logic selects its context.
    fn pose(&self) -> Option<&'static str> {
        None
    }

    /// The pointer is over, or pressed, the entry `row` of the vertical list.
    fn pick_row(&mut self, _cx: &mut Cx, _row: usize, _pick: Pick) {}
}

/// A screen without behaviour of its own: its package runs its scripts and nothing answers the game.
pub struct Plain;

impl ScreenLogic for Plain {
    fn message(&mut self, _cx: &mut Cx, _message: u32) {}
}
