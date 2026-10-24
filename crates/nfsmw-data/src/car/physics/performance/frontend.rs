//! Installed performance-package presentation metadata. See `garage-game-ui.md`.

use blackbox_attrib::Database;
use blackbox_hash::bstring_hash;

use super::{PerformanceCatalog, PerformanceCategory as Category};

#[derive(Clone, Copy, Debug, Default)]
pub struct PackageDetails {
    pub name: u32,
    pub descriptions: [u32; 3],
    pub brands: [u32; 3],
}

#[derive(Clone, Debug, Default)]
pub struct PerformanceDetails {
    /// Local stage indexes, including original equipment at index zero.
    pub packages: [Vec<PackageDetails>; 7],
    pub junkman: [PackageDetails; 7],
    pub turbo: bool,
}

impl PerformanceDetails {
    pub fn read(db: &Database, car: &str, catalog: &PerformanceCatalog, turbo: bool) -> Self {
        let frontend = db.collection("frontend", &car.to_ascii_lowercase());
        let rotary = matches!(car.to_ascii_uppercase().as_str(), "RX7" | "RX8");
        let packages = std::array::from_fn(|index| {
            let category = Category::ALL[index];
            let count = catalog.categories[index].max_level;
            (0..=count)
                .map(|local| {
                    let global = match local {
                        0 => 0,
                        _ => global_max(category).saturating_sub(count) + local,
                    };
                    let brands = match global {
                        0 => [stock_brand(category), 0, 0],
                        _ => std::array::from_fn(|line| {
                            frontend
                                .and_then(|fe| {
                                    fe.get(&format!("p_{}_{}", brand_field(category), global))?.item(line)?.as_u32()
                                })
                                .unwrap_or(0)
                        }),
                    };
                    PackageDetails {
                        name: bstring_hash(&format!("PN_LEVEL_{}", 6u8.saturating_sub(count) + local)),
                        descriptions: descriptions(category, global, turbo, rotary),
                        brands,
                    }
                })
                .collect()
        });
        let junkman = std::array::from_fn(|index| {
            let category = Category::ALL[index];
            PackageDetails {
                name: 0xedd1_4807,
                descriptions: descriptions(category, 0, turbo, rotary),
                brands: [stock_brand(category), 0, 0],
            }
        });
        Self { packages, junkman, turbo }
    }
}

fn global_max(category: Category) -> u8 {
    match category {
        Category::Brakes | Category::Transmission | Category::Engine => 4,
        _ => 3,
    }
}

fn brand_field(category: Category) -> &'static str {
    match category {
        Category::Chassis => "suspension",
        Category::Nitrous => "nitrous",
        _ => category.field(),
    }
}

fn stock_brand(category: Category) -> u32 {
    match category {
        Category::Tires | Category::Chassis => 0xad6a_0504,
        Category::Brakes => 0xa1a5_e9e5,
        Category::Transmission | Category::Nitrous => 0x98ed_935e,
        Category::Engine => 0x7d0a_c98f,
        Category::Induction => 0x9e8f_71ad,
    }
}

fn descriptions(category: Category, level: u8, turbo: bool, rotary: bool) -> [u32; 3] {
    if level == 0 {
        let hash = match category {
            Category::Tires => 0xe5c1_020c,
            Category::Brakes => 0x927d_b4fd,
            Category::Chassis => 0x8c96_b853,
            Category::Transmission => 0x2f52_5e4f,
            Category::Engine => 0xe74d_edbb,
            Category::Nitrous => 0x9a0e_f8f9,
            Category::Induction => match turbo {
                true => 0x5317_eb31,
                false => 0x704a_6d50,
            },
        };
        return [hash, 0, 0];
    }
    let category_name = match category {
        Category::Tires => "TIRES",
        Category::Brakes => "BRAKES",
        Category::Chassis => "CHASSIS",
        Category::Transmission => "TRANSMISSION",
        Category::Engine => "ENGINE",
        Category::Nitrous => "NITROUS",
        Category::Induction => match turbo {
            true => "TURBO",
            false => "SUPERCHARGER",
        },
    };
    std::array::from_fn(|index| {
        let suffix = match category == Category::Engine && rotary && matches!(level, 2 | 4) && index == 0 {
            true => "_ROTARY",
            false => "",
        };
        bstring_hash(&format!("PD_{category_name}_{level}_{}{suffix}", index + 1))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_levels_follow_the_cars_available_package_count() {
        let mut catalog = PerformanceCatalog::default();
        catalog.categories[Category::Engine as usize].max_level = 2;
        let details = PerformanceDetails::read(&Database::default(), "RX7", &catalog, true);
        let engine = &details.packages[Category::Engine as usize];
        assert_eq!(engine[1].name, bstring_hash("PN_LEVEL_5"));
        assert_eq!(engine[1].descriptions[0], bstring_hash("PD_ENGINE_3_1"));
        assert_eq!(engine[2].descriptions[0], bstring_hash("PD_ENGINE_4_1_ROTARY"));
        assert_eq!(details.junkman[Category::Engine as usize].descriptions[1], 0);
    }

    #[test]
    #[ignore = "requires an installed game"]
    fn installed_performance_brands_and_descriptions_are_readable() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let physics = crate::car::physics::PhysicsData::load(&dir).unwrap();
        let mut database = physics.database().clone();
        database.load(&crate::read_unwrapped(&dir, crate::career_cars::FE_ATTRIB_FILE).unwrap()).unwrap();
        let strings = blackbox_text::StringTable::from_file(&crate::shared::language(&dir).unwrap()).unwrap();
        let tables = crate::car::CarTables::load(&dir).unwrap();
        let mut brands = 0;
        let mut descriptions = 0;
        for car in tables.types.iter().filter(|car| car.usage_type == 0) {
            let Ok(catalog) = PerformanceCatalog::read(physics.database(), &car.type_name) else { continue };
            let details = PerformanceDetails::read(&database, &car.type_name, &catalog, true);
            for packages in details.packages {
                for package in packages.into_iter().skip(1) {
                    brands += package.brands.into_iter().filter(|hash| *hash != 0).count();
                    descriptions +=
                        package.descriptions.into_iter().filter(|hash| strings.get(*hash).is_some()).count();
                }
            }
        }
        assert!(brands > 100 && descriptions > 100, "{brands} brand references / {descriptions} localized lines");
        println!("{brands} brand references / {descriptions} localized performance description lines");
    }
}
