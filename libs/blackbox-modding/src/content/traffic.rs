//! The traffic side of a car manifest: `role = "traffic"` and the `[traffic]` table.
//!
//! A traffic car drives the roads on its own, among the cars the game puts there, instead of being driven or chasing. The
//! game's traffic manager picks its cars from the patterns of the neighbourhoods (which type, how often, how many at once);
//! a mod car joins those patterns with the numbers of this table, so the host's density, population and cop-share settings
//! apply to it exactly as they do to the game's own traffic. A car may be traffic only (`role = "traffic"`), or a player or
//! cop car that is also traffic (a `[traffic]` table next to another role).

use serde::Deserialize;

use super::ContentError;

/// The largest spawn weight, percent.
pub const MAX_WEIGHT: u32 = 100;
/// The most cars of one kind on the road at once that a manifest may allow (0 is no limit).
pub const MAX_INSTANCES: u32 = 16;
/// The range of the speed factor.
pub const SPEED_FACTOR: (f32, f32) = (0.5, 1.5);
/// The most patterns a manifest may name.
pub const MAX_PATTERNS: usize = 16;
/// The longest pattern name, in bytes.
const MAX_PATTERN_BYTES: usize = 32;

/// What kind of road car a traffic car is. The class sets the defaults of the other keys (a truck is rare, slow and one at
/// a time), tells the game which sounds the car uses (its horn and engine class) and is shown by the devtools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrafficClass {
    /// A small hatchback or compact car.
    Compact,
    /// A four-door saloon (the default).
    #[default]
    Sedan,
    /// A two-door coupe.
    Coupe,
    /// A sport utility vehicle or station wagon.
    Suv,
    /// A pickup truck.
    Pickup,
    /// A van or minivan.
    Van,
    /// A heavy truck.
    Truck,
    /// A bus.
    Bus,
    /// A taxi.
    Taxi,
}

impl TrafficClass {
    /// The name the manifest uses.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Sedan => "sedan",
            Self::Coupe => "coupe",
            Self::Suv => "suv",
            Self::Pickup => "pickup",
            Self::Van => "van",
            Self::Truck => "truck",
            Self::Bus => "bus",
            Self::Taxi => "taxi",
        }
    }

    /// The spawn weight of the class when the manifest does not give one, percent. The game's own common cars have 20 to
    /// 40, its taxis and vans 10 to 25, its rare trucks 10.
    pub const fn default_weight(self) -> u32 {
        match self {
            Self::Compact | Self::Sedan => 20,
            Self::Coupe => 12,
            Self::Suv | Self::Pickup => 15,
            Self::Van => 12,
            Self::Taxi => 10,
            Self::Truck => 6,
            Self::Bus => 5,
        }
    }

    /// How many cars of the kind may be on the road at once when the manifest does not say. Trucks and buses are one at a
    /// time, like the game's garbage and fire trucks; the other classes have no limit but their share.
    pub const fn default_max_instances(self) -> u32 {
        match self {
            Self::Truck | Self::Bus => 1,
            _ => 0,
        }
    }

    /// The factor on the pattern's cruising speeds when the manifest does not give one.
    pub const fn default_speed_factor(self) -> f32 {
        match self {
            Self::Coupe => 1.05,
            Self::Pickup => 0.97,
            Self::Van => 0.95,
            Self::Truck => 0.85,
            Self::Bus => 0.8,
            _ => 1.0,
        }
    }
}

/// A checked traffic entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Traffic {
    pub class: TrafficClass,
    /// How often the kind is picked among the pattern's cars, and the share of the traffic slots it may take at most:
    /// 1 to [`MAX_WEIGHT`], percent. The game's `traftaxi` is 20 in its default pattern.
    pub weight: u32,
    /// At most this many cars of the kind on the road at once; 0 is no limit.
    pub max_instances: u32,
    /// The factor on the pattern's street and highway speeds this car cruises at.
    pub speed_factor: f32,
    /// The traffic patterns (districts) the car appears in, lower case: `default`, `downtown`, `collegesouth`,
    /// `collegenorth`, `collegehighway`, `cityhighway`, `coastalsouth`, `coastalnorth`, `industrial`, `drag`. Empty: all.
    pub patterns: Vec<String>,
}

impl Traffic {
    /// The entry of a class with all its defaults.
    pub fn of_class(class: TrafficClass) -> Self {
        Self {
            class,
            weight: class.default_weight(),
            max_instances: class.default_max_instances(),
            speed_factor: class.default_speed_factor(),
            patterns: Vec::new(),
        }
    }
}

/// The table as serde reads it. Unknown keys are an error, like everywhere in a car manifest.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct Raw {
    class: Option<TrafficClass>,
    weight: Option<u32>,
    max_instances: Option<u32>,
    speed_factor: Option<f32>,
    patterns: Option<Vec<String>>,
}

/// Check the `[traffic]` table. `required` is a car whose role is `traffic`: it is traffic with the defaults of its class when
/// the table is left out. Any other role with no table is not traffic (`None`).
pub(super) fn check(raw: Option<Raw>, required: bool) -> Result<Option<Traffic>, ContentError> {
    let raw = match (raw, required) {
        (Some(raw), _) => raw,
        (None, true) => Raw::default(),
        (None, false) => return Ok(None),
    };
    let class = raw.class.unwrap_or_default();
    let mut traffic = Traffic::of_class(class);
    if let Some(weight) = raw.weight {
        if !(1..=MAX_WEIGHT).contains(&weight) {
            return Err(ContentError::Field { field: "traffic weight", reason: "is not from 1 to 100 percent" });
        }
        traffic.weight = weight;
    }
    if let Some(max) = raw.max_instances {
        if max > MAX_INSTANCES {
            return Err(ContentError::Field { field: "traffic max_instances", reason: "is not from 0 to 16" });
        }
        traffic.max_instances = max;
    }
    if let Some(factor) = raw.speed_factor {
        if !(SPEED_FACTOR.0..=SPEED_FACTOR.1).contains(&factor) {
            return Err(ContentError::Field { field: "traffic speed_factor", reason: "is not from 0.5 to 1.5" });
        }
        traffic.speed_factor = factor;
    }
    traffic.patterns = patterns(raw.patterns.unwrap_or_default())?;
    Ok(Some(traffic))
}

/// The pattern names, lower case, each once. A name is letters, digits and `_`, 1 to 32 bytes.
fn patterns(names: Vec<String>) -> Result<Vec<String>, ContentError> {
    if names.len() > MAX_PATTERNS {
        return Err(ContentError::Field { field: "traffic patterns", reason: "names more than 16 patterns" });
    }
    let mut out: Vec<String> = Vec::with_capacity(names.len());
    for name in names {
        let valid = !name.is_empty()
            && name.len() <= MAX_PATTERN_BYTES
            && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
        if !valid {
            return Err(ContentError::Field {
                field: "traffic patterns",
                reason: "names a pattern that is not letters, digits and _, 1 to 32 bytes",
            });
        }
        let name = name.to_ascii_lowercase();
        if out.contains(&name) {
            return Err(ContentError::Field { field: "traffic patterns", reason: "lists a pattern twice" });
        }
        out.push(name);
    }
    Ok(out)
}
