//! The pursuit look's data: the cold `uves` look and the timed effects that blend the frame to it, flash it and blur it
//! when a pursuit starts or its whole heat level rises. Spec: `docs/specs/pursuit-look.md`.
use super::{look, ordinates};
use blackbox_attrib::Database;
use blackbox_gfx::ColourTreatment;

/// A timed effect of a look (`visuallookeffect`): a curve of four Bezier ordinates over `length` seconds, scaled by
/// `magnitude`. It stops once the whole heat level falls under `heat_trigger`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    pub length: f32,
    pub magnitude: f32,
    pub heat_trigger: f32,
    pub graph: [f32; 4],
}

impl Effect {
    /// The value `elapsed` seconds after the effect started: the first ordinate at its start and the last one from its
    /// end on, each times the magnitude.
    pub fn value(&self, elapsed: f32) -> f32 {
        let t = (elapsed / self.length).clamp(0.0, 1.0);
        let u = 1.0 - t;
        let [a, b, c, d] = self.graph;
        (u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d) * self.magnitude
    }
}

/// The pursuit look. Each part is missing when its data is, which leaves out only that part.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Profile {
    /// The look the frame blends to.
    pub look: Option<ColourTreatment>,
    /// The blend's weight over time (`uves_transition`).
    pub transition: Option<Effect>,
    /// The flash: what is added to the frame's brightness of 1 (`uves_pulse`).
    pub pulse: Option<Effect>,
    /// The radial blur's strength (`uves_radialblur`).
    pub radial_blur: Option<Effect>,
    /// The arrest scene's look (`copcam`), used as it is while the scene plays.
    pub arrest: Option<ColourTreatment>,
}

pub fn load(db: &Database) -> Profile {
    Profile {
        look: look(db, "uves"),
        transition: effect(db, "uves_transition"),
        pulse: effect(db, "uves_pulse"),
        radial_blur: effect(db, "uves_radialblur"),
        arrest: look(db, "copcam"),
    }
}

/// An effect of `name`, `None` when its data is missing, not finite or has no length.
fn effect(db: &Database, name: &str) -> Option<Effect> {
    let collection = db.collection("visuallookeffect", name)?;
    let effect = Effect {
        length: collection.get("length")?.as_f32()?,
        magnitude: collection.get("magnitude")?.as_f32()?,
        heat_trigger: collection.get("heattrigger")?.as_f32()?,
        graph: ordinates(collection.get("graph")?)?,
    };
    let finite = [effect.length, effect.magnitude, effect.heat_trigger].iter().all(|v| v.is_finite());
    (finite && effect.length > 0.0 && effect.magnitude.abs() <= 16.0).then_some(effect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_effect_follows_its_curve_and_holds_its_ends() {
        let effect = Effect { length: 2.0, magnitude: 4.0, heat_trigger: 0.1, graph: [0.5, 1.0, 1.0, 0.0] };
        assert_eq!(effect.value(-1.0), 2.0);
        assert_eq!(effect.value(0.0), 2.0);
        assert!((effect.value(1.0) - 4.0 * (0.125 * 0.5 + 0.375 + 0.375)).abs() < 1e-6);
        assert_eq!(effect.value(2.0), 0.0);
        assert_eq!(effect.value(9.0), 0.0);
    }

    #[test]
    #[ignore = "requires NFSMW_GAME_DIR"]
    fn installed_pursuit_look_loads() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        let profile = load(&db);
        eprintln!("{profile:?}");
        let look = profile.look.expect("the uves look");
        assert!(look.tint[2] > look.tint[0], "the pursuit look is a cold one: {:?}", look.tint);
        let transition = profile.transition.expect("uves_transition");
        assert!(transition.length > 0.0 && transition.value(0.0) > 0.9 && transition.value(transition.length) == 0.0);
        let pulse = profile.pulse.expect("uves_pulse");
        assert!(pulse.length < transition.length && pulse.magnitude > 1.0);
        let blur = profile.radial_blur.expect("uves_radialblur");
        assert!(blur.length > transition.length && blur.heat_trigger > 0.0);
        let arrest = profile.arrest.expect("the copcam look");
        assert!(arrest.tint[2] > 0.5 && arrest.tint[0] < 0.2, "the arrest look is dark blue: {:?}", arrest.tint);
    }
}
