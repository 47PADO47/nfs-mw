//! The driver figure of a 2012 car bundle and where it sits. Spec: `docs/formats/chameleon-vehicle.md` ("The driver").
//!
//! One bundle of the install (the police Explorer) draws a seated driver, hands on the wheel, as a body mesh whose
//! material technique ends in `Driver`. No other car has one. The figure is modelled around its car's `SteeringWheel`
//! bone, so it is carried to another car by that car's own bone: [`Driver::seated`] moves it onto the wheel of a
//! [`Cabin`] and, where the roof or the floor leaves too little room, lowers and shrinks it a little.

use chameleon_bundle::{Bundle, Pool, ResourceKind};

use crate::skeleton::Skeleton;
use crate::vehicle::Surfaces;
use crate::{Lod, PartMesh, Vehicle};

/// The end of the technique name of the driver's material.
const TECHNIQUE: &str = "_Driver";
/// The bone the figure is modelled around, and every car's steering wheel hangs from.
const WHEEL_BONE: &str = "SteeringWheel";
/// The strip over the seat that gives a car's roof: half its width, and how far behind the wheel it starts and ends.
const SEAT_HALF_WIDTH: f32 = 0.25;
const SEAT_NEAR: f32 = 0.1;
const SEAT_FAR: f32 = 0.8;
/// How thick the roof is: the head stops this far below its top.
const ROOF_THICKNESS: f32 = 0.05;
/// The lowest the soles may go (metres above the ground): below this they would be through the floor of the car.
const FLOOR: f32 = 0.1;
/// The most the seat is lowered, and the smallest the figure is made, to fit a low roof.
const MAX_DROP: f32 = 0.12;
const MIN_SCALE: f32 = 0.85;

/// Whether `mesh` is the driver's.
pub fn is_driver(mesh: &PartMesh) -> bool {
    mesh.appearance.technique.ends_with(TECHNIQUE)
}

/// The positions of the driver's meshes in `lod`.
pub fn driver_meshes(lod: &Lod) -> Vec<usize> {
    lod.iter().enumerate().filter(|(_, mesh)| is_driver(mesh)).map(|(at, _)| at).collect()
}

/// Whether the bundle `bundle`, whose resources and the shared shaders `pool` holds, has a material of the driver's
/// technique.
pub fn has_driver(pool: &Pool, bundle: &Bundle) -> bool {
    let surfaces = Surfaces::default();
    let materials = ResourceKind::Material.id();
    let drivers = |id: u64| match surfaces.get(pool, id) {
        Ok(Some(surface)) => surface.1.technique.ends_with(TECHNIQUE),
        _ => false,
    };
    bundle.entries().iter().filter(|entry| entry.kind == materials).any(|entry| drivers(entry.id))
}

/// Where the car's steering wheel is (engine space), `None` when the bundle has no skeleton or no such bone.
pub fn steering_wheel(bundle: &Bundle) -> Option<[f32; 3]> {
    let skeleton = Skeleton::read(bundle)?;
    skeleton.find(WHEEL_BONE).map(|bone| skeleton.bones[bone].position)
}

/// A driver figure and the steering wheel it was modelled around (engine space: x left, y up, z forward).
#[derive(Debug, Clone, PartialEq)]
pub struct Driver {
    pub wheel: [f32; 3],
    pub meshes: Lod,
}

/// Take the driver's meshes out of the body of `vehicle` at every level of detail, so the body no longer draws it. The
/// finest level's meshes are the figure; `None` when there are none, or when the car's `wheel` is not known (a figure
/// that cannot be placed is of no use).
pub fn take_driver(vehicle: &mut Vehicle, wheel: Option<[f32; 3]>) -> Option<Driver> {
    let mut figure = Vec::new();
    for (level, lod) in vehicle.body.iter_mut().enumerate() {
        let (driver, rest): (Lod, Lod) = std::mem::take(lod).into_iter().partition(is_driver);
        *lod = rest;
        if level == 0 {
            figure = driver;
        }
    }
    (!figure.is_empty()).then_some(Driver { wheel: wheel?, meshes: figure })
}

/// The room a car gives its driver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cabin {
    /// Where the car's steering wheel is.
    pub wheel: [f32; 3],
    /// How high the body reaches over the seat: the roof, or the roll hoop and the head rest of an open car.
    pub roof: f32,
}

impl Cabin {
    /// The cabin of a car whose most detailed body is `body` and whose steering wheel is at `wheel`.
    pub fn of(body: &Lod, wheel: [f32; 3]) -> Self {
        let over_the_seat = |p: [f32; 3]| {
            (p[0] - wheel[0]).abs() <= SEAT_HALF_WIDTH && (wheel[2] - SEAT_FAR..=wheel[2] - SEAT_NEAR).contains(&p[2])
        };
        let roof = body
            .iter()
            .flat_map(|mesh| mesh.vertices.iter().map(|v| v.position))
            .filter(|&p| over_the_seat(p))
            .map(|p| p[1])
            .fold(f32::MIN, f32::max);
        Self { wheel, roof }
    }
}

impl Driver {
    /// The lowest and the highest point of the figure.
    fn heights(&self) -> (f32, f32) {
        let ys = self.meshes.iter().flat_map(|mesh| mesh.vertices.iter().map(|v| v.position[1]));
        ys.fold((f32::MAX, f32::MIN), |(low, high), y| (low.min(y), high.max(y)))
    }

    /// How far the seat is lowered and how much the figure is scaled (about the steering wheel, so the hands stay on it)
    /// for `cabin`: not at all when the figure fits between the floor and the roof, else the seat goes down as far as the
    /// floor lets it (at most [`MAX_DROP`]) and then the figure shrinks (to at least [`MIN_SCALE`]).
    fn fit(&self, cabin: &Cabin) -> (f32, f32) {
        let (low, high) = self.heights();
        let (above, below) = (high - self.wheel[1], self.wheel[1] - low);
        let height = cabin.wheel[1];
        let ceiling = cabin.roof - ROOF_THICKNESS;
        let excess = height + above - ceiling;
        let mut drop = 0.0;
        let mut scale: f32 = 1.0;
        if excess > 0.0 && cabin.roof > f32::MIN {
            let room = (height - below - FLOOR).max(0.0);
            drop = excess.min(MAX_DROP).min(room);
            scale = ((ceiling - height + drop) / above).clamp(MIN_SCALE, 1.0);
        }
        // Soles through the floor: a low car shrinks the figure instead of sinking it.
        let soles = ((height - drop - FLOOR) / below).clamp(MIN_SCALE, 1.0);
        (drop, scale.min(soles))
    }

    /// The figure sitting at the steering wheel of `cabin`, in the engine space of that car.
    pub fn seated(&self, cabin: &Cabin) -> Lod {
        let (drop, scale) = self.fit(cabin);
        let place = |p: [f32; 3]| {
            [
                cabin.wheel[0] + scale * (p[0] - self.wheel[0]),
                cabin.wheel[1] + scale * (p[1] - self.wheel[1]) - drop,
                cabin.wheel[2] + scale * (p[2] - self.wheel[2]),
            ]
        };
        let mut meshes = self.meshes.clone();
        for vertex in meshes.iter_mut().flat_map(|mesh| mesh.vertices.iter_mut()) {
            vertex.position = place(vertex.position);
        }
        meshes
    }
}

#[cfg(test)]
mod tests {
    use chameleon_bundle::{Bundle, Pool};
    use chameleon_gfx::Vertex;

    use super::*;
    use crate::{Appearance, SurfaceKind};

    fn mesh(technique: &str, points: &[[f32; 3]]) -> PartMesh {
        PartMesh {
            vertices: points.iter().map(|&position| Vertex { position, color: [255; 4], uv: [0.0; 2] }).collect(),
            triangles: vec![[0, 1, 2]],
            appearance: Appearance {
                kind: SurfaceKind::Body,
                technique: technique.to_owned(),
                diffuse_texture: None,
                diffuse_colour: None,
                alpha_test: false,
            },
        }
    }

    /// A figure 0.6 below and 0.5 above its wheel at (0.4, 1.0, 0.5).
    fn figure() -> Driver {
        let points = [[0.4, 0.4, 1.0], [0.4, 1.5, 0.2], [0.3, 1.0, 0.5]];
        Driver { wheel: [0.4, 1.0, 0.5], meshes: vec![mesh("Vehicle_Body_Driver", &points)] }
    }

    fn ys(lod: &Lod) -> (f32, f32) {
        let ys = lod.iter().flat_map(|m| m.vertices.iter().map(|v| v.position[1]));
        ys.fold((f32::MAX, f32::MIN), |(low, high), y| (low.min(y), high.max(y)))
    }

    #[test]
    fn the_drivers_mesh_is_told_by_its_technique() {
        assert!(is_driver(&mesh("VehicleNFS13_Body_Driver", &[])));
        assert!(!is_driver(&mesh("VehicleNFS13_Body", &[])));
    }

    #[test]
    fn the_drivers_meshes_are_found_by_position() {
        let lod = vec![mesh("A_Body", &[]), mesh("A_Body_Driver", &[]), mesh("A_Glass", &[]), mesh("B_Driver", &[])];
        assert_eq!(driver_meshes(&lod), [1, 3]);
    }

    #[test]
    fn the_driver_leaves_the_body_at_every_level() {
        let level = || vec![mesh("A_Body", &[[0.0; 3]; 3]), mesh("A_Body_Driver", &[[0.0; 3]; 3])];
        let mut vehicle = Vehicle { body: vec![level(), level()], wheels: Vec::new() };
        let driver = take_driver(&mut vehicle, Some([0.4, 1.0, 0.5])).unwrap();
        assert_eq!(driver.meshes.len(), 1);
        assert!(vehicle.body.iter().all(|lod| lod.len() == 1 && !is_driver(&lod[0])));
        let mut again = Vehicle { body: vec![level()], wheels: Vec::new() };
        assert!(take_driver(&mut again, None).is_none(), "a figure with no wheel to sit at is dropped");
        assert_eq!(again.body[0].len(), 1, "but the body is rid of it");
    }

    #[test]
    fn the_roof_is_the_highest_point_over_the_seat() {
        let body = vec![mesh("Body", &[[0.4, 1.4, 0.0], [0.4, 1.9, 1.0], [1.5, 1.8, 0.0]])];
        let cabin = Cabin::of(&body, [0.4, 1.0, 0.5]);
        assert_eq!(cabin.roof, 1.4, "behind the wheel and in line with it: the others are ahead and beside");
    }

    #[test]
    fn a_roomy_cabin_takes_the_figure_as_it_is_moved_onto_its_wheel() {
        let cabin = Cabin { wheel: [0.3, 1.2, 0.8], roof: 2.5 };
        let lod = figure().seated(&cabin);
        let at = lod[0].vertices[2].position;
        assert!(at.iter().zip([0.2, 1.2, 0.8]).all(|(a, b)| (a - b).abs() < 1e-5), "{at:?}");
        let (low, high) = ys(&lod);
        assert!((low - 0.6).abs() < 1e-5 && (high - 1.7).abs() < 1e-5, "{low} {high}");
    }

    #[test]
    fn a_low_roof_lowers_the_seat_then_shrinks_the_figure() {
        // Head 0.5 above the wheel under a roof 0.45 above it: 0.1 of excess, within the drop.
        let cabin = Cabin { wheel: [0.4, 1.0, 0.5], roof: 1.45 };
        let (drop, scale) = figure().fit(&cabin);
        assert!((drop - 0.1).abs() < 1e-5 && (scale - 1.0).abs() < 1e-5, "{drop} {scale}");
        // A roof 0.2 above the wheel: the full drop, then the smallest figure.
        let (drop, scale) = figure().fit(&Cabin { wheel: [0.4, 1.0, 0.5], roof: 1.2 });
        assert!((drop - MAX_DROP).abs() < 1e-5 && (scale - MIN_SCALE).abs() < 1e-5, "{drop} {scale}");
    }

    #[test]
    fn a_low_floor_shrinks_the_figure_instead_of_sinking_it() {
        // The soles would be 0.6 below a wheel 0.65 up: 0.05 above the ground, under the floor.
        let (drop, scale) = figure().fit(&Cabin { wheel: [0.4, 0.65, 0.5], roof: 2.0 });
        assert_eq!(drop, 0.0);
        assert!((MIN_SCALE..1.0).contains(&scale), "{scale}");
    }

    #[test]
    fn a_cabin_without_a_roof_measured_asks_for_nothing() {
        let (drop, scale) = figure().fit(&Cabin { wheel: [0.4, 1.0, 0.5], roof: f32::MIN });
        assert_eq!((drop, scale), (0.0, 1.0));
    }

    /// Only the police Explorer has a driver; seated in any car with a steering wheel the figure is inside the body.
    #[test]
    #[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
    fn the_explorers_driver_sits_inside_every_car_with_a_wheel() {
        let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
        let dir = std::path::Path::new(&dir);
        let textures = std::fs::read(dir.join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
        let shaders = std::fs::read(dir.join("SHADERS.BNDL")).unwrap();
        let (mut own, mut template, mut no_wheel, mut seated) = (0, None, Vec::new(), 0);
        let mut cars = Vec::new();
        for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
            let file = entry.file_name().to_string_lossy().to_uppercase();
            if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
                continue;
            }
            let data = std::fs::read(entry.path()).unwrap();
            let bundle = Bundle::parse(&data).unwrap();
            let mut pool = Pool::new();
            pool.add(bundle.clone());
            pool.add(Bundle::parse(&textures).unwrap());
            pool.add(Bundle::parse(&shaders).unwrap());
            let mut vehicle = crate::load(&pool, crate::find_spec(&bundle).unwrap()).unwrap();
            let wheel = steering_wheel(&bundle);
            let found = has_driver(&pool, &bundle);
            assert_eq!(found, vehicle.body[0].iter().any(is_driver), "{file}: the material and the mesh agree");
            let driver = take_driver(&mut vehicle, wheel);
            assert!(vehicle.body.iter().flatten().all(|m| !is_driver(m)), "{file}");
            own += usize::from(found);
            if template.is_none() {
                template = driver;
            }
            match wheel {
                Some(wheel) => cars.push((file, vehicle, wheel)),
                None => no_wheel.push(file),
            }
        }
        let template = template.expect("one bundle has a driver");
        assert_eq!(own, 1, "only one bundle has a driver");
        for (file, vehicle, wheel) in &cars {
            let cabin = Cabin::of(&vehicle.body[0], *wheel);
            let figure = template.seated(&cabin);
            let (mut low, mut high) = ([f32::MAX; 3], [f32::MIN; 3]);
            for v in vehicle.body[0].iter().flat_map(|m| m.vertices.iter()) {
                for a in 0..3 {
                    low[a] = low[a].min(v.position[a]);
                    high[a] = high[a].max(v.position[a]);
                }
            }
            for v in figure.iter().flat_map(|m| m.vertices.iter()) {
                let inside = (0..3).all(|a| v.position[a] >= low[a] - 0.15 && v.position[a] <= high[a] + 0.15);
                assert!(inside, "{file}: the driver pokes out of the body at {:?}", v.position);
            }
            seated += 1;
        }
        println!("{seated} cars seat the driver; {} have no steering wheel: {no_wheel:?}", no_wheel.len());
    }
}
