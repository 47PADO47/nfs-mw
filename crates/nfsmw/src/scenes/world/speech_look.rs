//! How the cops describe the player's car: its type and its colour, as the speech interface names them. The type is the
//! `VerbalType` of the car's `pvehicle` collection and the colour the `SPEECHCOLOUR` of the paint actually on the car:
//! the paint the garage applied, else the base paint the car was built with (its preset's or its customization's), as
//! the original reads the base-paint slot of the car's record. Only a car built with no paint of its own falls back to
//! its type's stock paint. A vinyl never changes it. A repaint with a custom colour (a rewrite extension) has none, and a
//! car the tables do not know has neither. Cached per car and paint, since reading the tables is not for every frame.

use std::cell::RefCell;

use nfsmw_data::car::CarTables;
use nfsmw_data::speech_chase::{paint_speech_colour, stock_speech_colour, verbal_type};

use super::WorldScene;

/// The car's type and colour bits (0: unknown).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Look {
    pub car_type: u32,
    pub colour: u32,
}

/// The paint a car's look is read for: a custom colour (none to say), a paint part by its name hash, or the stock paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintKey {
    Custom,
    Part(u32),
    Stock,
}

/// The look of the car last asked about, and the base paint the car was built with.
#[derive(Default)]
pub struct Cache {
    key: Option<(String, PaintKey)>,
    look: Look,
    /// The name hash of the base paint part the driven car's model was built with (`start_driving_setup`).
    built_paint: Option<u32>,
}

impl Cache {
    fn get(&mut self, car: &str, paint: PaintKey, read: impl FnOnce() -> Look) -> Look {
        if self.key.as_ref().is_some_and(|(c, p)| c == car && *p == paint) {
            return self.look;
        }
        self.look = read();
        self.key = Some((car.to_owned(), paint));
        self.look
    }

    /// The car was built with the base paint part named `paint` (its name hash), `None` for no paint of its own.
    pub fn set_built_paint(&mut self, paint: Option<u32>) {
        self.built_paint = paint;
    }
}

impl WorldScene {
    /// The look of the car being driven, `None` while none is.
    pub(super) fn player_look(&self, cache: &RefCell<Cache>) -> Option<Look> {
        let car = &self.players.lead()?.car_name;
        let applied = self.garage.applied.parts.parts[nfsmw_data::car::customization::Category::Paint as usize];
        let built = cache.borrow().built_paint;
        let paint = match (self.garage.applied.color, applied.or(built)) {
            (Some(_), _) => PaintKey::Custom,
            (None, Some(part)) => PaintKey::Part(part),
            (None, None) => PaintKey::Stock,
        };
        Some(cache.borrow_mut().get(car, paint, || self.read_look(car, paint)))
    }

    fn read_look(&self, car: &str, paint: PaintKey) -> Look {
        let Ok(tables) = CarTables::load(&self.dir) else { return Look::default() };
        let Some(info) = tables.car_type_for_folder(car) else { return Look::default() };
        let Ok(db) = nfsmw_data::shared::attributes(&self.dir) else { return Look::default() };
        let colour = match paint {
            PaintKey::Custom => 0,
            PaintKey::Part(part) => paint_speech_colour(&tables, info, part),
            PaintKey::Stock => stock_speech_colour(&tables, info),
        };
        Look { car_type: verbal_type(&db, &info.type_name), colour }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_look_is_read_once_per_car_and_paint() {
        let mut cache = Cache::default();
        let mut reads = 0;
        let mut read = |look: Look| {
            reads += 1;
            look
        };
        let a = cache.get("M3", PaintKey::Stock, || read(Look { car_type: 64, colour: 4 }));
        let b = cache.get("M3", PaintKey::Stock, || read(Look::default()));
        assert_eq!(a, b);
        let c = cache.get("M3", PaintKey::Custom, || read(Look { car_type: 64, colour: 0 }));
        assert_eq!(c.colour, 0);
        let d = cache.get("Elise", PaintKey::Custom, || read(Look { car_type: 2048, colour: 0 }));
        assert_eq!(d.car_type, 2048);
        let e = cache.get("Elise", PaintKey::Part(7), || read(Look { car_type: 2048, colour: 64 }));
        assert_eq!(e.colour, 64, "another paint on the same car is read again");
        assert_eq!(reads, 4);
    }
}
