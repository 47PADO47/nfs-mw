//! The 2012 install: which cars it has, and one car's meshes and textures read out of its bundles.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chameleon_bundle::{Bundle, Pool};
use chameleon_gfx::Texture;
use chameleon_vehicle::{Vehicle, VehiclePaint, find_spec, load, load_texture, paints, vehicle_name};

const VEHICLES: &str = "VEHICLES";
const SHARED_TEXTURES: &str = "VEHICLES/VEHICLETEX.BNDL";
const SHADERS: &str = "SHADERS.BNDL";

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
}

/// An opened 2012 install. The shared bundles stay in memory; each car's bundle is read when it is loaded.
pub struct Source {
    textures: Vec<u8>,
    shaders: Vec<u8>,
    cars: Vec<CarFile>,
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
        Ok(Self { textures, shaders, cars: list_cars(&root.join(VEHICLES))? })
    }

    /// The high-detail bundle of every car, in file-number order.
    pub fn cars(&self) -> &[CarFile] {
        &self.cars
    }

    /// The engine's object name of the car in `file`, read without decoding any geometry.
    pub fn object_name(&self, file: &CarFile) -> Option<String> {
        let data = std::fs::read(&file.path).ok()?;
        vehicle_name(&Bundle::parse(&data).ok()?)
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
        Ok(SourceCar { vehicle, textures, paints: car_paints })
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
