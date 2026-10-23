//! The safe house menu (`MainMenu_Sub.fng` opened with `Args::safehouse`): the items and where each one goes
//! (`docs/specs/career-safehouse.md` section 5.1, `docs/specs/career-safehouse-2.md`). The rules are in
//! `nfsmw_career::safehouse::menu`; this screen shows the items as the icon row and dispatches the selection.

use nfsmw_career::economy::aftermath::CarSelectEntry;
use nfsmw_career::safehouse::FrontEndContext;
use nfsmw_career::safehouse::menu::{MenuItem, MenuTarget, visible_items};
use nfsmw_career::settings::FIRST_RIVAL_STAGE;

use super::car_select::CarSelectAction;
use crate::career::CareerHandle;
use crate::frontend::confirm::Confirm;
use crate::frontend::icon_items::{Id, safehouse_icons};
use crate::frontend::icon_menu::{IconMenu, Kind};
use crate::frontend::ids::screen;
use crate::frontend::infractions::money;
use crate::frontend::logic::{Args, Command, Cx, Footer, Pick, Plain, ScreenLogic};
use crate::ui::theme::MenuList;
use crate::ui::{Text, UiAssets};

/// The label of the confirmation question (`CONFIRM_RESUME_FREE_ROAM`).
const CONFIRM_RESUME_LABEL: u32 = 0xEB69_4C0C;
/// The text objects the menu fills after the items: the stable bounty and the cash (`BOUNTY_DATA`, `CASH_DATA`).
const BOUNTY_DATA: u32 = 0x322B_18F9;
const CASH_DATA: u32 = 0x1930_B057;
/// The memory-card layer is always present on PC (`savefile::autosave`), so Save is always listed.
const MEMORY_CARD_LAYER: bool = true;

/// The career values the menu shows, read once when it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Totals {
    bounty: u64,
    cash: u32,
    /// The career completion percentage (`CAREER_DATA`).
    career: u8,
}

/// The safe house menu's logic: the icon row, with the two totals filled in when it starts.
struct SafeHouseMenu {
    menu: IconMenu,
    totals: Totals,
}

/// The logic for the safe house menu. Outside career mode there is no career to show, so the package runs as it is.
pub fn make(args: Args) -> Box<dyn ScreenLogic> {
    let Some((items, totals)) = CareerHandle::shared().with(|career| {
        let items = visible_items(&career.settings, MEMORY_CARD_LAYER);
        let totals =
            Totals { bounty: career.stable.total_bounty(), cash: career.settings.cash, career: career.card().career };
        (items, totals)
    }) else {
        return Box::new(Plain);
    };
    let menu = IconMenu::with_icons(Kind::SafeHouse, args, safehouse_icons(&items));
    Box::new(SafeHouseMenu { menu, totals })
}

impl ScreenLogic for SafeHouseMenu {
    fn start(&mut self, cx: &mut Cx) {
        self.menu.start(cx);
        if let Some(o) = cx.named("CAREER_DATA") {
            cx.rt.set_text(o, format!("{}%", self.totals.career));
        }
        set_text(cx, BOUNTY_DATA, &money_text(self.totals.bounty));
        set_text(cx, CASH_DATA, &money(self.totals.cash));
    }

    fn resume(&mut self, cx: &mut Cx) {
        self.menu.resume(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        self.menu.message(cx, message);
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        self.menu.tick(cx, dt);
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        self.menu.scroll(cx, steps);
    }

    fn footer(&self) -> Footer {
        self.menu.footer()
    }

    // A theme that draws the icon row as a vertical list (the Remastered menus) reads these from the icon menu.
    fn list(&self, assets: &UiAssets) -> Option<MenuList> {
        self.menu.list(assets)
    }

    fn vertical(&self) -> bool {
        self.menu.vertical()
    }

    fn rotate_pad(&self) -> bool {
        self.menu.rotate_pad()
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        self.menu.pick_row(cx, row, pick);
    }
}

/// Runs the selected icon: its item decides the screen, and every screen it opens stays in the safe house.
pub fn press(cx: &mut Cx, args: Args, id: Id) {
    let Some(item) = item_of(id) else { return };
    // The Rap Sheet does nothing before the world exists.
    if item == MenuItem::RapSheet && CareerHandle::shared().is_prelude() {
        return;
    }
    let target = select(item);
    cx.send(command_for(target, Args { safehouse: true, ..args }, cx.vertical_look()));
}

/// The menu item an icon stands for; none for the icons of other menus.
fn item_of(id: Id) -> Option<MenuItem> {
    match id {
        Id::SafeHouseResume => Some(MenuItem::ResumeFreeRoam),
        Id::SafeHouseBlacklist(stage) => Some(MenuItem::Blacklist(stage)),
        Id::SafeHouseCarSelect => Some(MenuItem::CarSelect),
        Id::SafeHouseRapSheet => Some(MenuItem::RapSheet),
        Id::SafeHouseSave => Some(MenuItem::Save),
        _ => None,
    }
}

/// What selecting an item opens. `MenuItem::select` sets the rap sheet's context bit on the context it is given; the
/// screen has no front-end context to keep it in, so the bit is set on a scratch copy and the flow must set it
/// (`FrontEndContext::RAP_SHEET`) when it opens `RapSheetLogin.fng`.
fn select(item: MenuItem) -> MenuTarget {
    let mut context = FrontEndContext::CAREER;
    item.select(&mut context)
}

/// The command a target opens. Resume Free Roam asks first, and the answer's command is `Resume`: the flow then
/// runs `safehouse::resume_free_roam` (the prologue check needs the career's node progress). `vertical` is the
/// Remastered look, which has no overview page and shows its list of rivals instead.
fn command_for(target: MenuTarget, args: Args, vertical: bool) -> Command {
    match target {
        MenuTarget::ConfirmResume => Command::Confirm(Confirm::new(Text::Game(CONFIRM_RESUME_LABEL), Command::Resume)),
        // The original opens `SafeHouseReputationOverview.fng` at the career's own stage, which the target carries
        // (`career-safehouse.md` 5.1, `reputation_overview.rs`). The Remastered look lists the rivals on the
        // Blacklist stepper's package instead.
        MenuTarget::ReputationOverview { .. } if vertical => {
            Command::Switch(screen::SAFE_HOUSE_RIVAL_CHALLENGE.into(), args)
        }
        MenuTarget::ReputationOverview { stage } => Command::Switch(
            screen::SAFE_HOUSE_REPUTATION_OVERVIEW.into(),
            Args { rival: stage.clamp(1, FIRST_RIVAL_STAGE), ..args },
        ),
        // The safe house's Car Select is the career select mode of the package (choose, release, sell); the car lot zone
        // opens the lot mode (`car_select.rs`, `docs/specs/busted-aftermath.md` section 3).
        MenuTarget::CarSelect => Command::CareerCar(CarSelectAction::Open(CarSelectEntry::Plain)),
        MenuTarget::RapSheetLogin => Command::Switch(screen::RAP_SHEET_LOGIN.into(), args),
        MenuTarget::SaveDialog => Command::Switch(screen::MC_MAIN.into(), args),
    }
}

/// The bounty as the menu shows it: the amount with commas, no currency sign (the label format is `%$d`, career-rapsheet-2
/// section 4; the sign is not in the label's text).
pub(super) fn money_text(bounty: u64) -> String {
    money(u32::try_from(bounty).unwrap_or(u32::MAX))
}

/// Sets a text object's string, if the package has it.
fn set_text(cx: &mut Cx, name: u32, text: &str) {
    let Some(o) = cx.object(name) else { return };
    cx.rt.set_text(o, text);
}

#[cfg(test)]
#[path = "safehouse_menu_tests.rs"]
mod tests;
