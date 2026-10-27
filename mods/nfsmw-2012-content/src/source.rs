//! The 2012 install: which cars it has, and one car's meshes and textures read out of its bundles.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use chameleon_bundle::{Bundle, Pool};
use chameleon_gfx::Texture;
use chameleon_vehicle::{
    Driver, Lamps, PaintPackGroup, PaintSurface, PoliceMarks, PolySoup, Vehicle, VehiclePaint, driver_meshes,
    find_spec, load, load_driver, load_poly_soup, load_texture, localised_text, paint_packs, paint_surfaces, paints,
    police_marks, read_lamps, remove_baked_driver, vehicle_name,
};

use crate::liveries::{self, Signature};
use crate::vehicle_list::{self, ListEntry};

const VEHICLES: &str = "VEHICLES";
const SHARED_TEXTURES: &str = "VEHICLES/VEHICLETEX.BNDL";
/// The vehicle list: which bundles are the same vehicle in another livery.
const VEHICLE_LIST: &str = "VEHICLES/VEHICLELIST.BNDL";
/// The paint packs: which colours and liveries each vehicle of the list is offered in, and the names of them.
const PAINT_PACKS: &str = "GAMELOGIC/GAMEPLAY.BNDL";
/// The English text tables, where the packs' names are.
const TEXTS: &str = "UI/LANGUAGE/0001.BNDL";
const SHADERS: &str = "SHADERS.BNDL";
/// The people of the game: the figures that sit in the cars; optional.
const CHARACTERS: &str = "CHARACTERS/ALL_CHARS.BNDL";
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

/// A driver figure of the install with the textures its meshes draw. The driver a bundle bakes into its body (one does) is not
/// in its [`SourceCar::vehicle`]: the character its driver setup names takes its place there, as in every other car.
pub struct DriverFigure {
    pub driver: Driver,
    pub textures: HashMap<u64, Texture>,
}

/// An opened 2012 install. The shared bundles stay in memory; each car's bundle is read when it is loaded.
pub struct Source {
    textures: Vec<u8>,
    shaders: Vec<u8>,
    /// The character bundle, when the install has one.
    characters: Option<Vec<u8>>,
    /// The driver figures read so far, by character key (`None` for one that does not read).
    figures: Mutex<HashMap<u32, Option<Arc<DriverFigure>>>>,
    /// The shared sample bundle, when the install has one that can be read.
    shared_samples: Option<Vec<u8>>,
    cars: Vec<CarFile>,
    /// The install's folder.
    root: PathBuf,
    /// The entries of the vehicle list; empty when the install has none that reads.
    list: Vec<ListEntry>,
    /// The paint pack groups: the colours and liveries each vehicle is offered in; empty when the install has none that reads.
    packs: Vec<PaintPackGroup>,
    /// The main block of the English text table, which names the packs; `None` when the install has none that reads.
    texts: Option<Vec<u8>>,
    /// The default licence plate texts and the game packs that choose among them; `None` when the install has none that read.
    plates: Option<crate::plate::Plates>,
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
        let packs = read_packs(&root.join(PAINT_PACKS));
        let texts = std::fs::read(root.join(TEXTS)).ok().and_then(|data| crate::brands::text_block(&data));
        Ok(Self {
            textures,
            shaders,
            characters: std::fs::read(root.join(CHARACTERS)).ok(),
            figures: Mutex::new(HashMap::new()),
            shared_samples,
            cars: list_cars(&root.join(VEHICLES))?,
            root: root.to_owned(),
            list,
            packs,
            texts,
            plates: crate::plate::Plates::read(root),
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

    /// The paint pack group of the vehicle whose bundle is `file`: the one its vehicle list entry names. `None` for a vehicle
    /// with no group (the police vehicles) and when the install's packs do not read.
    pub fn pack_group(&self, file: &CarFile) -> Option<&PaintPackGroup> {
        let entry = self.list.iter().find(|entry| entry.file() == file.number)?;
        self.packs.iter().find(|group| entry.group != 0 && group.id == entry.group)
    }

    /// The licence plate text the car in `file` wears by default: the text of the game pack its vehicle list entry names
    /// (`crate::plate`). `None` when the install's texts or list do not read.
    pub fn plate_text(&self, file: &CarFile) -> Option<String> {
        let entry = self.list.iter().find(|entry| entry.file() == file.number)?;
        Some(self.plates.as_ref()?.text(entry.pack))
    }

    /// The paints of the car in `file`, read without decoding the rest of its bundle; none when it does not read.
    pub fn paints_of(&self, file: &CarFile) -> Vec<VehiclePaint> {
        let Ok(data) = std::fs::read(&file.path) else { return Vec::new() };
        Bundle::parse(&data).map(|bundle| paints(&bundle)).unwrap_or_default()
    }

    /// The names the game gives the liveries in `files` (the vehicle's own bundle first), as its text table spells them
    /// (`HERO`): the paint pack of each bundle names it. `None` for a livery the packs or the table do not name.
    pub fn livery_names(&self, files: &[&CarFile]) -> Vec<Option<String>> {
        let group = files.first().and_then(|file| self.pack_group(file));
        let name = |file: &CarFile| -> Option<String> {
            let data = std::fs::read(&file.path).ok()?;
            let pack = group?.livery_of(&Bundle::parse(&data).ok()?)?;
            localised_text(self.texts.as_deref()?, pack.name)
        };
        files.iter().map(|file| name(file)).collect()
    }

    /// The text with string id `id` in the English table, as the game spells it (`SILVER METALLIC`); `None` when the
    /// install has no table that reads or the table lacks the id.
    pub fn text(&self, id: u32) -> Option<String> {
        localised_text(self.texts.as_deref()?, id)
    }

    /// The paint surfaces of every body level of the car in `file` and which colour each takes, read without decoding the
    /// meshes or the textures.
    pub fn paint_surfaces_of(&self, file: &CarFile) -> Result<Vec<Vec<PaintSurface>>> {
        let data = std::fs::read(&file.path).with_context(|| format!("reading {}", file.path.display()))?;
        let bundle = Bundle::parse(&data)?;
        let spec = find_spec(&bundle).context("the bundle has no vehicle graphics spec")?;
        let mut pool = Pool::new();
        pool.add(bundle.clone());
        pool.add(Bundle::parse(&self.textures)?);
        pool.add(Bundle::parse(&self.shaders)?);
        Ok(paint_surfaces(&pool, spec)?)
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
        let mut vehicle = load(&pool, spec)?;
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
        let mut hinged = crate::hinges::read(&bundle, &pool, spec, &vehicle);
        // The body without the driver the bundle bakes into it: the classes of the hinged parts lose the driver's meshes with it.
        let drivers = vehicle.body.first().map(driver_meshes).unwrap_or_default();
        if let Some(hinged) = hinged.as_mut() {
            for &at in drivers.iter().rev() {
                if at < hinged.classes.len() {
                    hinged.classes.remove(at);
                }
            }
        }
        remove_baked_driver(&mut vehicle);
        Ok(SourceCar { vehicle, textures, paints: car_paints, collision, hinged, lamps })
    }

    /// Whether the install has the character bundle the drivers are in.
    pub fn has_characters(&self) -> bool {
        self.characters.is_some()
    }

    /// The driver figure of the character `key` (the key a car's driver setup names), read on the first ask and kept: every car
    /// of a run shares the two characters. `None` when the install has no character bundle or the figure does not read.
    pub fn driver_figure(&self, key: u32) -> Option<Arc<DriverFigure>> {
        let mut known = self.figures.lock().ok()?;
        if let Some(found) = known.get(&key) {
            return found.clone();
        }
        let figure = match self.read_driver(key) {
            Ok(figure) => Some(Arc::new(figure)),
            Err(error) => {
                log::warn!("the driver {key:X} does not read: {error:#}");
                None
            }
        };
        known.insert(key, figure.clone());
        figure
    }

    fn read_driver(&self, key: u32) -> Result<DriverFigure> {
        let characters = self.characters.as_deref().context("the install has no character bundle")?;
        let mut pool = Pool::new();
        pool.add(Bundle::parse(characters)?);
        pool.add(Bundle::parse(&self.shaders)?);
        let driver = load_driver(&pool, key)?;
        let mut textures = HashMap::new();
        for id in driver.textures() {
            if pool.contains(id) {
                textures.insert(id, load_texture(&pool, id)?);
            }
        }
        Ok(DriverFigure { driver, textures })
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

/// The paint pack groups in the gameplay bundle at `path`; none, with a warning, when it does not read.
fn read_packs(path: &Path) -> Vec<PaintPackGroup> {
    let Ok(data) = std::fs::read(path) else { return Vec::new() };
    let groups = Bundle::parse(&data).map_err(chameleon_vehicle::Error::from).and_then(|bundle| paint_packs(&bundle));
    groups.unwrap_or_else(|error| {
        log::warn!("{}: the paint packs do not read: {error}", path.display());
        Vec::new()
    })
}
