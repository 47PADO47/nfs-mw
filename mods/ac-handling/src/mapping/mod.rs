//! From an NFS car to the tyre model (docs/specs/ac-handling/parameter-mapping.md): the grip level and sliding ratio of each
//! axle pick a compound preset, the car's size picks a class, and the per-car file overrides last. A pure function of the
//! spec, the presets and the car file: built once per car change and never touched per tick.

mod classify;
pub mod spec;

pub use classify::{class_for, compound_for, default_bias, grip_level, sliding_ratio};
pub use spec::{CarSpec, Drive};

use crate::keys::{KeyTable, Keys};
use crate::params::TyreParams;
use crate::presets::{CarClass, CarFile, ClassPreset, Compound, PresetSet};

/// The tyres of one car, built from its spec.
#[derive(Debug, Clone, PartialEq)]
pub struct CarParams {
    /// The front axle's tyre.
    pub front: TyreParams,
    /// The rear axle's tyre.
    pub rear: TyreParams,
    /// The compound class in use.
    pub compound: Compound,
    /// The car class in use.
    pub class: CarClass,
    /// The grip level `L` of the front and the rear axle.
    pub levels: [f64; 2],
    /// The `revision` of the spec these were built from.
    pub revision: u32,
}

impl CarParams {
    /// The tyre of wheel `index` (0 and 1 front, 2 and 3 rear). An index past 3 reads the rear axle.
    pub fn axle(&self, index: usize) -> &TyreParams {
        if index < 2 {
            return &self.front;
        }
        &self.rear
    }

    /// The one-line summary of the choice, for the log: `L=1.00 -> sport/sport`.
    pub fn summary(&self) -> String {
        format!("L={:.2} -> {}/{}", self.levels[0], self.compound.name(), self.class.name())
    }
}

/// What the mapping reads besides the spec.
#[derive(Debug, Clone, Copy, Default)]
pub struct MapOptions<'a> {
    /// The car's override file, when it has one.
    pub car: Option<&'a CarFile>,
    /// The compound the player's `preset` setting forces; `None` is automatic. It wins over the car file's `compound`.
    pub forced: Option<Compound>,
}

/// Set the keys a table names.
fn apply(keys: &mut Keys, table: &KeyTable) {
    for (key, value) in table.entries() {
        keys.set(key, value);
    }
}

/// `value = value * mul + add` for the keys a class names.
fn apply_class(keys: &mut Keys, class: &ClassPreset) {
    for (key, factor) in class.mul.entries() {
        if let Some(value) = keys.get(key) {
            keys.set(key, value * factor);
        }
    }
    for (key, offset) in class.add.entries() {
        if let Some(value) = keys.get(key) {
            keys.set(key, value + offset);
        }
    }
}

/// Keep every key inside its range, as the loader would accept it.
fn clamp_ranges(keys: &mut Keys) {
    for key in Keys::NAMES {
        let (Some(value), Some((lo, hi))) = (keys.get(key), Keys::range(key)) else {
            continue;
        };
        keys.set(key, value.clamp(lo, hi));
    }
}

/// Build the tyres of a car (section 4.4, steps 1 to 9).
pub fn map_car(spec: &CarSpec, presets: &PresetSet, options: &MapOptions<'_>) -> CarParams {
    let empty = CarFile::default();
    let car = options.car.unwrap_or(&empty);
    let levels = [grip_level(spec.peak_friction[0]), grip_level(spec.peak_friction[1])];
    let rhos = [
        sliding_ratio(spec.peak_friction[0], spec.sliding_friction[0]),
        sliding_ratio(spec.peak_friction[1], spec.sliding_friction[1]),
    ];
    let compound = options.forced.or(car.compound).unwrap_or_else(|| compound_for(levels[0]));
    let class = car.class.unwrap_or_else(|| class_for(spec));
    let preset = presets.compound(compound);

    // 1 and 2: the defaults, then the compound's keys
    let mut base = Keys::default();
    apply(&mut base, &preset.tyre);
    let mut axles = [base.clone(), base];
    for (axle, keys) in axles.iter_mut().enumerate() {
        // 3: grip and sliding levels of this axle
        keys.mu_y0 = (preset.tyre.mu_y0.unwrap_or(Keys::default().mu_y0)
            + preset.grip_slope * (levels[axle] - preset.level_ref))
            .clamp(0.6, 2.2);
        keys.fall_y = (rhos[axle] - 0.03).clamp(0.6, 0.95);
        keys.fall_x = (rhos[axle] - 0.06).clamp(0.6, 0.95);
        // 4 and 5: the car class, then the load exponents
        apply_class(keys, presets.class(class));
        keys.exp_y = keys.exp_y.clamp(0.6, 1.0);
        keys.exp_x = keys.exp_x.clamp(0.6, 1.0);
        // 6: the car file's [tyre] keys, for both axles
        apply(keys, &car.both);
    }
    // 7: split the mean peak angle between the axles
    let bias = car.bias.or(preset.tyre.axle_angle_bias_deg).unwrap_or_else(|| default_bias(spec.drive));
    let mean = axles[0].peak_angle_deg;
    axles[0].peak_angle_deg = mean + bias / 2.0;
    axles[1].peak_angle_deg = mean - bias / 2.0;
    // 8: the car file's [tyre.front] and [tyre.rear]
    apply(&mut axles[0], &car.front);
    apply(&mut axles[1], &car.rear);
    // 9: the derived numbers
    for keys in &mut axles {
        clamp_ranges(keys);
    }
    let [front, rear] = axles;
    CarParams {
        front: TyreParams::new(front, spec.tyre_width_m[0]),
        rear: TyreParams::new(rear, spec.tyre_width_m[1]),
        compound,
        class,
        levels,
        revision: spec.revision,
    }
}
