//! The resolved data of one vehicle: the `vehicles/<Brand>/<Vehicle>.json` file the physics dump writes
//! (`docs/formats/ride6-physics-data.md`, "The dump"). It holds the vehicle's own `Properties`, resolved over the class
//! defaults, and `Linked`, every physics asset it refers to, resolved the same way.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;

use super::node::Node;

/// Where the grip tables and tyre assets live in the content tree of the dump.
const TYRE_ROOT: &str = "/Tire/";
/// The grip table row of the road surface (`docs/formats/ride6-physics-tyre.md`, "`FTireModelData` rows").
const ROAD_ROW: &str = "Tarmac";
const ROAD_SURFACE: &str = "EPhysicalSurface::SurfaceType1";

/// One vehicle and the assets it links.
pub struct Vehicle {
    name: String,
    props: Value,
    linked: HashMap<String, Value>,
}

/// A package path as the dump spells it, in the case-insensitive form the lookups use. The asset names of RIDE 6 differ in
/// case between a reference and the asset it points at (`Maxienduro` and `MaxiEnduro`).
fn key(object_path: &str) -> String {
    object_path.to_ascii_lowercase()
}

impl Vehicle {
    /// Read the JSON text of a vehicle. `tyre_tables` is the `Tire` folder of the dump
    /// (`resolved/plugin/Physics/Tire`): a tyre asset or grip table the vehicle file does not carry is read from there.
    pub fn parse(text: &str, tyre_tables: Option<&Path>) -> Result<Self> {
        let mut root: Value = serde_json::from_str(text).context("the physics file is not JSON")?;
        let name = root.get("Name").and_then(Value::as_str).unwrap_or("").to_owned();
        let props = root
            .get_mut("Properties")
            .map(Value::take)
            .ok_or_else(|| anyhow!("the physics file has no `Properties`"))?;
        let mut linked = HashMap::new();
        if let Some(Value::Object(entries)) = root.get_mut("Linked").map(Value::take) {
            for (path, entry) in entries {
                linked.insert(key(&path), entry);
            }
        }
        let mut vehicle = Self { name, props, linked };
        vehicle.complete_tyres(tyre_tables)?;
        Ok(vehicle)
    }

    pub fn read(path: &Path, tyre_tables: Option<&Path>) -> Result<Self> {
        let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text, tyre_tables).with_context(|| format!("in {}", path.display()))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The vehicle's own resolved properties.
    pub fn props(&self) -> Node<'_> {
        Node::new(&self.props, "Properties")
    }

    /// A linked asset, by the object path a property gives (`/PhysicsRide6/Physics/Tire/TireData/..../FrontTyre_Road.0`).
    pub fn object(&self, object_path: &str) -> Result<Node<'_>> {
        let entry = self.linked.get(&key(object_path)).ok_or_else(|| {
            anyhow!("the physics file does not link `{object_path}` (give --tyre-tables for tyre assets)")
        })?;
        Ok(Node::new(entry, object_path))
    }

    /// The resolved properties of the asset an object reference (`{"ObjectName": .., "ObjectPath": ..}`) points at.
    pub fn target(&self, reference: &Node<'_>) -> Result<Node<'_>> {
        let path = reference.req("ObjectPath")?.text()?;
        self.object(path)?.req("Properties")
    }

    /// The properties of the vehicle element (controller) whose class path contains `kind` (`InputHandler`, `RiderHelp`,
    /// `WheelieControl`, `ABSVehicleElement`), or `None` when the vehicle does not run one.
    pub fn element(&self, kind: &str) -> Result<Option<Node<'_>>> {
        let kind = kind.to_ascii_lowercase();
        let Some(list) = self.props().get("VehicleElements") else { return Ok(None) };
        for reference in list.items()? {
            let Ok(path) = reference.req("ObjectPath").and_then(|p| p.text()) else { continue };
            if !path.to_ascii_lowercase().contains(&kind) {
                continue;
            }
            return Ok(Some(self.object(path)?.req("Properties")?));
        }
        Ok(None)
    }

    /// The road row of the grip table of a wheel (`FrontWheel`, `RearWheel`): the Magic Formula 94 coefficients and the
    /// rolling resistance.
    pub fn road_row(&self, wheel: &str) -> Result<Node<'_>> {
        let wheel_node = self.props().req(wheel)?;
        let tyre = self.target(&wheel_node.req("TireData")?)?;
        let table = self.object(tyre.req("TireModel")?.req("ObjectPath")?.text()?)?;
        let rows = table.req("Rows")?;
        if let Some(row) = rows.get(ROAD_ROW) {
            return Ok(row);
        }
        let found = match rows.value().as_object() {
            Some(map) => map.iter().find(|(_, row)| {
                row.get("surface").and_then(Value::as_str) == Some(ROAD_SURFACE)
                    && row.get("WET").and_then(Value::as_bool) == Some(false)
            }),
            None => None,
        };
        match found {
            Some((name, row)) => Ok(Node::new(row, format!("{}.Rows.{name}", table.path()))),
            None => bail!("{}: the grip table has no road row", table.path()),
        }
    }

    /// Load the tyre assets of both wheels that the file does not link from `tyre_tables`.
    fn complete_tyres(&mut self, tyre_tables: Option<&Path>) -> Result<()> {
        for wheel in ["FrontWheel", "RearWheel"] {
            let Some(reference) = self.props.get(wheel).and_then(|w| w.get("TireData")) else { continue };
            let Some(path) = reference.get("ObjectPath").and_then(Value::as_str).map(str::to_owned) else { continue };
            self.ensure(&path, tyre_tables)?;
            let model = self
                .linked
                .get(&key(&path))
                .and_then(|entry| entry.get("Properties"))
                .and_then(|p| p.get("TireModel"))
                .and_then(|m| m.get("ObjectPath"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            if let Some(model) = model {
                self.ensure(&model, tyre_tables)?;
            }
        }
        Ok(())
    }

    fn ensure(&mut self, object_path: &str, tyre_tables: Option<&Path>) -> Result<()> {
        if self.linked.contains_key(&key(object_path)) {
            return Ok(());
        }
        let Some(dir) = tyre_tables else { return Ok(()) };
        if let Some(entry) = read_from_tables(dir, object_path)? {
            self.linked.insert(key(object_path), entry);
        }
        Ok(())
    }
}

/// The file of an asset under the `Tire` folder of the dump: `/PhysicsRide6/Physics/Tire/TireData/Family/Name.0` is
/// `<dir>/TireData/Family/Name.json`, each folder and the file found whatever their case.
fn table_file(dir: &Path, object_path: &str) -> Option<PathBuf> {
    let at = object_path.to_ascii_lowercase().find(&TYRE_ROOT.to_ascii_lowercase())?;
    let relative = &object_path[at + TYRE_ROOT.len()..];
    let relative = match relative.rsplit_once('.') {
        Some((stem, version)) if version.chars().all(|c| c.is_ascii_digit()) => stem,
        _ => relative,
    };
    let mut path = dir.to_path_buf();
    let parts: Vec<&str> = relative.split('/').filter(|part| !part.is_empty()).collect();
    for (index, part) in parts.iter().enumerate() {
        let wanted = match index + 1 == parts.len() {
            true => format!("{part}.json"),
            false => (*part).to_owned(),
        };
        let found = fs::read_dir(&path)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(&wanted))?;
        path = found.path();
    }
    Some(path)
}

fn read_from_tables(dir: &Path, object_path: &str) -> Result<Option<Value>> {
    let Some(file) = table_file(dir, object_path) else { return Ok(None) };
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
    let exports: Value = serde_json::from_str(&text).with_context(|| format!("{} is not JSON", file.display()))?;
    // The dump writes a list of the package's exports; a tyre asset and a grip table have one.
    Ok(exports.get(0).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vehicle_json() -> Value {
        json!({
            "Name": "Test_Bike",
            "Properties": {
                "FrontWheel": {"TireData": {"ObjectPath": "/PhysicsRide6/Physics/Tire/TireData/Fam/Front.0"}},
                "VehicleElements": [
                    {"ObjectPath": "/PhysicsRide6/Physics/VehicleElements/InputHandler/Test_InputHandler.0"},
                    {"ObjectPath": "/PhysicsRide6/Physics/VehicleElements/ABSVehicleElement/Test_ABS.0"}
                ]
            },
            "Linked": {
                "/PhysicsRide6/Physics/Tire/tiredata/fam/front.0":
                    {"Properties": {"TireModel": {"ObjectPath": "/PhysicsRide6/Physics/Tire/GripCurve/Fam/Table.0"}}},
                "/PhysicsRide6/Physics/Tire/GripCurve/Fam/Table.0":
                    {"Rows": {"Tarmac": {"A0": 1.5}, "Grass": {"A0": 0.5}}},
                "/PhysicsRide6/Physics/VehicleElements/InputHandler/Test_InputHandler.0":
                    {"Properties": {"LeanK": 1000.0}}
            }
        })
    }

    #[test]
    fn links_are_found_whatever_their_case_and_elements_by_class() {
        let vehicle = Vehicle::parse(&vehicle_json().to_string(), None).unwrap();
        assert_eq!(vehicle.name(), "Test_Bike");
        assert_eq!(vehicle.road_row("FrontWheel").unwrap().f("A0").unwrap(), 1.5);
        let handler = vehicle.element("InputHandler").unwrap().unwrap();
        assert_eq!(handler.f("LeanK").unwrap(), 1000.0);
        assert!(vehicle.element("WheelieControl").unwrap().is_none());
        // Listed but not linked is an error, not a missing element.
        assert!(vehicle.element("ABSVehicleElement").is_err());
    }

    #[test]
    fn a_missing_tyre_asset_is_read_from_the_tables_folder_in_any_case() {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-tyres-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("TireData/FAM")).unwrap();
        std::fs::create_dir_all(dir.join("GripCurve/Fam")).unwrap();
        let data =
            json!([{"Properties": {"TireModel": {"ObjectPath": "/PhysicsRide6/Physics/Tire/GripCurve/Fam/Table.0"}}}]);
        std::fs::write(dir.join("TireData/FAM/FRONT.json"), data.to_string()).unwrap();
        std::fs::write(dir.join("GripCurve/Fam/Table.json"), json!([{"Rows": {"Tarmac": {"A0": 2.5}}}]).to_string())
            .unwrap();
        let mut value = vehicle_json();
        value["Linked"] = json!({});
        let vehicle = Vehicle::parse(&value.to_string(), Some(&dir)).unwrap();
        assert_eq!(vehicle.road_row("FrontWheel").unwrap().f("A0").unwrap(), 2.5);
        assert!(Vehicle::parse(&value.to_string(), None).unwrap().road_row("FrontWheel").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
