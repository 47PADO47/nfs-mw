//! The safe house menu (`docs/specs/career-safehouse.md` section 5.1, `docs/specs/career-flow.md` section 7.2).

use super::context::FrontEndContext;
use crate::settings::{CareerSettings, Flags};

/// One item of the `MainMenu_Sub.fng` safe house menu, in menu order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    /// Resume Free Roam: opens a confirmation dialog first.
    ResumeFreeRoam,
    /// Blacklist N, with the current stage as N.
    Blacklist(u8),
    /// Car Select.
    CarSelect,
    /// Rap Sheet: only when the rap sheet flag is set.
    RapSheet,
    /// Save: only when the memory-card layer is present.
    Save,
}

/// The screen or dialog an item opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuTarget {
    /// The confirmation dialog; its accept message is the Resume Free Roam action
    /// ([`super::resume::resume_free_roam`]).
    ConfirmResume,
    /// `SafehouseReputationOverview.fng`, with the current stage stored as the displayed stage.
    ReputationOverview {
        /// The stage to display.
        stage: u8,
    },
    /// `Car_Select.fng` in the safe house context.
    CarSelect,
    /// `RapSheetLogin.fng`, with context bit 9 set.
    RapSheetLogin,
    /// The save dialog (its message is unconfirmed in the spec).
    SaveDialog,
}

/// The items shown, in order: Resume Free Roam, Blacklist N and Car Select always; Rap Sheet when settings flag
/// bit 4 is set; Save when the memory-card layer is present.
pub fn visible_items(settings: &CareerSettings, memory_card_layer: bool) -> Vec<MenuItem> {
    let mut items = vec![MenuItem::ResumeFreeRoam, MenuItem::Blacklist(settings.stage), MenuItem::CarSelect];
    if settings.flags.contains(Flags::HAS_RAP_SHEET) {
        items.push(MenuItem::RapSheet);
    }
    if memory_card_layer {
        items.push(MenuItem::Save);
    }
    items
}

impl MenuItem {
    /// Selecting the item: returns what opens. The Rap Sheet item sets context bit 9.
    pub fn select(self, context: &mut FrontEndContext) -> MenuTarget {
        match self {
            MenuItem::ResumeFreeRoam => MenuTarget::ConfirmResume,
            MenuItem::Blacklist(stage) => MenuTarget::ReputationOverview { stage },
            MenuItem::CarSelect => MenuTarget::CarSelect,
            MenuItem::RapSheet => {
                context.insert(FrontEndContext::RAP_SHEET);
                MenuTarget::RapSheetLogin
            }
            MenuItem::Save => MenuTarget::SaveDialog,
        }
    }
}

/// The three text bindings the screen fills after the items: `CAREER_DATA` (the stage), `BOUNTY_DATA` (the total
/// bounty of the stable) and `CASH_DATA` (the cash).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuBindings {
    /// The stage text value.
    pub stage: u8,
    /// The stable total bounty.
    pub bounty: u64,
    /// The cash.
    pub cash: u32,
}

/// Fills the three bindings.
pub fn bindings(settings: &CareerSettings, stable: &crate::stable::Stable) -> MenuBindings {
    MenuBindings { stage: settings.stage, bounty: stable.total_bounty(), cash: settings.cash }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stable::{CarRecord, CareerRecord, Stable};

    #[test]
    fn a_new_profile_without_a_rap_sheet_has_four_items_with_a_card() {
        let s = CareerSettings::new_career();
        assert_eq!(
            visible_items(&s, true),
            [MenuItem::ResumeFreeRoam, MenuItem::Blacklist(16), MenuItem::CarSelect, MenuItem::Save]
        );
    }

    #[test]
    fn the_rap_sheet_needs_its_flag_and_save_needs_the_card_layer() {
        let mut s = CareerSettings::new_career();
        s.stage = 13;
        s.flags.insert(Flags::HAS_RAP_SHEET);
        assert_eq!(
            visible_items(&s, false),
            [MenuItem::ResumeFreeRoam, MenuItem::Blacklist(13), MenuItem::CarSelect, MenuItem::RapSheet]
        );
        assert_eq!(visible_items(&s, true).last(), Some(&MenuItem::Save));
        assert_eq!(visible_items(&s, true).len(), 5);
    }

    #[test]
    fn selecting_opens_the_documented_targets() {
        let mut ctx = FrontEndContext::CAREER;
        assert_eq!(MenuItem::ResumeFreeRoam.select(&mut ctx), MenuTarget::ConfirmResume);
        assert_eq!(MenuItem::Blacklist(9).select(&mut ctx), MenuTarget::ReputationOverview { stage: 9 });
        assert_eq!(MenuItem::CarSelect.select(&mut ctx), MenuTarget::CarSelect);
        assert_eq!(MenuItem::Save.select(&mut ctx), MenuTarget::SaveDialog);
        assert_eq!(ctx, FrontEndContext::CAREER, "none of those changes the context");
        assert_eq!(MenuItem::RapSheet.select(&mut ctx), MenuTarget::RapSheetLogin);
        assert!(ctx.contains(FrontEndContext::RAP_SHEET));
    }

    #[test]
    fn the_bindings_show_stage_bounty_and_cash() {
        let mut s = CareerSettings::new_career();
        s.stage = 12;
        s.cash = 9_000;
        let mut stable = Stable::empty();
        stable.cars[0] = CarRecord { handle: 1, flags: 0xF_0002, career_index: 0, ..CarRecord::free() };
        stable.careers[0] = CareerRecord { bounty: 777, ..CareerRecord::new(0) };
        stable.sold.bounty = 1_000;
        assert_eq!(bindings(&s, &stable), MenuBindings { stage: 12, bounty: 1_777, cash: 9_000 });
    }
}
