/// The `SceneType` field of a scene header: which placement and car rules the scene uses.
///
/// Placement is in [`crate::placement`]; the rules that the game applies to cars and cops are not in this crate.
/// Spec: `docs/specs/nis-player.md`, section 6.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SceneType {
    /// Type 0: placed at the marker (the scene's origin is the marker point and it is turned by the marker heading).
    Type0 = 0,
    /// Type 1: never used by the install. Placed at the origin with no turn, like [`SceneType::Type4`].
    Type1 = 1,
    /// Type 2: placed at the marker, like type 0.
    Type2 = 2,
    /// Type 3: placed at the marker, like type 0.
    Type3 = 3,
    /// Type 4: placed at the origin with no turn. The scene is authored in the render frame.
    Type4 = 4,
}

impl SceneType {
    /// The scene type with this raw value, or `None` for a value the format does not define.
    pub fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            0 => Some(Self::Type0),
            1 => Some(Self::Type1),
            2 => Some(Self::Type2),
            3 => Some(Self::Type3),
            4 => Some(Self::Type4),
            _ => None,
        }
    }

    /// True for the types placed at the marker (0, 2 and 3). The others use the origin and no turn.
    pub fn places_at_marker(self) -> bool {
        matches!(self, Self::Type0 | Self::Type2 | Self::Type3)
    }
}
