//! The 2012 install: which cars it has, and one car's meshes and textures read out of its bundles.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chameleon_bundle::{Bundle, Pool};
use chameleon_gfx::Texture;
use chameleon_vehicle::{
    Lamps, PoliceMarks, PolySoup, Vehicle, VehiclePaint, find_spec, load, load_poly_soup, load_texture, paints,
    police_marks, read_lamps, vehicle_name,
};

use crate::liveries::{self, Signature};
use crate::vehicle_list::{self, ListEntry};

const VEHICLES: &str = "VEHICLES";
const SHARED_TEXTURES: &str = "VEHICLES/VEHICLETEX.BNDL";
/// The vehicle list: which bundles are the same vehicle in another livery.
const VEHICLE_LIST: &str = "VEHICLES/VEHICLELIST.BNDL";
const SHADERS: &str = "SHADERS.BNDL";
const GAME_LOGIC: &str = "GAMELOGIC";
/// The one-shots the cars share (the turbo's blow-off is one); optional.
const SHARED_SAMPLES: &str = "VEHICLES/WAVES.BNDL";

/// One car's bundle file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarFile {
    /// The number in the file name (`VEH_1085007_HI.BNDL`).
    pub number: u32,
    pub path: PathBuf,
}

/// A car read out of its bundle.
pub struct SourceCar {
    pub vehicle: Vehicle,
    /// The diffuse textures the car's meshes use, decoded.
    pub textures: HashMap<u64, Texture>,
    /// The paints the car is offered in, from the colour records of its bundle.
    pub paints: Vec<VehiclePaint>,
    /// The car's collision mesh, when its bundle has one that reads (92 of the 93 cars do).
    pub collision: Option<PolySoup>,
    /// The hinged parts of the body and the triangles they own, when the car has any (`hinges.rs`).
    pub hinged: Option<crate::hinges::Hinged>,
    /// The car's lamp locators and lamp records; empty when the bundle's do not read.
    pub lamps: Lamps,
}

/// An opened 2012 install. The shared bundles stay in memory; each car's bundle is read when it is loaded.
pub struct Source {
    textures: Vec<u8>,
    shaders: Vec<u8>,
    /// The shared sample bundle, when the install has one that can be read.
    shared_samples: Option<Vec<u8>>,
    cars: Vec<CarFile>,
    /// The install's folder.
    root: PathBuf,
    /// The entries of the vehicle list; empty when the install has none that reads.
    list: Vec<ListEntry>,
}

/// A bundle's car as far as liveries are concerned: what it is, and the picture its paint surfaces draw.
pub struct LiveryView {
    pub signature: Signature,
    pub picture: Option<Texture>,
}

impl Source {
    /// Open the install at `root`, which must hold the vehicle and shader bundles.
    pub fn open(root: &Path) -> Result<Self> {
        let find = |rel: &str| -> Result<PathBuf> {
            let path = root.join(rel);
            if !path.is_file() {
                bail!("{} is not an NFS Most Wanted (2012) install: {rel} is missing", root.display());
            }
            Ok(path)
        };
        let textures = std::fs::read(find(SHARED_TEXTURES)?).context("reading the shared vehicle textures")?;
        let shaders = std::fs::read(find(SHADERS)?).context("reading the shaders")?;
        let shared_samples = std::fs::read(root.join(SHARED_SAMPLES)).ok();
        let list = std::fs::read(root.join(VEHICLE_LIST)).ok().and_then(|data| vehicle_list::read(&data));
        let list = list.unwrap_or_default();
        Ok(Self {
            textures,
            shaders,
            shared_samples,
            cars: list_cars(&root.join(VEHICLES))?,
            root: root.to_owned(),
            list,
        })
    }

    /// A file of the install, by its path under the install's folder.
    pub fn read_file(&self, rel: &str) -> Result<Vec<u8>> {
        let path = self.root.join(rel);
        std::fs::read(&path).with_context(|| format!("reading {}", path.display()))
    }

    /// The bundles of the game's logic (`GAMELOGIC/*.BNDL`), where the sound sets of the heat levels, the nitrous's among them,
    /// are kept. Sorted by name; empty when the folder is not there.
    pub fn game_logic_bundles(&self) -> Vec<PathBuf> {
        let entries = std::fs::read_dir(self.root.join(GAME_LOGIC)).into_iter().flatten().flatten();
        let mut files: Vec<PathBuf> = entries
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("bndl")))
            .collect();
        files.sort();
        files
    }

    /// The high-detail bundle of every car, in file-number order.
    pub fn cars(&self) -> &[CarFile] {
        &self.cars
    }

    /// The entries of the vehicle list, in list order. Empty when the install's list does not read: then no bundle is a
    /// livery of another and every bundle is a car.
    pub fn list(&self) -> &[ListEntry] {
        &self.list
    }

    /// The signature and the paint picture of the car in `file`, read without decoding its other textures.
    pub fn livery_view(&self, file: &CarFile) -> Result<LiveryView> {
        let data = std::fs::read(&file.path).with_context(|| format!("reading {}", file.path.display()))?;
        let bundle = Bundle::parse(&data)?;
        let spec = find_spec(&bundle).context("the bundle has no vehicle graphics spec")?;
        let mut pool = Pool::new();
        pool.add(bundle.clone());
        pool.add(Bundle::parse(&self.textures)?);
        pool.add(Bundle::parse(&self.shaders)?);
        let vehicle = load(&pool, spec)?;
        let signature = liveries::signature(&vehicle);
        let picture = match signature.picture {
            Some(id) if pool.contains(id) => Some(load_texture(&pool, id)?),
            _ => None,
        };
        Ok(LiveryView { signature, picture })
    }

    /// The engine's object name of the car in `file`, read without decoding any geometry.
    pub fn object_name(&self, file: &CarFile) -> Option<String> {
        let data = std::fs::read(&file.path).ok()?;
        vehicle_name(&Bundle::parse(&data).ok()?)
    }

    /// The engine's object name of the car in `file` and what its bundle says about the vehicle being a police vehicle, read
    /// without decoding any geometry. A police vehicle whose behaviour objects carry no model year has no object name; its
    /// siren corona object names it instead.
    pub fn identify(&self, file: &CarFile) -> (Option<String>, PoliceMarks) {
        let Ok(data) = std::fs::read(&file.path) else { return (None, PoliceMarks::default()) };
        let Ok(bundle) = Bundle::parse(&data) else { return (None, PoliceMarks::default()) };
        let marks = police_marks(&bundle);
        let name = vehicle_name(&bundle).or_else(|| marks.siren_name.clone());
        (name, marks)
    }

    /// The bytes of the file `rel` of the install (`SOUND/METALCAR.BNDL`), `None` when it is missing or unreadable.
    pub fn install_file(&self, rel: &str) -> Option<Vec<u8>> {
        std::fs::read(self.root.join(rel)).ok()
    }

    /// The shared sample bundle (`VEHICLES/WAVES.BNDL`), if the install has it.
    pub fn shared_samples(&self) -> Option<&[u8]> {
        self.shared_samples.as_deref()
    }

    /// The bytes of the car's high-detail bundle.
    pub fn bundle_bytes(&self, file: &CarFile) -> Result<Vec<u8>> {
        std::fs::read(&file.path).with_context(|| format!("reading {}", file.path.display()))
    }

    pub fn load(&self, file: &CarFile) -> Result<SourceCar> {
        let data = std::fs::read(&file.path).with_context(|| format!("reading {}", file.path.display()))?;
        let bundle = Bundle::parse(&data)?;
        let spec = find_spec(&bundle).context("the bundle has no vehicle graphics spec")?;
        let car_paints = paints(&bundle);
        let lamps = read_lamps(&bundle).unwrap_or_else(|error| {
            log::warn!("{}: the lamps do not read: {error}", file.path.display());
            Lamps::default()
        });
        let mut pool = Pool::new();
        pool.add(bundle.clone());
        pool.add(Bundle::parse(&self.textures)?);
        pool.add(Bundle::parse(&self.shaders)?);
        let vehicle = load(&pool, spec)?;
        let mut textures = HashMap::new();
        for mesh in vehicle.all_meshes() {
            let Some(id) = mesh.appearance.diffuse_texture else { continue };
            if textures.contains_key(&id) || !pool.contains(id) {
                continue;
            }
            textures.insert(id, load_texture(&pool, id)?);
        }
        let collision = load_poly_soup(&pool, spec).unwrap_or_else(|error| {
            log::warn!("{}: the collision mesh does not read: {error}", file.path.display());
            None
        });
        let hinged = crate::hinges::read(&bundle, &pool, spec, &vehicle);
        Ok(SourceCar { vehicle, textures, paints: car_paints, collision, hinged, lamps })
    }
}

fn list_cars(dir: &Path) -> Result<Vec<CarFile>> {
    let mut cars = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("listing {}", dir.display()))?.flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        let number = name.strip_prefix("VEH_").and_then(|rest| rest.strip_suffix("_HI.BNDL"));
        let Some(number) = number.and_then(|digits| digits.parse().ok()) else { continue };
        cars.push(CarFile { number, path: entry.path() });
    }
    cars.sort_by_key(|car| car.number);
    Ok(cars)
}
