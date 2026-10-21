//! What the Quick Play brief writes, as pure functions: the length line, the race kind's picture and name, the car's logo
//! names and the unlock sentence (`docs/specs/quick-play.md` sections 4 to 7).

use nfsmw_data::gameplay::RaceType;

use super::pool::{PoolCar, car_unlocked};

/// Metres in a mile as the exe divides by (`1609.34`, the other unit divides by 1000).
const METRES_PER_MILE: f32 = 1609.34;

/// The `Length:` value: the route in kilometres or miles with one decimal and the unit word (`%$0.1f %s`), **[exe]**
/// the brief's sibling screen for the format and the divisors (`docs/provenance/quick-play.md`).
pub fn length_text(metres: f32, metric: bool, unit: &str) -> String {
    let divisor = match metric {
        true => 1000.0,
        false => METRES_PER_MILE,
    };
    format!("{:.1} {unit}", metres / divisor)
}

/// The language label of the unit word after the length: `KMs` or `Miles`.
pub fn unit_label(metric: bool) -> u32 {
    match metric {
        true => 0x8569_A26A,
        false => 0x867D_CFD9,
    }
}

/// The texture of the race kind's icon: Sprint, Circuit, Drag, Lap Knockout, Tollbooth, Speedtrap.
pub fn mode_icon(kind: RaceType) -> u32 {
    match kind {
        RaceType::Circuit => 0xE963_8D3E,
        RaceType::Drag => 0xAAAB_31E9,
        RaceType::Knockout => 0x3A01_5595,
        RaceType::Tollbooth => 0x1A09_1045,
        RaceType::SpeedTrap => 0x66C9_A7B6,
        _ => 0x2521_E5EB,
    }
}

/// The language label of the race kind's name.
pub fn mode_label(kind: RaceType) -> u32 {
    match kind {
        RaceType::Circuit => 0x034F_A2C1,
        RaceType::Drag => 0x6F54_7E4C,
        RaceType::Knockout => 0x4930_F5FC,
        RaceType::Tollbooth => 0xA15E_4505,
        RaceType::SpeedTrap => 0xEE1E_DC76,
        _ => 0xB94F_D70E,
    }
}

/// The label key of a track's name, from its event id: `TRACKNAME_` and the id with its dots as underscores
///, for example `TRACKNAME_10_2_1`.
pub fn track_label_key(event_id: &str) -> String {
    format!("TRACKNAME_{}", event_id.replace('.', "_"))
}

/// The manufacturers by index, as the exe names them; the logo texture is named after them.
const MANUFACTURERS: [&str; 25] = [
    "BMW",
    "FORD",
    "SUBARU",
    "PORSCHE",
    "AUDI",
    "MAZDA",
    "GENERAL_MOTORS",
    "DODGE",
    "TOYOTA",
    "MITSUBISHI",
    "MCLAREN",
    "MERCEDES",
    "NISSAN",
    "LOTUS",
    "LAMBORGHINI",
    "RENAULT",
    "LEXUS",
    "PONTIAC",
    "CHEVROLET",
    "VAUXHALL",
    "ASTONMARTIN",
    "VOLKSWAGEN",
    "FIAT",
    "CADILLAC",
    "CORVETTE",
];

/// The manufacturer's name for an index (1 is BMW); `None` for 0 and for an index past the table.
pub fn manufacturer(index: u8) -> Option<&'static str> {
    MANUFACTURERS.get(usize::from(index).checked_sub(1)?).copied()
}

/// The manufacturer logo texture of a car, `CARSELECT_MANUFACTURER_<NAME>`; the generic 128 pixel logo when the car has no
/// manufacturer.
pub fn manufacturer_texture(car: &PoolCar) -> String {
    match car.manufacturer.and_then(manufacturer) {
        Some(name) => format!("CARSELECT_MANUFACTURER_{name}"),
        None => "GENERIC_LOGO_128".to_owned(),
    }
}

/// The model-name picture of a car, `SECONDARY_LOGO_<front-end name>`; the generic 256 pixel logo when the car has no
/// manufacturer.
pub fn model_texture(car: &PoolCar) -> String {
    match car.manufacturer.and_then(manufacturer) {
        Some(_) => format!("SECONDARY_LOGO_{}", car.name),
        None => "GENERIC_LOGO_256".to_owned(),
    }
}

/// What the unlock line of a car says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unlock {
    /// The car is the player's: the line is hidden.
    None,
    /// A fixed sentence (the bonus cars): its language label.
    Label(u32),
    /// `Unlock this car by beating %s, Blacklist member %d.` for this rival.
    Rival(u8),
}

/// The sentence of a bonus car or a rival's car by its stable handle, `None` for any other handle.
pub fn handle_label(handle: u32) -> Option<u32> {
    match handle {
        0x2CF3_70F0 => Some(0xBD8B_AC94),
        0x03A9_4520 => Some(0xBD8B_AC93),
        0x2CF3_85B2 => Some(0xBD8B_AC92),
        0xCB6A_AF2F => Some(0xBD8B_AC91),
        0x0013_624E..=0x0013_6253 | 0x0000_965F..=0x0000_9666 => Some(RIVAL_LABEL),
        _ => None,
    }
}

/// The label of the rival sentence (`Unlock this car by beating %s, Blacklist member %d.`).
pub const RIVAL_LABEL: u32 = 0x4EF2_A115;

/// The Blacklist member whose car a handle names (`0x965F..0x9666` are members 2 to 9, `0x13624E..0x136253`
/// members 10 to 15), `None` for any other handle.
pub fn handle_member(handle: u32) -> Option<u8> {
    match handle {
        0x0000_965F..=0x0000_9666 => u8::try_from(handle - 0x965F + 2).ok(),
        0x0013_624E..=0x0013_6253 => u8::try_from(handle - 0x13_624E + 10).ok(),
        _ => None,
    }
}

/// The unlock line of the picked car at a stage. A car the profile has hides it. A bonus or rival car says what its handle
/// says. Any other car that is still locked is opened by beating the member after its rank (rank 14 comes with member 15, the
/// first rival): **[ours]**, the exe has the sentence for the stable's preset cars only. A rank past the last member, or no
/// rival to name, gives no line.
pub fn unlock_of(car: &PoolCar, stage: u8) -> Unlock {
    if car_unlocked(car.rank, stage) {
        return Unlock::None;
    }
    if let Some(label) = handle_label(car.handle) {
        return match (label == RIVAL_LABEL, handle_member(car.handle)) {
            (true, Some(member)) => Unlock::Rival(member),
            (true, None) => Unlock::None,
            (false, _) => Unlock::Label(label),
        };
    }
    match car.rank.checked_add(1) {
        Some(member) if (2..=15).contains(&member) => Unlock::Rival(member),
        _ => Unlock::None,
    }
}

/// The key of a rival's alias label, `BLACKLIST_RIVAL_NN_AKA`: the exe formats it in lower case and hashes it upper-cased
///, and `label_hash` is case sensitive.
pub fn alias_key(member: u8) -> String {
    format!("BLACKLIST_RIVAL_{member:02}_AKA")
}

/// A sentence with its `%s` and `%d` filled (the first of each).
pub fn fill_sentence(template: &str, alias: &str, member: u8) -> String {
    template.replacen("%s", alias, 1).replacen("%d", &member.to_string(), 1)
}
