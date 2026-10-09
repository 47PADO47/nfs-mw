//! The icon menus: the main menu, the option categories and the pause menu (docs/specs/frontend-menus.md,
//! section 2). A row of icons scrolls under a fixed cursor; the package supplies the icon objects, the title
//! and the leave animation, this logic supplies the icons, the scrolling and what a press does.

use blackbox_feng::ObjectRef;
use glam::Vec2;

use super::ids::{self, screen};
use super::logic::{Args, Category, Command, Cx, ScreenLogic};
use super::scroller::{self, Scroll};
use blackbox_feng::ids::{BUTTON_PRESSED, PAD_BACK, PAD_LEFT, PAD_RIGHT, PAD_START};

/// Which icon menu a package is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Main,
    /// The option categories; opened from the pause menu when `pause` is set.
    Categories {
        pause: bool,
    },
    Pause,
}

/// What an icon does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    Career,
    Challenge,
    QuickRace,
    MyCars,
    Options,
    Category(Category),
    Resume,
    QuitToMenu,
}

#[derive(Clone, Copy, Debug)]
struct Icon {
    id: Id,
    /// The texture key of the icon, and the language labels of its name.
    texture: u32,
    name: u32,
    enabled: bool,
}

const fn icon(id: Id, texture: u32, name: u32, enabled: bool) -> Icon {
    Icon { id, texture, name, enabled }
}

/// The icons of a menu: `(texture, name label)` as the original screens give them.
fn icons(kind: Kind) -> Vec<Icon> {
    match kind {
        // Career and Quick Race both start free roam here; the rest of the original menu has no counterpart yet.
        Kind::Main => vec![
            icon(Id::Career, 0x0370_4F3D, 0x5815_A2B5, true),
            icon(Id::Challenge, 0x9A96_2438, 0xCC8C_B746, false),
            icon(Id::QuickRace, 0x4E6F_BB02, 0x5402_0A7A, true),
            icon(Id::MyCars, 0xB0C4_6023, 0x1AFD_5BE6, false),
            icon(Id::Options, 0x3058_FE37, 0x19A8_C0AF, true),
        ],
        Kind::Categories { .. } => vec![
            icon(Id::Category(Category::Audio), 0xF37A_F144, 0xE76C_D783, true),
            icon(Id::Category(Category::Video), 0x8A00_6328, 0xE8E2_4508, true),
            icon(Id::Category(Category::Gameplay), 0x4DF9_8FB2, 0xD0CF_6EE1, true),
        ],
        Kind::Pause => vec![
            icon(Id::Resume, 0x12BB_5EA2, 0x01BD_185C, true),
            icon(Id::Options, 0x520D_E4E3, 0x2B5A_03A8, true),
            icon(Id::QuitToMenu, 0x4C9E_34E6, 0xE950_B7AF, true),
        ],
    }
}

/// An icon object of the package and the place it takes on the strip.
struct Slot {
    obj: ObjectRef,
    guid: u32,
    size: Vec2,
    offset: f32,
    /// The index into the icons; none for the empty icons that pad the ends.
    icon: Option<usize>,
}

/// What the screen was told and will do once it has left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    Pressed(Id),
    Back,
    Start,
}

/// Empty icons before the first and after the last real one.
const BOOK_ENDS: usize = 2;
/// The colours the icons fade between: idle and faded, as `0xAARRGGBB`.
const WHITE: (u32, u32) = (0xFFFF_FFFF, 0x00FF_FFFF);
const ORANGE: (u32, u32) = (0xFFFF_AE40, 0x00FF_AE40);

pub struct IconMenu {
    kind: Kind,
    args: Args,
    icons: Vec<Icon>,
    slots: Vec<Slot>,
    /// The selected slot.
    selected: usize,
    scroll: Scroll,
    center: Vec2,
    colours: (u32, u32),
    /// Frames since the screen appeared, for the fade in.
    age: f32,
    reacts: bool,
    pending: Option<Pending>,
}

impl IconMenu {
    pub fn new(kind: Kind, args: Args) -> Self {
        let colours = if matches!(kind, Kind::Main | Kind::Categories { pause: false }) { WHITE } else { ORANGE };
        Self {
            kind,
            args,
            icons: icons(kind),
            slots: Vec::new(),
            selected: 0,
            scroll: Scroll::default(),
            center: Vec2::ZERO,
            colours,
            age: 0.0,
            reacts: true,
            pending: None,
        }
    }

    /// What the screen remembers its selection under: the package name and which menu it is (the pause package
    /// is both the pause menu and its option categories).
    fn memory_key(&self, cx: &Cx) -> String {
        format!("{}:{:?}", cx.name.to_ascii_lowercase(), self.kind)
    }

    fn pause(&self) -> bool {
        matches!(self.kind, Kind::Categories { pause: true } | Kind::Pause)
    }

    /// The icon of the selected slot, if it is a real one.
    fn current_icon(&self) -> Option<usize> {
        self.slots.get(self.selected).and_then(|s| s.icon)
    }

    fn first_slot(&self) -> usize {
        BOOK_ENDS
    }

    fn last_slot(&self) -> usize {
        BOOK_ENDS + self.icons.len() - 1
    }

    /// Makes `slot` the selection: the package focus, the scroll target and the header.
    fn select(&mut self, cx: &mut Cx, slot: usize) {
        self.selected = slot;
        cx.rt.set_focus(cx.package, self.slots[slot].guid);
        self.scroll.seek(-self.slots[slot].offset);
        self.refresh_header(cx);
    }

    /// The title shows the name of the icon; at either end the package dims its arrow.
    fn refresh_header(&self, cx: &mut Cx) {
        let Some(i) = self.current_icon() else { return };
        let name = self.icons[i].name;
        for hash in [ids::ICON_TITLE, ids::ICON_TITLE_SHADOW] {
            if let Some(o) = cx.object(hash) {
                cx.label(o, name);
            }
        }
        if self.selected == self.first_slot() {
            cx.rt.post_to_package(cx.package, ids::END_PAD_LEFT);
        }
        if self.selected == self.last_slot() {
            cx.rt.post_to_package(cx.package, ids::END_PAD_RIGHT);
        }
    }

    /// The next icon in a direction, none at the end (the original does not wrap here). Icons whose screens do not
    /// exist yet are greyed out but can still be selected, so the whole row can be browsed.
    fn neighbour(&self, step: isize) -> Option<usize> {
        let slot = self.selected.checked_add_signed(step)?;
        (self.first_slot()..=self.last_slot()).contains(&slot).then_some(slot)
    }

    fn scroll_by(&mut self, cx: &mut Cx, step: isize) {
        if !self.reacts {
            return;
        }
        if let Some(slot) = self.neighbour(step) {
            self.select(cx, slot);
        }
    }

    /// Posts the leave message: the package plays its leave scripts and sends `EXIT_COMPLETE` when they are done.
    fn leave(&mut self, cx: &mut Cx, pending: Pending) {
        self.pending = Some(pending);
        self.reacts = false;
        cx.rt.post_to_package(cx.package, ids::LEAVE_SCREEN);
    }

    /// Plays the package's own leave timing (the event handler's forward script) instead of the leave message.
    fn leave_by_script(&mut self, cx: &mut Cx, pending: Pending) {
        self.pending = Some(pending);
        self.reacts = false;
        cx.script(ids::EVENT_HANDLER, ids::SCRIPT_FORWARD);
    }

    /// The screen has left: do what was pressed.
    fn act(&mut self, cx: &mut Cx, pending: Pending) {
        let pause = self.pause();
        match (self.kind, pending) {
            (Kind::Main, Pending::Pressed(Id::Career | Id::QuickRace)) => cx.send(Command::StartFreeRoam),
            (Kind::Main, Pending::Pressed(Id::Options)) => {
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), Args { options: true, ..self.args }));
            }
            (Kind::Categories { .. }, Pending::Pressed(Id::Category(category))) => {
                let name = if pause { screen::PAUSE_OPTIONS } else { screen::OPTIONS };
                cx.send(Command::Switch(name.into(), Args { category, options: true, ..self.args }));
            }
            (Kind::Categories { .. }, Pending::Back) if pause => {
                cx.send(Command::Switch(screen::PAUSE_MENU.into(), Args { pause: true, options: false, ..self.args }));
            }
            (Kind::Categories { .. }, Pending::Back) => {
                cx.send(Command::Switch(screen::MAIN_MENU.into(), Args::default()));
            }
            (Kind::Categories { .. } | Kind::Pause, Pending::Start) | (Kind::Pause, Pending::Back) => {
                cx.send(Command::Resume);
            }
            (Kind::Pause, Pending::Pressed(Id::Resume)) => cx.send(Command::Resume),
            (Kind::Pause, Pending::Pressed(Id::Options)) => {
                cx.send(Command::Switch(screen::PAUSE_MENU.into(), Args { pause: true, options: true, ..self.args }));
            }
            (Kind::Pause, Pending::Pressed(Id::QuitToMenu)) => cx.send(Command::QuitToMenu),
            _ => {}
        }
    }

    /// Writes where each icon is, how big and how opaque, from the scroll value and the clock.
    fn place(&self, cx: &mut Cx) {
        let scroll = self.scroll.value();
        let fade = (self.age / scroller::FADE_FRAMES).min(1.0);
        let half = scroller::WIDTH * 0.5;
        for slot in &self.slots {
            let x = self.center.x + scroll + slot.offset;
            let s = scroller::scale(x, self.center.x, scroller::WIDTH);
            let x = scroller::pulled(x, self.center.x, slot.size.x, s);
            let visible = x >= self.center.x - half && x <= self.center.x + half;
            cx.rt.set_hidden(slot.obj, !visible);
            let s = s * fade;
            cx.rt.set_position_xy(slot.obj, x, self.center.y);
            cx.rt.set_size_xy(slot.obj, slot.size.x * s, slot.size.y * s);
            let mut colour = scroller::fade_colour(self.colours.0, self.colours.1, s);
            if slot.icon.is_some_and(|i| !self.icons[i].enabled) {
                colour[3] = colour[3].min(150);
            }
            cx.rt.set_colour_rgba(slot.obj, colour);
        }
    }

    /// Finds the icon objects `OPTION_1` .. and puts the icons on them.
    fn build(&mut self, cx: &mut Cx) {
        let objects: Vec<ObjectRef> =
            (1..).map_while(|i| cx.named(&format!("OPTION_{i}"))).take(self.icons.len() + 2 * BOOK_ENDS).collect();
        let mut order: Vec<Option<usize>> = vec![None; BOOK_ENDS];
        order.extend((0..self.icons.len()).map(Some));
        order.extend(vec![None; BOOK_ENDS]);
        let mut offset = 0.0;
        for (obj, icon) in objects.into_iter().zip(order) {
            let size = cx.rt.size(obj).map_or(Vec2::splat(64.0), |s| Vec2::new(s.x.abs(), s.y.abs()));
            let guid = cx.rt.package(cx.package).map_or(0, |p| p.objects[obj.index].guid);
            let texture = icon.map_or(ids::END_OF_SCROLLER, |i| self.icons[i].texture);
            cx.rt.set_texture(obj, texture);
            self.slots.push(Slot { obj, guid, size, offset, icon });
            offset += size.x + scroller::SPACING;
        }
    }
}

impl ScreenLogic for IconMenu {
    fn start(&mut self, cx: &mut Cx) {
        // The cursor brackets sit on the stand-in icon (`OPTION_MASTER`), so that is where the selected icon goes:
        // the region's own position is up to 17 units off it (MainMenu.fng), which left the icon out of its brackets.
        let region = cx.named("ICON_SCROLL_REGION");
        let master = cx.object(ids::OPTION_MASTER);
        if let Some(at) = master.or(region).and_then(|o| cx.rt.position(o)) {
            self.center = Vec2::new(at.x, at.y);
        }
        if let Some(region) = region {
            cx.rt.set_hidden(region, true);
        }
        // The package leaves a stand-in icon at the cursor; the scroller draws the real ones.
        cx.hide(ids::OPTION_MASTER, true);
        self.build(cx);
        if self.slots.len() < self.icons.len() + 2 * BOOK_ENDS {
            log::warn!("{}: only {} icon slots", cx.name, self.slots.len());
            self.icons.truncate(self.slots.len().saturating_sub(2 * BOOK_ENDS));
        }
        if self.icons.is_empty() {
            return;
        }
        match self.kind {
            Kind::Main => {
                cx.label_group(ids::TITLE_GROUP, 0xB24A_AE58);
                // No profile is loaded: no player name and no game statistics.
                cx.hide(ids::NAME_GROUP, true);
                cx.hide(ids::GAME_STATS_GROUP, true);
            }
            Kind::Categories { pause: false } => cx.label_group(ids::TITLE_GROUP, 0x4ECA_678C),
            Kind::Categories { pause: true } => cx.label_group(ids::PAUSE_HEADER, 0x1D7B_B6C9),
            Kind::Pause => cx.label_group(ids::PAUSE_HEADER, 0x6C83_9FBE),
        }
        let remembered = cx.memory.get(&self.memory_key(cx)).copied().unwrap_or(0);
        let slot = (self.first_slot() + remembered).min(self.last_slot());
        let slot = if self.icons[slot - BOOK_ENDS].enabled { slot } else { self.neighbour_from(slot) };
        self.selected = slot;
        cx.rt.set_focus(cx.package, self.slots[slot].guid);
        self.scroll = Scroll::at(-self.slots[slot].offset);
        self.refresh_header(cx);
        self.place(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT => self.scroll_by(cx, -1),
            PAD_RIGHT => self.scroll_by(cx, 1),
            BUTTON_PRESSED => {
                if !self.reacts {
                    return;
                }
                let Some(i) = self.current_icon() else { return };
                if !self.icons[i].enabled {
                    // The package switched its buttons off for the leave animation that will not come.
                    cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE);
                    return;
                }
                cx.memory.insert(self.memory_key(cx), i);
                let id = self.icons[i].id;
                // The pause package has no leave message: its screens run the event handler's script themselves.
                if self.pause() {
                    self.leave_by_script(cx, Pending::Pressed(id));
                    return;
                }
                self.leave(cx, Pending::Pressed(id));
            }
            ids::QUIT_CLICKED if self.kind == Kind::Main => cx.send(Command::QuitGame),
            PAD_BACK => match self.kind {
                Kind::Main => {}
                Kind::Categories { pause: false } => {
                    self.pending = Some(Pending::Back);
                    self.reacts = false;
                }
                Kind::Categories { pause: true } | Kind::Pause => self.leave_by_script(cx, Pending::Back),
            },
            PAD_START => {
                if self.kind != Kind::Main {
                    self.leave_by_script(cx, Pending::Start);
                }
            }
            ids::EXIT_COMPLETE => {
                if let Some(pending) = self.pending.take() {
                    self.act(cx, pending);
                }
            }
            _ => {}
        }
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        if self.slots.is_empty() || self.icons.is_empty() {
            return;
        }
        self.scroll.update(dt);
        self.age += dt * 60.0;
        self.place(cx);
    }
}

impl IconMenu {
    /// The enabled icon nearest `slot`, for a remembered selection that is no longer available.
    fn neighbour_from(&self, slot: usize) -> usize {
        (self.first_slot()..=self.last_slot())
            .filter(|&s| self.slots[s].icon.is_some_and(|i| self.icons[i].enabled))
            .min_by_key(|&s| s.abs_diff(slot))
            .unwrap_or(slot)
    }
}

#[cfg(test)]
mod tests {
    use blackbox_feng::fe_hash_upper;

    use super::*;

    #[test]
    fn the_menus_have_the_original_icons() {
        let main = icons(Kind::Main);
        assert_eq!(main.len(), 5);
        assert!(main[0].enabled && !main[1].enabled, "career works, the challenge series do not exist yet");
        assert_eq!(icons(Kind::Categories { pause: false }).len(), 3);
        assert_eq!(icons(Kind::Pause)[0].name, 0x01BD_185C, "Resume Free Roam");
    }

    #[test]
    fn icon_objects_are_named_option_n() {
        assert_eq!(fe_hash_upper("OPTION_1"), 0x4B4E_3888);
        assert_eq!(fe_hash_upper("OPTION_10"), 0xB515_49B8);
    }
}
