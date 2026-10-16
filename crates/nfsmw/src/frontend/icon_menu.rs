//! The icon menus: the main menu, the option categories and the pause menu (docs/specs/frontend-menus.md,
//! section 2). A row of icons scrolls under a fixed cursor; the package supplies the icon objects, the title
//! and the leave animation, this logic supplies the icons, the scrolling and what a press does. A menu can list
//! more icons than its package has objects: the row is windowed over them (`icon_window.rs`).

use blackbox_feng::ObjectRef;
use glam::Vec2;

use super::confirm::Confirm;
use super::icon_items::{Icon, Id, draw_scale, icons};
use super::icon_window;
use super::ids::{self, screen};
use super::logic::{Args, Category, Command, Cx, ScreenLogic};
use super::scroller::{self, Fade, Scroll};
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
}

/// An icon object of the package and the place it takes on the strip.
struct Slot {
    obj: ObjectRef,
    guid: u32,
    size: Vec2,
    offset: f32,
    /// The picture rectangle the package gave the object, put back when the object shows another icon.
    uv: [f32; 4],
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
/// The picture rectangle of a bundled image that is drawn whole.
const WHOLE_IMAGE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

pub struct IconMenu {
    kind: Kind,
    args: Args,
    icons: Vec<Icon>,
    slots: Vec<Slot>,
    /// The strip index of the first slot: the slot `k` shows strip entry `base + k`.
    base: usize,
    /// The selected strip entry: always a real icon, never a book end.
    selected: usize,
    scroll: Scroll,
    center: Vec2,
    colours: (u32, u32),
    fade: Fade,
    reacts: bool,
    pending: Option<Pending>,
}

impl IconMenu {
    pub fn new(kind: Kind, args: Args) -> Self {
        let colours = if matches!(kind, Kind::Main | Kind::Categories { pause: false, .. }) { WHITE } else { ORANGE };
        Self {
            kind,
            args,
            icons: icons(kind),
            slots: Vec::new(),
            base: 0,
            selected: 0,
            scroll: Scroll::default(),
            center: Vec2::ZERO,
            colours,
            fade: Fade::default(),
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
        matches!(self.kind, Kind::Categories { pause: true, .. } | Kind::Pause)
    }

    /// The strip: a book end, the icons, a book end.
    fn strip_len(&self) -> usize {
        self.icons.len() + 2 * BOOK_ENDS
    }

    fn first_slot(&self) -> usize {
        BOOK_ENDS
    }

    fn last_slot(&self) -> usize {
        BOOK_ENDS + self.icons.len() - 1
    }

    /// The icon at a strip entry; none for the book ends and for the entries past the row.
    fn icon_at(&self, strip: usize) -> Option<usize> {
        let i = strip.checked_sub(BOOK_ENDS)?;
        (i < self.icons.len()).then_some(i)
    }

    /// The icon of the selection, if it is a real one.
    fn current_icon(&self) -> Option<usize> {
        self.icon_at(self.selected)
    }

    fn enabled_at(&self, strip: usize) -> bool {
        self.icon_at(strip).is_some_and(|i| self.icons[i].enabled)
    }

    /// The slot that shows a strip entry, which must be in the window.
    fn slot_of(&self, strip: usize) -> usize {
        strip - self.base
    }

    /// The distance between neighbouring slots on the strip.
    fn pitch(&self) -> f32 {
        self.slots.get(1).map_or(0.0, |s| s.offset - self.slots[0].offset)
    }

    /// Makes `strip` the selection: the window follows it, the package focus, the scroll target and the header.
    fn select(&mut self, cx: &mut Cx, strip: usize) {
        let base = icon_window::base_for(strip, self.slots.len(), self.strip_len());
        if base != self.base {
            self.slide(cx, base);
        }
        self.selected = strip;
        let slot = self.slot_of(strip);
        cx.rt.set_focus(cx.package, self.slots[slot].guid);
        self.scroll.seek(-self.slots[slot].offset);
        self.refresh_header(cx);
    }

    /// Moves the window to `base`: the slots show the icons from there, and the scroll moves with them, so the
    /// icons stay where they are on screen.
    fn slide(&mut self, cx: &mut Cx, base: usize) {
        self.scroll.shift(icon_window::slide_shift(self.base, base, self.pitch()));
        self.base = base;
        self.bind(cx);
    }

    /// Puts the strip entries of the window on the slots: the textures, and the picture rectangle of the controller.
    fn bind(&self, cx: &mut Cx) {
        for (k, slot) in self.slots.iter().enumerate() {
            let icon = self.icon_at(self.base + k);
            let texture = icon.map_or(ids::END_OF_SCROLLER, |i| self.icons[i].texture);
            cx.rt.set_texture(slot.obj, texture);
            let uv = match icon {
                Some(i) if self.icons[i].id == Id::ControlOptions => crate::ui::input_icons::Glyph::Controller.uv(),
                Some(i) if self.icons[i].texture == crate::ui::discord_icon::TEXTURE => WHOLE_IMAGE,
                _ => slot.uv,
            };
            cx.rt.set_uv(slot.obj, uv);
        }
    }

    /// The title shows the name of the icon; at either end the package dims its arrow.
    fn refresh_header(&self, cx: &mut Cx) {
        let Some(i) = self.current_icon() else { return };
        let name = self.icons[i].name;
        let text = match self.icons[i].id {
            Id::ControlOptions => Some("Control Options"),
            Id::Category(Category::Discord) => Some("Discord"),
            Id::Category(Category::Mods) => Some("Mods"),
            _ => None,
        };
        for hash in [ids::ICON_TITLE, ids::ICON_TITLE_SHADOW] {
            let Some(o) = cx.object(hash) else { continue };
            match text {
                Some(text) => cx.rt.set_text(o, text),
                None => cx.label(o, name),
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
        let strip = self.selected.checked_add_signed(step)?;
        (self.first_slot()..=self.last_slot()).contains(&strip).then_some(strip)
    }

    fn scroll_by(&mut self, cx: &mut Cx, step: isize) {
        if !self.reacts {
            return;
        }
        if let Some(strip) = self.neighbour(step) {
            self.select(cx, strip);
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
            (Kind::Main, Pending::Pressed(Id::Career)) => cx.send(Command::StartFreeRoam),
            (Kind::Main, Pending::Pressed(Id::QuickRace)) => {
                cx.send(Command::Switch(screen::TRACK_SELECT.into(), Args::default()));
            }
            (Kind::Main, Pending::Pressed(Id::Options)) => {
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), Args { options: true, ..self.args }));
            }
            (Kind::Main, Pending::Pressed(Id::AliasManager)) => {
                let args = Args { category: Category::AliasManager, options: true, ..self.args };
                cx.send(Command::Switch(screen::OPTIONS.into(), args));
            }
            (Kind::Categories { .. }, Pending::Pressed(Id::Category(category))) => self.open_options(cx, category),
            (Kind::Categories { .. }, Pending::Pressed(Id::Controls)) => self.open_controls(cx),
            (Kind::Categories { .. }, Pending::Pressed(Id::EaTrax)) => self.open_jukebox(cx),
            (Kind::Categories { .. }, Pending::Pressed(Id::ControlOptions)) => {
                self.open_options(cx, Category::Controls)
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
            _ => {}
        }
    }

    /// The option list of a category: the pause menu's in game, the main menu's otherwise.
    fn open_options(&self, cx: &mut Cx, category: Category) {
        let name = if self.pause() { screen::PAUSE_OPTIONS } else { screen::OPTIONS };
        cx.send(Command::Switch(name.into(), Args { category, options: true, ..self.args }));
    }

    /// The key list (`controls_screen.rs`), which goes back to these categories when it is left.
    fn open_controls(&self, cx: &mut Cx) {
        let name = if self.pause() { screen::PAUSE_CONTROLS } else { screen::CONTROLS };
        cx.send(Command::Switch(name.into(), Args { options: true, ..self.args }));
    }

    /// The EA Trax jukebox (`jukebox.rs`), which goes back to these categories when it is left.
    fn open_jukebox(&self, cx: &mut Cx) {
        cx.send(Command::Switch(screen::EA_TRAX_JUKEBOX.into(), Args { options: true, ..self.args }));
    }

    /// Writes where each icon is, how big and how opaque, from the scroll value and the clock.
    fn place(&self, cx: &mut Cx) {
        let scroll = self.scroll.value();
        let fade = self.fade.value();
        let half = scroller::WIDTH * 0.5;
        for (k, slot) in self.slots.iter().enumerate() {
            let x = self.center.x + scroll + slot.offset;
            let s = scroller::scale(x, self.center.x, scroller::WIDTH);
            let x = scroller::pulled(x, self.center.x, slot.size.x, s);
            let visible = x >= self.center.x - half && x <= self.center.x + half;
            cx.rt.set_hidden(slot.obj, !visible);
            let s = s * fade;
            let art = self.icon_at(self.base + k).map_or(1.0, |i| draw_scale(&self.icons[i]));
            cx.rt.set_position_xy(slot.obj, x, self.center.y);
            cx.rt.set_size_xy(slot.obj, slot.size.x * s * art, slot.size.y * s * art);
            let mut colour = scroller::fade_colour(self.colours.0, self.colours.1, s);
            if self.icon_at(self.base + k).is_some_and(|i| !self.icons[i].enabled) {
                colour[3] = colour[3].min(150);
            }
            cx.rt.set_colour_rgba(slot.obj, colour);
        }
    }

    /// Finds the icon objects `OPTION_1` .. of the package; the row windows over however many there are.
    fn build(&mut self, cx: &mut Cx) {
        let objects: Vec<ObjectRef> = (1..).map_while(|i| cx.named(&format!("OPTION_{i}"))).collect();
        let mut offset = 0.0;
        for obj in objects {
            let size = cx.rt.size(obj).map_or(Vec2::splat(64.0), |s| Vec2::new(s.x.abs(), s.y.abs()));
            let (guid, uv) = cx
                .rt
                .package(cx.package)
                .and_then(|p| p.objects.get(obj.index))
                .map_or((0, [0.0, 0.0, 1.0, 1.0]), |def| (def.guid, def.data.uv()));
            self.slots.push(Slot { obj, guid, size, offset, uv });
            offset += size.x + scroller::SPACING;
        }
    }
}

impl ScreenLogic for IconMenu {
    fn start(&mut self, cx: &mut Cx) {
        // Pause's glow INIT retains its idle colour; the event handler starts its entrance fade later.
        // Keep it transparent until that fade takes over, just like the incoming icons and brackets.
        if self.pause()
            && let Some(glow) = cx.object(ids::ICON_SELECTION_GLOW)
        {
            cx.rt.set_alpha(glow, 0);
        }
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
        if self.icons.is_empty() || self.slots.is_empty() {
            return;
        }
        match self.kind {
            Kind::Main => {
                cx.label_group(ids::TITLE_GROUP, 0xB24A_AE58);
                // No profile is loaded: no player name and no game statistics.
                cx.hide(ids::NAME_GROUP, true);
                cx.hide(ids::GAME_STATS_GROUP, true);
            }
            Kind::Categories { pause: false, .. } => cx.label_group(ids::TITLE_GROUP, 0x4ECA_678C),
            Kind::Categories { pause: true, .. } => cx.label_group(ids::PAUSE_HEADER, 0x1D7B_B6C9),
            Kind::Pause => cx.label_group(ids::PAUSE_HEADER, 0x6C83_9FBE),
        }
        let remembered = cx.memory.get(&self.memory_key(cx)).copied().unwrap_or(0);
        let strip = (self.first_slot() + remembered).min(self.last_slot());
        let strip = if self.enabled_at(strip) { strip } else { self.neighbour_from(strip) };
        self.base = icon_window::base_for(strip, self.slots.len(), self.strip_len());
        self.selected = strip;
        self.bind(cx);
        let slot = self.slot_of(strip);
        cx.rt.set_focus(cx.package, self.slots[slot].guid);
        self.scroll = Scroll::at(-self.slots[slot].offset);
        self.refresh_header(cx);
        self.place(cx);
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        for _ in 0..steps.unsigned_abs() {
            self.scroll_by(cx, if steps > 0 { -1 } else { 1 });
        }
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
                // Quit asks at once, without the pause menu leaving: its icons stay as they are, so the menu still
                // reacts when the question is answered No or dismissed.
                if self.pause() && matches!(id, Id::QuitToMenu) {
                    cx.send(Command::Confirm(Confirm::quit_to_menu()));
                    return;
                }
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
            ids::EXIT_STARTED => self.fade.leave(),
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
        self.fade.update(dt);
        self.place(cx);
    }
}

impl IconMenu {
    /// The enabled strip entry nearest `strip`, for a remembered selection that is no longer available.
    fn neighbour_from(&self, strip: usize) -> usize {
        (self.first_slot()..=self.last_slot())
            .filter(|&s| self.enabled_at(s))
            .min_by_key(|&s| s.abs_diff(strip))
            .unwrap_or(strip)
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
