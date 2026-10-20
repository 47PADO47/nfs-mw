//! How the cops describe the player's car: its type and its colour, as the speech interface names them. The type is the
//! `VerbalType` of the car's `pvehicle` collection and the colour the `SPEECHCOLOUR` of its stock paint. A repaint with a
//! custom colour has no speech colour (the original describes only the stock paints), and a car the tables do not know
//! has neither. Cached per car, since reading the tables is not for every frame.

use std::cell::RefCell;

use nfsmw_data::car::CarTables;
use nfsmw_data::speech_chase::{stock_speech_colour, verbal_type};

use super::WorldScene;

/// The car's type and colour bits (0: unknown).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Look {
    pub car_type: u32,
    pub colour: u32,
}

/// The look of the car last asked about.
#[derive(Default)]
pub struct Cache {
    key: Option<(String, bool)>,
    look: Look,
}

impl Cache {
    fn get(&mut self, car: &str, custom_paint: bool, read: impl FnOnce() -> Look) -> Look {
        if self.key.as_ref().is_some_and(|(c, p)| c == car && *p == custom_paint) {
            return self.look;
        }
        self.look = read();
        self.key = Some((car.to_owned(), custom_paint));
        self.look
    }
}

impl WorldScene {
    /// The look of the car being driven, `None` while none is.
    pub(super) fn player_look(&self, cache: &RefCell<Cache>) -> Option<Look> {
        let car = &self.drive.as_ref()?.car_name;
        let custom = self.garage.applied.color.is_some();
        Some(cache.borrow_mut().get(car, custom, || self.read_look(car, custom)))
    }

    fn read_look(&self, car: &str, custom_paint: bool) -> Look {
        let Ok(tables) = CarTables::load(&self.dir) else { return Look::default() };
        let Some(info) = tables.car_type_for_folder(car) else { return Look::default() };
        let Ok(db) = nfsmw_data::shared::attributes(&self.dir) else { return Look::default() };
        let colour = match custom_paint {
            true => 0,
            false => stock_speech_colour(&tables, info),
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
        let a = cache.get("M3", false, || read(Look { car_type: 64, colour: 4 }));
        let b = cache.get("M3", false, || read(Look::default()));
        assert_eq!(a, b);
        let c = cache.get("M3", true, || read(Look { car_type: 64, colour: 0 }));
        assert_eq!(c.colour, 0);
        let d = cache.get("Elise", true, || read(Look { car_type: 2048, colour: 0 }));
        assert_eq!(d.car_type, 2048);
        assert_eq!(reads, 3);
    }
}
