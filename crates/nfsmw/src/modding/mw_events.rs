//! The `nfsmw.*` notices as the mods receive them: each notice becomes its payload struct, and the struct goes out as a
//! `BbEvent` of its kind (ADR 0005 section 4; the payloads are in `nfsmw_modding::abi::events`).

use std::mem::size_of;

use blackbox_modding::abi::BbEvent;
use nfsmw_modding::abi::{
    BbMwBusted, BbMwEscaped, BbMwHeatChanged, BbMwInfraction, BbMwPursuitEnded, BbMwPursuitStarted, MW_EVENT_BUSTED,
    MW_EVENT_ESCAPED, MW_EVENT_HEAT_CHANGED, MW_EVENT_INFRACTION_RECORDED, MW_EVENT_PURSUIT_ENDED,
    MW_EVENT_PURSUIT_STARTED,
};
use nfsmw_modding::host::mw_event;

use super::pursuit_feed::Notice;

/// The payload of one notice: the struct the event carries, of the kind it names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Payload {
    Started(BbMwPursuitStarted),
    Ended(BbMwPursuitEnded),
    HeatChanged(BbMwHeatChanged),
    Busted(BbMwBusted),
    Escaped(BbMwEscaped),
    Infraction(BbMwInfraction),
}

impl Payload {
    /// The event of this payload, sent at `frame`. The event points into `self`, so it is valid while `self` is.
    pub fn event(&self, frame: u64) -> BbEvent {
        match self {
            Payload::Started(payload) => mw_event(MW_EVENT_PURSUIT_STARTED, frame, payload),
            Payload::Ended(payload) => mw_event(MW_EVENT_PURSUIT_ENDED, frame, payload),
            Payload::HeatChanged(payload) => mw_event(MW_EVENT_HEAT_CHANGED, frame, payload),
            Payload::Busted(payload) => mw_event(MW_EVENT_BUSTED, frame, payload),
            Payload::Escaped(payload) => mw_event(MW_EVENT_ESCAPED, frame, payload),
            Payload::Infraction(payload) => mw_event(MW_EVENT_INFRACTION_RECORDED, frame, payload),
        }
    }
}

/// The payload of `notice`, with the size word the host fills in.
pub fn payload_of(notice: &Notice) -> Payload {
    match *notice {
        Notice::PursuitStarted { level, cops, heat } => {
            Payload::Started(BbMwPursuitStarted { size: size_of::<BbMwPursuitStarted>() as u32, level, cops, heat })
        }
        Notice::PursuitEnded { reason, bounty, duration } => Payload::Ended(BbMwPursuitEnded {
            size: size_of::<BbMwPursuitEnded>() as u32,
            reason: reason as u32,
            bounty,
            duration,
            _pad: 0,
        }),
        Notice::HeatChanged { level, old_heat, new_heat } => Payload::HeatChanged(BbMwHeatChanged {
            size: size_of::<BbMwHeatChanged>() as u32,
            level,
            old_heat,
            new_heat,
        }),
        Notice::Busted { level, bounty } => {
            Payload::Busted(BbMwBusted { size: size_of::<BbMwBusted>() as u32, level, bounty })
        }
        Notice::Escaped { level, duration } => {
            Payload::Escaped(BbMwEscaped { size: size_of::<BbMwEscaped>() as u32, level, duration })
        }
        Notice::InfractionRecorded { infraction, fine } => Payload::Infraction(BbMwInfraction {
            size: size_of::<BbMwInfraction>() as u32,
            infraction: infraction as u32,
            fine,
        }),
    }
}

/// Send each notice, in order, as its event of `frame`. `send` gets each event for the call only.
pub fn send_all(notices: &[Notice], frame: u64, send: &mut dyn FnMut(&BbEvent)) {
    for notice in notices {
        let payload = payload_of(notice);
        send(&payload.event(frame));
    }
}

#[cfg(test)]
mod tests {
    use nfsmw_modding::abi::{EndReason, Infraction};

    use super::*;

    #[test]
    fn a_notice_becomes_the_payload_of_its_kind_with_its_size_word() {
        let busted = payload_of(&Notice::Busted { level: 6, bounty: 4_000 });
        assert_eq!(
            busted,
            Payload::Busted(BbMwBusted { size: size_of::<BbMwBusted>() as u32, level: 6, bounty: 4_000 })
        );
        let ended = payload_of(&Notice::PursuitEnded { reason: EndReason::Aborted, bounty: 9, duration: 1.5 });
        assert_eq!(
            ended,
            Payload::Ended(BbMwPursuitEnded {
                size: size_of::<BbMwPursuitEnded>() as u32,
                reason: 2,
                bounty: 9,
                duration: 1.5,
                _pad: 0
            })
        );
        let fine = payload_of(&Notice::InfractionRecorded { infraction: Infraction::OffRoad, fine: 250 });
        assert_eq!(
            fine,
            Payload::Infraction(BbMwInfraction { size: size_of::<BbMwInfraction>() as u32, infraction: 7, fine: 250 })
        );
    }

    #[test]
    fn each_notice_reaches_the_runtime_once_in_order_with_its_kind_and_frame() {
        // A fake runtime: it records the kind and frame of every event it is sent.
        let notices = [
            Notice::PursuitStarted { level: 2, cops: 4, heat: 2.1 },
            Notice::HeatChanged { level: 3, old_heat: 2.9, new_heat: 3.0 },
            Notice::Escaped { level: 3, duration: 40.0 },
            Notice::PursuitEnded { reason: EndReason::Evaded, bounty: 0, duration: 40.0 },
        ];
        let mut received: Vec<(u32, u64)> = Vec::new();
        send_all(&notices, 77, &mut |event| received.push((event.kind, event.frame)));
        assert_eq!(
            received,
            [
                (MW_EVENT_PURSUIT_STARTED, 77),
                (MW_EVENT_HEAT_CHANGED, 77),
                (MW_EVENT_ESCAPED, 77),
                (MW_EVENT_PURSUIT_ENDED, 77),
            ]
        );
    }
}
