//! One 2012 car read out of its bundle, fitted to the 2005 car space and converted: the part of an import that makes the
//! car's geometry, textures and collision.
//!
//! The conversion run ([`crate::import`]) does this for every car and writes the rest of the car (manifest, sounds,
//! cameras) from what it returns. The lazy files ([`crate::lazy`]) do it again, for one car, when the game first opens
//! that car's `GEOMETRY.BIN` or `TEXTURES.BIN`, so the two make the same files from the same steps.

use anyhow::{Context, Result};
use chameleon_bundle::Bundle;
use chameleon_gfx::Texture;
use chameleon_vehicle::{Lod, VehicleStats, driver_setup, read_stats};

use crate::convert;
use crate::fit::{Fit, to_car_space};
use crate::lamp_coronas::Coronas;
use crate::liveries::LiveryChoice;
use crate::source::{CarFile, Source, SourceCar};
use crate::wheels::{self, Placed};
use crate::{two_colour, variants};

/// The settings that shape a car's files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// Biggest texture side in pixels (`max_texture_size`).
    pub max_texture_size: u32,
    /// Whether the driver is posed for the steering and the look back (`driver_poses`).
    pub driver_poses: bool,
}

/// A car converted, with what the rest of its import needs.
pub struct Prepared {
    pub car: SourceCar,
    /// The car's bundle.
    pub bytes: Vec<u8>,
    /// What the car is, from its bundle, with the axles of the wheels the model is drawn on.
    pub stats: VehicleStats,
    pub placed: Placed,
    pub fit: Fit,
    /// The liveries the car offers: each choice and its picture.
    pub pictures: Vec<(LiveryChoice, Texture)>,
    /// The lamp sprites and tables of the car, when it has any (`lamp_coronas.rs`).
    pub coronas: Option<Coronas>,
    /// The car's `GEOMETRY.BIN`, `TEXTURES.BIN` and `BOUNDS.BIN`.
    pub built: convert::Built,
    /// The front end of the body along the car, in the 2012 engine's space.
    pub front_end: f32,
}

/// Read the car in `file` and convert it as the car `id`. `liveries` are the bundles of its other liveries, which are not
/// cars of their own.
pub fn prepare(
    source: &Source,
    file: &CarFile,
    id: &str,
    liveries: &[&CarFile],
    settings: &Settings,
) -> Result<Prepared> {
    let mut car = source.load(file)?;
    // What the car is comes from its own bundle; the user's multipliers, if any, change it later.
    let bytes = source.bundle_bytes(file)?;
    let mut stats = read_stats(&Bundle::parse(&bytes)?).context("the stats of the car could not be read")?;
    let body = car.vehicle.body.first().context("the car has no body")?;
    let (rear_end, front_end) = body
        .iter()
        .flat_map(|mesh| mesh.vertices.iter().map(|v| v.position[2]))
        .fold((f32::MAX, f32::MIN), |(low, high), z| (low.min(z), high.max(z)));
    let placed = wheels::place(&car.vehicle, (rear_end, front_end), stats.axle_z)
        .context("the wheel layout of the car could not be measured")?;
    // The stats' wheelbase and weight distribution are those of the hubs the wheels are drawn on.
    stats.axle_z = placed.axles;
    placed.seat_hubs(&mut car.vehicle);
    let fit = Fit::own(placed.layout);
    let driver = seat_driver(source, &mut car, &bytes);
    let driver_poses = if settings.driver_poses { crate::driver_poses::bake(source, &bytes) } else { Vec::new() };
    // A car with two-colour paint packs offers each as a livery (and takes the texture coordinates of their pictures); any
    // other car offers the pictures of its livery bundles, or its own.
    let pictures = two_colour::apply(source, file, &mut car, id).unwrap_or_else(|| {
        let livery_names = variants::livery_names(source, file, liveries);
        variants::pictures(source, &car, id, liveries, &livery_names)
    });
    // The mid-detail level of the car's low-detail bundle becomes its `C` level (`low_detail.rs`); a car with liveries keeps its own.
    if pictures.is_empty() {
        crate::low_detail::add_mid(source, file, &mut car);
    }
    // The 2012 coronas become the lamp sprites of the car and, on a police car, its light bar (`lamp_coronas.rs`); their pictures
    // go into the car's texture pack with the liveries.
    let coronas = crate::lamp_coronas::build(
        &|rel| source.install_file(rel),
        &bytes,
        &car.lamps,
        &|p| fit.apply(to_car_space(p)),
        id,
    );
    let mut options = convert::Options {
        max_texture_size: settings.max_texture_size,
        liveries: pictures.iter().map(|(choice, picture)| (choice.texture.clone(), picture.clone())).collect(),
        driver,
        driver_poses,
    };
    if let Some(coronas) = &coronas {
        options.liveries.extend(coronas.sprites.iter().cloned());
    }
    let built = convert::build(&car, &fit, id, &options)?;
    Ok(Prepared { car, bytes, stats, placed, fit, pictures, coronas, built, front_end })
}

/// The driver of `car` (the bundle `bytes`): the character the game seats in it (the officer in a police car), seated where the setup says, at every level
/// of detail, with the figure's textures added to the car's. `None` when the car has no driver (the BMW 1M, whose setup
/// marks no seat), or the install has no figure for its character.
fn seat_driver(source: &Source, car: &mut SourceCar, bytes: &[u8]) -> Option<Vec<Lod>> {
    let setup = driver_setup(&Bundle::parse(bytes).ok()?)?;
    let seat = setup.seat?;
    let figure = source.driver_figure(setup.character)?;
    car.textures.extend(figure.textures.iter().map(|(id, texture)| (*id, texture.clone())));
    Some(figure.driver.seated(seat))
}
