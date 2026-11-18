use blackbox_feng::ids::{BUTTON_PRESSED, PAD_ACCEPT, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};
use blackbox_feng::{PackageId, Runtime};

pub(super) fn accept(message: u32) -> bool {
    matches!(message, PAD_ACCEPT | BUTTON_PRESSED | 0x7B6B_89D7)
}

pub(super) fn horizontal(message: u32) -> bool {
    matches!(message, PAD_LEFT | PAD_RIGHT | 0x4812_2792 | 0x4AC5_E165)
}

pub(super) fn vertical(message: u32) -> bool {
    matches!(message, PAD_UP | PAD_DOWN | 0x480D_F13F | 0xB205_316C)
}

pub(super) fn scroller_direction(message: u32) -> bool {
    horizontal(message) || vertical(message)
}

pub(super) fn dialog_choice_changed(last: &mut Option<u32>, runtime: &Runtime, package: PackageId) -> bool {
    let Some(button) = runtime.current_button(package) else { return false };
    let Some(name) = runtime.package(package).and_then(|def| def.objects.get(button.index)).map(|obj| obj.name_hash)
    else {
        return false;
    };
    last.replace(name) != Some(name)
}

pub(super) fn memcard_sound(runtime: &Runtime, package: PackageId, message: u32, cue: Option<i32>) -> Option<i32> {
    if horizontal(message)
        && !runtime.buttons(package).into_iter().any(|button| runtime.object(button).is_some_and(|state| !state.hidden))
    {
        return None;
    }
    cue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_visible_memcard_button_blocks_only_horizontal_feedback() {
        let runtime = Runtime::new();
        let package = PackageId(0);
        assert_eq!(memcard_sound(&runtime, package, 0x4812_2792, Some(2)), None);
        assert_eq!(memcard_sound(&runtime, package, PAD_ACCEPT, Some(4)), Some(4));
    }

    #[test]
    #[ignore = "needs the stock dialog package"]
    fn installed_dialog_direction_requires_a_different_focused_button() {
        use crate::ui::{Catalog, SCREEN_FILES};
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut runtime = Runtime::new();
        let package = runtime.load(catalog.find("Dialog.fng").unwrap().clone());
        let buttons = runtime.buttons(package);
        let mut last = None;
        for button in &buttons[..2] {
            runtime.set_hidden(*button, false);
            runtime.focus_button(*button);
            assert!(dialog_choice_changed(&mut last, &runtime, package));
            assert!(!dialog_choice_changed(&mut last, &runtime, package));
        }
    }
}
