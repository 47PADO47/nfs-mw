//! What both ends of a session check before a seat (`protocol.md` section 8): the build and the data identity, the
//! car catalogue a loadout is checked against, and the server setup of the game's world.

use nfsmw_net::loadout::{CarInfo, Catalog, Loadout};
use nfsmw_net::schema::GameSchema;
use nfsmw_net::version::{BuildInfo, DataId, ProtoVersion, ServerPolicy, SimBuild};
use nfsmw_server::ServerSetup;

use crate::scenes::world::net::catalog::car_type_id;

/// The game's sim build: this version of the program on this machine's architecture. Two ends with different code
/// are told apart before they predict differently.
pub fn sim_build() -> SimBuild {
    SimBuild::from_parts(concat!("nfsmw-", env!("CARGO_PKG_VERSION")), std::env::consts::ARCH)
}

/// The data id of an install, from its car folders (sorted, upper case): the car type ids and the entity type numbers
/// of a session are only the same on two machines that list the same cars. The executable's hash is not read (the
/// install's `speed.exe` is not opened by the game), so it is all zeros.
pub fn data_id(cars: &[String]) -> DataId {
    let mut names: Vec<String> = cars.iter().map(|car| car.to_ascii_uppercase()).collect();
    names.sort();
    names.dedup();
    let files: Vec<(&str, u64)> = names.iter().map(|name| (name.as_str(), 0)).collect();
    DataId::compute(&[0; 32], &files)
}

/// The build a game server and its clients share.
pub fn build_info(cars: &[String]) -> BuildInfo {
    BuildInfo { proto: ProtoVersion::CURRENT, sim_build: sim_build(), data_id: data_id(cars) }
}

/// The cars of the install as loadouts name them (`car_type_id` of each folder). Every car is class 0, and parts,
/// paints and wheel sets are not checked: the host does not apply parts yet (`world-host.md` section 7).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameCatalog {
    types: Vec<u32>,
}

impl GameCatalog {
    pub fn new(cars: &[String]) -> Self {
        let mut types: Vec<u32> = cars.iter().map(|car| car_type_id(car)).collect();
        types.sort_unstable();
        types.dedup();
        Self { types }
    }
}

impl Catalog for GameCatalog {
    fn car(&self, car_type: u32) -> Option<CarInfo> {
        self.types.binary_search(&car_type).is_ok().then_some(CarInfo { class: 0 })
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

/// The setup of a server over the game's world: the game's build, the default version policy, the install's cars and
/// the session seed.
pub fn server_setup(cars: &[String], seed: u64) -> ServerSetup {
    ServerSetup {
        build: build_info(cars),
        policy: ServerPolicy::default(),
        catalog: Box::new(GameCatalog::new(cars)),
        schema: GameSchema::new(),
        session_seed: seed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cars() -> Vec<String> {
        ["elise", "BMWM3GTR", "copmidsize"].map(String::from).to_vec()
    }

    #[test]
    fn the_data_id_ignores_order_and_case_but_not_the_cars() {
        let mut shuffled = cars();
        shuffled.reverse();
        shuffled[0] = shuffled[0].to_ascii_uppercase();
        assert_eq!(data_id(&cars()), data_id(&shuffled));
        assert_ne!(data_id(&cars()), data_id(&cars()[..2]));
    }

    #[test]
    fn the_catalogue_knows_the_install_cars_by_type_id() {
        let catalog = GameCatalog::new(&cars());
        assert!(catalog.car(car_type_id("ELISE")).is_some());
        assert!(catalog.car(car_type_id("bmwm3gtr")).is_some());
        assert!(catalog.car(car_type_id("NOSUCHCAR")).is_none());
        assert!(catalog.car(0).is_none(), "the toy world's type numbers are not cars");
    }
}
