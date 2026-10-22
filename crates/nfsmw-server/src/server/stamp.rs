//! The per-connection sequence number of gameplay events (protocol-messages.md section 7.6): every message with an
//! [`nfsmw_net::message::EventHead`] gets the connection's next `seq` when it is sent.

use nfsmw_net::message::Message;

/// Gives `seq` to the head of `message` when it has one. Returns whether it did.
pub(crate) fn stamp(message: &mut Message, seq: u32) -> bool {
    macro_rules! heads {
        ($($variant:ident),* $(,)?) => {
            match message {
                $( Message::$variant(body) => body.head.seq = seq, )*
                _ => return false,
            }
        };
    }
    heads!(
        RaceStart,
        Checkpoint,
        LapComplete,
        RaceFinish,
        RaceResults,
        PursuitStart,
        PursuitEnd,
        HeatChange,
        Infraction,
        BustedProgress,
        RoadblockEvent,
        SpikeEvent,
        Collision,
        Damage,
        Puncture,
        Repair,
        Respawn,
        PhaseChange,
        EntityRemoved,
        PlayerJoined,
        PlayerLeft,
        RoleChanged,
        ClockEvent,
    );
    true
}
