//! The horizontal icon strip every carousel screen shares (docs/specs/frontend-menus.md, section 2): the package's
//! `OPTION_n` objects scroll under a fixed cursor, the selection eases to the centre, and the whole row grows in and
//! shrinks out. The original has one `IconScrollerMenu` for this; so does the rewrite. A screen supplies what each icon
//! looks like ([`Face`]) and what a selection or a click means; the strip does the slots, the window over them
//! (`icon_window.rs`), the scroll, the fade and the hit test, so the screens cannot drift apart.

use blackbox_feng::ObjectRef;
use glam::Vec2;

use super::icon_window;
use super::ids;
use super::logic::Cx;
use super::scroller::{self, Fade, Scroll};

/// Empty icons before the first and after the last real one.
pub const BOOK_ENDS: usize = 2;
/// The alpha of an icon whose screen does not exist yet.
pub const DISABLED_ALPHA: u8 = 150;
/// The colours the icons fade between: idle and faded, as `0xAARRGGBB`.
pub type Colours = (u32, u32);
pub const WHITE: Colours = (0xFFFF_FFFF, 0x00FF_FFFF);
pub const ORANGE: Colours = (0xFFFF_AE40, 0x00FF_AE40);

/// What one icon of the strip shows.
#[derive(Clone, Copy, Debug)]
pub struct Face {
    pub texture: u32,
    /// The picture rectangle to draw it with; the one the package gave the object when none.
    pub uv: Option<[f32; 4]>,
    /// How much of its slot the picture fills.
    pub art: f32,
    pub enabled: bool,
}

impl Face {
    pub fn new(texture: u32, enabled: bool) -> Self {
        Self { texture, uv: None, art: 1.0, enabled }
    }
}

/// What a point on the strip landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// The icon with this index in the faces the strip was made with.
    Icon(usize),
    /// An empty slot: nothing to select, but the click is the strip's.
    BookEnd,
}

/// An icon object of the package and the place it takes on the strip.
struct Slot {
    obj: ObjectRef,
    guid: u32,
    size: Vec2,
    offset: f32,
    /// The picture rectangle the package gave the object, put back when a face names none.
    uv: [f32; 4],
}

pub struct IconStrip {
    faces: Vec<Face>,
    slots: Vec<Slot>,
    /// The strip index of the first slot: the slot `k` shows strip entry `base + k`.
    base: usize,
    /// The selected strip entry: always a real icon, never a book end.
    selected: usize,
    scroll: Scroll,
    center: Vec2,
    colours: Colours,
    fade: Fade,
}

impl IconStrip {
    pub fn new(faces: Vec<Face>, colours: Colours) -> Self {
        Self {
            faces,
            slots: Vec::new(),
            base: 0,
            selected: 0,
            scroll: Scroll::default(),
            center: Vec2::ZERO,
            colours,
            fade: Fade::default(),
        }
    }

    /// Whether there is nothing to show: no icons, or a package without icon objects.
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty() || self.slots.is_empty()
    }

    /// The strip: a book end pair, the icons, a book end pair.
    fn len(&self) -> usize {
        self.faces.len() + 2 * BOOK_ENDS
    }

    /// The strip entry of the first real icon.
    pub fn first(&self) -> usize {
        BOOK_ENDS
    }

    /// The strip entry of the last real icon.
    pub fn last(&self) -> usize {
        BOOK_ENDS + self.faces.len().saturating_sub(1)
    }

    /// The selected strip entry.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The icon at a strip entry; none for the book ends and for the entries past the row.
    pub fn icon_at(&self, entry: usize) -> Option<usize> {
        let i = entry.checked_sub(BOOK_ENDS)?;
        (i < self.faces.len()).then_some(i)
    }

    /// The icon of the selection.
    pub fn selected_icon(&self) -> Option<usize> {
        self.icon_at(self.selected)
    }

    fn enabled_at(&self, entry: usize) -> bool {
        self.icon_at(entry).is_some_and(|i| self.faces[i].enabled)
    }

    /// The next icon in a direction, none at the end (the original does not wrap here). Icons whose screens do not
    /// exist yet are greyed out but can still be selected, so the whole row can be browsed.
    pub fn neighbour(&self, step: isize) -> Option<usize> {
        let entry = self.selected.checked_add_signed(step)?;
        (self.first()..=self.last()).contains(&entry).then_some(entry)
    }

    /// The enabled strip entry nearest `entry`, for a remembered selection that is no longer available.
    pub fn nearest_enabled(&self, entry: usize) -> usize {
        (self.first()..=self.last()).filter(|&e| self.enabled_at(e)).min_by_key(|&e| e.abs_diff(entry)).unwrap_or(entry)
    }

    /// The package guid of the slot that shows the selection, for the package's own focus.
    pub fn focus_guid(&self) -> u32 {
        let slot = self.selected.checked_sub(self.base).and_then(|k| self.slots.get(k));
        slot.map_or(0, |slot| slot.guid)
    }

    /// The slot that shows a strip entry, which must be in the window.
    fn slot_of(&self, entry: usize) -> &Slot {
        &self.slots[entry - self.base]
    }

    /// The distance between neighbouring slots on the strip.
    fn pitch(&self) -> f32 {
        self.slots.get(1).map_or(0.0, |s| s.offset - self.slots[0].offset)
    }

    /// Finds the cursor and the icon objects `OPTION_1` .. of the package; the row windows over however many there
    /// are. The cursor brackets sit on the stand-in icon (`OPTION_MASTER`), so that is where the selected icon goes:
    /// the scroll region's own position is up to 17 units off it (MainMenu.fng), which left the icon out of its brackets.
    pub fn build(&mut self, cx: &mut Cx) {
        let region = cx.named("ICON_SCROLL_REGION");
        let master = cx.object(ids::OPTION_MASTER);
        if let Some(at) = master.or(region).and_then(|o| cx.rt.position(o)) {
            self.center = Vec2::new(at.x, at.y);
        }
        if let Some(region) = region {
            cx.rt.set_hidden(region, true);
        }
        cx.hide(ids::OPTION_MASTER, true);
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

    /// Shows the strip with `entry` selected and no scroll in motion.
    pub fn open(&mut self, cx: &mut Cx, entry: usize) {
        if self.is_empty() {
            return;
        }
        self.base = icon_window::base_for(entry, self.slots.len(), self.len());
        self.selected = entry;
        self.bind(cx);
        self.scroll = Scroll::at(-self.slot_of(entry).offset);
        self.place(cx);
    }

    /// Makes `entry` the selection: the window follows it and the scroll eases to it.
    pub fn select(&mut self, cx: &mut Cx, entry: usize) {
        if self.is_empty() {
            return;
        }
        let base = icon_window::base_for(entry, self.slots.len(), self.len());
        if base != self.base {
            self.slide(cx, base);
        }
        self.selected = entry;
        self.scroll.seek(-self.slot_of(entry).offset);
    }

    /// Moves the window to `base`: the slots show the icons from there, and the scroll moves with them, so the
    /// icons stay where they are on screen.
    fn slide(&mut self, cx: &mut Cx, base: usize) {
        self.scroll.shift(icon_window::slide_shift(self.base, base, self.pitch()));
        self.base = base;
        self.bind(cx);
    }

    /// Puts the strip entries of the window on the slots: the textures and the picture rectangles.
    fn bind(&self, cx: &mut Cx) {
        for (k, slot) in self.slots.iter().enumerate() {
            let face = self.icon_at(self.base + k).map(|i| self.faces[i]);
            cx.rt.set_texture(slot.obj, face.map_or(ids::END_OF_SCROLLER, |f| f.texture));
            cx.rt.set_uv(slot.obj, face.and_then(|f| f.uv).unwrap_or(slot.uv));
        }
    }

    /// The row shrinks out: it keeps its current size if it was still growing, and stays at zero from there.
    pub fn leave(&mut self) {
        self.fade.leave();
    }

    /// The row is full size again, for a menu that stays on screen after its package began to leave.
    pub fn restore(&mut self) {
        self.fade.restore();
    }

    pub fn tick(&mut self, dt: f32) {
        self.scroll.update(dt);
        self.fade.update(dt);
    }

    /// Writes where each icon is, how big and how opaque, from the scroll value and the fade.
    pub fn place(&self, cx: &mut Cx) {
        // Authored entrance/exit scripts can show their stand-in again after start.
        // The host owns the actual strip, including every frame of its leave fade.
        cx.hide(ids::OPTION_MASTER, true);
        let scroll = self.scroll.value();
        let fade = self.fade.value();
        for (k, slot) in self.slots.iter().enumerate() {
            let at = scroller::placed(self.center.x + scroll + slot.offset, self.center.x, slot.size.x);
            cx.rt.set_hidden(slot.obj, !at.visible);
            let scale = at.scale * fade;
            let face = self.icon_at(self.base + k).map(|i| self.faces[i]);
            let art = face.map_or(1.0, |f| f.art);
            cx.rt.set_position_xy(slot.obj, at.x, self.center.y);
            cx.rt.set_size_xy(slot.obj, slot.size.x * scale * art, slot.size.y * scale * art);
            let mut colour = scroller::fade_colour(self.colours.0, self.colours.1, scale);
            if face.is_some_and(|f| !f.enabled) {
                colour[3] = colour[3].min(DISABLED_ALPHA);
            }
            cx.rt.set_colour_rgba(slot.obj, colour);
        }
    }

    /// What `point` landed on, if it is on one of the strip's objects. The object is read through the window, so a
    /// strip that has slid still resolves the icon the object shows now.
    pub fn hit(&self, cx: &Cx, point: Vec2) -> Option<Hit> {
        let objects: Vec<ObjectRef> = self.slots.iter().map(|slot| slot.obj).collect();
        let hit = cx.rt.object_at(&objects, point)?;
        let slot = self.slots.iter().position(|slot| slot.obj == hit)?;
        Some(self.icon_at(self.base + slot).map_or(Hit::BookEnd, Hit::Icon))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(icons: usize) -> IconStrip {
        IconStrip::new(vec![Face::new(1, true); icons], WHITE)
    }

    #[test]
    fn the_book_ends_frame_the_icons() {
        let strip = strip(3);
        assert_eq!((strip.first(), strip.last(), strip.len()), (2, 4, 7));
        assert_eq!([0, 1, 2, 4, 5, 6].map(|e| strip.icon_at(e)), [None, None, Some(0), Some(2), None, None]);
    }

    #[test]
    fn selection_stops_at_the_first_and_last_icon() {
        let mut strip = strip(3);
        strip.selected = strip.first();
        assert_eq!((strip.neighbour(-1), strip.neighbour(1)), (None, Some(3)));
        strip.selected = strip.last();
        assert_eq!((strip.neighbour(-1), strip.neighbour(1)), (Some(3), None));
    }

    #[test]
    fn a_remembered_icon_that_is_gone_falls_back_to_the_nearest_enabled_one() {
        let mut strip = strip(4);
        strip.faces[1].enabled = false;
        strip.faces[2].enabled = false;
        assert_eq!(strip.nearest_enabled(strip.first() + 1), strip.first());
        assert_eq!(strip.nearest_enabled(strip.first() + 2), strip.last());
        assert_eq!(strip.nearest_enabled(strip.first() + 3), strip.last());
    }

    #[test]
    fn a_strip_without_icons_or_objects_is_empty() {
        assert!(strip(0).is_empty());
        assert!(strip(3).is_empty(), "no package objects yet");
        assert_eq!(strip(0).last(), strip(0).first());
    }
}
