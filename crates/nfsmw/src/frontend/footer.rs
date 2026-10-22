//! The footer of a screen: which prompts the screen's footer shows besides its package's own (`prompts.rs`).

/// What the footer of a screen offers besides what its package name gives (`prompts.rs`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Footer {
    #[default]
    Standard,
    /// Only Back: the menus that open from the main menu and are left by Back (the career menu, the alias manager, the Quick
    /// Race menu, the safe house menu, the profile list).
    BackOnly,
    /// Back and Accept: the custom race's track list and the mode select of a mode with no tutorial.
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
}

impl Footer {
    /// Whether the footer is one of the custom race's car select (`Car_Select.fng` also serves the career's car lot).
    pub fn is_custom_car_select(self) -> bool {
        matches!(self, Footer::CarSelect | Footer::CarSelectLocked)
    }
}
