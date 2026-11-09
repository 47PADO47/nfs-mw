//! The sections of the option pages (docs/specs/menu-sections.md): rows grouped under a centred title, built on the abstract
//! menu model of `blackbox-ui` (`menu::SectionSet`). The first section of a page is the default one and has no title; every
//! row that is not named below is in it, so a page keeps the look it had. A section with no visible row shows no title.
//!
//! Which rows are in a named section is decided here, by their setting:
//!
//! - **Experimental**, on the Gameplay page: the rewrite's own additions that are off by default and have no counterpart in
//!   the original (Turn Signals, Cop Health Bars). On the Video page: Ray Tracing (it needs wgpu's experimental ray queries),
//!   Ambient Occlusion (`docs/ambient-occlusion.md`) and Speed Trails with their style (`docs/vehicle-effects.md`).
//! - **RIDE 6**, on the Gameplay page: the rider rows (the gear switch and the gear a mod offers, the riding styles, the
//!   fingers on the levers), which only matter on RIDE 6 bikes.
//! - Then one section for each title the options mods declared (`bb.settings`, `mod_options::gameplay_rows`), after those,
//!   in the order the mods registered them. A mod that names no section has its rows in the default one.
//!
//! Sections that have the same title text are one section, so a mod can put a row under Experimental.

use blackbox_gfx::Capabilities;
use blackbox_ui::menu::SectionSet;

use super::category_rows::rows as own_rows;
use super::{Row, Setting, Title, row};
use crate::frontend::graphics_options::GraphicsSetting;
use crate::frontend::logic::Category;
use crate::frontend::mod_options::SectionedRow;
use crate::frontend::renderer_options::RendererSetting;

impl Setting {
    /// Whether the row is a section title.
    pub fn is_heading(self) -> bool {
        matches!(self, Setting::Heading | Setting::ModHeading(_))
    }
}

impl Row {
    /// How the list treats the row (`blackbox_ui::menu`): a section title is not selected, pressed or stopped on.
    pub fn flags(&self) -> blackbox_ui::menu::EntryFlags {
        match self.setting.is_heading() {
            true => blackbox_ui::menu::EntryFlags::TITLE,
            false => blackbox_ui::menu::EntryFlags::ROW,
        }
    }
}

/// A named section of a page the game itself fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Group {
    Experimental,
    Ride6,
}

impl Group {
    /// The title the section shows.
    fn title(self) -> Title {
        Title::Key(match self {
            Group::Experimental => "options-section-experimental",
            Group::Ride6 => "options-section-ride6",
        })
    }

    /// The sections of a page, in the order they are listed after the default one.
    fn of_page(category: Category) -> &'static [Group] {
        match category {
            Category::Gameplay => &[Group::Experimental, Group::Ride6],
            Category::Video => &[Group::Experimental],
            _ => &[],
        }
    }
}

/// The named section a row of the page is in, `None` for the default one.
pub(super) fn group_of(category: Category, setting: Setting) -> Option<Group> {
    match (category, setting) {
        (Category::Gameplay, Setting::TurnSignals | Setting::CopHealthBars | Setting::AirControl) => {
            Some(Group::Experimental)
        }
        (Category::Gameplay, Setting::RiderGear | Setting::Rider(_)) => Some(Group::Ride6),
        (
            Category::Video,
            Setting::Renderer(RendererSetting::RayTracing)
            | Setting::Graphics(GraphicsSetting::AmbientOcclusion)
            | Setting::SpeedTrails
            | Setting::SpeedTrailStyle,
        ) => Some(Group::Experimental),
        _ => None,
    }
}

/// The rows of a category in the order the page lists them (the rows in their sections), section titles left out and the
/// rows of mods not included: what the selection can stop on, one press of Down at a time.
#[cfg_attr(not(test), allow(dead_code))]
pub fn rows(category: Category, caps: &Capabilities) -> Vec<Row> {
    let mut rows = page(category, caps, Vec::new());
    rows.retain(|row| !row.setting.is_heading());
    rows
}

/// A section title row.
fn heading(title: Title) -> Row {
    row(Setting::Heading, title)
}

/// The rows of a page as the menu lists them: its own rows in their sections, then the rows `mods` declared (by section),
/// every section's title before its rows. Pages with no sections are their rows, as they were.
pub fn page(category: Category, caps: &Capabilities, mods: Vec<SectionedRow>) -> Vec<Row> {
    let own = own_rows(category, caps);
    let groups = Group::of_page(category);
    if groups.is_empty() && mods.is_empty() {
        return own;
    }
    let mut set = SectionSet::new();
    for group in groups {
        set.declare(&group.title().text(), heading(group.title()));
    }
    for row in own {
        match group_of(category, row.setting) {
            None => set.push_default(row),
            Some(group) => set.push(&group.title().text(), || heading(group.title()), row),
        }
    }
    for SectionedRow { row, section, heading } in mods {
        set.push(&section, || heading, row);
    }
    set.into_rows()
}

#[cfg(test)]
#[path = "options_sections_tests.rs"]
mod tests;
