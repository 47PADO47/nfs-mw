//! The files side of [`super::borrow`]: a vehicle whose data has no rake and trail (the Italjet Dragster 200 CC, whose
//! `RakeTrailOverride` is an empty list in RIDE 6's own asset) takes them from the nearest vehicle of the dump that has them.
//!
//! The dump is `vehicles/<Brand>/<Vehicle>.json`; the lenders are searched in the vehicle's own brand folder first and, when none of
//! that make has a row, in every brand's (a bike of the nearest wheelbase, [`nearest_lender`]).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use super::borrow::{Borrowed, Lender, nearest_lender};
use super::vehicle::Vehicle;

/// A vehicle of the dump with a rake and trail to lend: who it is, its wheelbase, the rake in degrees and the trail in centimetres.
struct Row {
    lender: Lender,
    rake: f64,
    trail: f64,
}

/// The `.json` files of a folder, sorted by name.
fn json_files(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else { return Vec::new() };
    let mut files: Vec<PathBuf> =
        entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
    files.sort();
    files
}

/// The rake and trail a vehicle file has (its first `RakeTrailOverride` row), or `None` when it has none or is not a vehicle.
fn row_of(file: &Path) -> Option<Row> {
    let text = fs::read_to_string(file).ok()?;
    let doc: Value = serde_json::from_str(&text).ok()?;
    let skeleton = doc.get("Properties")?.get("Skeleton")?;
    let first = skeleton.get("RakeTrailOverride")?.as_array()?.first()?;
    let name = file.file_stem()?.to_str()?.to_owned();
    Some(Row {
        lender: Lender { name, wheelbase_cm: skeleton.get("OverrideWheelBase")?.as_f64()? },
        rake: first.get("Rake")?.as_f64()?,
        trail: first.get("Trail")?.as_f64()?,
    })
}

/// The rows of the vehicle files of `folders`.
fn rows(folders: &[PathBuf]) -> Vec<Row> {
    folders.iter().flat_map(|folder| json_files(folder)).filter_map(|file| row_of(&file)).collect()
}

/// Give `vehicle` (read from `file`, `<dump>/vehicles/<Brand>/<Vehicle>.json`) the rake and trail of the nearest model when its data
/// has none; a vehicle that has them is left alone. An error only when no vehicle of the dump has any to lend.
pub fn complete(vehicle: &mut Vehicle, file: &Path) -> Result<()> {
    if !vehicle.lacks_rake_trail() {
        return Ok(());
    }
    let brand = file.parent().context("the vehicle file has no folder")?;
    let vehicles = brand.parent().context("the vehicle file has no brand folder")?;
    let own = file.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_owned();
    let wheelbase = vehicle.props().get("Skeleton").and_then(|s| s.f("OverrideWheelBase").ok()).unwrap_or(140.0);
    let target = Lender { name: own, wheelbase_cm: wheelbase };
    let mut candidates = rows(&[brand.to_path_buf()]);
    let same_brand = candidates.iter().any(|r| r.lender.name != target.name);
    if !same_brand {
        let all: Vec<PathBuf> = fs::read_dir(vehicles)?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        candidates = rows(&all);
    }
    let lenders: Vec<Lender> = candidates.iter().map(|r| r.lender.clone()).collect();
    let (chosen, rule) = nearest_lender(&target, &lenders).with_context(|| {
        format!("{}: no rake and trail, and no vehicle of the dump has any to lend", file.display())
    })?;
    let row = candidates.iter().find(|r| r.lender.name == chosen.name).context("the lender's row is gone")?;
    vehicle.borrow_rake_trail(
        row.rake,
        row.trail,
        Borrowed {
            field: "Skeleton.RakeTrailOverride".to_owned(),
            source: format!("{} (rake {} deg, trail {} cm)", row.lender.name, row.rake, row.trail),
            rule: rule.to_owned(),
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A vehicle to write: its brand folder, its name, its wheelbase and its rake and trail (`None`: an empty list).
    type Spec<'a> = (&'a str, &'a str, f64, Option<(f64, f64)>);

    fn vehicle_file(rows: &[Spec<'_>], dir: &Path) {
        for (brand, name, wheelbase, rake_trail) in rows {
            std::fs::create_dir_all(dir.join(brand)).unwrap();
            let list = match rake_trail {
                Some((rake, trail)) => json!([{"Rake": rake, "Trail": trail}]),
                None => json!([]),
            };
            let doc = json!({"Name": name, "Properties": {"Skeleton": {
                "bOverrideSkeletonData": true, "OverrideWheelBase": wheelbase, "RakeTrailOverride": list}}, "Linked": {}});
            std::fs::write(dir.join(brand).join(format!("{name}.json")), doc.to_string()).unwrap();
        }
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-borrow-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn read(dir: &Path, brand: &str, name: &str) -> Vehicle {
        Vehicle::read(&dir.join(brand).join(format!("{name}.json")), None).unwrap()
    }

    #[test]
    fn a_vehicle_with_no_rake_takes_the_nearest_model_of_its_make() {
        let dir = scratch("make");
        vehicle_file(
            &[
                ("Italjet", "Italjet_Dragster200CC_2022", 135.0, None),
                ("Italjet", "Italjet_Dragster700Twin_2024", 143.5, Some((24.0, 9.0))),
                ("Honda", "Honda_PCX125_2021", 135.0, Some((26.5, 8.0))),
            ],
            &dir,
        );
        let mut vehicle = read(&dir, "Italjet", "Italjet_Dragster200CC_2022");
        assert!(vehicle.lacks_rake_trail());
        complete(&mut vehicle, &dir.join("Italjet").join("Italjet_Dragster200CC_2022.json")).unwrap();
        assert!(!vehicle.lacks_rake_trail());
        let first =
            vehicle.props().req("Skeleton").unwrap().req("RakeTrailOverride").unwrap().items().unwrap().remove(0);
        assert_eq!((first.f("Rake").unwrap(), first.f("Trail").unwrap()), (24.0, 9.0));
        let [choice] = vehicle.borrowed() else { panic!("one choice expected") };
        assert_eq!(choice.field, "Skeleton.RakeTrailOverride");
        assert!(choice.source.starts_with("Italjet_Dragster700Twin_2024"), "{}", choice.source);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn with_no_model_of_the_make_the_nearest_wheelbase_of_any_make_is_taken() {
        let dir = scratch("wheelbase");
        vehicle_file(
            &[
                ("Italjet", "Italjet_Dragster200CC_2022", 135.0, None),
                ("Honda", "Honda_PCX125_2021", 131.0, Some((26.5, 8.0))),
                ("Kawasaki", "Kawasaki_Z900_2021", 145.0, Some((24.5, 10.5))),
            ],
            &dir,
        );
        let mut vehicle = read(&dir, "Italjet", "Italjet_Dragster200CC_2022");
        complete(&mut vehicle, &dir.join("Italjet").join("Italjet_Dragster200CC_2022.json")).unwrap();
        assert!(vehicle.borrowed()[0].source.starts_with("Honda_PCX125_2021"));
        assert!(vehicle.borrowed()[0].rule.contains("wheelbase"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_vehicle_that_has_a_rake_is_left_alone_and_a_dump_with_nothing_to_lend_is_an_error() {
        let dir = scratch("alone");
        vehicle_file(&[("Italjet", "Italjet_Dragster700Twin_2024", 143.5, Some((24.0, 9.0)))], &dir);
        let file = dir.join("Italjet").join("Italjet_Dragster700Twin_2024.json");
        let mut vehicle = read(&dir, "Italjet", "Italjet_Dragster700Twin_2024");
        complete(&mut vehicle, &file).unwrap();
        assert!(vehicle.borrowed().is_empty());
        vehicle_file(&[("Italjet", "Italjet_Dragster200CC_2022", 135.0, None)], &dir);
        std::fs::remove_file(&file).unwrap();
        let mut bare = read(&dir, "Italjet", "Italjet_Dragster200CC_2022");
        let error = complete(&mut bare, &dir.join("Italjet").join("Italjet_Dragster200CC_2022.json")).unwrap_err();
        assert!(error.to_string().contains("no vehicle of the dump has any to lend"), "{error:#}");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
