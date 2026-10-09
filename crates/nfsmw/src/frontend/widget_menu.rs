//! The option screens: numbered rows of a title, a data string or a slider, and arrows
//! (docs/specs/frontend-menus.md, section 3). The package supplies the row objects and the highlight scripts;
//! this logic puts the settings on them, moves the selection and changes the values.

use blackbox_feng::ids::{BUTTON_PRESSED, PAD_ACCEPT, PAD_BACK, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_START, PAD_UP};
use blackbox_feng::{ObjectRef, fe_hash_upper};
use glam::Vec2;

use super::ids::{self, screen};
use super::logic::{Args, Command, Cx, ScreenLogic};
use super::options::{self, Control, Data, Row, Title};

/// Rows the screen shows at once (ten from the pause menu).
const ROWS_ON_SCREEN: usize = 9;
const ROWS_ON_SCREEN_PAUSED: usize = 10;
/// Slider geometry: the fill bar's left edge sits this far in from the base, and its top this far above it
/// (the fill image is 32 high and the base 6), and the base sits this far below the row's top.
const FILL_INSET: Vec2 = Vec2::new(2.0, -12.0);
const SLIDER_DROP: f32 = 9.5;
/// The alpha of a disabled row's title, value and arrows (the enabled ones are fully opaque).
const DISABLED_ALPHA: u8 = 110;

/// The objects of one row.
struct RowObjects {
    name: ObjectRef,
    guid: u32,
    data: Option<ObjectRef>,
    left: Option<ObjectRef>,
    right: Option<ObjectRef>,
    base: Option<ObjectRef>,
    fill: Option<ObjectRef>,
}

/// Where rows go, from the package's two master objects (the title column and the data column).
struct Layout {
    /// Top-left of the title column, and its size.
    title: Vec2,
    title_size: Vec2,
    /// Top-left of the data column, and its size.
    data: Vec2,
    data_size: Vec2,
}

pub struct WidgetMenu {
    args: Args,
    rows: Vec<Row>,
    objects: Vec<RowObjects>,
    layout: Option<Layout>,
    selected: usize,
    /// The first row on screen.
    top: usize,
    on_screen: usize,
    leaving: bool,
}

fn top_left(cx: &Cx, o: ObjectRef) -> Option<(Vec2, Vec2)> {
    let p = cx.rt.position(o)?;
    let s = cx.rt.size(o)?;
    let size = Vec2::new(s.x.abs(), s.y.abs());
    Some((Vec2::new(p.x, p.y) - size * 0.5, size))
}

impl WidgetMenu {
    pub fn new(args: Args) -> Self {
        let on_screen = if args.pause { ROWS_ON_SCREEN_PAUSED } else { ROWS_ON_SCREEN };
        Self {
            args,
            rows: Vec::new(),
            objects: Vec::new(),
            layout: None,
            selected: 0,
            top: 0,
            on_screen,
            leaving: false,
        }
    }

    fn find_rows(&mut self, cx: &mut Cx) {
        for n in 1.. {
            let Some(name) = cx.named(&format!("OPTION_NAME_{n}")) else { break };
            let guid = cx.rt.package(cx.package).map_or(0, |p| p.objects[name.index].guid);
            self.objects.push(RowObjects {
                name,
                guid,
                data: cx.named(&format!("OPTION_DATA_{n}")),
                left: cx.named(&format!("LEFT_ARROW_{n}")),
                right: cx.named(&format!("RIGHT_ARROW_{n}")),
                base: cx.named(&format!("BASE_SLIDER_{n}")),
                fill: cx.named(&format!("FILLBAR_SLIDER_{n}")),
            });
        }
        let title = cx.named("TITLE_MASTER").and_then(|o| top_left(cx, o));
        let data = cx.named("DATA_MASTER").and_then(|o| top_left(cx, o));
        if let (Some((title, title_size)), Some((data, data_size))) = (title, data) {
            self.layout = Some(Layout { title, title_size, data, data_size });
        }
    }

    /// The objects of every row, however many the package has.
    fn each_object(r: &RowObjects) -> impl Iterator<Item = ObjectRef> {
        [Some(r.name), r.data, r.left, r.right, r.base, r.fill].into_iter().flatten()
    }

    /// Places screen slot `slot`: the title right-aligned against the title column, the data
    /// centred in the data column, the arrows at its ends, the slider under its middle.
    fn place(&self, cx: &mut Cx, slot: usize) {
        let (Some(layout), Some(r)) = (&self.layout, self.objects.get(slot)) else { return };
        let top = layout.title.y + slot as f32 * layout.title_size.y;
        let middle = top + layout.title_size.y * 0.5;
        // Each string sits in its column by its own justification (the title right aligned, the data centred).
        cx.align_string(r.name, layout.title.x, layout.title_size.x, middle);
        if let Some(data) = r.data {
            cx.align_string(data, layout.data.x, layout.data_size.x, middle);
        }
        if let Some(left) = r.left {
            cx.rt.set_position_xy(left, layout.data.x, middle);
        }
        if let Some(right) = r.right {
            cx.rt.set_position_xy(right, layout.data.x + layout.data_size.x, middle);
        }
        if let Some(base) = r.base {
            let size = cx.rt.size(base).map_or(Vec2::ZERO, |s| Vec2::new(s.x.abs(), s.y.abs()));
            let tl = Vec2::new(layout.data.x + layout.data_size.x * 0.5 - size.x * 0.5, top + SLIDER_DROP);
            cx.rt.set_position_xy(base, tl.x + size.x * 0.5, tl.y + size.y * 0.5);
        }
    }

    /// Shows the rows in view and hides the rest, positions them and refreshes their values.
    fn layout_rows(&mut self, cx: &mut Cx) {
        let (first, last) = (self.top, (self.top + self.on_screen).min(self.rows.len()));
        for slot in 0..self.objects.len() {
            let visible = slot < last - first;
            for o in Self::each_object(&self.objects[slot]) {
                cx.rt.set_hidden(o, !visible);
            }
            if visible {
                self.place(cx, slot);
                self.draw_row(cx, first + slot, slot);
            }
        }
        self.mark_selection(cx);
    }

    /// Writes the title, the value and the slider of a row from the settings.
    fn draw_row(&self, cx: &mut Cx, index: usize, slot: usize) {
        let (Some(row), Some(r)) = (self.rows.get(index), self.objects.get(slot)) else { return };
        match row.title {
            Title::Label(label) => cx.rt.set_label(r.name, label),
            Title::Text(text) => cx.rt.set_text(r.name, text),
        }
        let control = row.setting.control(cx.settings);
        let slider = matches!(control, Control::Slider(_));
        for o in [r.data, r.left, r.right].into_iter().flatten() {
            // A slider row has no data string; its arrows stay.
            if Some(o) == r.data {
                cx.rt.set_hidden(o, slider);
            }
        }
        for o in [r.base, r.fill].into_iter().flatten() {
            cx.rt.set_hidden(o, !slider);
        }
        Self::dim(cx, r, row, row.setting.enabled(cx.settings));
        match control {
            Control::Toggle => {
                let Some(data) = r.data else { return };
                match row.setting.data_in(cx.settings, &cx.caps) {
                    Data::Label(label) => cx.rt.set_label(data, label),
                    Data::Text(text) => cx.rt.set_text(data, text),
                }
            }
            Control::Slider(percent) => self.draw_slider(cx, r, f32::from(percent) / 100.0),
        }
    }

    /// Dims a disabled row. The package's scripts own the alpha of the arrows and the data (they show the arrows
    /// on the highlighted row only), so this only caps it: a hidden object stays hidden, a shown one is dimmed.
    /// A row that is enabled again gets back the full alpha it had before it was dimmed. A retail label keeps its
    /// authored title alpha animation while it is enabled.
    fn dim(cx: &mut Cx, r: &RowObjects, row: &Row, enabled: bool) {
        let title_owned = !(enabled && matches!(row.title, Title::Label(_)));
        let objects = [r.data, r.left, r.right].into_iter().flatten().chain(title_owned.then_some(r.name));
        for o in objects {
            let Some(current) = cx.rt.alpha(o) else { continue };
            let alpha = match (enabled, current) {
                (true, DISABLED_ALPHA) => 255,
                (true, _) => continue,
                (false, _) => current.min(DISABLED_ALPHA),
            };
            cx.rt.set_alpha(o, alpha);
        }
    }

    /// Redraws every row in view, so a change that alters another row's state (vsync and the frame limit) shows.
    fn draw_visible(&self, cx: &mut Cx) {
        let (first, last) = (self.top, (self.top + self.on_screen).min(self.rows.len()));
        for index in first..last {
            self.draw_row(cx, index, index - first);
        }
    }

    /// The fill bar is as wide as the base times the value, starting just inside the base.
    fn draw_slider(&self, cx: &mut Cx, r: &RowObjects, value: f32) {
        let (Some(base), Some(fill)) = (r.base, r.fill) else { return };
        let (Some(b), Some(f)) = (cx.rt.position(base), cx.rt.size(fill)) else { return };
        let Some(base_size) = cx.rt.size(base) else { return };
        let width = base_size.x.abs() * value;
        let base_tl = Vec2::new(b.x - base_size.x.abs() * 0.5, b.y - base_size.y.abs() * 0.5);
        let tl = base_tl + FILL_INSET;
        cx.rt.set_size_xy(fill, width, f.y);
        cx.rt.set_position_xy(fill, tl.x + width * 0.5, tl.y + f.y * 0.5);
        cx.rt.set_uv(fill, [0.0, 0.0, value, 1.0]);
    }

    /// The package focus and the highlight scripts of the data and the slider follow the selection.
    fn mark_selection(&self, cx: &mut Cx) {
        let selected_slot = self.selected.saturating_sub(self.top);
        for (slot, r) in self.objects.iter().enumerate() {
            let on = slot == selected_slot;
            let script = if on { ids::SCRIPT_HIGHLIGHT } else { ids::SCRIPT_UNHIGHLIGHT };
            for o in [r.data, r.base, r.fill].into_iter().flatten() {
                cx.rt.run_script(o, script);
            }
        }
        if let Some(r) = self.objects.get(selected_slot) {
            cx.rt.set_focus(cx.package, r.guid);
        }
        let slot = self.selected.saturating_sub(self.top) + 1;
        if let Some(cursor) = cx.object(CURSOR) {
            cx.rt.run_script(cursor, fe_hash_upper(&format!("POS{slot}")));
        }
    }

    /// Custom settings stay readable through the title's authored highlight alpha pulse.
    /// Keep the package's RGB, retail language-label animations and screen exit fades.
    fn keep_selected_title_readable(&self, cx: &mut Cx) {
        if self.leaving {
            return;
        }
        let Some(row) = self.rows.get(self.selected) else { return };
        if !matches!(row.title, Title::Text(_)) {
            return;
        }
        let Some(r) = self.objects.get(self.selected.saturating_sub(self.top)) else { return };
        let alpha = if row.setting.enabled(cx.settings) { 255 } else { DISABLED_ALPHA };
        cx.rt.set_alpha(r.name, alpha);
    }

    /// Selects the previous or next row, wrapping round, and scrolls the list when it must.
    fn move_selection(&mut self, cx: &mut Cx, forward: bool) {
        let n = self.rows.len();
        if n == 0 || self.on_screen == 0 || self.leaving {
            return;
        }
        self.selected = if forward { (self.selected + 1) % n } else { (self.selected + n - 1) % n };
        if self.selected < self.top {
            self.top = self.selected;
        }
        if self.selected >= self.top + self.on_screen {
            self.top = self.selected + 1 - self.on_screen;
        }
        self.layout_rows(cx);
    }

    /// Left or right on the selected row changes its setting; the new value is in effect at once.
    fn change(&mut self, cx: &mut Cx, forward: bool) {
        let Some(row) = self.rows.get(self.selected).copied() else { return };
        if self.leaving {
            return;
        }
        row.setting.advance(cx.settings, cx.changed, forward, &cx.caps);
        self.draw_visible(cx);
    }

    fn leave(&mut self, cx: &mut Cx) {
        if self.leaving {
            return;
        }
        self.leaving = true;
        cx.rt.post_to_package(cx.package, ids::LEAVE_SCREEN);
    }

    /// Back to the screen the options were opened from.
    fn finish(&mut self, cx: &mut Cx) {
        cx.send(Command::SaveSettings);
        let next = if self.args.pause {
            Command::Switch(screen::PAUSE_MENU.into(), Args { pause: true, options: true, ..self.args })
        } else {
            Command::Switch(screen::MAIN_MENU_SUB.into(), Args { options: true, ..self.args })
        };
        cx.send(next);
    }
}

/// The group whose scripts `POS1` .. `POSn` move the cursor to a row slot (its name is only known as a hash).
const CURSOR: u32 = 0x0674_5352;

impl ScreenLogic for WidgetMenu {
    fn start(&mut self, cx: &mut Cx) {
        self.rows = options::rows(self.args.category, &cx.caps);
        self.find_rows(cx);
        // Packages supply reusable screen slots, not one object for every logical setting.
        self.on_screen = self.on_screen.min(self.objects.len());
        // The original screens that do not apply: the defaults button and the unused parts.
        for hidden in DEFAULTS_HINT.iter().chain(&[SCROLL_ARROW_1, SCROLL_ARROW_2]) {
            cx.hide(*hidden, true);
        }
        let title = match (self.args.category, self.args.pause) {
            (super::logic::Category::Audio, false) => 0x3932_C2E4,
            (super::logic::Category::Video, false) => 0x4847_8029,
            (super::logic::Category::Gameplay, false) => 0x01CC_E8C2,
            (super::logic::Category::Audio, true) => 0xB142_6DFA,
            (super::logic::Category::Video, true) => 0xD94E_A03F,
            (super::logic::Category::Gameplay, true) => 0x3936_D9F8,
            (super::logic::Category::Controls, _) => 0,
        };
        cx.label_group(ids::HEADER_TEXT, title);
        if self.args.category == super::logic::Category::Controls {
            cx.text_group(ids::HEADER_TEXT, "Controls");
        }
        self.layout_rows(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            // Package INIT scripts can focus the original footer button after start().
            // Restore our first setting row once those scripts have completed.
            ids::INIT_COMPLETE => self.mark_selection(cx),
            PAD_UP => self.move_selection(cx, false),
            PAD_DOWN => self.move_selection(cx, true),
            PAD_LEFT => self.change(cx, false),
            PAD_RIGHT | BUTTON_PRESSED => self.change(cx, true),
            // The package runs its own leave animation for accept; back asks for the same one.
            PAD_ACCEPT => self.leaving = true,
            PAD_BACK => self.leave(cx),
            PAD_START if self.args.pause => cx.send(Command::Resume),
            ids::EXIT_COMPLETE => self.finish(cx),
            _ => {}
        }
    }

    /// The package's scripts write positions (the arrows' scripts hold their placeholder place); the rows are put
    /// back where the layout wants them after every update.
    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        let (first, last) = (self.top, (self.top + self.on_screen).min(self.rows.len()));
        for index in first..last {
            let slot = index - first;
            self.place(cx, slot);
            let (Some(row), Some(r)) = (self.rows.get(index), self.objects.get(slot)) else { continue };
            if let Control::Slider(percent) = row.setting.control(cx.settings) {
                self.draw_slider(cx, r, f32::from(percent) / 100.0);
            }
            // The package's scripts reset the title's alpha, so a disabled row is dimmed every frame.
            if !row.setting.enabled(cx.settings) {
                Self::dim(cx, r, row, false);
            }
        }
        self.keep_selected_title_readable(cx);
    }
}

/// The hint for the defaults button (a group in one package, strings and an icon in the others), and the arrows of
/// the player screen's list, which this logic does not use.
const DEFAULTS_HINT: [u32; 5] = [0xD646_3100, 0x28B8_FD2F, 0x2C42_0D64, 0x3FD8_A341, 0xD9A2_2505];
const SCROLL_ARROW_1: u32 = 0x4449_69FD;
const SCROLL_ARROW_2: u32 = 0x4449_69FE;

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_feng::Runtime;
    use game_install::GameDir;
    use std::collections::HashMap;

    use crate::settings::{Partial, Settings};
    use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn vehicle_effects_retail_title_animation_matches_the_unmodified_asset() {
        let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let assets = UiAssets::load(&dir).unwrap();
        for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
            let package = catalog.find(name).unwrap().clone();
            let (mut baseline, mut bound) = (Runtime::new(), Runtime::new());
            let baseline_id = baseline.load(package.clone());
            let bound_id = bound.load(package);
            // Finish the enter scripts before explicitly starting the retail title's highlight.
            // The pause package can clear its initial focus during those scripts.
            for _ in 0..120 {
                baseline.update(1.0 / 60.0);
                bound.update(1.0 / 60.0);
            }
            let hash = fe_hash_upper("OPTION_NAME_1");
            let original = baseline.find(baseline_id, hash).unwrap();
            let tested = bound.find(bound_id, hash).unwrap();
            baseline.run_script(original, ids::SCRIPT_HIGHLIGHT);
            bound.run_script(tested, ids::SCRIPT_HIGHLIGHT);
            let mut menu =
                WidgetMenu::new(Args { pause, category: super::super::logic::Category::Video, ..Args::default() });
            let (mut settings, mut changed) = (Settings::from(Partial::default()), Partial::default());
            let (mut commands, mut memory) = (Vec::new(), HashMap::new());
            let mut cx = Cx {
                rt: &mut bound,
                package: bound_id,
                assets: &assets,
                settings: &mut settings,
                changed: &mut changed,
                caps: crate::settings::test_caps::native(),
                commands: &mut commands,
                memory: &mut memory,
                name,
            };
            menu.find_rows(&mut cx);
            let (mut min_alpha, mut max_alpha) = (255, 0);
            for frame in 0..160 {
                baseline.update(1.0 / 60.0);
                cx.rt.update(1.0 / 60.0);
                menu.keep_selected_title_readable(&mut cx);
                let expected = baseline.object(original).unwrap().data.colour_rgba();
                let actual = cx.rt.object(tested).unwrap().data.colour_rgba();
                min_alpha = min_alpha.min(expected[3]);
                max_alpha = max_alpha.max(expected[3]);
                assert_eq!(actual, expected, "{name}, frame {frame}: retail RGBA must match the untouched asset");
            }
            assert!(
                min_alpha < max_alpha,
                "{name}: explicitly highlighted baseline must exercise animation (min {min_alpha}, max {max_alpha})"
            );
        }
    }
}
