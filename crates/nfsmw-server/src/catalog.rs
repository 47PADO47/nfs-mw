//! The game data a loadout is checked against ([`nfsmw_net::loadout::Catalog`]). The game passes its own catalogue over
//! its car and part tables; the toy server uses [`OpenCatalog`].

use nfsmw_net::loadout::{CarInfo, Catalog, Loadout};

/// A catalogue that knows car types `0..car_types` (all of class 0) and accepts every part, paint and wheel set: the
/// toy world's cars have no parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenCatalog {
    pub car_types: u32,
}

impl Catalog for OpenCatalog {
    fn car(&self, car_type: u32) -> Option<CarInfo> {
        (car_type < self.car_types).then_some(CarInfo { class: 0 })
    }

    fn part_fits(&self, _car_type: u32, _slot: u16, _part: u32) -> bool {
        true
    }

    fn paint_known(&self, _paint: u32) -> bool {
        true
    }

    fn wheel_set_known(&self, _wheel_set: u32) -> bool {
        true
    }

    fn performance(&self, _loadout: &Loadout) -> Option<u16> {
        Some(0)
    }
}

/// A boxed catalogue, so the server is not generic over it.
pub type BoxedCatalog = Box<dyn Catalog + Send>;

/// Borrows a boxed catalogue as a sized [`Catalog`] for the rule functions that take `&impl Catalog`.
pub(crate) struct CatalogRef<'a>(pub &'a (dyn Catalog + Send));

impl Catalog for CatalogRef<'_> {
    fn car(&self, car_type: u32) -> Option<CarInfo> {
        self.0.car(car_type)
    }

    fn part_fits(&self, car_type: u32, slot: u16, part: u32) -> bool {
        self.0.part_fits(car_type, slot, part)
    }

    fn paint_known(&self, paint: u32) -> bool {
        self.0.paint_known(paint)
    }

    fn wheel_set_known(&self, wheel_set: u32) -> bool {
        self.0.wheel_set_known(wheel_set)
    }

    fn performance(&self, loadout: &Loadout) -> Option<u16> {
        self.0.performance(loadout)
    }
}
