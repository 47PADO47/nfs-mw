//! The console panel: the log and the command line, drawn from [`Console`] and the log buffer.

use egui::{Align2, Color32, Event, Frame, Key, Modifiers, RichText, ScrollArea, TextEdit, vec2};

use super::Console;
use super::parse::complete;
use crate::devtools::logbuf::{self, Kind};

/// How much of the window height the console covers.
const HEIGHT_FRACTION: f32 = 0.45;

fn colour(kind: Kind) -> Color32 {
    match kind {
        Kind::Log(log::Level::Error) | Kind::Error => Color32::from_rgb(255, 110, 110),
        Kind::Log(log::Level::Warn) => Color32::from_rgb(255, 205, 110),
        Kind::Log(log::Level::Info) => Color32::from_gray(200),
        Kind::Log(_) => Color32::from_gray(130),
        Kind::Output => Color32::from_rgb(150, 220, 150),
        Kind::Input => Color32::WHITE,
    }
}

pub fn show(ctx: &egui::Context, console: &mut Console) {
    if !console.open {
        return;
    }
    let screen = ctx.content_rect();
    let height = screen.height() * HEIGHT_FRACTION;
    egui::Window::new("console")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .anchor(Align2::LEFT_TOP, vec2(0.0, 0.0))
        .fixed_size(vec2(screen.width(), height))
        .frame(Frame::new().fill(Color32::from_black_alpha(215)).inner_margin(6.0))
        .show(ctx, |ui| {
            let input_height = 28.0;
            ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .max_height(height - input_height - 12.0)
                .show(ui, |ui| {
                    for line in logbuf::lines() {
                        ui.label(RichText::new(line.text).monospace().color(colour(line.kind)));
                    }
                });
            ui.separator();
            command_line(ui, console);
        });
}

fn command_line(ui: &mut egui::Ui, console: &mut Console) {
    // Tab completes instead of moving focus; Up and Down walk the history.
    let tab = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab));
    let up = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp));
    let down = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown));
    if tab {
        tab_complete(console);
    }
    if up {
        history_step(console, true);
    }
    if down {
        history_step(console, false);
    }

    let response = ui.add(
        TextEdit::singleline(&mut console.input)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .hint_text("type help"),
    );
    let entered = response.lost_focus()
        && ui.input(|i| i.events.iter().any(|e| matches!(e, Event::Key { key: Key::Enter, pressed: true, .. })));
    if entered {
        let line = std::mem::take(&mut console.input).trim().to_owned();
        if !line.is_empty() {
            if console.history.last() != Some(&line) {
                console.history.push(line.clone());
            }
            console.pending.push(line);
        }
        console.history_at = None;
    }
    // The console always owns the keyboard while it is open.
    response.request_focus();
}

fn history_step(console: &mut Console, older: bool) {
    if console.history.is_empty() {
        return;
    }
    let last = console.history.len() - 1;
    let at = match (console.history_at, older) {
        (None, true) => last,
        (None, false) => return,
        (Some(i), true) => i.saturating_sub(1),
        (Some(i), false) if i < last => i + 1,
        (Some(_), false) => {
            console.history_at = None;
            console.input.clear();
            return;
        }
    };
    console.history_at = Some(at);
    console.input.clone_from(&console.history[at]);
}

/// Complete the command name being typed; list the choices if there are several.
fn tab_complete(console: &mut Console) {
    if console.input.contains(char::is_whitespace) {
        return;
    }
    let matches = complete(&console.input, console.scene_commands);
    match matches.as_slice() {
        [] => {}
        [only] => console.input = format!("{only} "),
        many => {
            let prefix = common_prefix(many);
            if prefix.len() > console.input.len() {
                console.input = prefix;
            } else {
                logbuf::output(&many.join("  "));
            }
        }
    }
}

fn common_prefix(words: &[String]) -> String {
    let first = &words[0];
    let len =
        (0..first.len()).take_while(|&i| words.iter().all(|w| w.as_bytes().get(i) == first.as_bytes().get(i))).count();
    first[..len].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console_with(history: &[&str]) -> Console {
        Console { history: history.iter().map(|s| (*s).to_owned()).collect(), ..Console::default() }
    }

    #[test]
    fn history_walks_back_and_forth() {
        let mut c = console_with(&["a", "b", "c"]);
        history_step(&mut c, false);
        assert_eq!(c.input, "", "down with nothing selected does nothing");
        history_step(&mut c, true);
        assert_eq!(c.input, "c");
        history_step(&mut c, true);
        history_step(&mut c, true);
        history_step(&mut c, true);
        assert_eq!(c.input, "a", "stops at the oldest");
        history_step(&mut c, false);
        assert_eq!(c.input, "b");
        history_step(&mut c, false);
        history_step(&mut c, false);
        assert_eq!(c.input, "", "past the newest clears the line");
    }

    #[test]
    fn tab_completes_a_unique_name_and_the_common_prefix() {
        let _guard = logbuf::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut c = Console { input: "res".into(), ..Console::default() };
        tab_complete(&mut c);
        assert_eq!(c.input, "resolution ");

        let mut c =
            Console { input: "c".into(), scene_commands: &[("car <x>", ""), ("cars", "")], ..Console::default() };
        tab_complete(&mut c);
        assert_eq!(c.input, "c", "car, cars and clear share only c");
        let mut c =
            Console { input: "ca".into(), scene_commands: &[("car <x>", ""), ("cars", "")], ..Console::default() };
        tab_complete(&mut c);
        assert_eq!(c.input, "car");
    }
}
