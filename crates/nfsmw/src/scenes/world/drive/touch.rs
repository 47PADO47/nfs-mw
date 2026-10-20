//! How long ago the player's car last touched a prop (a placed model: a pump, a pillar, a sign). A breaker's cinematic
//! moment only plays for a structure the player's own car touched a moment ago (`docs/specs/pursuit-breaker-moment.md`).

use super::walls::{ContactKind, ContactPoint};

/// Seconds since the last prop contact; `None` before the first.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct PropTouch {
    age: Option<f32>,
}

impl PropTouch {
    /// One physics step of `dt` seconds with the contacts the car had in it.
    pub fn step(&mut self, contacts: &[ContactPoint], dt: f32) {
        let touched = contacts.iter().any(|c| matches!(c.kind, ContactKind::PropRigid | ContactKind::PropMovable));
        self.age = match touched {
            true => Some(0.0),
            false => self.age.map(|age| age + dt),
        };
    }

    pub fn age(&self) -> Option<f32> {
        self.age
    }
}

impl super::Drive {
    /// Seconds since the car last touched a prop.
    pub fn prop_touch_age(&self) -> Option<f32> {
        self.prop_touch.age()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn contact(kind: ContactKind) -> ContactPoint {
        ContactPoint { point: Vec3::ZERO, normal: Vec3::Y, kind }
    }

    #[test]
    fn a_prop_contact_resets_the_age_and_other_contacts_do_not() {
        let mut touch = PropTouch::default();
        assert_eq!(touch.age(), None);
        touch.step(&[contact(ContactKind::Barrier)], 0.1);
        assert_eq!(touch.age(), None, "a guard rail is not a prop");
        touch.step(&[contact(ContactKind::PropMovable)], 0.1);
        assert_eq!(touch.age(), Some(0.0));
        touch.step(&[], 0.5);
        touch.step(&[contact(ContactKind::Face)], 0.25);
        assert_eq!(touch.age(), Some(0.75));
        touch.step(&[contact(ContactKind::PropRigid)], 0.1);
        assert_eq!(touch.age(), Some(0.0));
    }
}
