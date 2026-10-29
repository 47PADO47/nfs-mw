//! The footer of a screen: which prompts the screen's footer shows besides its package's own (`prompts.rs`).

/// What the footer of a screen offers besides what its package name gives (`prompts.rs`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Footer {
    #[default]
    Standard,
    ControlsPages,
    ControlsCapture,
    /// EA Trax changes its navigation hints while reordering.
    Jukebox {
        reorder: bool,
        has_songs: bool,
    },
    /// Only Back: the alias manager, Quick Race and profile list; also the Remastered career and safe house lists.
    BackOnly,
    /// Back then Accept: the original category, career and safe house carousels; the custom race's track list and
    /// the mode select of a mode with no tutorial.
    BackAccept,
    /// Back, Accept and `2 View Tutorial`: the mode select on a mode that has a tutorial movie.
    Tutorial,
    /// Back, Accept and `1 Defaults`: the custom race's options.
    Defaults,
    /// Back, Accept and `3 Showcase`: the custom race's car select.
    CarSelect,
    /// Back only, on the custom race's car select: the car shown is locked.
    CarSelectLocked,
    /// Back, `9 0 Zoom` and the arrows `View Car`: the Showcase.
    Orbit,
    GarageManager {
        add: bool,
        /// A real selected/previewed car is available; the manager's Add tile has none.
        showcase: bool,
    },
    GarageMain {
        shop: bool,
    },
    GarageCategories {
        max_performance: bool,
    },
    GarageParts,
    GarageSelect,
    GarageCart {
        active: bool,
    },
    GarageTuning,
    GarageTuningHelp,
    GarageNumbers,
}

impl Footer {
    pub fn is_garage(self) -> bool {
        matches!(
            self,
            Self::GarageManager { .. }
                | Self::GarageMain { .. }
                | Self::GarageCategories { .. }
                | Self::GarageParts
                | Self::GarageSelect
                | Self::GarageCart { .. }
                | Self::GarageTuning
                | Self::GarageTuningHelp
                | Self::GarageNumbers
        )
    }
    /// Whether the footer is one of the custom race's car select (`Car_Select.fng` also serves the career's car lot).
    pub fn is_custom_car_select(self) -> bool {
        matches!(self, Footer::CarSelect | Footer::CarSelectLocked)
    }
}
