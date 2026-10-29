//! Keyboard and controller views reuse the existing native fields; controller mapping is a remake feature.
use super::controls_screen::{HUD_ROWS, KEY_PLACEHOLDER, ROWS, waiting_text};
use crate::input::{Bindings, Control};

pub(super) fn fields(all: &[[String; 2]], controller: bool) -> &[[String; 2]] {
    let count = ROWS.len() + HUD_ROWS.len();
    if controller {
        return all.get(count..).unwrap_or_default();
    }
    all.get(..count.min(all.len())).unwrap_or_default()
}

pub(super) fn texts(
    bindings: &Bindings,
    keyboard_wait: Option<(Control, usize)>,
    controller_wait: Option<(Control, usize, String)>,
) -> Vec<[String; 2]> {
    let mut texts = super::controls_screen::key_texts(bindings, keyboard_wait);
    texts.extend(ROWS.iter().chain(&HUD_ROWS).map(|control| {
        let names = bindings.gamepad_labels(*control);
        let slot = |at| {
            if let Some((wanted, slot, text)) = &controller_wait
                && *wanted == *control
                && *slot == at
            {
                return text.clone();
            }
            names.get(at).cloned().unwrap_or_else(|| KEY_PLACEHOLDER.into())
        };
        [slot(0), slot(1)]
    }));
    texts
}

pub(super) fn waiting(text: &str) -> bool {
    if text == waiting_text() {
        return true;
    }
    ["controls-connect-controller", "controls-release-input", "controls-waiting-input"]
        .into_iter()
        .any(|key| text == crate::i18n::text(key, &crate::i18n::Args::new()).to_uppercase())
}

pub(super) fn title(controller: bool) -> String {
    let id = match controller {
        true => "controls-controller-configuration",
        false => "controls-keyboard-configuration",
    };
    crate::i18n::text(id, &crate::i18n::Args::new())
}
