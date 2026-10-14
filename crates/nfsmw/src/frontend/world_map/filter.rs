//! The blip types the map can show. Each filter is a set of blip styles (`hud/blip_styles.rs`), so the map's icons
//! and colours are the HUD's own.

use crate::hud::{
    AUTO_SHOP, CAR_LOT, RACE_CIRCUIT, RACE_DRAG, RACE_KNOCKOUT, RACE_SPEEDTRAP, RACE_SPRINT, RACE_TOLLBOOTH, SAFE_HOUSE,
};

/// A tab of the map's legend. The map opens on [`Filter::DEFAULT`], which is [`Filter::Locations`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    /// The shops, the safe houses and the car lots.
    Locations,
    /// The race events, of every type.
    Races,
}

const LOCATION_STYLES: [&str; 3] = [AUTO_SHOP, SAFE_HOUSE, CAR_LOT];
const RACE_STYLES: [&str; 6] = [RACE_SPRINT, RACE_DRAG, RACE_CIRCUIT, RACE_KNOCKOUT, RACE_TOLLBOOTH, RACE_SPEEDTRAP];

impl Filter {
    /// The tabs in the order the legend cycles them.
    pub const ALL: [Filter; 2] = [Filter::Locations, Filter::Races];

    /// The filter the map opens on.
    pub const DEFAULT: Filter = Filter::Locations;

    pub fn label(self) -> &'static str {
        match self {
            Filter::Locations => "Locations",
            Filter::Races => "Races",
        }
    }

    /// The blip styles this filter shows.
    pub fn styles(self) -> &'static [&'static str] {
        match self {
            Filter::Locations => &LOCATION_STYLES,
            Filter::Races => &RACE_STYLES,
        }
    }

    /// Whether a blip of `style` belongs to this filter.
    pub fn contains(self, style: &str) -> bool {
        self.styles().contains(&style)
    }

    /// The next tab, wrapping from the last to the first.
    pub fn next(self) -> Filter {
        Filter::ALL[(self.index() + 1) % Filter::ALL.len()]
    }

    /// The previous tab, wrapping from the first to the last.
    pub fn previous(self) -> Filter {
        Filter::ALL[(self.index() + Filter::ALL.len() - 1) % Filter::ALL.len()]
    }

    fn index(self) -> usize {
        Filter::ALL.iter().position(|f| *f == self).unwrap_or(0)
    }
}
