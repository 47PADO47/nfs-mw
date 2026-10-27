//! When the collision spark variant plays a contact, before the profile's gates say how strong it is
//! (`docs/modding/effect-variants-sparks-xenon.md`, "What stays the game's").
//!
//! A car against a wall or a prop (`Kind::World`, `Kind::Object`) is played by the profile alone: the impact needs the contact to
//! close (the install's rule: the squared closing speed above 1) and the scrape needs only that the car slides; the profile's gate
//! then says how hard, so a slide the install would not spark (its scrape starts at 5 m/s, the 2012 gate at 0.73) still does. The
//! install's links stay for two things: whether the surface has a particle group at all, and the distance it is drawn within.
//! The ground and the car's underside (`Kind::Ground`, `bottom`) keep the install's own acceptance.

use nfsmw_data::vehicle_effects::contacts::{Action, Link, SourceEffects};

use super::super::{Kind, RawContact};

/// The squared closing speed above which a contact is an impact, m/s squared (the install's rule).
const MIN_CLOSING_SQUARED: f32 = 1.0;

/// The distance within which the install's group for a contact is drawn, where it has a particle group.
fn cull(link: Option<Link>) -> Option<f32> {
    link.and_then(|link| link.particles.map(|_| link.visual_cull))
}

/// The cull distances of the impact and of the scrape of `c` for the variant, or `None` where it is not played. `actions` are the
/// install's impact and scrape actions of the contact, `installed` the links the install accepted for them.
pub(in super::super) fn culls(
    profile: &SourceEffects,
    actions: (Action, Action),
    c: &RawContact,
    installed: (Option<Link>, Option<Link>),
) -> (Option<f32>, Option<f32>) {
    if c.bottom || !matches!(c.kind, Kind::World | Kind::Object) {
        return (cull(installed.0), cull(installed.1));
    }
    let closes = c.closing.length_squared() > MIN_CLOSING_SQUARED;
    let hit = closes.then(|| cull(profile.pick(actions.0, c.surface))).flatten();
    (hit, cull(profile.pick(actions.1, c.surface)))
}

#[cfg(test)]
#[path = "trigger_tests.rs"]
mod tests;
