//! The icon menus: the main menu, the option categories and the pause menu (docs/specs/frontend-menus.md,
//! section 2). A row of icons scrolls under a fixed cursor; the package supplies the icon objects, the title
//! and the leave animation, this logic supplies the icons and what a press does, and the scrolling is the shared
//! `icon_strip.rs`.

use glam::Vec2;

use super::career_screens::safehouse_menu;
use super::confirm::Confirm;
use super::icon_items::{Icon, Id, draw_scale, icons, pause_icons};
use super::icon_strip::{Face, IconStrip, ORANGE, WHITE};
use super::ids::{self, screen};
use super::logic::{Args, Category, Command, Cx, Footer, Pick, ScreenLogic};
use crate::ui::UiAssets;
use crate::ui::theme::MenuList;
use blackbox_feng::ids::{BUTTON_PRESSED, PAD_BACK, PAD_LEFT, PAD_RIGHT, PAD_START};

/// Which icon menu a package is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Main,
    /// The option categories; opened from the pause menu when `pause` is set. `mods` adds the Mods page (a mod has rows).
    Categories {
        pause: bool,
        mods: bool,
    },
    Pause,
    /// The safe house menu (`MainMenu_Sub.fng` with `Args::safehouse`): its row is built from the career's items.
    SafeHouse,
    /// The career menu (`MainMenu_Sub.fng` with `Sub::Career`): its row is built from the loaded profile.
    Career,
    /// The Quick Race menu (`MainMenu_Sub.fng` with `Sub::QuickRace`).
    QuickRace,
    /// The alias manager (`MC_ProfileManager.fng`).
    AliasManager,
    /// The kinds of custom race (`MainMenu_Sub.fng` with `Sub::CustomRace`, `custom_race/`).
    CustomRace,
}

/// What the screen was told and will do once it has left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    Pressed(Id),
    Back,
    Start,
}

/// The fifth face button, key 2: View Tutorial on the mode select (`PAD_BUTTON4`).
const TUTORIAL_BUTTON: u32 = blackbox_feng::ids::PAD_BUTTON0 + 4;
/// The picture rectangle of a bundled image that is drawn whole.
const WHOLE_IMAGE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

mod pointer;
mod press;
mod profile;
mod vertical;

pub struct IconMenu {
    kind: Kind,
    args: Args,
    icons: Vec<Icon>,
    strip: IconStrip,
    reacts: bool,
    pending: Option<Pending>,
    /// Whether the active theme lays the menu out as a vertical list (`themed`). The package then only supplies the
    /// order of the entries and the messages: it draws nothing, so the menu acts at once instead of playing its leave.
    vertical: bool,
    /// The entry a vertical list shows first (`follow`).
    top: usize,
    /// The main menu's statistics card is showing (`profile.rs`).
    stats_open: bool,
}

impl IconMenu {
    pub fn new(kind: Kind, args: Args) -> Self {
        match kind {
            Kind::Pause => Self::with_icons(kind, args, pause_icons(args.pause_ctx)),
            _ => Self::with_icons(kind, args, icons(kind)),
        }
    }

    /// A menu over icons the caller built (the safe house row, `icon_items::safehouse_icons`).
    pub fn with_icons(kind: Kind, args: Args, icons: Vec<Icon>) -> Self {
        let colours = if !matches!(kind, Kind::Pause | Kind::Categories { pause: true, .. }) { WHITE } else { ORANGE };
        let strip = IconStrip::new(icons.iter().map(Self::face).collect(), colours);
        Self { kind, args, icons, strip, reacts: true, pending: None, vertical: false, top: 0, stats_open: false }
    }

    /// What the screen remembers its selection under: the package name and which menu it is (the pause package
    /// is both the pause menu and its option categories).
    fn memory_key(&self, cx: &Cx) -> String {
        format!("{}:{:?}", cx.name.to_ascii_lowercase(), self.kind)
    }

    fn pause(&self) -> bool {
        matches!(self.kind, Kind::Categories { pause: true, .. } | Kind::Pause)
    }

    /// How the strip draws an icon.
    fn face(icon: &Icon) -> Face {
        Face { uv: Self::icon_uv(icon), art: draw_scale(icon), ..Face::new(icon.texture, icon.enabled) }
    }

    /// The picture rectangle of an icon that is not drawn from the package's own: the controller's glyph of the
    /// input atlas, and a bundled image that is drawn whole.
    fn icon_uv(icon: &Icon) -> Option<[f32; 4]> {
        match icon.id {
            Id::ControlOptions => Some(crate::ui::input_icons::Glyph::Controller.uv()),
            _ if icon.texture == crate::ui::discord_icon::TEXTURE => Some(WHOLE_IMAGE),
            _ => None,
        }
    }

    /// The icon of the selection, if it is a real one.
    fn current_icon(&self) -> Option<usize> {
        self.strip.selected_icon()
    }

    /// Makes `entry` the selection: the strip follows it, then the package focus, the list window and the header.
    fn select(&mut self, cx: &mut Cx, entry: usize) {
        self.strip.select(cx, entry);
        self.follow();
        cx.rt.set_focus(cx.package, self.strip.focus_guid());
        self.refresh_header(cx);
    }

    /// The title shows the name of the icon; at either end the package dims its arrow.
    fn refresh_header(&self, cx: &mut Cx) {
        let Some(i) = self.current_icon() else { return };
        let name = self.icons[i].name;
        let text = Self::special_text(self.icons[i].id);
        for hash in [ids::ICON_TITLE, ids::ICON_TITLE_SHADOW] {
            let Some(o) = cx.object(hash) else { continue };
            match &text {
                Some(text) => cx.rt.set_text(o, text.as_str()),
                None => cx.label(o, name),
            }
        }
        if self.strip.selected() == self.strip.first() {
            cx.rt.post_to_package(cx.package, ids::END_PAD_LEFT);
        }
        if self.strip.selected() == self.strip.last() {
            cx.rt.post_to_package(cx.package, ids::END_PAD_RIGHT);
        }
    }

    fn scroll_by(&mut self, cx: &mut Cx, step: isize) {
        if !self.reacts {
            return;
        }
        if let Some(entry) = self.strip.neighbour(step) {
            self.select(cx, entry);
        }
    }
}

impl ScreenLogic for IconMenu {
    fn accepts_input(&self) -> bool {
        self.reacts
    }

    fn resume(&mut self, cx: &mut Cx) {
        // A retained menu regains control after its dialog/list closes, including callers that began leaving.
        self.strip.restore();
        self.reacts = true;
        self.pending = None;
        // A load over this menu may have changed the profile.
        match self.kind {
            Kind::Career => cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), self.args)),
            _ => self.show_profile(cx),
        }
    }

    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.list_look();
        // Pause's glow INIT retains its idle colour; the event handler starts its entrance fade later.
        // Keep it transparent until that fade takes over, just like the incoming icons and brackets.
        if self.pause()
            && let Some(glow) = cx.object(ids::ICON_SELECTION_GLOW)
        {
            cx.rt.set_alpha(glow, 0);
        }
        // The package leaves a stand-in icon at the cursor; the strip hides it and draws the real ones.
        self.strip.build(cx);
        if self.strip.is_empty() {
            return;
        }
        let title = self.title_label();
        match self.kind {
            Kind::Main => cx.label_group(ids::TITLE_GROUP, title),
            Kind::Categories { pause: false, .. }
            | Kind::SafeHouse
            | Kind::Career
            | Kind::QuickRace
            | Kind::CustomRace
            | Kind::AliasManager => cx.label_group(ids::TITLE_GROUP, title),
            Kind::Categories { pause: true, .. } | Kind::Pause => cx.label_group(ids::PAUSE_HEADER, title),
        }
        self.show_profile(cx);
        let remembered = cx.memory.get(&self.memory_key(cx)).copied().unwrap_or(0);
        let entry = (self.strip.first() + remembered).min(self.strip.last());
        let entry = self.strip.nearest_enabled(entry);
        self.strip.open(cx, entry);
        self.follow();
        cx.rt.set_focus(cx.package, self.strip.focus_guid());
        self.refresh_header(cx);
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        for _ in 0..steps.unsigned_abs() {
            self.scroll_by(cx, if steps > 0 { -1 } else { 1 });
        }
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn footer(&self) -> Footer {
        match self.kind {
            Kind::Career | Kind::SafeHouse | Kind::Categories { pause: false, .. } if !self.vertical => {
                Footer::BackAccept
            }
            Kind::Career | Kind::QuickRace | Kind::AliasManager | Kind::SafeHouse => Footer::BackOnly,
            Kind::CustomRace if self.tutorial().is_some() => Footer::Tutorial,
            Kind::CustomRace => Footer::BackAccept,
            _ => Footer::Standard,
        }
    }

    fn rotate_pad(&self) -> bool {
        self.vertical
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        self.pick(cx, row, pick);
    }

    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        self.click_icon(cx, point)
    }

    fn list(&self, assets: &UiAssets) -> Option<MenuList> {
        self.menu_list(assets)
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT => self.scroll_by(cx, -1),
            PAD_RIGHT => self.scroll_by(cx, 1),
            BUTTON_PRESSED => self.press(cx),
            TUTORIAL_BUTTON => self.play_tutorial(cx),
            ids::QUIT_CLICKED if self.kind == Kind::Main => cx.send(Command::Confirm(Confirm::quit_game())),
            profile::GAME_STATS_BUTTON => self.toggle_stats(cx),
            PAD_BACK => match self.kind {
                Kind::Main => {}
                // The safe house menu that stands before the world goes back to the career menu; in the world it stays.
                Kind::SafeHouse => cx.send(Command::LeaveSafeHouse),
                Kind::Career | Kind::QuickRace | Kind::CustomRace | Kind::AliasManager if self.vertical => {
                    self.act(cx, Pending::Back)
                }
                Kind::Career | Kind::QuickRace | Kind::CustomRace | Kind::AliasManager => {
                    self.pending = Some(Pending::Back);
                    self.reacts = false;
                }
                Kind::Categories { pause: false, .. } if self.vertical => self.act(cx, Pending::Back),
                Kind::Categories { pause: false, .. } => {
                    self.pending = Some(Pending::Back);
                    self.reacts = false;
                }
                Kind::Categories { pause: true, .. } | Kind::Pause => self.leave_by_script(cx, Pending::Back),
            },
            PAD_START => {
                if self.pause() {
                    self.leave_by_script(cx, Pending::Start);
                }
            }
            ids::EXIT_STARTED => self.strip.leave(),
            ids::EXIT_COMPLETE => {
                if let Some(pending) = self.pending.take() {
                    self.act(cx, pending);
                }
            }
            _ => {}
        }
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        self.vertical = cx.list_look();
        if self.strip.is_empty() {
            return;
        }
        self.strip.tick(dt);
        self.strip.place(cx);
    }
}

#[cfg(test)]
mod tests {
    use blackbox_feng::fe_hash_upper;

    #[test]
    fn icon_objects_are_named_option_n() {
        assert_eq!(fe_hash_upper("OPTION_1"), 0x4B4E_3888);
        assert_eq!(fe_hash_upper("OPTION_10"), 0xB515_49B8);
    }
}
