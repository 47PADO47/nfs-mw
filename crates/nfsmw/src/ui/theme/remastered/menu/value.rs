//! The value of a settings row, to the right of its label: a value between two chevrons, or a slider. Both are the shared
//! base components (`Choice`, `PillBar`, `Chevron`), read from the Remastered skin.

use blackbox_ui::{Chevron, Choice, ChoiceStyle, Facing, PillBar, PillStyle, Rect, Text, bounce, justify};

use super::Parts;
use super::layout as place;
use crate::ui::theme::remastered::skin::keys;
use crate::ui::theme::{MenuEntry, MenuValue};

/// The value's text size, and the chevrons' size at a slider's ends.
const VALUE_SCALE: f32 = 1.7;
const CHEVRON: f32 = 14.0;
/// A slider's bar: its height, and the room kept left of it for the chevron and right of it for the number.
const BAR_HEIGHT: f32 = 9.0;
const BAR_LEFT_ROOM: f32 = 20.0;
const BAR_RIGHT_ROOM: f32 = 62.0;

impl Parts<'_, '_> {
    /// Draws the value of `entry` in its column on the row at `y`. `nudge` is `(forward, age)` of the last press on this row.
    pub(super) fn value(&mut self, entry: &MenuEntry, selected: bool, y: f32, left: f32, nudge: Option<(bool, f32)>) {
        let column = place::value_column(y, left);
        let colour = match (entry.enabled, selected) {
            (false, _) => keys::MENU_TEXT_DISABLED,
            (true, true) => keys::MENU_TEXT_SELECTED,
            (true, false) => keys::MENU_TEXT,
        };
        let live = selected && entry.enabled;
        let facing = nudge.map(|(forward, age)| (if forward { Facing::Right } else { Facing::Left }, age));
        match &entry.value {
            MenuValue::None => {}
            MenuValue::Choice { text, arrows } => {
                let style = ChoiceStyle {
                    font: keys::FONT_MENU,
                    text: colour,
                    scale: VALUE_SCALE,
                    arrow: keys::MENU_ARROW,
                    arrow_colour: keys::MENU_TITLE,
                };
                let choice = Choice::new(style, column, &text.to_uppercase(), live && *arrows, facing, self.cx);
                self.add(choice);
            }
            MenuValue::Slider(percent) => self.slider(*percent, column, colour, live, facing),
        }
    }

    /// A slider: the bar in the middle of the column, the number at its right, and the chevrons at its ends.
    fn slider(&mut self, percent: u8, column: Rect, colour: &str, live: bool, nudge: Option<(Facing, f32)>) {
        let width = column.w - BAR_LEFT_ROOM - BAR_RIGHT_ROOM;
        let bar = Rect::new(column.min_x() + BAR_LEFT_ROOM + width / 2.0, column.y, width, BAR_HEIGHT);
        let style = PillStyle { track: keys::MENU_TRACK, border: keys::MENU_RULE, fill: keys::MENU_BOLT };
        self.add(PillBar::new(style, bar, f32::from(percent) / 100.0, false, self.cx));
        let number = Text::new(keys::FONT_MENU, percent.to_string())
            .with_colour_key(colour)
            .with_scale(VALUE_SCALE)
            .with_justification(justify::H_RIGHT | justify::V_CENTRE);
        self.push(number, Rect::new(column.max_x() - CHEVRON - 8.0 - 20.0, column.y, 40.0, 12.0));
        if !live {
            return;
        }
        let tint = self.cx.skin.colour(keys::MENU_TITLE).unwrap_or([255; 4]);
        for facing in [Facing::Left, Facing::Right] {
            let x = match facing {
                Facing::Left => column.min_x() + CHEVRON / 2.0 + 1.0,
                Facing::Right => column.max_x() - CHEVRON / 2.0 - 1.0,
            };
            let pushed = nudge.filter(|(side, _)| *side == facing).map_or(0.0, |(_, age)| bounce(age));
            let rect = Rect::new(x, column.y, CHEVRON, CHEVRON);
            self.add(Chevron::new(keys::MENU_ARROW, facing, rect, tint, pushed, self.cx));
        }
    }
}
