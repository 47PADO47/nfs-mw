//! Loading the car models the traffic manager asks for.

use blackbox_render::Renderer;
use nfsmw_data::pursuit::ai_vehicle;

use super::{CopCar, CopModel, ModelRequest, TrafficModel};
use crate::scenes::world::drive::CarRig;
use crate::scenes::world::{WorldScene, load_car};

/// The folder of a traffic car: its `pvehicle` name, or that name without the `traf` prefix (`traftaxi` is
/// the folder `TAXI`).
fn folders(vehicle: &str) -> Vec<String> {
    let upper = vehicle.to_ascii_uppercase();
    let mut found = vec![upper.clone()];
    if let Some(rest) = upper.strip_prefix("TRAF") {
        found.push(rest.to_owned());
    }
    found
}

/// Loads one traffic car model by its `pvehicle` name.
fn load_traffic(scene: &mut WorldScene, renderer: &mut Renderer, vehicle: &str) -> Result<TrafficModel, String> {
    let mut last = format!("no car folder for {vehicle}");
    for folder in folders(vehicle) {
        let (folder, model) = match load_car(&scene.dir, &folder) {
            Ok(car) => car,
            Err(e) => {
                last = format!("{e:#}");
                continue;
            }
        };
        let physics = scene
            .physics_of(&model)
            .or_else(|_| scene.physics.car(vehicle))
            .map_err(|e| format!("{vehicle} has no physics: {e:#}"))?;
        let rig = std::rc::Rc::new(CarRig::upload(renderer, model));
        return Ok(TrafficModel { name: folder, rig, physics });
    }
    Err(last)
}

/// Loads the cop cars `names` that are not loaded yet.
pub(super) fn load_cop_models(scene: &mut WorldScene, renderer: &mut Renderer, names: &[String]) -> Result<(), String> {
    for name in names {
        if scene.traffic.as_ref().is_some_and(|t| t.has_cop_model(name)) {
            continue;
        }
        let (folder, model) = load_car(&scene.dir, &name.to_ascii_uppercase()).map_err(|e| format!("{name}: {e:#}"))?;
        let car_type = model.car_type.clone().unwrap_or_else(|| name.clone());
        let physics = scene.physics_of(&model).map_err(|e| format!("{name}: {e:#}"))?;
        let ai = ai_vehicle(scene.physics.database(), &car_type);
        let performance = blackbox_vehicle::performance::measure(&physics.spec);
        log::info!(
            "cop car {folder}: top speed {:.0} m/s, grip {:.2} to {:.2}, AI limit {:.0} km/h x{} x{}",
            performance.top_speed,
            performance.start_grip,
            performance.end_grip,
            ai.max_speed_kmh,
            ai.top_speed_multiplier,
            ai.acceleration_multiplier
        );
        let rig = std::rc::Rc::new(CarRig::upload(renderer, model));
        let traffic = scene.traffic.as_mut().ok_or("this track has no road network")?;
        traffic.add_cop_model(CopModel {
            model: TrafficModel { name: folder, rig, physics },
            car: CopCar { performance, ai },
        });
    }
    Ok(())
}

/// Loads the first model the traffic manager is waiting for (one per call, so loading does not stall a frame).
pub(in crate::scenes::world) fn load_requested(scene: &mut WorldScene, renderer: &mut Renderer) {
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
