//! Small closed sets that the `nfsmw.*` calls name by number. Across the boundary they are `u32`; a number that is not
//! listed is `BB_ERR_ARG`. A number is never reused (ADR 0005 section 1), and a new one takes the next free value.

/// Declares a closed set as a `repr(u32)` enum with `from_raw` (`None` for an unlisted number) and `raw`.
macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $value:expr),+ $(,)? }) => {
        $(#[$meta])*
        #[repr(u32)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name {
            $(
                #[doc = concat!("Wire number ", stringify!($value), ".")]
                $variant = $value,
            )+
        }

        impl $name {
            /// The variant a wire number names. `None` for a number that is not listed.
            pub const fn from_raw(raw: u32) -> Option<Self> {
                $(
                    if raw == $value {
                        return Some($name::$variant);
                    }
                )+
                None
            }

            /// The wire number of this variant.
            pub const fn raw(self) -> u32 {
                self as u32
            }
        }
    };
}

wire_enum! {
    /// The cop table a call names: free roam (`heattable`) or a race event (`racetable`).
    CopTable {
        Roam = 0,
        Race = 1,
    }
}

wire_enum! {
    /// How a call ends the pursuit. The numbers are shared with [`EndReason`].
    EndMode {
        Evade = 0,
        Bust = 1,
        Abort = 2,
    }
}

wire_enum! {
    /// Why a pursuit ended, as the `PursuitEnded` event reports it.
    EndReason {
        Evaded = 0,
        Busted = 1,
        Aborted = 2,
    }
}

wire_enum! {
    /// An infraction, in the order of `libs/blackbox-pursuit` (`Infraction`).
    Infraction {
        Speeding = 0,
        Racing = 1,
        RecklessDriving = 2,
        Assault = 3,
        HitAndRun = 4,
        DamageToProperty = 5,
        ResistingArrest = 6,
        OffRoad = 7,
    }
}

wire_enum! {
    /// The kind of a shop. [unconfirmed] against the career spec.
    ShopKind {
        AutoShop = 0,
        CarLot = 1,
        Performance = 2,
        Paint = 3,
    }
}
