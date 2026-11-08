//! The worked examples of the three games as synthetic fixtures (no game data, invented names and numbers), and the
//! rules of the library on small definitions.

mod core;
mod nfs2005;
mod nfs2012;
mod ride6;
mod wire;

use crate::{Catalog, VehicleDef};

pub(crate) fn catalog(text: &str) -> Catalog {
    toml::from_str(text).expect("the catalog parses")
}

pub(crate) fn vehicle(text: &str) -> VehicleDef {
    toml::from_str(text).expect("the vehicle parses")
}
