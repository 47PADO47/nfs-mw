//! Loading the car models the traffic manager asks for. The cars are read on the loader's worker thread
//! (`model_loader.rs`). Here the physics is made from them, and the rigs are uploaded, a few a frame.

use std::rc::Rc;

use blackbox_gfx::RenderBackend;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::cop_lights::CopLightFx;
use nfsmw_data::car::physics::CarPhysics;
use nfsmw_data::pursuit::ai_vehicle;

use super::cop_lights::CopLightSet;
use super::model_loader::{
    CarParts, CopExtrasLoad, CopLoad, Done, Job, Loaded, Lookup, ModelLoader, Request, StagedCop, TrafficLoad,
};
use super::{CopCar, CopModel, ModelRequest, TrafficModel};
use crate::scenes::world::drive::CarRig;
use crate::scenes::world::{WorldScene, keys, sun};

/// The models a frame installs while they arrive in the background.
const INSTALLS_PER_FRAME: usize = 2;
const NO_ROADS: &str = "this track has no road network";

#[cfg(test)]
mod shading_tests;

/// The car folders to try for the `pvehicle` `vehicle`, best first: its own name, that name without the `traf`
/// prefix (`traftaxi` is the folder `TAXI`), then the records it inherits from that are folders themselves
/// (`semib` has no model of its own and uses the tractor's, `SEMI`). `cars` are the folders of the install and
/// `lineage` the vehicle and its ancestors.
pub(super) fn folders(cars: &[String], vehicle: &str, lineage: &[String]) -> Vec<String> {
    let upper = vehicle.to_ascii_uppercase();
    let mut found = vec![upper.clone()];
    if let Some(rest) = upper.strip_prefix("TRAF") {
        found.push(rest.to_owned());
    }
    for ancestor in lineage.iter().skip(1) {
        let name = ancestor.to_ascii_uppercase();
        if cars.contains(&name) && !found.contains(&name) {
            found.push(name);
        }
    }
    found
}

/// Installed class IDs; missing fields remain explicitly unavailable.
fn audio_profile(db: &blackbox_attrib::Database, vehicle: &str) -> crate::audio::traffic::TrafficProfile {
    let collection = db.collection("pvehicle", &vehicle.to_ascii_lowercase());
    crate::audio::traffic::TrafficProfile {
        horn: collection.as_ref().and_then(|c| c.get_u32("HornType")),
        engine: collection.as_ref().and_then(|c| c.get_u32("TrafficEngType")),
        whoosh: collection.as_ref().and_then(|c| c.get_u32("WooshType")),
    }
}

/// Whether a load a command asks for must finish before the command returns: the scene is in the warm-up of a
/// screenshot run (`traffic warmup`), which has to see the same cars on the same frame every time.
fn warming_up(scene: &WorldScene) -> bool {
    scene.traffic.as_ref().is_some_and(|t| !t.settled())
}

/// The lookup of `vehicle`'s car folders: its name and its lineage on the scene's physics data.
fn lookup(scene: &WorldScene, vehicle: &str) -> Lookup {
    Lookup { vehicle: vehicle.to_owned(), lineage: scene.physics.lineage(vehicle) }
}

/// Asks the loader for `request`. The first request makes the loader.
fn ask(scene: &mut WorldScene, request: Request, job: Job) {
    let Some(traffic) = scene.traffic.as_mut() else { return };
    if traffic.model_loader.is_none() {
        traffic.model_loader = Some(ModelLoader::new(scene.dir.clone()));
    }
    if let Some(loader) = traffic.model_loader.as_mut() {
        loader.submit(request, job);
    }
}

/// The job for a traffic car: its model, and the model of the trailer it pulls when it is a semi tractor.
fn traffic_job(scene: &WorldScene, vehicle: &str) -> Job {
    let trailer = scene.physics.trailer_of(vehicle).map(|name| lookup(scene, &name));
    Job::Traffic { vehicle: lookup(scene, vehicle), trailer }
}

/// Asks for the models the manager is waiting for (when `submit`: the scene's residency is complete), and installs the
/// ones that arrived. A frame installs at most `INSTALLS_PER_FRAME`. A `blocking` frame waits for every model that is
/// pending, so a warm-up run sees the same cars on the same frame each time.
pub(in crate::scenes::world) fn load_requested(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    submit: bool,
    blocking: bool,
) {
    let requests = match scene.traffic.as_mut() {
        Some(traffic) if submit => traffic.take_model_requests(),
        _ => Vec::new(),
    };
    for request in requests {
        match request {
            ModelRequest::Traffic(vehicle) => {
                let job = traffic_job(scene, &vehicle);
                ask(scene, Request::Traffic(vehicle), job);
            }
            ModelRequest::Patrol(name) => {
                let job = Job::Cop(lookup(scene, &name));
                ask(scene, Request::Cop(name), job);
            }
            ModelRequest::Support(name) => {
                // Loaded since it was asked for: a second copy would be installed beside the first.
                if scene.traffic.as_ref().is_some_and(|t| t.has_cop_model(&name)) {
                    continue;
                }
                let job = Job::Cop(lookup(scene, &name));
                ask(scene, Request::Cop(name), job);
            }
        }
    }
    pump(scene, renderer, blocking);
}

/// Loads the cop cars `names` that are not loaded yet. A cop car that is not loaded yet is not spawned (`spawn_cop_at`),
/// so a wave started now fills in as its cars arrive. In a warm-up this waits for them and reports the ones that did not
/// load.
pub(super) fn load_cop_models(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    names: &[String],
) -> Result<(), String> {
    if scene.traffic.is_none() {
        return Err(NO_ROADS.to_owned());
    }
    for name in names {
        if scene.traffic.as_ref().is_some_and(|t| t.has_cop_model(name)) {
            continue;
        }
        let job = Job::Cop(lookup(scene, name));
        ask(scene, Request::Cop(name.clone()), job);
    }
    if !warming_up(scene) {
        return Ok(());
    }
    pump(scene, renderer, true);
    let missing = names.iter().find(|name| !scene.traffic.as_ref().is_some_and(|t| t.has_cop_model(name)));
    match missing {
        Some(name) => Err(format!("{name} did not load")),
        None => Ok(()),
    }
}

/// Loads the police helicopter's rig, once per track. The helicopter is not a car: its `pvehicle` record has no chassis,
/// tires or brakes (`docs/specs/ai-helicopter.md` section 1), so only the model is loaded, with no `CarPhysics`.
pub(super) fn load_helicopter_model(scene: &mut WorldScene, renderer: &mut dyn RenderBackend) -> Result<(), String> {
    let Some(traffic) = scene.traffic.as_ref() else { return Err(NO_ROADS.to_owned()) };
    if traffic.has_helicopter_rig() {
        return Ok(());
    }
    ask(scene, Request::Helicopter, Job::Helicopter);
    if !warming_up(scene) {
        return Ok(());
    }
    pump(scene, renderer, true);
    if !scene.traffic.as_ref().is_some_and(|t| t.has_helicopter_rig()) {
        return Err("the police helicopter did not load".to_owned());
    }
    Ok(())
}

/// Installs the finished requests: at most `INSTALLS_PER_FRAME`, or all of them when `blocking`.
fn pump(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, blocking: bool) {
    let limit = match blocking {
        true => usize::MAX,
        false => INSTALLS_PER_FRAME,
    };
    for _ in 0..limit {
        let Some(done) = next_done(scene, blocking) else { return };
        handle(scene, renderer, done);
    }
}

fn next_done(scene: &mut WorldScene, blocking: bool) -> Option<Done> {
    scene.traffic.as_mut()?.model_loader.as_mut()?.next(blocking)
}

fn handle(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, done: Done) {
    let Done { request, result } = done;
    match (request, result) {
        (Request::Traffic(vehicle), Ok(Loaded::Traffic(load))) => install_traffic(scene, renderer, &vehicle, *load),
        (Request::Traffic(vehicle), Err(error)) => fail(scene, &vehicle, &error),
        (Request::Cop(name), Ok(Loaded::Cop(load))) => cop_arrived(scene, &name, *load),
        (Request::Cop(_), Ok(Loaded::CopExtras(load))) => install_cop(scene, renderer, *load),
        (Request::Cop(name), Err(error)) => fail_cop(scene, &name, &error),
        (Request::Helicopter, Ok(Loaded::Helicopter(model))) => install_helicopter(scene, renderer, *model),
        (Request::Helicopter, Err(error)) => log::warn!("the police helicopter has no model: {error}"),
        _ => log::warn!("model loader: a result that does not match its request"),
    }
}

/// A model that did not load: its cars are not spawned (`TrafficWorld::model_failed`).
fn fail(scene: &mut WorldScene, vehicle: &str, error: &str) {
    log::warn!("traffic: {vehicle}: {error}");
    if let Some(traffic) = scene.traffic.as_mut() {
        traffic.model_failed(vehicle);
    }
}

/// A cop model that did not load: as `fail`, and a chase that is running drops the wave's entries of it, so the wave does
/// not wait on a model that cannot load (what `begin_pursuit` does for the models that fail at the start).
fn fail_cop(scene: &mut WorldScene, name: &str, error: &str) {
    fail(scene, name, error);
    if let Some(traffic) = scene.traffic.as_mut() {
        traffic.cop_model_failed(name);
    }
}

fn install_traffic(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, vehicle: &str, load: TrafficLoad) {
    match traffic_model(scene, renderer, vehicle, load) {
        Ok(model) => {
            log::info!("traffic: loaded {vehicle}");
            if let Some(traffic) = scene.traffic.as_mut() {
                traffic.add_model(model);
            }
        }
        Err(error) => fail(scene, vehicle, &error),
    }
}

/// The traffic model of `vehicle` with the trailer it pulls. Both cars' physics are made before either rig is uploaded.
fn traffic_model(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    vehicle: &str,
    load: TrafficLoad,
) -> Result<TrafficModel, String> {
    let TrafficLoad { body, trailer } = load;
    let body = car_parts(scene, vehicle, body)?;
    let trailer = trailer
        .map(|(name, model)| car_parts(scene, &name, model).map_err(|e| format!("its trailer {name}: {e}")))
        .transpose()?;
    let mut model = upload(scene, renderer, body);
    model.trailer = trailer.map(|parts| Box::new(upload(scene, renderer, parts)));
    Ok(model)
}

/// The first trip of a cop car is in: its physics and light effect are made here, and its second trip goes out.
fn cop_arrived(scene: &mut WorldScene, name: &str, load: CopLoad) {
    match cop_job(scene, name, load) {
        Ok(job) => ask(scene, Request::Cop(name.to_owned()), job),
        Err(error) => fail_cop(scene, name, &error),
    }
}

/// The second trip of a cop car: its performance is measured for its type, and its light textures are read, by
/// the hashes its light effect names.
fn cop_job(scene: &mut WorldScene, name: &str, load: CopLoad) -> Result<Job, String> {
    let CopLoad { folder, model } = load;
    let car_type = model.car_type.clone().unwrap_or_else(|| name.to_owned());
    let physics = scene.physics_of(&model).map_err(|e| format!("{name}: {e:#}"))?;
    let ai = ai_vehicle(scene.physics.database(), &car_type);
    let light_fx = CopLightFx::read(scene.physics.database(), &model);
    let hashes: Vec<u32> = light_fx
        .as_ref()
        .map_or_else(Vec::new, |fx| fx.lanes.iter().flat_map(|lane| lane.emitters.iter().map(|e| e.texture)).collect());
    let car = parts(scene, folder, &car_type, model, physics);
    Ok(Job::CopExtras { car_type, hashes, staged: Box::new(StagedCop { car, ai, light_fx }) })
}

fn install_cop(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, load: CopExtrasLoad) {
    let CopExtrasLoad { staged, performance, textures } = load;
    let StagedCop { car, ai, light_fx } = staged;
    log::info!(
        "cop car {}: top speed {:.0} m/s, grip {:.2} to {:.2}, AI limit {:.0} km/h x{} x{}",
        car.name,
        performance.top_speed,
        performance.start_grip,
        performance.end_grip,
        ai.max_speed_kmh,
        ai.top_speed_multiplier,
        ai.acceleration_multiplier
    );
    let cop_lights = light_fx.and_then(|fx| CopLightSet::build(renderer, &fx, &textures));
    let model = upload(scene, renderer, car);
    let Some(traffic) = scene.traffic.as_mut() else { return };
    traffic.add_cop_model(CopModel { model: TrafficModel { cop_lights, ..model }, car: CopCar { performance, ai } });
}

fn install_helicopter(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, model: CarModel) {
    let rig = Rc::new(CarRig::upload(renderer, keys::next_car(), model, scene.car_shading, sun::to_sun()));
    if let Some(traffic) = scene.traffic.as_mut() {
        traffic.set_helicopter_rig(rig);
    }
}

/// The physics of a traffic car, which the scene's physics data makes (its `pvehicle` first, then its model's type).
fn car_parts(scene: &mut WorldScene, vehicle: &str, model: CarModel) -> Result<CarParts, String> {
    let physics = scene
        .physics
        .car(vehicle)
        .or_else(|_| scene.physics_of(&model))
        .map_err(|e| format!("{vehicle} has no physics: {e:#}"))?;
    Ok(parts(scene, vehicle.to_ascii_uppercase(), vehicle, model, physics))
}

/// A loaded car with its flags as the installed data gives them for `type_name`.
fn parts(scene: &WorldScene, name: String, type_name: &str, model: CarModel, physics: CarPhysics) -> CarParts {
    CarParts {
        name,
        model,
        physics,
        tractor: scene.physics.is_tractor(type_name),
        audio_profile: audio_profile(scene.physics.database(), type_name),
    }
}

/// The GPU part of loading a car: its rig.
fn upload(scene: &WorldScene, renderer: &mut dyn RenderBackend, car: CarParts) -> TrafficModel {
    let CarParts { name, model, physics, tractor, audio_profile } = car;
    let rig = Rc::new(CarRig::upload(renderer, keys::next_car(), model, scene.car_shading, sun::to_sun()));
    TrafficModel { name, rig, physics, tractor, trailer: None, cop_lights: None, audio_profile }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_traffic_car_tries_its_own_folder_then_the_one_without_the_prefix() {
        let cars = names(&["TAXI", "TRAFTAXI"]);
        assert_eq!(folders(&cars, "traftaxi", &names(&["traftaxi", "street"])), ["TRAFTAXI", "TAXI"]);
    }

    #[test]
    fn a_tractor_variant_uses_the_folder_of_the_tractor_it_inherits_from() {
        let cars = names(&["SEMI", "TRAILERB"]);
        let lineage = names(&["semib", "semi", "tractors", "cars", "default"]);
        assert_eq!(folders(&cars, "semib", &lineage), ["SEMIB", "SEMI"]);
        assert_eq!(folders(&cars, "trailerb", &names(&["trailerb", "trailers", "default"])), ["TRAILERB"]);
    }
}
