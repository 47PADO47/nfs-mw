//! The flares and glows of a mod car's lamps (the `[[lamp_flare]]` and `[[light_bar]]` tables of its manifest,
//! docs/modding/car-lamps.md and docs/modding/cop-cars.md).
//!
//! A car that publishes lamps here is drawn with these sprites in place of the game's shared flare profiles for the lamps
//! they cover; a car that publishes none keeps the markers and the profiles. The sprites are textures of the car's own
//! `TEXTURES.BIN`, named here and found by name when the car is drawn.

use std::sync::{PoisonError, RwLock};

/// When a lamp is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lit {
    Head,
    Tail,
    Brake,
    Reverse,
    Always,
}

/// Which lamp of the car a sprite belongs to (the damage bit that puts it out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Centre,
}

/// The colour a siren lamp is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Siren {
    Red,
    Blue,
    White,
}

/// The look both kinds of lamp share.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    /// The texture's name in the car's `TEXTURES.BIN`.
    pub sprite: String,
    /// In the car's space, metres.
    pub position: [f32; 3],
    /// A unit vector: the way the lamp shines.
    pub direction: [f32; 3],
    pub colour: [f32; 3],
    /// Brightness at the near and the far distance of `range`.
    pub alpha: [f32; 2],
    /// Radius in metres at the near and the far distance of `range`.
    pub radius: [f32; 2],
    pub range: [f32; 2],
    /// The exponent of the cosine of the angle off the axis.
    pub falloff: f32,
    pub z_bias: f32,
}

/// One flare of a lamp.
#[derive(Debug, Clone, PartialEq)]
pub struct Flare {
    pub lit: Lit,
    pub side: Side,
    pub look: Look,
}

/// One siren lamp: its flash pattern, looped.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub siren: Siren,
    pub look: Look,
    pub pattern: Vec<f32>,
    pub step_seconds: f32,
    pub phase_seconds: f32,
}

/// The lamps of one car.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lamps {
    pub flares: Vec<Flare>,
    pub bars: Vec<Bar>,
}

impl Lamps {
    /// Whether the car draws the flares of this kind of lamp itself: the markers of that kind are then not drawn.
    pub fn covers(&self, lit: Lit) -> bool {
        self.flares.iter().any(|flare| flare.lit == lit)
    }

    /// The pattern's brightness at `seconds`: the step the loop is in, 0 to 1.
    pub fn bar_level(bar: &Bar, seconds: f32) -> f32 {
        if bar.pattern.is_empty() || !seconds.is_finite() || bar.step_seconds <= 0.0 {
            return 0.0;
        }
        let step = ((seconds + bar.phase_seconds) / bar.step_seconds).floor();
        let at = (step as i64).rem_euclid(bar.pattern.len() as i64) as usize;
        bar.pattern[at]
    }
}

/// The cars with lamps, by folder. Each publication leaks the new list, as the mod cars do.
static PUBLISHED: RwLock<&'static [(String, Lamps)]> = RwLock::new(&[]);

/// Register the lamps of the mod car in `folder`. A folder already registered keeps its first.
pub fn publish(folder: &str, lamps: Lamps) {
    let mut published = PUBLISHED.write().unwrap_or_else(PoisonError::into_inner);
    if published.iter().any(|(known, _)| known.eq_ignore_ascii_case(folder)) {
        return;
    }
    let mut all: Vec<(String, Lamps)> = published.to_vec();
    all.push((folder.to_owned(), lamps));
    *published = Box::leak(all.into_boxed_slice());
}

/// The lamps of the mod car in `folder`. `None` for a stock car and for a mod car that gives none.
pub fn of(folder: &str) -> Option<&'static Lamps> {
    let published = *PUBLISHED.read().unwrap_or_else(PoisonError::into_inner);
    published.iter().find(|(known, _)| known.eq_ignore_ascii_case(folder)).map(|(_, lamps)| lamps)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> Look {
        Look {
            sprite: "CAR_FLARE0".into(),
            position: [2.0, 0.6, 0.6],
            direction: [1.0, 0.0, 0.0],
            colour: [1.0; 3],
            alpha: [1.0; 2],
            radius: [0.2, 0.5],
            range: [2.0, 100.0],
            falloff: 1.0,
            z_bias: 0.0,
        }
    }

    fn bar() -> Bar {
        Bar { siren: Siren::Red, look: look(), pattern: vec![1.0, 0.0, 0.0], step_seconds: 0.5, phase_seconds: 0.0 }
    }

    #[test]
    fn a_pattern_plays_step_by_step_in_a_loop_from_its_phase() {
        let mut bar = bar();
        let level = |bar: &Bar, t| Lamps::bar_level(bar, t);
        assert_eq!((level(&bar, 0.1), level(&bar, 0.6), level(&bar, 1.1)), (1.0, 0.0, 0.0));
        assert_eq!(level(&bar, 1.6), 1.0, "after three steps it starts again");
        bar.phase_seconds = 0.5;
        assert_eq!((level(&bar, 0.1), level(&bar, 1.1)), (0.0, 1.0));
        assert_eq!(level(&bar, -0.4), 1.0, "a time before zero loops back");
        assert_eq!(level(&bar, f32::NAN), 0.0);
        bar.pattern.clear();
        assert_eq!(level(&bar, 1.0), 0.0);
    }

    #[test]
    fn a_car_covers_the_kinds_of_lamp_it_gives_flares_for() {
        let lamps = Lamps { flares: vec![Flare { lit: Lit::Head, side: Side::Left, look: look() }], bars: vec![bar()] };
        assert!(lamps.covers(Lit::Head));
        assert!(!lamps.covers(Lit::Brake));
        assert!(!Lamps::default().covers(Lit::Head));
    }

    #[test]
    fn published_lamps_are_found_by_folder_in_any_case() {
        publish("LAMPCAR1", Lamps { flares: Vec::new(), bars: vec![bar()] });
        assert_eq!(of("lampcar1").map(|lamps| lamps.bars.len()), Some(1));
        assert!(of("NOLAMPS").is_none());
    }
}
