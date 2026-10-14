//! The object GUIDs of the two controls packages: `Options_PC_Controller.fng` (the main menu's list) and
//! `Pause_PC_Controller.fng` (its in-game copy). Both draw eight rows, and each row has a title, and a primary and a
//! secondary button with a text object inside each. The tables come from `nfsmw dump-screen`, top row first
//! (docs/provenance/controls-packages.md).

use super::ids::screen;

/// The rows each package draws.
pub const ROW_COUNT: usize = 8;

/// The GUIDs of one package's controls list, top row first.
pub struct Layout {
    /// The title text of each row (the package's `option_data` text at the left).
    pub title: [u32; ROW_COUNT],
    /// The primary button of each row, and the text object inside it.
    pub primary: [u32; ROW_COUNT],
    pub primary_text: [u32; ROW_COUNT],
    /// The secondary button of each row, and the text object inside it.
    pub secondary: [u32; ROW_COUNT],
    pub secondary_text: [u32; ROW_COUNT],
}

/// `Options_PC_Controller.fng`: 140 objects.
pub static OPTIONS: Layout = Layout {
    title: [0x000e_1917, 0x000f_a36b, 0x000f_a37c, 0x000f_a388, 0x0010_4073, 0x0010_4077, 0x0010_407b, 0x0010_407e],
    primary: [0x000f_a349, 0x0010_4689, 0x0010_468e, 0x0010_4693, 0x0010_469a, 0x0010_469f, 0x0010_46a4, 0x0010_46a9],
    primary_text: [
        0x000f_a34a,
        0x000f_a36f,
        0x000f_a380,
        0x000f_a38c,
        0x0010_409b,
        0x0010_409f,
        0x0010_40a3,
        0x0010_40a7,
    ],
    secondary: [0x000f_a366, 0x0010_46dc, 0x0010_46e4, 0x0010_46ed, 0x0010_46f5, 0x0010_46fe, 0x0010_4707, 0x0010_4710],
    secondary_text: [
        0x000f_a367,
        0x000f_a373,
        0x000f_a384,
        0x000f_a390,
        0x0010_40c3,
        0x0010_40c7,
        0x0010_40cb,
        0x0010_40cf,
    ],
};

/// `Pause_PC_Controller.fng`: 116 objects. Its GUIDs are not the options package's; the rows sit at x = -289 for the
/// titles, -30 and 130 for the buttons. Read from the dump's y positions (-48 to 162, step 30).
pub static PAUSE: Layout = Layout {
    title: [0x0010_597f, 0x0010_597e, 0x0010_597d, 0x0010_597c, 0x0010_5997, 0x0010_598c, 0x0010_598d, 0x0010_598e],
    primary: [0x0010_5981, 0x0010_5998, 0x0010_599a, 0x0010_599c, 0x0010_599e, 0x0010_59a0, 0x0010_59a2, 0x0010_59a4],
    primary_text: [
        0x0010_5980,
        0x0010_5983,
        0x0010_5984,
        0x0010_5985,
        0x0010_5996,
        0x0010_598f,
        0x0010_5990,
        0x0010_5991,
    ],
    secondary: [0x0010_5986, 0x0010_5999, 0x0010_599b, 0x0010_599d, 0x0010_599f, 0x0010_59a1, 0x0010_59a3, 0x0010_59a5],
    secondary_text: [
        0x0010_5987,
        0x0010_5988,
        0x0010_5989,
        0x0010_598a,
        0x0010_5995,
        0x0010_5992,
        0x0010_5993,
        0x0010_5994,
    ],
};

/// The layout of the package called `name` (any case). Only the in-game copy has the pause GUIDs.
pub fn for_package(name: &str) -> &'static Layout {
    if name.eq_ignore_ascii_case(screen::PAUSE_CONTROLS) {
        return &PAUSE;
    }
    &OPTIONS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_guids(layout: &Layout) -> Vec<u32> {
        [layout.title, layout.primary, layout.primary_text, layout.secondary, layout.secondary_text]
            .iter()
            .flatten()
            .copied()
            .collect()
    }

    #[test]
    fn every_guid_in_a_layout_is_distinct() {
        for layout in [&OPTIONS, &PAUSE] {
            let mut guids = all_guids(layout);
            guids.sort_unstable();
            let count = guids.len();
            guids.dedup();
            assert_eq!(guids.len(), count);
        }
    }

    #[test]
    fn the_pause_package_has_its_own_guids() {
        let options = all_guids(&OPTIONS);
        assert!(all_guids(&PAUSE).iter().all(|g| !options.contains(g)));
    }

    #[test]
    fn the_package_name_picks_the_layout() {
        assert!(std::ptr::eq(for_package("Pause_PC_Controller.fng"), &PAUSE));
        assert!(std::ptr::eq(for_package("pause_pc_controller.FNG"), &PAUSE));
        assert!(std::ptr::eq(for_package("Options_PC_Controller.fng"), &OPTIONS));
    }
}
