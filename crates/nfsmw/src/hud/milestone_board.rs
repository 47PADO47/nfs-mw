//! The pursuit HUD's milestone board (`Hud_MilestoneBoard`, `docs/specs/hud-milestone-board.md`): during a free-roam
//! chase in the career, up to four icons for the current bin's milestones, the number done, and one milestone's
//! progress as `value/goal`. With two or more unfinished milestones the progress line moves on to the next one every
//! three seconds; a value that changes brings its milestone forward at once and flashes its icon's backing.

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

/// The board's top group in the package; the layout shows it during a chase (`elements.rs`).
pub(super) const BOARD: [u32; 1] = [0x2269_D232];
/// The four slots the board holds.
pub const SLOTS: usize = 4;
/// Seconds before the progress line moves on to the next unfinished milestone.
const ROTATE_SECS: f32 = 3.0;
/// Alpha of a finished milestone's icon, of 255 (an unfinished one is drawn at full alpha).
const DONE_ALPHA: f32 = 0.25;

/// Scripts of the package's objects.
const REST: u32 = 0x0017_44B3;
const HIDDEN: u32 = 0x0016_A259;
const SHOWN: u32 = 0x001C_A7C0;
const GROUP_IN: u32 = 0x5079_C8F8;
const GROUP_OUT: u32 = 0x0331_13AC;
const LINE_OUT: u32 = 0xAFF3_7F61;
const LINE_IN: u32 = 0xD6C9_50A0;
const HIGHLIGHT: u32 = 0x249D_B7B7;
const FLASH: u32 = 0x0382_6A28;

/// How a milestone's numbers are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    /// Minutes and seconds (pursuit length and evasion time).
    Time,
    /// A whole number with thousands separators.
    Number,
}

/// One milestone on the board.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardSlot {
    /// The icon's texture key; 0 keeps the package's own picture.
    pub icon: u32,
    pub display: Display,
    /// The statistic's current value and the milestone's goal.
    pub value: f32,
    pub goal: f32,
    /// At stake or banked: the icon is dimmed and the progress line skips it.
    pub done: bool,
}

/// What the board shows this frame: the current bin's milestones, at most [`SLOTS`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MilestoneBoard {
    pub slots: Vec<BoardSlot>,
}

/// The board's objects and its rotation.
pub struct Binding {
    info_group: Option<ObjectRef>,
    icon_group: Option<ObjectRef>,
    done_count: Option<ObjectRef>,
    icons: [Option<ObjectRef>; SLOTS],
    backings: [Option<ObjectRef>; SLOTS],
    line_backing: Option<ObjectRef>,
    line_group: Option<ObjectRef>,
    line: Option<ObjectRef>,
    /// The slot whose progress the line shows; `None` when every milestone is done.
    highlight: Option<usize>,
    /// Seconds the line has shown the highlighted slot.
    shown_for: f32,
    /// The values of the last frame, to see a value change.
    last: [Option<f32>; SLOTS],
}

impl Binding {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        let find = |name: &str| rt.find(package, fe_hash_upper(name));
        Self {
            info_group: find("MILESTONE_INFO_GROUP"),
            icon_group: find("MILESTONE_ICON_GROUP"),
            done_count: rt.find(package, 0x8946_62C5),
            icons: std::array::from_fn(|i| find(&format!("MILSTONE_ICON_{}", i + 1))),
            backings: std::array::from_fn(|i| find(&format!("MILESTONE_ICON_BACKING_{}", i + 1))),
            line_backing: rt.find(package, 0x5C69_7702),
            line_group: rt.find(package, 0xF440_5EC0),
            line: rt.find(package, 0xC3E4_8FBF),
            highlight: None,
            shown_for: 0.0,
            last: [None; SLOTS],
        }
    }

    /// Puts this frame's board into the objects; `dt` is the frame's seconds.
    pub fn apply(&mut self, rt: &mut Runtime, board: Option<&MilestoneBoard>, dt: f32) {
        let slots = board.map_or(&[][..], |b| &b.slots[..b.slots.len().min(SLOTS)]);
        if slots.is_empty() {
            self.shown_for = 0.0;
            self.last = [None; SLOTS];
            for group in [self.info_group, self.icon_group].into_iter().flatten() {
                if rt.script_of(group) == Some(GROUP_IN) {
                    rt.run_script(group, GROUP_OUT);
                }
            }
            return;
        }
        let open: Vec<usize> = (0..slots.len()).filter(|&i| !slots[i].done).collect();
        let line_script = if open.is_empty() { HIDDEN } else { SHOWN };
        for object in [self.line_backing, self.line_group].into_iter().flatten() {
            if rt.script_of(object) == Some(if open.is_empty() { SHOWN } else { HIDDEN }) {
                rt.run_script(object, line_script);
            }
        }
        for group in [self.info_group, self.icon_group].into_iter().flatten() {
            if rt.script_of(group) != Some(GROUP_IN) {
                rt.run_script(group, GROUP_IN);
            }
        }
        if let Some(count) = self.done_count {
            rt.set_text(count, (slots.len() - open.len()).to_string());
        }
        self.rotate(rt, &open, dt);
        self.flash_changes(rt, slots);
        self.draw_slots(rt, slots);
        if let (Some(line), Some(slot)) = (self.line, self.highlight.and_then(|i| slots.get(i))) {
            rt.set_text(line, progress(slot));
        }
    }

    /// Moves the progress line on: to the only open slot, or every [`ROTATE_SECS`] to the next open one, sliding the
    /// line out and back in.
    fn rotate(&mut self, rt: &mut Runtime, open: &[usize], dt: f32) {
        self.shown_for += dt;
        if open.len() < 2 {
            self.highlight = open.first().copied();
            return;
        }
        if self.highlight.is_none_or(|i| !open.contains(&i)) {
            self.highlight = open.first().copied();
            self.shown_for = 0.0;
        }
        if self.shown_for < ROTATE_SECS {
            return;
        }
        let Some(group) = self.line_group else {
            self.highlight = next_open(open, self.highlight);
            self.shown_for = 0.0;
            return;
        };
        match rt.script_of(group) {
            Some(SHOWN) => rt.run_script(group, LINE_OUT),
            Some(LINE_OUT) if finished(rt, group) => {
                rt.run_script(group, LINE_IN);
                self.highlight = next_open(open, self.highlight);
                self.shown_for = 0.0;
            }
            _ => {}
        }
    }

    /// A changed value of an open milestone (not a time) brings it forward and flashes its backing.
    fn flash_changes(&mut self, rt: &mut Runtime, slots: &[BoardSlot]) {
        for (i, slot) in slots.iter().enumerate() {
            let changed = self.last[i].is_some_and(|v| v != slot.value);
            self.last[i] = Some(slot.value);
            if !changed || slot.done || slot.display == Display::Time {
                continue;
            }
            self.highlight = Some(i);
            self.shown_for = 0.0;
            for object in [self.backings[i], self.line_backing].into_iter().flatten() {
                rt.run_script(object, FLASH);
            }
            if let Some(group) = self.line_group {
                rt.run_script(group, LINE_IN);
            }
        }
    }

    fn draw_slots(&mut self, rt: &mut Runtime, slots: &[BoardSlot]) {
        for i in 0..SLOTS {
            if let Some(backing) = self.backings[i] {
                let want = match i < slots.len() && Some(i) == self.highlight {
                    true => HIGHLIGHT,
                    false => REST,
                };
                let flashing = rt.script_of(backing) == Some(FLASH) && !finished(rt, backing);
                if rt.script_of(backing) != Some(want) && !(want == HIGHLIGHT && flashing) {
                    rt.run_script(backing, want);
                }
            }
            let Some(icon) = self.icons[i] else { continue };
            let Some(slot) = slots.get(i) else {
                if rt.script_of(icon) != Some(HIDDEN) {
                    rt.run_script(icon, HIDDEN);
                }
                continue;
            };
            if rt.script_of(icon) != Some(SHOWN) {
                rt.run_script(icon, SHOWN);
            }
            if slot.icon != 0 {
                rt.set_texture(icon, slot.icon);
            }
            let alpha = if slot.done { DONE_ALPHA } else { 1.0 };
            rt.set_alpha(icon, (alpha * 255.0).round() as u8);
        }
    }
}

/// Whether the object's current script has played to its end.
fn finished(rt: &Runtime, o: ObjectRef) -> bool {
    rt.object(o).is_some_and(|s| s.current.and_then(|c| s.scripts.get(c)).is_none_or(|sc| s.time >= sc.length))
}

/// The open slot after `from`, wrapping round.
fn next_open(open: &[usize], from: Option<usize>) -> Option<usize> {
    let at = from.and_then(|f| open.iter().position(|&i| i == f));
    match at {
        Some(p) => open.get((p + 1) % open.len()).copied(),
        None => open.first().copied(),
    }
}

/// The progress line of a slot: `value/goal`, the value no higher than the goal.
pub fn progress(slot: &BoardSlot) -> String {
    let value = slot.value.min(slot.goal).max(0.0);
    match slot.display {
        Display::Time => format!("{}/{}", clock(value), clock(slot.goal)),
        Display::Number => format!("{}/{}", grouped(value), grouped(slot.goal)),
    }
}

/// `m:ss` of a number of seconds.
fn clock(secs: f32) -> String {
    let whole = secs.max(0.0).round() as u32;
    format!("{}:{:02}", whole / 60, whole % 60)
}

/// A whole number with thousands separators: `10,000,000`.
fn grouped(value: f32) -> String {
    let digits = (value.max(0.0).round() as u64).to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(display: Display, value: f32, goal: f32) -> BoardSlot {
        BoardSlot { icon: 0, display, value, goal, done: false }
    }

    #[test]
    fn numbers_are_grouped_and_held_at_the_goal() {
        assert_eq!(progress(&slot(Display::Number, 1234.4, 10_000_000.0)), "1,234/10,000,000");
        assert_eq!(progress(&slot(Display::Number, 12_000_000.0, 10_000_000.0)), "10,000,000/10,000,000");
        assert_eq!(progress(&slot(Display::Number, 7.0, 25.0)), "7/25");
    }

    #[test]
    fn times_are_minutes_and_seconds() {
        assert_eq!(progress(&slot(Display::Time, 125.0, 780.0)), "2:05/13:00");
    }

    #[test]
    fn the_line_moves_on_through_the_open_slots() {
        assert_eq!(next_open(&[0, 2, 3], Some(0)), Some(2));
        assert_eq!(next_open(&[0, 2, 3], Some(3)), Some(0), "wraps round");
        assert_eq!(next_open(&[1, 3], None), Some(1));
    }
}
