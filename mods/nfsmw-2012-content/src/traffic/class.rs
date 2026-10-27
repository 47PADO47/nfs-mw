//! What the 2012 traffic bundles do not say about their cars and the converter has to: the kind of road car each is, how it is
//! driven, and the numbers a car of that kind has.
//!
//! A traffic bundle holds a body, wheels, lamps and a collision soup, and nothing about handling (`TRAFFICATTRIBS.BNDL` holds
//! eleven sets of handling objects, but which set belongs to which car is not decoded; docs/formats/chameleon-traffic.md). So
//! the class of each of the eleven cars comes from its name, the drive from what the real car is, and the numbers from the
//! class: ordinary engineering values for a road car of that kind, **design values, not 2012 data**. A traffic bundle of a
//! name the table does not know is classed by its wheelbase.

use chameleon_vehicle::TrafficName;

/// The kind of road car, by the names the manifest's `[traffic]` table `class` key takes (the mod is built with the SDK only,
/// so it spells them itself instead of using the host's `blackbox_modding::content::TrafficClass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficClass {
    Compact,
    Sedan,
    Pickup,
    Van,
    Truck,
}

impl TrafficClass {
    /// The name the manifest uses.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Sedan => "sedan",
            Self::Pickup => "pickup",
            Self::Van => "van",
            Self::Truck => "truck",
        }
    }
}

/// How the wheels are driven: the `drive` key of `[stats]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    Front,
    Rear,
}

/// The facts about one traffic car that its bundle does not give.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Facts {
    pub class: TrafficClass,
    pub drive: Drive,
    /// The name as the shop would show it, when the engine's spelling is not it.
    pub display: Option<&'static str>,
}

/// The eleven cars of the install: `(Make_Model as the lamp object spells it, class, drive, display name)`.
const KNOWN: [(&str, TrafficClass, Drive, &str); 11] = [
    ("Audi_A3", TrafficClass::Compact, Drive::Front, "Audi A3"),
    ("Cadillac_CTSV", TrafficClass::Sedan, Drive::Rear, "Cadillac CTS-V"),
    ("Chevrolet_ExpressVan", TrafficClass::Van, Drive::Rear, "Chevrolet Express Van"),
    ("Dodge_Caliber", TrafficClass::Compact, Drive::Front, "Dodge Caliber"),
    ("Dodge_GrandCaravan", TrafficClass::Van, Drive::Front, "Dodge Grand Caravan"),
    ("Dodge_Ram1500", TrafficClass::Pickup, Drive::Rear, "Dodge Ram 1500"),
    ("GMchevrolet_Cobalt", TrafficClass::Compact, Drive::Front, "Chevrolet Cobalt"),
    ("Infiniti_G35", TrafficClass::Sedan, Drive::Rear, "Infiniti G35"),
    ("Nissan_Frontier", TrafficClass::Pickup, Drive::Rear, "Nissan Frontier"),
    ("Nissan_Versa", TrafficClass::Compact, Drive::Front, "Nissan Versa"),
    ("GMC_Topkick", TrafficClass::Truck, Drive::Rear, "GMC Topkick"),
];

/// The facts about the traffic car `name`, whose wheels are `wheelbase` metres apart.
pub fn facts(name: &TrafficName, wheelbase: f32) -> Facts {
    let object = name.object_name();
    if let Some((_, class, drive, display)) = KNOWN.iter().find(|(known, ..)| *known == object) {
        return Facts { class: *class, drive: *drive, display: Some(*display) };
    }
    Facts { class: class_by_wheelbase(wheelbase), drive: Drive::Rear, display: None }
}

/// The class of a car nothing is known about: a wheelbase under 2.7 m is compact, under 3.0 a saloon, under 3.7 a van and
/// anything longer a truck.
pub fn class_by_wheelbase(wheelbase: f32) -> TrafficClass {
    match wheelbase {
        w if w < 2.7 => TrafficClass::Compact,
        w if w < 3.0 => TrafficClass::Sedan,
        w if w < 3.7 => TrafficClass::Van,
        _ => TrafficClass::Truck,
    }
}

/// The spawn weight of a class in the 2012 traffic, percent: lower than the class default of the game's API, so the 2005 cars
/// keep most of the road and the 2012 cars come in among them.
pub const fn weight(class: TrafficClass) -> u32 {
    match class {
        TrafficClass::Compact | TrafficClass::Sedan => 10,
        TrafficClass::Pickup | TrafficClass::Van => 8,
        TrafficClass::Truck => 4,
    }
}

/// The traffic patterns a class keeps to (the neighbourhoods and highways the game names), or none for every pattern: the
/// heavy trucks drive the highways and the industrial pattern, like the game's own semis and dump trucks.
pub fn patterns(class: TrafficClass) -> &'static [&'static str] {
    match class {
        TrafficClass::Truck => &["cityhighway", "collegehighway", "industrial"],
        _ => &[],
    }
}

/// The numbers of a road car of a class.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Numbers {
    pub mass_kg: f32,
    /// Share of the weight on the front axle, percent.
    pub front_weight_pct: f32,
    pub cg_height_m: f32,
    pub ride_height_m: f32,
    /// The engine's peak torque, N m, and the speed it comes at.
    pub peak_torque_nm: f32,
    pub peak_rpm: f32,
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    pub gears: &'static [f32],
    pub reverse_ratio: f32,
    pub final_drive: f32,
    pub top_speed_kph: f32,
    pub tire_grip: f32,
}

/// The numbers of a road car of `class`.
pub const fn numbers(class: TrafficClass) -> Numbers {
    match class {
        TrafficClass::Compact => Numbers {
            mass_kg: 1300.0,
            front_weight_pct: 58.0,
            cg_height_m: 0.52,
            ride_height_m: 0.13,
            peak_torque_nm: 190.0,
            peak_rpm: 4200.0,
            idle_rpm: 800.0,
            redline_rpm: 6500.0,
            gears: &[3.6, 1.95, 1.28, 0.95, 0.76],
            reverse_ratio: 3.3,
            final_drive: 4.0,
            top_speed_kph: 185.0,
            tire_grip: 1.1,
        },
        TrafficClass::Sedan => Numbers {
            mass_kg: 1700.0,
            front_weight_pct: 52.0,
            cg_height_m: 0.53,
            ride_height_m: 0.13,
            peak_torque_nm: 330.0,
            peak_rpm: 4000.0,
            idle_rpm: 750.0,
            redline_rpm: 6500.0,
            gears: &[3.5, 2.1, 1.4, 1.0, 0.8],
            reverse_ratio: 3.2,
            final_drive: 3.5,
            top_speed_kph: 210.0,
            tire_grip: 1.15,
        },
        TrafficClass::Pickup => Numbers {
            mass_kg: 2000.0,
            front_weight_pct: 54.0,
            cg_height_m: 0.68,
            ride_height_m: 0.2,
            peak_torque_nm: 400.0,
            peak_rpm: 3800.0,
            idle_rpm: 700.0,
            redline_rpm: 5500.0,
            gears: &[3.6, 2.2, 1.5, 1.0, 0.8],
            reverse_ratio: 3.4,
            final_drive: 3.7,
            top_speed_kph: 175.0,
            tire_grip: 1.0,
        },
        TrafficClass::Van => Numbers {
            mass_kg: 2300.0,
            front_weight_pct: 52.0,
            cg_height_m: 0.72,
            ride_height_m: 0.17,
            peak_torque_nm: 380.0,
            peak_rpm: 3600.0,
            idle_rpm: 700.0,
            redline_rpm: 5500.0,
            gears: &[3.6, 2.2, 1.5, 1.0, 0.8],
            reverse_ratio: 3.4,
            final_drive: 3.9,
            top_speed_kph: 165.0,
            tire_grip: 1.0,
        },
        TrafficClass::Truck => Numbers {
            mass_kg: 5800.0,
            front_weight_pct: 45.0,
            cg_height_m: 0.95,
            ride_height_m: 0.3,
            peak_torque_nm: 700.0,
            peak_rpm: 2400.0,
            idle_rpm: 650.0,
            redline_rpm: 3500.0,
            gears: &[6.0, 3.4, 2.0, 1.4, 1.0, 0.8],
            reverse_ratio: 5.5,
            final_drive: 4.1,
            top_speed_kph: 130.0,
            tire_grip: 0.95,
        },
    }
}

/// The colours the cars are painted in, in the order the cars take them: realistic traffic paint, `(sRGB, metallic)`. The 2012
/// traffic bundles carry no paint records, so the car's colour is chosen here, one per car.
pub const COLOURS: [([u8; 3], bool); 8] = [
    ([196, 199, 203], true),
    ([240, 240, 238], false),
    ([30, 31, 34], true),
    ([34, 52, 98], true),
    ([128, 28, 34], true),
    ([112, 116, 120], true),
    ([196, 182, 150], false),
    ([38, 74, 56], true),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn name(make: &str, model: &str) -> TrafficName {
        TrafficName { make: make.to_owned(), model: model.to_owned() }
    }

    #[test]
    fn the_known_cars_have_their_class_drive_and_name() {
        let a3 = facts(&name("Audi", "A3"), 2.58);
        assert_eq!((a3.class, a3.drive, a3.display), (TrafficClass::Compact, Drive::Front, Some("Audi A3")));
        let ram = facts(&name("Dodge", "Ram1500"), 3.57);
        assert_eq!((ram.class, ram.drive), (TrafficClass::Pickup, Drive::Rear));
        let topkick = facts(&name("GMC", "Topkick"), 5.74);
        assert_eq!(topkick.class, TrafficClass::Truck);
        assert_eq!(facts(&name("GMchevrolet", "Cobalt"), 2.62).display, Some("Chevrolet Cobalt"));
    }

    #[test]
    fn a_car_the_table_does_not_know_is_classed_by_its_wheelbase() {
        let unknown = facts(&name("Volvo", "V70"), 2.8);
        assert_eq!((unknown.class, unknown.drive, unknown.display), (TrafficClass::Sedan, Drive::Rear, None));
        assert_eq!(class_by_wheelbase(2.4), TrafficClass::Compact);
        assert_eq!(class_by_wheelbase(3.2), TrafficClass::Van);
        assert_eq!(class_by_wheelbase(5.7), TrafficClass::Truck);
    }

    #[test]
    fn every_class_has_numbers_the_manifest_accepts() {
        for class in
            [TrafficClass::Compact, TrafficClass::Sedan, TrafficClass::Pickup, TrafficClass::Van, TrafficClass::Truck]
        {
            let n = numbers(class);
            assert!(n.gears.windows(2).all(|pair| pair[1] < pair[0]), "{class:?}: each gear below the one before");
            assert!(n.idle_rpm < n.peak_rpm && n.peak_rpm < n.redline_rpm, "{class:?}");
            assert!((200.0..=40_000.0).contains(&n.mass_kg) && (0.3..=3.0).contains(&n.tire_grip), "{class:?}");
            assert!((1..=100).contains(&weight(class)), "{class:?}");
        }
    }

    #[test]
    fn the_heavy_trucks_keep_to_the_highways_and_the_others_go_everywhere() {
        assert!(patterns(TrafficClass::Truck).contains(&"cityhighway"));
        assert!(patterns(TrafficClass::Compact).is_empty());
    }
}
