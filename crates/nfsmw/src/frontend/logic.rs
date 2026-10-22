//! The seam between a package and the code that gives it meaning: a [`ScreenLogic`] receives the messages the
//! package sends to the game and changes the package's objects, the way the original screen classes do
//! (docs/specs/frontend-menus.md, section 1).

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};
use blackbox_gfx::Capabilities;
use glam::Vec2;
use nfsmw_career::safehouse::BlacklistStepper;

use super::confirm::{Alert, Confirm};
use super::world_map::WorldMapFrame;
use crate::input::Control;
use crate::settings::{Partial, Settings, UiStyle};
use crate::ui::UiAssets;
use crate::ui::theme::MenuList;

/// The option categories the option screens show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Audio,
    Video,
    Gameplay,
    Player,
    Controls,
    /// Discord Rich Presence: its own icon and sub-screen (the toggles are `options_rows.rs`'s rows).
    Discord,
    /// The rows of the custom race's options (`custom_race/settings.rs`, `track_options.rs`).
    CustomRace,
    /// The rows the mods registered (`bb.menus`): listed on the page only when a mod has rows (`mod_options.rs`).
    Mods,
}

/// Which icon row `MainMenu_Sub.fng` is. The package is the option categories, the career menu and the Quick Race menu
/// (the original's `UIOptionsMain`, the career main menu and `UIQRMainMenu` all run on it); the safe house is its own
/// flag (`Args::safehouse`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sub {
    #[default]
    Options,
    /// Resume Career, Start New Career, Load (`career_screens/career_menu.rs`).
    Career,
    /// Quick Play, Custom Race, Split Screen (`race_menu.rs`).
    QuickRace,
    /// The kinds of custom race (`custom_race/`).
    CustomRace,
}

/// Where the pause menu's quit item leads, and so what it says: the menu the game in progress was started from
/// (`docs/specs/career-parity-menus.md`, section 5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QuitTo {
    /// Free roam or a race started from the main menu: Quit to Main Menu.
    #[default]
    Main,
    /// A Quick Race started from the Quick Race menu: Quit to Quick Race Menu.
    QuickRace,
    /// The career: Quit to Career Main Menu.
    Career,
}

/// What the pause menu offers: a race is running (Resume Race and Restart instead of Resume Free Roam) and where Quit goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PauseCtx {
    pub race: bool,
    pub quit: QuitTo,
}

pub use super::footer::Footer;

mod args;
pub use args::Args;
/// Something a screen asks the front end to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Replace the top screen with another.
    Switch(String, Args),
    Push(String, Args),
    Pop,
    /// Ask a question with two answers on top of the current screen (`confirm.rs`).
    Confirm(Confirm),
    /// Show a notice with one answer on top of the current screen (`confirm.rs`).
    #[allow(dead_code)] // no screen raises one yet
    Alert(Alert),
    /// Leave the menus and drive, outside career mode.
    #[allow(dead_code)] // nothing sends it since Career starts a career; Quick Race will
    StartFreeRoam,
    /// The challenge series: leave the menus and start this event (a gameplay collection name) with the transmission
    /// the player chose, outside career mode (`challenge_series/`).
    StartChallenge {
        race: String,
        transmission: crate::settings::Transmission,
    },
    /// Play a tutorial movie (`MOVIES/<name>`) full screen, then come back to the screen that asked (`back`, opened with
    /// `args`).
    PlayTutorial {
        movie: String,
        back: String,
        args: Args,
    },
    /// Play any movie (`MOVIES/<name>`, a tutorial or a story movie) full screen, then do what `then` says: open a screen
    /// or run a command (`flow_movie.rs`). The career sends it for the story movies.
    #[allow(dead_code)] // no caller yet; the career's story movies will send it
    PlayMovie {
        movie: String,
        then: super::flow::flow_movie::AfterMovie,
    },
    /// The custom race's options page, Defaults answered Yes: the current mode's options go back to their defaults.
    CustomDefaults,
    /// Custom Race, after the transmission: start this race (a gameplay collection name) with this car and options
    /// (`custom_race/start.rs`).
    StartCustomRace(Box<CustomStart>),
    /// Split Screen, after player 1's car (and gearbox): store the gearbox, then Press Start for player 2.
    SplitFirstCar(crate::settings::Transmission),
    /// Split Screen, after player 2's car: start this race with the two players (`flow_split.rs`).
    StartSplitRace {
        race: String,
        options: crate::scenes::world::RaceOptions,
        setup: crate::scenes::world::SplitSetup,
    },
    /// Close the pause menu and go on driving.
    Resume,
    /// The first car is picked in the car lot: start the drive to the first safe house in the world with it (`flow_prologue.rs`).
    StartFirstDrive,
    /// Set the GPS route to a point on the road (render space x, y), then close the map and go on driving.
    SetWaypoint([f32; 2]),
    /// Show the lap table of a race (`PostRace_Results.fng`) in place of the screen on top (`photo.rs`).
    ShowResults(crate::viewer::RaceReport),
    /// The post-race screen's answer: run the race again or leave it (`post_race.rs`).
    RaceAnswer(crate::viewer::RaceAnswer),
    /// Pay the bust's fines on the infractions ticket, with a marker or with cash. The bust takes it (`flow.rs`); the
    /// game goes on driving when it does.
    PayBust {
        marker: bool,
    },
    /// Leave the game in progress for the menu it was started from (`flow.rs`, `quit_to`).
    QuitTo(QuitTo),
    /// Run the race in progress again from its grid (the pause menu's Restart).
    RestartRace,
    /// Career menu, Resume Career: play the loaded profile, into the safe house menu or the prologue (`flow_career.rs`).
    ResumeCareer,
    /// Leave the safe house menu that stands before the world for the career menu (its Back).
    LeaveSafeHouse,
    /// The Blacklist's challenge button, accepted for this bin: the world starts its first boss race (`flow_rival.rs`).
    ChallengeRival {
        bin: u8,
    },
    /// The Blacklist sequence goes on from this stepper state: one call of the stepper and the screen or movie it names
    /// (`flow_first_visit.rs`).
    BlacklistStep(BlacklistStepper),
    /// Ask for an alias name over the screen that is up (`name_entry.rs`); the answer is the command the purpose names.
    EnterName(super::name_entry::NameFor),
    /// Alias manager, Create: make a new alias, load it and say so (`flow_save.rs`).
    AliasCreate(String),
    /// Quick Play, Accept: start this race (a gameplay collection name) with this car (`quick_play.rs`).
    StartQuickPlay {
        race: String,
        car: String,
    },
    QuitGame,
    /// Write the settings to the config file.
    SaveSettings,
    /// Open the shared graphics editor over this menu.
    OpenGraphics,
    /// Change one slot (0 primary, 1 secondary) of a control to the next key pressed (`controls_screen.rs`).
    Remap(Control, usize),
    /// The boot flow goes on to its next step.
    NextBootStep,
    /// Save the career being played to its profile (`MC_Main.fng`, `flow_save.rs`).
    CareerSave,
    /// Load the profile of this alias and play it (`flow_save.rs`).
    CareerLoad(String),
    /// Make a new career profile under this alias and play it (`flow_save.rs`).
    CareerNew(String),
    /// Delete the profile of this alias (`flow_save.rs`).
    CareerDelete(String),
    /// Buy or sell a car on the car lot, after its confirmation (`career_screens/car_lot.rs`).
    CarLot(super::career_screens::car_lot::CarLotAction),
    /// The career car select asks for something (`career_screens/car_select.rs`, `flow_bust.rs`).
    CareerCar(super::career_screens::car_select::CarSelectAction),
    /// Co-op: player 2 chose their car from the shared garage (`flow_coop.rs`). The folder is the car's name the world loads.
    CoopSecondCar {
        folder: String,
        handle: u32,
        transmission: crate::settings::Transmission,
    },
}

pub use super::custom_race::start::CustomStart;

/// The settings, the part of them the menus changed since they were last saved, and what the renderer can do
/// (the option rows follow it).
pub struct Env<'a> {
    pub settings: &'a mut Settings,
    pub changed: &'a mut Partial,
    pub caps: Capabilities,
    /// The mods while they run: the Mods page's switches and rows (`bb.menus`); `None` without mods.
    pub mods: Option<&'a crate::modding::Mods>,
}

/// What a screen can touch while it handles a message.
pub struct Cx<'a> {
    pub rt: &'a mut Runtime,
    pub package: PackageId,
    pub assets: &'a UiAssets,
    pub settings: &'a mut Settings,
    /// Settings the menus changed; the front end writes them to the config file.
    pub changed: &'a mut Partial,
    /// What the renderer can do: the option screens show only rows and values it offers.
    pub caps: Capabilities,
    pub commands: &'a mut Vec<Command>,
    /// The option index the screen last left on, per screen name.
    pub memory: &'a mut std::collections::HashMap<String, usize>,
    /// The key names of the controls list, per row: the primary and the secondary (`controls_screen::key_texts`).
    pub key_texts: &'a [[String; 2]],
    /// The world map's frame while the map is open (`world_map/screen`); `None` on every other screen.
    pub world_map: Option<&'a WorldMapFrame>,
    /// The mods, for the Mods page's switches and rows; `None` without mods.
    pub mods: Option<&'a crate::modding::Mods>,
    pub name: &'a str,
}

impl Cx<'_> {
    /// Whether the Remastered look draws the menus as vertical lists: the setting names it and no mod theme took over.
    pub fn vertical_look(&self) -> bool {
        self.settings.ui_style == UiStyle::Remastered && self.mods.is_none_or(|mods| mods.ui_theme_choice().is_none())
    }

    /// Sets host-authored text on every string directly inside a group, including its shadow.
    pub fn text_group(&mut self, group_hash: u32, text: &str) {
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
            self.rt.set_text(ObjectRef { package: self.package, index }, text);
        }
    }
    /// Sets the text of the string object with this name hash (and nothing else). Nothing when the screen has none.
    pub fn text_object(&mut self, name_hash: u32, text: &str) {
        let Some(object) = self.object(name_hash) else { return };
        self.rt.set_text(object, text);
    }

    /// The object with this GUID in the screen's package.
    pub fn guid(&self, guid: u32) -> Option<ObjectRef> {
        self.rt.find_guid(self.package, guid)
    }

    /// Sets the texture rectangle `[u0, v0, u1, v1]` of the image with this GUID (a bar's LED strip). Nothing when the
    /// screen has no such object.
    pub fn set_uv(&mut self, guid: u32, uv: [f32; 4]) {
        let Some(object) = self.guid(guid) else { return };
        self.rt.set_uv(object, uv);
    }

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

/// Which way a click on a row of a vertical list presses: back (the left chevron), forward (the right one), or just the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Back,
    Stay,
    Forward,
}

/// What the pointer does to a row of a vertical list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pick {
    /// It moved over the row.
    Hover,
    /// It pressed the row: toward which chevron, and how far along the row's value column, from 0 at its left to 1 at its right.
    Press(Way, f32),
}

pub trait ScreenLogic: Send + Sync {
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
    /// icon menus have one. The hints and the aspect are filled in by [`Screens`](super::screens::Screens).
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
    fn typed(&mut self, _cx: &mut Cx, _key: super::name_entry::TextKey) {}

    /// The footer hints of the screen (`prompts::hint_set`).
    fn footer(&self) -> Footer {
        Footer::Standard
    }

    /// The backdrop this screen wants right now, when it depends on what the screen shows (the Quick Play brief shows the car it
    /// picked). `None` leaves it to the screen's look by name (`factory::look_for`).
    fn look(&self) -> Option<super::look::Look> {
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
