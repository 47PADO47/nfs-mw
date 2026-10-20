//! The lines the pursuit's radio can ask for: one variant per sentence function of the original's speech interface,
//! named for what is said. The game maps a [`Line`] to its speech event; the machine only chooses lines and fills their
//! [`Params`](super::Params).

/// A voice: 1 is dispatch, 2 the helicopter, the others the cops ([`Layout`](super::Layout) says which). 0 means no
/// particular voice (a line with no speaker).
pub type Speaker = u8;

macro_rules! lines {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        /// One line of the radio.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Line {
            $($(#[$doc])* $name),*
        }

        impl Line {
            /// Every line, in declaration order.
            pub const ALL: &'static [Line] = &[$(Line::$name),*];
        }
    };
}

lines! {
    // Opening exchange.
    Spotter, SpotterWanted, SpotterReply, AttemptVehicleStop, GoAhead, PrimaryEngage, InitiatePursuit,
    SuspectConfirmed, ReinitiatePursuit, VehicleReport, LocationReport, SelfStrategy, InitialCallForBackup,
    InitialCallForBackupMulti, Bullhorn, VehicleDescription, NoVehicleDescription, MoreDetails, DriverHistory,
    // Backup.
    CallForBackup, UnitBackupReply, DispBackupReply, CallForSwarming, DispBackupEta, BackupReminder,
    NegativeBackupReply, DispBackupUpdate, BackupArrives,
    // Roadblocks.
    CallForRoadblock, RoadblockReminder, NegativeRoadblockReply, DispRoadblockReply, DispRoadblockUpdate,
    PursuitApproaching, RoadblockApproach, RoadblockEngage, RoadblockAverted, CallForSubRoadblock, DispSubRoadblock,
    RoadblockWarning, RoadblockPosition, ExtraRoadblockEngage, ExtraRoadblockAverted,
    // Rolling strategies and their outcome.
    InitiateStrategy, CallToPosition, CallToPositionReminder, StrategyExecute, AnticipateFail, AnticipateSuccess,
    OutcomeFail, StrategyReset,
    // Arrest.
    BullhornArrest, Arrest, DispArrestReply,
    // Collisions and the player's driving.
    CollisionWorld, CollWorldCivi, CollWorldSpin, CollWorldAir, CollWorldFlip, SuspectUTurn, SuspectOutrun,
    SuspectBrake, SuspectBehaviour, OffroadMoment, DirectionHigh,
    // Sight and the pursuit's course.
    LostVisual, RegainVisual, LostSuspect, DispBreakAway, DispTimeExpired, DispPursuitEscalation,
    DispPursuitEscalationGeneric, DispPursuitUpdate, PursuitUpdateReply, Spotted, FocusChange, HeatJump,
    DispJurisShift, WeatherReport,
    // 911.
    Disp911Report, Unit911Reply,
    // Cops going down.
    UnitDisabled, CallForEv, IntentToRam, Bailout, BailoutDeny,
    // Interrupts and replies.
    Interrupt, InterruptExpletive, InterruptViolent, RamRearEnd, RamHeadOn, RamSideSwipe, RamTBone, Acknowledge,
    // Lost-contact fiction and the swarming replies (no speaker).
    SwarmingReply, SwarmingReplyFollow, QuadrantForming, SuspectPossiblyGone, QuadrantMoving, OtherLead,
    PossibleSuspect, WrongSuspect,
    // The helicopter.
    HeliSelfStrategy, HeliLostVisual, HeliIntentToBail, HeliBailout, HeliSwarming, HeliSpotter, HeliHazardAlert,
    HeliQuadrant, HeliQuadrantMoving, HeliBullhornArrest,
}

impl Line {
    /// Whether the line is said with no unit's voice (the fiction of the lost-contact sequence, the roadblock warning).
    pub fn is_voiceless(self) -> bool {
        matches!(
            self,
            Line::SwarmingReply
                | Line::SwarmingReplyFollow
                | Line::QuadrantForming
                | Line::SuspectPossiblyGone
                | Line::QuadrantMoving
                | Line::OtherLead
                | Line::PossibleSuspect
                | Line::WrongSuspect
                | Line::RoadblockWarning
                | Line::RoadblockPosition
                | Line::ExtraRoadblockEngage
                | Line::ExtraRoadblockAverted
        )
    }
}
