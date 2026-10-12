//! The detection rules: which telemetry is an infraction. Each rule is a pure function of what the caller
//! measured this frame, so the thresholds can be tested without a world.

use crate::infraction::Infraction;

/// Closing speed (m/s) above which a deliberate hit on traffic counts as hit and run.
pub const HIT_AND_RUN_CLOSING: f32 = 4.0;
/// Distance (m) to the nearest cop within which a hit counts while the cops see the player.
pub const COP_NEAR: f32 = 25.0;

/// What a contact struck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Traffic,
    Cop,
    /// A breakable object (a fence, a sign, a stall).
    Breakable,
}

/// A contact of the player's car with something, and the state of the chase at that moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    pub target: Target,
    /// The player meant the hit. The caller decides this (see the spec's open questions).
    pub deliberate: bool,
    /// Relative speed along the contact normal, m/s.
    pub closing_speed: f32,
    /// A chase is on.
    pub in_pursuit: bool,
    /// The nearest cop can see the player.
    pub cop_sees: bool,
    /// Distance to the nearest cop, metres.
    pub nearest_cop: f32,
    /// The hit is the first that damages this cop.
    pub first_damage: bool,
}

/// The infraction a contact is, if any. A cop hit is assault; traffic and breakables need the cops to see the
/// player, the nearest cop to be close, and (for traffic) a closing speed above [`HIT_AND_RUN_CLOSING`].
pub fn contact_infraction(contact: &Contact) -> Option<Infraction> {
    if !contact.deliberate || !contact.in_pursuit {
        return None;
    }
    match contact.target {
        Target::Cop if contact.first_damage => Some(Infraction::Assault),
        Target::Cop => None,
        Target::Traffic if contact.closing_speed > HIT_AND_RUN_CLOSING && watched(contact) => {
            Some(Infraction::HitAndRun)
        }
        Target::Traffic => None,
        Target::Breakable if watched(contact) => Some(Infraction::DamageToProperty),
        Target::Breakable => None,
    }
}

fn watched(contact: &Contact) -> bool {
    contact.cop_sees && contact.nearest_cop < COP_NEAR
}

/// Speeding: the player's speed (m/s, either direction) is over the limit.
pub fn speeding(speed: f32, limit: f32) -> bool {
    speed.abs() > limit
}

/// Off road: the player is not on a legal road, the cops see it, and the nearest cop is close.
pub fn off_road(on_legal_road: bool, cop_sees: bool, nearest_cop: f32) -> bool {
    !on_legal_road && cop_sees && nearest_cop < COP_NEAR
}

/// Resisting arrest: the cops lost sight of the player this frame (the chase starts to cool down).
pub fn lost_sight(seen_before: bool, seen_now: bool) -> bool {
    seen_before && !seen_now
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contact(target: Target) -> Contact {
        Contact {
            target,
            deliberate: true,
            closing_speed: 10.0,
            in_pursuit: true,
            cop_sees: true,
            nearest_cop: 10.0,
            first_damage: true,
        }
    }

    #[test]
    fn a_deliberate_hit_on_a_cop_that_damages_it_for_the_first_time_is_assault() {
        assert_eq!(contact_infraction(&contact(Target::Cop)), Some(Infraction::Assault));
        let again = Contact { first_damage: false, ..contact(Target::Cop) };
        assert_eq!(contact_infraction(&again), None);
    }

    #[test]
    fn traffic_hit_needs_closing_speed_and_a_watching_cop() {
        assert_eq!(contact_infraction(&contact(Target::Traffic)), Some(Infraction::HitAndRun));
        let slow = Contact { closing_speed: HIT_AND_RUN_CLOSING, ..contact(Target::Traffic) };
        assert_eq!(contact_infraction(&slow), None, "the threshold is strict");
        let unseen = Contact { cop_sees: false, ..contact(Target::Traffic) };
        assert_eq!(contact_infraction(&unseen), None);
        let far = Contact { nearest_cop: COP_NEAR, ..contact(Target::Traffic) };
        assert_eq!(contact_infraction(&far), None, "25 m is out");
    }

    #[test]
    fn breakables_need_only_the_watching_cop() {
        assert_eq!(contact_infraction(&contact(Target::Breakable)), Some(Infraction::DamageToProperty));
        let speed_irrelevant = Contact { closing_speed: 0.0, ..contact(Target::Breakable) };
        assert_eq!(contact_infraction(&speed_irrelevant), Some(Infraction::DamageToProperty));
    }

    #[test]
    fn accidents_and_contacts_outside_a_chase_are_not_infractions() {
        let accident = Contact { deliberate: false, ..contact(Target::Cop) };
        assert_eq!(contact_infraction(&accident), None);
        let no_chase = Contact { in_pursuit: false, ..contact(Target::Traffic) };
        assert_eq!(contact_infraction(&no_chase), None);
    }

    #[test]
    fn speeding_is_strictly_over_the_limit_either_way() {
        assert!(speeding(30.0, 29.06));
        assert!(!speeding(29.06, 29.06));
        assert!(speeding(-30.0, 29.06), "reverse counts");
    }

    #[test]
    fn off_road_needs_the_cops_watching_close_by() {
        assert!(off_road(false, true, 10.0));
        assert!(!off_road(true, true, 10.0));
        assert!(!off_road(false, false, 10.0));
        assert!(!off_road(false, true, 40.0));
    }

    #[test]
    fn losing_sight_is_the_edge_not_the_state() {
        assert!(lost_sight(true, false));
        assert!(!lost_sight(false, false));
        assert!(!lost_sight(true, true));
    }
}
