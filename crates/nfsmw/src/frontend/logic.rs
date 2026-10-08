//! The seam between a package and the code that gives it meaning: a [`ScreenLogic`] receives the messages the
//! package sends to the game and changes the package's objects, the way the original screen classes do
//! (docs/specs/frontend-menus.md, section 1).

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};
use glam::Vec2;

use crate::settings::{Partial, Settings};
use crate::ui::UiAssets;

/// The option categories the option screens show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Audio,
    Video,
    Gameplay,
}

/// What a screen is opened with (the original passes an argument and reads the game mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Args {
    /// Opened from the pause menu (the screens use their in-game look and return to the pause menu).
    pub pause: bool,
    /// The option categories instead of the screen's usual content (the pause menu shows them in the same package).
    pub options: bool,
    pub category: Category,
}

impl Default for Args {
    fn default() -> Self {
        Self { pause: false, options: false, category: Category::Audio }
    }
}

/// Something a screen asks the front end to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Replace the top screen with another.
    Switch(String, Args),
    Push(String, Args),
    Pop,
    /// Leave the menus and drive.
    StartFreeRoam,
    /// Close the pause menu and go on driving.
    Resume,
    /// Leave the game in progress for the main menu.
    QuitToMenu,
    QuitGame,
    /// Write the settings to the config file.
    SaveSettings,
    /// The boot flow goes on to its next step.
    NextBootStep,
}

/// The settings, and the part of them the menus changed since they were last saved.
pub struct Env<'a> {
    pub settings: &'a mut Settings,
    pub changed: &'a mut Partial,
}

/// What a screen can touch while it handles a message.
pub struct Cx<'a> {
    pub rt: &'a mut Runtime,
    pub package: PackageId,
    pub assets: &'a UiAssets,
    pub settings: &'a mut Settings,
    /// Settings the menus changed; the front end writes them to the config file.
    pub changed: &'a mut Partial,
    pub commands: &'a mut Vec<Command>,
    /// The option index the screen last left on, per screen name.
    pub memory: &'a mut std::collections::HashMap<String, usize>,
    pub name: &'a str,
}

impl Cx<'_> {
    /// The object with this name hash in the screen's package.
    pub fn object(&self, name_hash: u32) -> Option<ObjectRef> {
        self.rt.find(self.package, name_hash)
    }

    /// The object called `name` (any case).
    pub fn named(&self, name: &str) -> Option<ObjectRef> {
        self.rt.find(self.package, fe_hash_upper(name))
    }

    /// Switches an object to a script, if the object exists.
    pub fn script(&mut self, name_hash: u32, script: u32) {
        if let Some(o) = self.object(name_hash) {
            self.rt.run_script(o, script);
        }
    }

    /// Shows a language label on every string directly inside a group (the original sets a label on a group).
    pub fn label_group(&mut self, group_hash: u32, label: u32) {
        let Some(group) = self.object(group_hash) else { return };
        let Some(def) = self.rt.package(self.package) else { return };
        let guid = def.objects[group.index].guid;
        let strings: Vec<usize> = def
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.parent == Some(guid) && o.string.is_some())
            .map(|(i, _)| i)
            .collect();
        for index in strings {
            self.rt.set_label(ObjectRef { package: self.package, index }, label);
        }
    }

    /// Hides or shows an object by name hash (and everything inside it).
    pub fn hide(&mut self, name_hash: u32, hidden: bool) {
        if let Some(o) = self.object(name_hash) {
            self.rt.set_hidden(o, hidden);
        }
    }

    /// Shows a language label on a string object.
    pub fn label(&mut self, o: ObjectRef, label: u32) {
        self.rt.set_label(o, label);
    }

    /// The size of a string object's text as drawn (the object's scale applied), 0 when it cannot be measured.
    pub fn text_size(&self, o: ObjectRef) -> Vec2 {
        crate::ui::text_size(self.rt, self.assets, o)
    }

    /// Puts a string in a column of the screen (parent coordinates) the way its justification says: left aligned
    /// to the column's left edge, centred in the column, or right aligned to its right edge; the text's box is
    /// centred vertically on `y`.
    ///
    /// The original centres the text's box on the column's left, middle or right *edge* (`FEngSetCenter`), which
    /// pushes a right-aligned name half its width out of the column over the arrows; this keeps the text inside
    /// (a deliberate difference, docs/specs/frontend-menus.md section 3).
    pub fn align_string(&mut self, o: ObjectRef, left: f32, width: f32, y: f32) {
        let Some(info) = self.rt.string_info(o) else { return };
        let size = self.text_size(o);
        let anchor_x = left + width * anchor_fraction(info.justification);
        let down = anchor_offset(info.justification, size.y, 4, 8);
        self.rt.set_position_xy(o, anchor_x, y - (down + size.y * 0.5));
    }

    pub fn send(&mut self, command: Command) {
        self.commands.push(command);
    }
}

/// How far across a column a string's anchor sits: left 0, centred a half, right the whole width.
fn anchor_fraction(justification: u32) -> f32 {
    if justification & 1 != 0 {
        return 0.5;
    }
    if justification & 2 != 0 {
        return 1.0;
    }
    0.0
}

/// Where the text starts relative to the string's position along one axis: 0, minus half the length when the
/// justification has the `centre` bit, minus the whole length with the `far` bit.
fn anchor_offset(justification: u32, length: f32, centre: u32, far: u32) -> f32 {
    if justification & centre != 0 {
        return -length * 0.5;
    }
    if justification & far != 0 {
        return -length;
    }
    0.0
}

pub trait ScreenLogic: Send + Sync {
    /// Called once after the package is loaded.
    fn start(&mut self, _cx: &mut Cx) {}

    /// A message the package sent to the game.
    fn message(&mut self, cx: &mut Cx, message: u32);

    /// Every frame after the runtime updated, `dt` in seconds.
    fn tick(&mut self, _cx: &mut Cx, _dt: f32) {}
}

/// A screen without behaviour of its own: its package runs its scripts and nothing answers the game.
pub struct Plain;

impl ScreenLogic for Plain {
    fn message(&mut self, _cx: &mut Cx, _message: u32) {}
}
