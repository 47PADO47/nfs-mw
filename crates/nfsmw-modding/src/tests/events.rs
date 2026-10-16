//! `nfsmw.*` events: the game builds a payload and an event, and the mod decodes the payload bytes. The bytes are the
//! payload's own memory, as the host would hand them over.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr::from_ref;

use crate::abi::{
    BbMwBusted, BbMwEscaped, BbMwHeatChanged, BbMwInfraction, BbMwPursuitEnded, BbMwPursuitStarted, MW_EVENT_BUSTED,
    MW_EVENT_ESCAPED, MW_EVENT_HEAT_CHANGED, MW_EVENT_INFRACTION_RECORDED, MW_EVENT_PURSUIT_ENDED,
    MW_EVENT_PURSUIT_STARTED,
};
use crate::host::mw_event;
use crate::sdk::{EndReason, Infraction, MwEvent};

/// The bytes of a plain value, as the host lays it out.
fn bytes_of<T>(value: &T) -> &[u8] {
    // SAFETY: `value` is a live reference to `size_of::<T>()` bytes; the slice borrows it for the same lifetime.
    unsafe { core::slice::from_raw_parts(from_ref(value).cast::<u8>(), size_of::<T>()) }
}

#[test]
fn the_event_points_at_its_payload() {
    let payload = BbMwPursuitStarted { size: size_of::<BbMwPursuitStarted>() as u32, level: 3, cops: 5, heat: 4.5 };
    let event = mw_event(MW_EVENT_PURSUIT_STARTED, 99, &payload);
    assert_eq!(event.kind, MW_EVENT_PURSUIT_STARTED);
    assert_eq!(event.frame, 99);
    assert_eq!(event.payload, from_ref(&payload).cast::<c_void>());
}

#[test]
fn every_kind_decodes_from_the_bytes_the_host_writes() {
    let started = BbMwPursuitStarted { size: size_of::<BbMwPursuitStarted>() as u32, level: 3, cops: 5, heat: 4.5 };
    assert_eq!(
        MwEvent::decode(MW_EVENT_PURSUIT_STARTED, 1, bytes_of(&started)),
        MwEvent::PursuitStarted { frame: 1, level: 3, cops: 5, heat: 4.5 }
    );

    let ended = BbMwPursuitEnded {
        size: size_of::<BbMwPursuitEnded>() as u32,
        reason: EndReason::Busted.raw(),
        bounty: 1_500,
        duration: 42.0,
        _pad: 0,
    };
    assert_eq!(
        MwEvent::decode(MW_EVENT_PURSUIT_ENDED, 2, bytes_of(&ended)),
        MwEvent::PursuitEnded { frame: 2, reason: EndReason::Busted, bounty: 1_500, duration: 42.0 }
    );

    let heat = BbMwHeatChanged { size: size_of::<BbMwHeatChanged>() as u32, level: 4, old_heat: 3.0, new_heat: 4.25 };
    assert_eq!(
        MwEvent::decode(MW_EVENT_HEAT_CHANGED, 3, bytes_of(&heat)),
        MwEvent::HeatChanged { frame: 3, level: 4, old_heat: 3.0, new_heat: 4.25 }
    );

    let busted = BbMwBusted { size: size_of::<BbMwBusted>() as u32, level: 5, bounty: 9_000 };
    assert_eq!(
        MwEvent::decode(MW_EVENT_BUSTED, 4, bytes_of(&busted)),
        MwEvent::Busted { frame: 4, level: 5, bounty: 9_000 }
    );

    let escaped = BbMwEscaped { size: size_of::<BbMwEscaped>() as u32, level: 2, duration: 17.5 };
    assert_eq!(
        MwEvent::decode(MW_EVENT_ESCAPED, 5, bytes_of(&escaped)),
        MwEvent::Escaped { frame: 5, level: 2, duration: 17.5 }
    );

    let infraction =
        BbMwInfraction { size: size_of::<BbMwInfraction>() as u32, infraction: Infraction::Racing.raw(), fine: 250 };
    assert_eq!(
        MwEvent::decode(MW_EVENT_INFRACTION_RECORDED, 6, bytes_of(&infraction)),
        MwEvent::InfractionRecorded { frame: 6, infraction: Infraction::Racing, fine: 250 }
    );
}

#[test]
fn an_older_host_with_a_shorter_payload_still_decodes_the_fields_it_has() {
    // A payload of a host that does not know `duration` yet: the size word covers only the first 16 bytes.
    let mut ended = BbMwPursuitEnded {
        size: size_of::<BbMwPursuitEnded>() as u32,
        reason: EndReason::Evaded.raw(),
        bounty: 0,
        duration: 1.0,
        _pad: 0,
    };
    ended.size = 16;
    assert_eq!(
        MwEvent::decode(MW_EVENT_PURSUIT_ENDED, 7, bytes_of(&ended)),
        MwEvent::Unknown { kind: MW_EVENT_PURSUIT_ENDED, frame: 7 }
    );
}

#[test]
fn a_payload_cut_short_or_an_unknown_number_is_unknown_not_a_read_past_the_end() {
    let busted = BbMwBusted { size: size_of::<BbMwBusted>() as u32, level: 5, bounty: 9_000 };
    let short = &bytes_of(&busted)[..8];
    assert_eq!(MwEvent::decode(MW_EVENT_BUSTED, 8, short), MwEvent::Unknown { kind: MW_EVENT_BUSTED, frame: 8 });

    let bad_reason =
        BbMwPursuitEnded { size: size_of::<BbMwPursuitEnded>() as u32, reason: 9, bounty: 0, duration: 0.0, _pad: 0 };
    assert_eq!(
        MwEvent::decode(MW_EVENT_PURSUIT_ENDED, 9, bytes_of(&bad_reason)),
        MwEvent::Unknown { kind: MW_EVENT_PURSUIT_ENDED, frame: 9 }
    );

    assert_eq!(MwEvent::decode(0x1234, 10, &[]), MwEvent::Unknown { kind: 0x1234, frame: 10 });
}
