//! The animation type ids: the u16 at offset 0 of every animation and channel record.
//!
//! The numbers and names are the decomp's `AnimTypeId` enumeration, as listed in `docs/specs/eagl-animation-channels.md`
//! section 2.3. Every id below 26 has a name; the install uses only a few of them (bank spec section 5).

/// An animation or channel type id.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeId {
    RawPose = 0,
    RawEvent = 1,
    RawLinear = 2,
    Cycle = 3,
    EventBlender = 4,
    Graft = 5,
    PoseBlender = 6,
    PoseMirror = 7,
    RunBlender = 8,
    TurnBlender = 9,
    DeltaLerp = 10,
    DeltaQuat = 11,
    KeyLerp = 12,
    KeyQuat = 13,
    Phase = 14,
    Compound = 15,
    RawState = 16,
    DeltaQ = 17,
    DeltaQFast = 18,
    DeltaSingleQ = 19,
    DeltaF3 = 20,
    DeltaF1 = 21,
    StatelessQ = 22,
    StatelessF3 = 23,
    CsisEvent = 24,
    PoseAnim = 25,
}

impl TypeId {
    /// The id for `value`, or `None` when it is not one of the 26 ids.
    pub fn from_u16(value: u16) -> Option<Self> {
        let id = match value {
            0 => Self::RawPose,
            1 => Self::RawEvent,
            2 => Self::RawLinear,
            3 => Self::Cycle,
            4 => Self::EventBlender,
            5 => Self::Graft,
            6 => Self::PoseBlender,
            7 => Self::PoseMirror,
            8 => Self::RunBlender,
            9 => Self::TurnBlender,
            10 => Self::DeltaLerp,
            11 => Self::DeltaQuat,
            12 => Self::KeyLerp,
            13 => Self::KeyQuat,
            14 => Self::Phase,
            15 => Self::Compound,
            16 => Self::RawState,
            17 => Self::DeltaQ,
            18 => Self::DeltaQFast,
            19 => Self::DeltaSingleQ,
            20 => Self::DeltaF3,
            21 => Self::DeltaF1,
            22 => Self::StatelessQ,
            23 => Self::StatelessF3,
            24 => Self::CsisEvent,
            25 => Self::PoseAnim,
            _ => return None,
        };
        Some(id)
    }

    /// Whether this is a delta codec that the reader can decode (the codecs of `docs/specs/eagl-animation-codecs.md`):
    /// DeltaLerp, DeltaQuat, DeltaF3, DeltaF1 and DeltaQ. The other ids are either containers, blenders or codecs
    /// the install does not use.
    pub fn is_supported_leaf(self) -> bool {
        matches!(self, Self::DeltaLerp | Self::DeltaQuat | Self::DeltaF3 | Self::DeltaF1 | Self::DeltaQ)
    }
}
