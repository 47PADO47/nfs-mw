//! The `cars/<id>.toml` of a traffic car: its role, names, `[stats]`, the single paint it wears, its plate, its `[wheels]` and the
//! `[traffic]` table that puts it in the game's traffic patterns (docs/modding/api-content.md "Cars", docs/modding/api-traffic.md).

use super::class::TrafficClass;
use crate::paints::{self, PaintChoice};
use crate::stats::ImportedStats;

/// How a traffic car joins the traffic: the `[traffic]` table.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub class: TrafficClass,
    pub weight: u32,
    /// 0 leaves the key out: the class's own limit applies.
    pub max_instances: u32,
    pub patterns: Vec<String>,
}

impl Entry {
    /// The `[traffic]` table, to follow every other table of the manifest.
    pub fn table(&self) -> String {
        let mut text = format!("\n[traffic]\nclass = \"{}\"\nweight = {}\n", self.class.name(), self.weight);
        if self.max_instances > 0 {
            text.push_str(&format!("max_instances = {}\n", self.max_instances));
        }
        if !self.patterns.is_empty() {
            let names: Vec<String> = self.patterns.iter().map(|name| format!("\"{name}\"")).collect();
            text.push_str(&format!("patterns = [{}]\n", names.join(", ")));
        }
        text
    }
}

/// What the manifest of a traffic car says.
pub struct Car<'a> {
    pub name: &'a str,
    pub manufacturer: &'a str,
    pub stats: &'a ImportedStats,
    /// The car's paint first (it wears it: `default_paint = 0`), then the colours it could wear.
    pub paints: &'a [PaintChoice],
    /// The `[customization.plate]` table, or empty.
    pub plate: &'a str,
    /// The `[wheels]` tables (`wheels::Placed::manifest_table`).
    pub wheels: &'a str,
    pub traffic: &'a Entry,
}

/// The text of the manifest. The tables come after the plain keys, as TOML puts every key below a table header inside it.
pub fn text(car: &Car) -> String {
    let mut text = String::from("role = \"traffic\"\n");
    text.push_str(&format!("name = {}\n", quoted(car.name)));
    if !car.manufacturer.is_empty() {
        text.push_str(&format!("manufacturer = {}\n", quoted(car.manufacturer)));
    }
    text.push_str(&car.stats.manifest_table());
    text.push_str(&paints::manifest_table(car.paints, &[], None));
    text.push_str(car.plate);
    text.push_str(car.wheels);
    text.push_str(&car.traffic.table());
    text
}

fn quoted(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_modding::content::parse_car;

    use crate::fit::Layout;
    use crate::traffic::class::{Drive, Facts};
    use crate::wheels::{Placed, Tyre};

    fn placed() -> Placed {
        let tyre = |x: f32| Tyre { x, z: 0.325, face: 0.9, radius: 0.325, width: 0.2 };
        Placed {
            layout: Layout { wheelbase: 2.58, x_mid: 0.34, track: 0.9, hub_z: 0.325, radius: 0.325 },
            front: tyre(1.63),
            rear: tyre(-0.95),
            axles: [1.63, -0.95],
        }
    }

    #[test]
    fn the_manifest_is_one_the_games_parser_reads_as_a_traffic_car() {
        let placed = placed();
        let facts = Facts { class: TrafficClass::Compact, drive: Drive::Front, display: None };
        let stats = crate::traffic::stats::of(&placed, &facts);
        let entry = Entry { class: TrafficClass::Compact, weight: 10, max_instances: 0, patterns: Vec::new() };
        let paints = [PaintChoice { color: [196, 199, 203], metallic: true }];
        let plate = crate::plate::manifest_table(Some("NEED4SPD"), "AUDA3_TR_PLATE");
        let wheels = placed.manifest_table();
        let car = Car {
            name: "Audi A3 (traffic)",
            manufacturer: "Audi",
            stats: &stats,
            paints: &paints,
            plate: &plate,
            wheels: &wheels,
            traffic: &entry,
        };
        let manifest = parse_car("auda3_tr", &text(&car)).unwrap_or_else(|error| panic!("{error}\n{}", text(&car)));
        assert!(manifest.role.is_traffic() && !manifest.role.is_player() && !manifest.role.is_cop());
        let traffic = manifest.traffic.unwrap();
        assert_eq!(
            (traffic.class.name(), traffic.weight, traffic.max_instances),
            (TrafficClass::Compact.name(), 10, 0)
        );
        assert!(traffic.patterns.is_empty());
        assert_eq!(manifest.name.as_deref(), Some("Audi A3 (traffic)"));
        let customization = manifest.customization.unwrap();
        assert_eq!(customization.paints.len(), 1);
        assert_eq!(customization.plate.unwrap().text.as_deref(), Some("NEED4SPD"));
        assert!(manifest.wheels.is_some());
    }

    #[test]
    fn a_limit_and_patterns_are_written_only_when_there_are_some() {
        let plain = Entry { class: TrafficClass::Sedan, weight: 10, max_instances: 0, patterns: Vec::new() };
        assert_eq!(plain.table(), "\n[traffic]\nclass = \"sedan\"\nweight = 10\n");
        let truck = Entry {
            class: TrafficClass::Truck,
            weight: 4,
            max_instances: 1,
            patterns: vec!["cityhighway".to_owned(), "industrial".to_owned()],
        };
        assert_eq!(
            truck.table(),
            "\n[traffic]\nclass = \"truck\"\nweight = 4\nmax_instances = 1\npatterns = [\"cityhighway\", \"industrial\"]\n"
        );
    }
}
