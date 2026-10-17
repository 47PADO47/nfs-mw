//! Loading the car models the traffic manager asks for.

use blackbox_gfx::RenderBackend;
use nfsmw_data::pursuit::ai_vehicle;

use super::{CopCar, CopModel, ModelRequest, TrafficModel};
use crate::scenes::world::drive::CarRig;
use crate::scenes::world::{WorldScene, keys, load_car, sun};

#[cfg(test)]
mod shading_tests;

/// The car folders to try for the `pvehicle` `vehicle`, best first: its own name, that name without the `traf`
/// prefix (`traftaxi` is the folder `TAXI`), then the records it inherits from that are folders themselves
/// (`semib` has no model of its own and uses the tractor's, `SEMI`). `cars` are the folders of the install and
/// `lineage` the vehicle and its ancestors.
fn folders(cars: &[String], vehicle: &str, lineage: &[String]) -> Vec<String> {
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

/// Loads the model and physics of one `pvehicle`, without the trailer it may pull.
fn load_body(scene: &mut WorldScene, renderer: &mut dyn RenderBackend, vehicle: &str) -> Result<TrafficModel, String> {
    let mut last = format!("no car folder for {vehicle}");
    let cars = nfsmw_data::car::list(&scene.dir);
    let lineage = scene.physics.lineage(vehicle);
    for folder in folders(&cars, vehicle, &lineage) {
        let (_, model) = match load_car(&scene.dir, &folder) {
            Ok(car) => car,
            Err(e) => {
                last = format!("{e:#}");
                continue;
            }
        };
        let physics = scene
            .physics
            .car(vehicle)
            .or_else(|_| scene.physics_of(&model))
            .map_err(|e| format!("{vehicle} has no physics: {e:#}"))?;
        let rig = std::rc::Rc::new(CarRig::upload(renderer, keys::next_car(), model, scene.car_shading, sun::to_sun()));
        let audio_profile = audio_profile(scene.physics.database(), vehicle);
        let tractor = scene.physics.is_tractor(vehicle);
        return Ok(TrafficModel {
            name: vehicle.to_ascii_uppercase(),
            rig,
            physics,
            tractor,
            trailer: None,
            cop_lights: None,
            audio_profile,
        });
    }
    Err(last)
}

/// Loads one traffic car model by its `pvehicle` name, with the trailer it pulls when it is a semi tractor.
fn load_traffic(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    vehicle: &str,
) -> Result<TrafficModel, String> {
    let mut model = load_body(scene, renderer, vehicle)?;
    let Some(trailer) = scene.physics.trailer_of(vehicle) else { return Ok(model) };
    let trailer = load_body(scene, renderer, &trailer).map_err(|e| format!("its trailer {trailer}: {e}"))?;
    model.trailer = Some(Box::new(trailer));
    Ok(model)
}

/// Loads the cop cars `names` that are not loaded yet.
pub(super) fn load_cop_models(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    names: &[String],
) -> Result<(), String> {
    for name in names {
        if scene.traffic.as_ref().is_some_and(|t| t.has_cop_model(name)) {
            continue;
        }
        let (folder, model) = {
            let cars = nfsmw_data::car::list(&scene.dir);
            let lineage = scene.physics.lineage(name);
            let mut last = format!("no car folder for {name}");
            let mut found = None;
            for folder in folders(&cars, name, &lineage) {
                match load_car(&scene.dir, &folder) {
                    Ok(car) => {
                        found = Some(car);
                        break;
                    }
                    Err(e) => last = format!("{e:#}"),
                }
            }
            found.ok_or(last)?
        };
        let car_type = model.car_type.clone().unwrap_or_else(|| name.clone());
        let physics = scene.physics_of(&model).map_err(|e| format!("{name}: {e:#}"))?;
        let ai = ai_vehicle(scene.physics.database(), &car_type);
        let performance =
            blackbox_vehicle::performance::measure_with_integration(&physics.spec, physics.tire_integration);
        log::info!(
            "cop car {folder}: top speed {:.0} m/s, grip {:.2} to {:.2}, AI limit {:.0} km/h x{} x{}",
            performance.top_speed,
            performance.start_grip,
            performance.end_grip,
            ai.max_speed_kmh,
            ai.top_speed_multiplier,
            ai.acceleration_multiplier
        );
        let cop_lights =
            nfsmw_data::car::cop_lights::CopLightFx::read(scene.physics.database(), &model).and_then(|fx| {
                let hashes: Vec<u32> = fx.lanes.iter().flat_map(|l| l.emitters.iter().map(|e| e.texture)).collect();
                let found = nfsmw_data::car::exhaust::particle_textures(&scene.dir, &hashes).unwrap_or_default();
                super::cop_lights::CopLightSet::build(renderer, &fx, &found)
            });
        let rig = std::rc::Rc::new(CarRig::upload(renderer, keys::next_car(), model, scene.car_shading, sun::to_sun()));
        let traffic = scene.traffic.as_mut().ok_or("this track has no road network")?;
        traffic.add_cop_model(CopModel {
            model: TrafficModel {
                name: folder,
                rig,
                physics,
                tractor: scene.physics.is_tractor(&car_type),
                trailer: None,
                cop_lights,
                audio_profile: audio_profile(scene.physics.database(), &car_type),
            },
            car: CopCar { performance, ai },
        });
    }
    Ok(())
}

/// Loads the police helicopter's rig, once per track. The helicopter is not a car: its `pvehicle` record has no chassis,
/// tires or brakes (`docs/specs/ai-helicopter.md` section 1), so only the model is loaded, with no `CarPhysics`.
pub(super) fn load_helicopter_model(scene: &mut WorldScene, renderer: &mut dyn RenderBackend) -> Result<(), String> {
    let traffic = scene.traffic.as_mut().ok_or("this track has no road network")?;
    if traffic.has_helicopter_rig() {
        return Ok(());
    }
    let (_, model) = load_car(&scene.dir, "COPHELI").map_err(|e| format!("copheli: {e:#}"))?;
    let rig = std::rc::Rc::new(CarRig::upload(renderer, keys::next_car(), model, scene.car_shading, sun::to_sun()));
    let traffic = scene.traffic.as_mut().ok_or("this track has no road network")?;
    traffic.set_helicopter_rig(rig);
    Ok(())
}

/// Loads the first model the traffic manager is waiting for (one per call, so loading does not stall a frame).
pub(in crate::scenes::world) fn load_requested(scene: &mut WorldScene, renderer: &mut dyn RenderBackend) {
    let Some(traffic) = scene.traffic.as_mut() else { return };
    let mut requests = traffic.take_model_requests();
    if requests.is_empty() {
        return;
    }
    let request = requests.remove(0);
    traffic.requeue(requests);
    match request {
        ModelRequest::Traffic(vehicle) => match load_traffic(scene, renderer, &vehicle) {
            Ok(model) => {
                log::info!("traffic: loaded {vehicle}");
                if let Some(traffic) = scene.traffic.as_mut() {
                    traffic.add_model(model);
                }
            }
            Err(e) => {
                log::warn!("traffic: {vehicle}: {e}");
                if let Some(traffic) = scene.traffic.as_mut() {
                    traffic.model_failed(&vehicle);
                }
            }
        },
        ModelRequest::Patrol(vehicle) => {
            if let Err(e) = load_cop_models(scene, renderer, std::slice::from_ref(&vehicle)) {
                log::warn!("traffic: patrol {vehicle}: {e}");
                if let Some(traffic) = scene.traffic.as_mut() {
                    traffic.model_failed(&vehicle);
                }
            }
        }
    }
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
