//! Wheel placement inputs from the car's `ecar` gameplay record
//! (docs/specs/car-assembly.md §3, docs/formats/cardata.md "AttribSys ecar").

use blackbox_attrib::{CollectionRef, Database};

/// Everything §3 reads from `ecar`. Wheel order: front left, front right, rear right, rear left.
#[derive(Debug, Clone, PartialEq)]
pub struct WheelSetup {
    /// x, y, z of each wheel centre (car space, metres) and the tyre radius.
    pub tire_offsets: [[f32; 4]; 4],
    /// Front-end suspension compression added to z: front, rear.
    pub fe_compressions: [f32; 2],
    /// Wanted tyre width per wheel.
    pub skid_width: [f32; 4],
    /// Width multiplier per body kit: x front, y rear.
    pub skid_width_kit_scale: Vec<[f32; 2]>,
    /// Extra track per body kit, millimetres: front, rear.
    pub kit_wheel_offset: [Vec<f32>; 2],
    /// Camber amount: front, rear (degrees = amount × 7).
    pub camber: [f32; 2],
    /// Negative: mirror the left wheels instead of rotating them (directional rims).
    pub spoke_count: i32,
}

impl WheelSetup {
    /// The setup of the car whose base model name is `base_model_name` (e.g. `BMWM3GTR`).
    pub fn read(db: &Database, base_model_name: &str) -> Option<Self> {
        let car = db.collection("ecar", &base_model_name.to_ascii_lowercase())?;
        let f = |field: &str, i: usize| car.get_at(field, i).and_then(|v| v.as_f32()).unwrap_or(0.0);
        Some(Self {
            tire_offsets: std::array::from_fn(|i| {
                car.get_at("TireOffsets", i).and_then(|v| v.as_vector4()).unwrap_or_default()
            }),
            fe_compressions: [f("FECompressions", 0), f("FECompressions", 1)],
            skid_width: std::array::from_fn(|i| f("TireSkidWidth", i)),
            skid_width_kit_scale: items(&car, "TireSkidWidthKitScale", |v| v.as_vector2()),
            kit_wheel_offset: [
                items(&car, "KitWheelOffsetFront", |v| v.as_i64().map(|n| n as f32)),
                items(&car, "KitWheelOffsetRear", |v| v.as_i64().map(|n| n as f32)),
            ],
            camber: [f("CamberFront", 0), f("CamberRear", 0)],
            spoke_count: car.get_i32("WheelSpokeCount").unwrap_or(0),
        })
    }

    /// Width scale of body kit `kit` for wheel `i`.
    pub fn kit_scale(&self, kit: usize, i: usize) -> f32 {
        self.skid_width_kit_scale.get(kit).map_or(1.0, |s| if is_front(i) { s[0] } else { s[1] })
    }

    /// Extra track of body kit `kit` for wheel `i`, metres.
    pub fn kit_offset(&self, kit: usize, i: usize) -> f32 {
        self.kit_wheel_offset[usize::from(!is_front(i))].get(kit).copied().unwrap_or(0.0) * 0.001
    }
}

pub fn is_front(wheel: usize) -> bool {
    wheel < 2
}

fn items<T>(car: &CollectionRef<'_>, field: &str, f: impl Fn(&blackbox_attrib::Value) -> Option<T>) -> Vec<T> {
    let Some(value) = car.get(field) else { return Vec::new() };
    match value.as_array() {
        Some(items) => items.iter().map_while(&f).collect(),
        None => f(value).into_iter().collect(),
    }
}
