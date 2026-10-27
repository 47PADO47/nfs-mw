//! The cars of the mods: each `cars/<id>.toml` is read and checked, and the cars are registered against the car table
//! of the install (`nfsmw_data::car::mods`). A car that fails any check is logged and left out; the game goes on.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use blackbox_modding::content::{CARS_FOLDER, CarManifest, CopClass, parse_car};
use game_install::{GameDir, Provider};
use nfsmw_data::car::CarTables;
use nfsmw_data::car::mods::custom;
use nfsmw_data::car::mods::hinged;
use nfsmw_data::car::mods::wheel_layout;
use nfsmw_data::car::mods::{
    self as car_mods, COP_CHANCE, COP_COUNT, CarEffects, CarSpec, CarStats, CopKind, CopUnit, CopWave, EngineSound,
    SharedSound,
};
use nfsmw_data::pursuit::HEAT_LEVELS;

use super::ModDir;

/// Register the cars that the mods declare and publish them: at start-up for every loaded mod, and again for one mod
/// that added cars later. A car that is already published is skipped, so a repeated call adds only what is new.
/// Without any new car request the tables are not read at all. Returns how many cars were added.
pub(super) fn register(dir: &GameDir, mods: &[ModDir]) -> usize {
    let published = car_mods::published();
    let is_new = |spec: &CarSpec| !published.iter().any(|car| car.folder.eq_ignore_ascii_case(&spec.folder));
    let requests: Vec<(CarSpec, Option<custom::Customization>)> =
        mods.iter().flat_map(|m| specs_of(dir, m)).filter(|(spec, _)| is_new(spec)).collect();
    let mut customizations: HashMap<String, custom::Customization> = HashMap::new();
    let mut specs = Vec::with_capacity(requests.len());
    for (spec, own) in requests {
        if let Some(own) = own {
            customizations.entry(spec.folder.clone()).or_insert(own);
        }
        specs.push(spec);
    }
    if specs.is_empty() {
        return 0;
    }
    let tables = match CarTables::load(dir) {
        Ok(tables) => tables,
        Err(error) => {
            log::warn!("mod cars: the car tables cannot be read, so {} car(s) are not added: {error:#}", specs.len());
            return 0;
        }
    };
    let (cars, rejected) = car_mods::register(&tables.types, &specs);
    for refused in &rejected {
        log::warn!("mod {}: car {} is not added: {}", refused.owner, refused.folder, refused.reason);
    }
    // The plate texts the player typed are kept beside the profile's save (`garage/plates.rs`).
    custom::plate::set_source(crate::garage::plates::text_for);
    for car in &cars {
        log::info!("mod {}: car {} added", car.owner, car.folder);
        if let Some(own) = customizations.remove(&car.folder) {
            custom::publish(&car.folder, own);
        }
        publish_hinges(mods, car);
    }
    let added = cars.len();
    car_mods::publish(cars);
    added
}

/// The hinged parts (`[[damage_parts]]`) of an admitted car, read again from its manifest and published for the game.
fn publish_hinges(mods: &[ModDir], car: &car_mods::ModCar) {
    let Some(owner) = mods.iter().find(|m| m.id == car.owner) else { return };
    for path in manifest_files(&owner.dir.join(CARS_FOLDER)) {
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
        if !stem.eq_ignore_ascii_case(&car.folder) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else { return };
        let Ok(manifest) = parse_car(stem, &text) else { return };
        hinged::publish(&car.folder, hinges_of(&manifest.damage_parts, manifest.wing.as_ref()));
        if let Some(layout) = &manifest.wheels {
            wheel_layout::publish(&car.folder, wheel_layout_of(layout));
        }
        return;
    }
}

/// The manifest's `[[damage_parts]]` as the game's hinged parts; a part that hangs from another carries that part's
/// movement (the manifest check puts the parent before the child).
fn hinges_of(
    parts: &[blackbox_modding::content::DamagePart],
    wing: Option<&blackbox_modding::content::Wing>,
) -> Vec<hinged::Hinge> {
    let mut out: Vec<hinged::Hinge> = Vec::with_capacity(parts.len());
    for part in parts {
        let mut hinge = hinge_of(part);
        if !part.tracks.is_empty() {
            hinge.wing = wing.map(wing_of);
        }
        let parent = part.parent.as_ref().and_then(|name| out.iter().find(|known| known.name == *name));
        if let Some(parent) = parent {
            let own = hinged::Ancestor { active: parent.active, tracks: parent.tracks.clone(), pivot: parent.pivot };
            hinge.ancestors = parent.ancestors.iter().cloned().chain(Some(own)).collect();
        }
        out.push(hinge);
    }
    out
}

/// A manifest's `[wheels]` table as the game's wheel layout.
fn wheel_layout_of(layout: &blackbox_modding::content::WheelLayout) -> wheel_layout::WheelLayout {
    let tyre = |tyre: &blackbox_modding::content::Tyre| wheel_layout::Tyre {
        x: tyre.x_m,
        z: tyre.z_m,
        face: tyre.face_m,
        radius: tyre.radius_m,
        width: tyre.width_m,
    };
    wheel_layout::WheelLayout { front: tyre(&layout.front), rear: tyre(&layout.rear) }
}

/// The manifest's `[wing]` as the game's wing machine.
fn wing_of(wing: &blackbox_modding::content::Wing) -> hinged::Wing {
    hinged::Wing {
        open_mph: wing.open_mph,
        brake_mph: wing.brake_mph,
        top_mph: wing.top_mph,
        brake_hold: wing.brake_hold_seconds,
        fade: wing.fade_seconds,
        seconds: wing.slot_seconds,
    }
}

/// A manifest's `[[damage_parts]]` entry as the game's hinged part.
fn hinge_of(part: &blackbox_modding::content::DamagePart) -> hinged::Hinge {
    hinged::Hinge {
        name: part.name.clone(),
        model: part.model.clone(),
        zones: part.zones.iter().map(|zone| zone.index()).collect(),
        pivot: part.pivot,
        axis: part.axis,
        angle: part.angle,
        stages: part.stages.clone(),
        detach_at: part.detach_at,
        detach: part.detach,
        mass_kg: part.mass_kg,
        active: part.active.map(|active| hinged::Active {
            offset: active.offset,
            rotation: active.rotation,
            above_mph: active.above_mph,
            below_mph: active.below_mph,
            seconds: active.seconds,
            gas: active.gas,
            delay: active.delay,
        }),
        ancestors: Vec::new(),
        tracks: part
            .tracks
            .iter()
            .map(|track| hinged::WingTrack {
                slot: track.slot,
                translation: track.translation.clone(),
                rotation: track.rotation.clone(),
            })
            .collect(),
        drives: part.drives.iter().map(drive_of).collect(),
        wing: None,
    }
}

/// A manifest's `[[damage_parts.drive]]` as the game's drive.
fn drive_of(drive: &blackbox_modding::content::PartDrive) -> hinged::Drive {
    use blackbox_modding::content::{DriveInput, DriveOp, DriveTest};
    let test = |test: &DriveTest| hinged::DriveTest {
        input: match test.input {
            DriveInput::Speed => hinged::DriveInput::Speed,
            DriveInput::Gas => hinged::DriveInput::Gas,
            DriveInput::Brake => hinged::DriveInput::Brake,
            DriveInput::Steering => hinged::DriveInput::Steering,
        },
        op: match test.op {
            DriveOp::Eq => hinged::DriveOp::Eq,
            DriveOp::Ne => hinged::DriveOp::Ne,
            DriveOp::Gt => hinged::DriveOp::Gt,
            DriveOp::Lt => hinged::DriveOp::Lt,
            DriveOp::Ge => hinged::DriveOp::Ge,
            DriveOp::Le => hinged::DriveOp::Le,
        },
        value: test.value,
    };
    hinged::Drive {
        open: drive.open.iter().map(test).collect(),
        open_any: drive.open_any,
        open_after: drive.open_after_seconds,
        close: drive.close.iter().map(test).collect(),
        close_any: drive.close_any,
    }
}

/// The car requests of one mod: its `cars/*.toml` files, in name order.
fn specs_of(dir: &GameDir, m: &ModDir) -> Vec<(CarSpec, Option<custom::Customization>)> {
    manifest_files(&m.dir.join(CARS_FOLDER))
        .into_iter()
        .filter_map(|path| match read_spec(dir, &m.id, &path) {
            Ok(spec) => Some(spec),
            Err(reason) => {
                log::warn!("mod {}: {} is not read: {reason}", m.id, path.display());
                None
            }
        })
        .collect()
}

/// The `.toml` files directly in `folder`, sorted by name. A missing folder has none.
fn manifest_files(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("toml")))
        .collect();
    files.sort();
    files
}

/// Read and check one manifest, and turn it into a request the car table can check.
fn read_spec(dir: &GameDir, owner: &str, path: &Path) -> Result<(CarSpec, Option<custom::Customization>), String> {
    let stem = path.file_stem().and_then(|stem| stem.to_str()).ok_or("the file name is not text")?;
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let manifest = parse_car(stem, &text).map_err(|error| error.to_string())?;
    let cop = manifest.role.is_cop().then(|| cop_unit(&manifest));
    let stats = CarStats::from(&manifest.stats);
    let geometry = format!("CARS/{}/GEOMETRY.BIN", manifest.id);
    let geometry_in_mod = matches!(dir.provider(&geometry), Some(Provider::Overlay(id)) if id == owner);
    let own = manifest.customization.as_ref().map(customization_of);
    let spec = CarSpec {
        folder: manifest.id,
        owner: owner.to_owned(),
        name: manifest.name,
        manufacturer: manifest.manufacturer,
        player: manifest.role.is_player(),
        cop,
        geometry_in_mod,
        stats,
        engine_sound: manifest.engine_sound.as_ref().map(engine_sound_of),
        car_effects: manifest.car_sound.as_ref().map(car_effects_of),
    };
    Ok((spec, own))
}

/// The manifest's `[customization]` as the game's own customization of a mod car.
fn customization_of(table: &blackbox_modding::content::Customization) -> custom::Customization {
    use blackbox_modding::content::Finish;
    let part =
        |part: &blackbox_modding::content::Part| custom::Part { name: part.name.clone(), model: part.model.clone() };
    custom::Customization {
        paints: table
            .paints
            .iter()
            .map(|paint| custom::Paint {
                name: paint.name.clone(),
                color: paint.color,
                finish: match paint.finish {
                    Finish::Gloss => custom::Finish::Gloss,
                    Finish::Metallic => custom::Finish::Metallic,
                    Finish::Chrome => custom::Finish::Chrome,
                },
            })
            .collect(),
        default_paint: table.default_paint,
        rim_paints: table
            .rim_paints
            .iter()
            .map(|rim| custom::RimPaint { name: rim.name.clone(), color: rim.color })
            .collect(),
        tints: table
            .tints
            .iter()
            .map(|tint| custom::Tint { name: tint.name.clone(), color: tint.color, opacity: tint.opacity })
            .collect(),
        rims: table.rims.iter().map(part).collect(),
        spoilers: table.spoilers.iter().map(part).collect(),
        roof_scoops: table.roof_scoops.iter().map(part).collect(),
        vinyls: table
            .vinyls
            .iter()
            .map(|vinyl| custom::Vinyl {
                name: vinyl.name.clone(),
                bitmap: vinyl.bitmap.clone(),
                mask: vinyl.mask.clone(),
                channels: vinyl.channels,
            })
            .collect(),
        vinyl_colors: table
            .vinyl_colors
            .iter()
            .map(|color| custom::VinylColor { name: color.name.clone(), color: color.color })
            .collect(),
        liveries: table
            .liveries
            .iter()
            .map(|livery| custom::Livery { name: livery.name.clone(), texture: livery.texture.clone() })
            .collect(),
        default_livery: table.default_livery,
        plate: table.plate.as_ref().map(plate_of),
    }
}

/// The manifest's `[customization.plate]` as the game's own plate.
fn plate_of(table: &blackbox_modding::content::Plate) -> custom::Plate {
    custom::Plate {
        text: table.text.clone(),
        picture: table.picture.clone(),
        glyphs: table.glyphs.clone(),
        columns: table.columns,
        rows: table.rows,
        slots: table.slots,
        aliases: table.aliases.clone(),
    }
}

/// The manifest's `[car_sound]` as the game's effect recordings.
fn car_effects_of(sound: &blackbox_modding::content::CarEffects) -> CarEffects {
    CarEffects {
        shift: sound.shift.clone(),
        shift_up: sound.shift_up.clone(),
        shift_down: sound.shift_down.clone(),
        turbo_whine: sound.turbo_whine.clone(),
        turbo_blowoff: sound.turbo_blowoff.clone(),
        backfire: sound.backfire.clone(),
        pops: sound.pops.clone(),
        no_clunk: sound.no_clunk,
        shared: shared_sound_of(&sound.shared),
    }
}

/// The manifest's `[car_sound.shared]` as the game's shared recordings.
fn shared_sound_of(shared: &blackbox_modding::content::SharedSound) -> SharedSound {
    SharedSound {
        impact_small: shared.impact_small.clone(),
        impact_medium: shared.impact_medium.clone(),
        impact_big: shared.impact_big.clone(),
        scrape: shared.scrape.clone(),
        pops: shared.pops.clone(),
    }
}

/// The manifest's `[engine_sound]` as the game's engine loops.
fn engine_sound_of(sound: &blackbox_modding::content::EngineSound) -> EngineSound {
    EngineSound {
        idle_rpm: sound.idle_rpm,
        redline_rpm: sound.redline_rpm,
        layers: sound.layers.iter().map(super::engine_layers::layer_of).collect(),
    }
}

/// The police unit of a cop manifest: its class and its wave entries. A manifest that names no level is in every level of
/// the game with the default numbers. A level the game does not have is refused by the car table (`car_mods::register`).
fn cop_unit(manifest: &CarManifest) -> CopUnit {
    let kind = match manifest.cop_class {
        CopClass::Patrol => CopKind::Patrol,
        CopClass::Pursuit => CopKind::Pursuit,
        CopClass::Heavy => CopKind::Heavy,
    };
    if manifest.waves.is_empty() {
        let waves = (1..=HEAT_LEVELS).map(|heat| CopWave { heat, count: COP_COUNT, chance: COP_CHANCE }).collect();
        return CopUnit { kind, waves };
    }
    let waves = manifest.waves.iter().map(|w| CopWave { heat: w.heat, count: w.count, chance: w.chance }).collect();
    CopUnit { kind, waves }
}
