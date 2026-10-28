//! The world as entity views (`nfsmw_sim::EntityView`), in ascending id order: the players' cars on their `PlayerId`,
//! then the traffic, police and roadblock cars, the helicopter, the spike strips and the props in motion on the ids of
//! the [`EntityTable`]. Poses and velocities are render space (x forward, y left, z up, metres), after the last tick and
//! not interpolated. A parked player (a race seat or a cutscene holds the car) and a car still waiting for its road are
//! not in the world for the clients, as in co-op.

use glam::{Quat, Vec3};
use nfsmw_sim::{EntityId, EntityKind, EntityView, PlayerId};

use super::super::WorldScene;
use super::super::ai::{NetCar, NetCarKind};
use super::super::props::MovingProp;
use super::super::space;
use super::catalog::CarCatalog;
use super::flags::{
    FLAG_BRAKING, FLAG_HEADLIGHTS, FLAG_LIGHTS, FLAG_ROADBLOCK_CAR, FLAG_SPIKE_STRIP, FLAG_WRECKED, SPIKE_STRIP_TYPE,
};
use super::ids::{EntityTable, Native};

/// The brake lamp from which a player's brake lights count as lit [ours].
const BRAKING: f32 = 0.05;

/// Where a thing is and how it moves.
struct Motion {
    position: Vec3,
    rotation: Quat,
    linear: Vec3,
    angular: Vec3,
}

/// The things the world has to replicate now besides the players, by their world identity. `props` are the props in
/// motion after the tick (`WorldSimHost::props`).
pub fn natives(scene: &WorldScene, props: &[MovingProp]) -> Vec<Native> {
    let mut natives = Vec::new();
    if let Some(traffic) = scene.traffic.as_ref() {
        natives.extend(
            traffic.net_cars().filter(|car| person_of(scene, car).is_none()).map(|car| Native::Car(car.entity)),
        );
        natives.extend(traffic.net_helicopter().map(|_| Native::Helicopter));
        natives.extend(traffic.spike_strips().map(|(block, index, _)| strip(block, index)));
    }
    natives.extend(props.iter().map(|prop| Native::Prop(prop.id)));
    natives
}

fn strip(block: u32, index: usize) -> Native {
    Native::Strip(block, u16::try_from(index).unwrap_or(u16::MAX))
}

/// Every entity of the world appended to `out`, in ascending id order. A thing the table has no id for yet (it
/// appeared after the table's last refresh) is left out until it has one. `props` are the props in motion after the
/// tick, the same the table was refreshed with.
pub fn export(
    scene: &WorldScene,
    props: &[MovingProp],
    table: &EntityTable,
    catalog: &CarCatalog,
    out: &mut Vec<EntityView>,
) {
    let start = out.len();
    players(scene, catalog, out);
    if let Some(traffic) = scene.traffic.as_ref() {
        for car in traffic.net_cars() {
            if let Some(player) = person_of(scene, &car) {
                out.push(person_car(player, &car, catalog));
                continue;
            }
            let Some(id) = table.get(Native::Car(car.entity)) else { continue };
            out.push(ai_car(id, &car, catalog));
        }
        if let (Some((position, rotation, linear)), Some(id)) =
            (traffic.net_helicopter(), table.get(Native::Helicopter))
        {
            let motion = Motion { position, rotation, linear, angular: Vec3::ZERO };
            out.push(view(id, EntityKind::Helicopter, &motion, 0, 0));
        }
        for (block, index, frame) in traffic.spike_strips() {
            let Some(id) = table.get(strip(block, index)) else { continue };
            let (_, rotation, position) = frame.to_scale_rotation_translation();
            let motion = Motion { position, rotation, linear: Vec3::ZERO, angular: Vec3::ZERO };
            out.push(view(id, EntityKind::Roadblock, &motion, SPIKE_STRIP_TYPE, FLAG_SPIKE_STRIP));
        }
    }
    for prop in props {
        let Some(id) = table.get(Native::Prop(prop.id)) else { continue };
        let motion = Motion {
            position: prop.position,
            rotation: prop.rotation,
            linear: prop.linear_velocity,
            angular: prop.angular_velocity,
        };
        out.push(view(id, EntityKind::MovingProp, &motion, prop.name_hash as u16, 0));
    }
    out[start..].sort_by_key(|view| view.id);
}

/// The players in play whose car is on the road.
fn players(scene: &WorldScene, catalog: &CarCatalog, out: &mut Vec<EntityView>) {
    for player in scene.players.iter() {
        let drive = &player.drive;
        let Some(sim) = drive.sim() else { continue };
        let pose = drive.stepped_pose();
        let body = sim.body();
        let motion = Motion {
            position: pose.position,
            rotation: pose.rotation,
            linear: space::to_render(body.linear_velocity.to_array()),
            angular: space::to_render(body.angular_velocity.to_array()),
        };
        let mut flags = 0;
        if drive.brake_lamp > BRAKING {
            flags |= FLAG_BRAKING;
        }
        if drive.headlights && !drive.lights_dark() {
            flags |= FLAG_HEADLIGHTS;
        }
        let mut view = view(EntityId::from(player.id.0), EntityKind::PlayerCar, &motion, 0, flags);
        view.type_id = catalog.type_number(&drive.car_name);
        view.owner = Some(player.id);
        out.push(view);
    }
}

/// The player a race seats in this car: a network seat drives a racer of the traffic world while its own car is parked
/// (`players/race.rs`), and the clients know that racer as the player's car.
fn person_of(scene: &WorldScene, car: &NetCar<'_>) -> Option<PlayerId> {
    let racer = car.racer.filter(|&racer| racer >= 1)?;
    scene.race_people.get(racer - 1).copied()
}

/// A person's racer car as their player car: the player's id, kind and owner.
fn person_car(player: PlayerId, car: &NetCar<'_>, catalog: &CarCatalog) -> EntityView {
    let mut view = ai_car(EntityId::from(player.0), car, catalog);
    view.kind = EntityKind::PlayerCar;
    view.owner = Some(player);
    view
}

/// A computer-driven car: a roadblock's car is a roadblock entity, flagged as a car.
fn ai_car(id: EntityId, car: &NetCar<'_>, catalog: &CarCatalog) -> EntityView {
    let (kind, mut flags) = match car.kind {
        NetCarKind::Traffic => (EntityKind::TrafficCar, 0),
        NetCarKind::Police => (EntityKind::PoliceCar, 0),
        NetCarKind::Roadblock => (EntityKind::Roadblock, FLAG_ROADBLOCK_CAR),
    };
    for (on, flag) in [(car.braking, FLAG_BRAKING), (car.lights, FLAG_LIGHTS), (car.wrecked, FLAG_WRECKED)] {
        if on {
            flags |= flag;
        }
    }
    let motion = Motion {
        position: car.position,
        rotation: car.rotation,
        linear: car.linear_velocity,
        angular: car.angular_velocity,
    };
    view(id, kind, &motion, catalog.type_number(car.model), flags)
}

fn view(id: EntityId, kind: EntityKind, motion: &Motion, type_id: u16, flags: u32) -> EntityView {
    EntityView {
        id,
        kind,
        owner: None,
        position: motion.position.to_array(),
        orientation: motion.rotation.to_array(),
        linear_velocity: motion.linear.to_array(),
        angular_velocity: motion.angular.to_array(),
        type_id,
        flags,
    }
}
