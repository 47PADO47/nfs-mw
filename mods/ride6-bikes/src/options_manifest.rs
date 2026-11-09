//! The components of a bike as manifest text (ADR 0014, `docs/modding/ride6-bike-upgrades.md`): its `[[slot]]` and `[parts]` tables,
//! and the catalog file of components every bike's slots name.
//!
//! The slots a bike gets are the ones its data has: `exhaust`, `rims`, `brake_discs` and `bar_pad` where the catalogue lists
//! aftermarket parts that sit on it (and the stock part is a solid of its own, [`crate::build_pieces`]), and the upgrade
//! ladders `perf_engine`, `perf_chassis` and `perf_brakes` where its physics asset's upgrade tables change a number. The
//! components are defined once for all bikes; a part is bound to a solid per bike.

use std::collections::BTreeMap;

use crate::build_pieces::{At, Mounts, Piece, ROAD_HOMOLOGATION, ROAD_MIRRORS};
use crate::options::{BikeOptions, Pair, Upgrade};
use crate::options_pack::{pack_path, solid_name};
use crate::partfile::Anchor;
use crate::physics::toml_text::{Toml, quote};
use crate::silencer;

/// Levels of the ladders: the engine kit has three, the brake kit and the chassis two (the icons the game ships for them).
pub const ENGINE_LEVELS: u32 = 3;
pub const CHASSIS_LEVELS: u32 = 2;
pub const BRAKE_LEVELS: u32 = 2;

/// The id of the component that shows the road equipment.
pub const ROAD_ON: &str = "r6:road.on";

/// One component of the catalog file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Component {
    pub name: String,
    pub tier: u32,
    /// The part ids it shows.
    pub shows: Vec<String>,
    /// `(parameter key, multiplier)` of each number it changes.
    pub muls: Vec<(&'static str, f64)>,
}

/// One slot of a bike.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    pub id: &'static str,
    pub group: &'static str,
    pub accepts: Vec<String>,
    pub default: String,
    /// The slot can be emptied (the road equipment's: `vehicle group road off`).
    pub optional: bool,
    /// The components of other slots a component of this one brings (`bundle` of its offer): the brake ladder's level and the
    /// disc kit of the same level.
    pub bundles: BTreeMap<String, Vec<String>>,
}

/// One part bound to a solid.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub id: String,
    pub solid: String,
    pub pack: Option<String>,
    pub role: &'static str,
    pub matrix: Option<[f32; 12]>,
}

/// What a bike gets: its slots, its part bindings and the components they name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    pub slots: Vec<Slot>,
    pub parts: Vec<Part>,
    pub components: Vec<(String, Component)>,
}

/// A name that is an id: lower case, only letters, digits and `_ . -`; a long one is cut and given a hash of the whole.
pub fn slug(text: &str) -> String {
    let plain: String = text
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-') { c } else { '_' })
        .collect();
    if plain.len() <= 40 {
        return plain;
    }
    let hash = plain.bytes().fold(0x811C_9DC5_u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
    format!("{}-{hash:08x}", &plain[..31])
}

/// The name of a row of a table for the shop: its prefix and its `_Livery-NN` suffix off, words apart.
pub fn display(row: &str) -> String {
    let body = row.split_once('_').map_or(row, |(_, rest)| rest);
    let body = body.split("_Livery-").next().unwrap_or(body);
    let words: Vec<&str> = body.split(['_', '-']).filter(|word| !word.is_empty()).collect();
    words.join(" ")
}

/// The multiplier of a ladder's tier `level` of `levels`: the multiplier at full level, reached in equal steps.
pub fn step(full: f64, level: u32, levels: u32) -> f64 {
    1.0 + (full - 1.0) * f64::from(level) / f64::from(levels)
}

fn ladder_id(kind: &str, level: u32, multiplier: f64) -> String {
    format!("r6:{kind}.l{level}.{}", (multiplier * 10_000.0).round() as i64)
}

/// The bike being planned: its car folder, its frames and the stock pieces it has.
struct Bike<'a> {
    car: &'a str,
    mounts: Mounts,
    have: &'a [&'a Piece],
}

impl Bike<'_> {
    fn piece(&self, tag: &str) -> Option<&Piece> {
        self.have.iter().copied().find(|piece| piece.tag == tag)
    }

    /// The binding of a stock piece to the solid of this car.
    fn stock(&self, piece: &Piece) -> Part {
        let solid = format!("{}_KIT00_{}", self.car.to_ascii_uppercase(), piece.tag);
        Part { id: piece.part.to_owned(), solid, pack: None, role: piece.role, matrix: None }
    }

    /// The binding of a part of the catalogue: the solid `solid_id` of the pack of `asset`, mounted by `anchor` in the frame `at`.
    fn fitted(&self, id: String, (asset, solid_id): (&str, &str), role: &'static str, mount: (At, &Anchor)) -> Part {
        let matrix = Some(self.mounts.matrix(mount.0, mount.1));
        Part { id, solid: solid_name(solid_id), pack: Some(pack_path(asset)), role, matrix }
    }
}

fn component(name: String, shows: Vec<String>) -> Component {
    Component { name, shows, ..Component::default() }
}

impl Plan {
    fn add(&mut self, id: &str, component: Component) {
        if !self.components.iter().any(|(known, _)| known == id) {
            self.components.push((id.to_owned(), component));
        }
    }

    /// A slot with its stock component (which shows `stock_parts`) and the others it accepts.
    fn slot(&mut self, slot: (&'static str, &'static str), stock: (&str, &[&str]), offered: Vec<(String, Component)>) {
        let (id, shows) = stock;
        self.add(id, component("Stock".to_owned(), shows.iter().map(|s| (*s).to_owned()).collect()));
        let mut accepts = vec![id.to_owned()];
        for (offer, component) in offered {
            accepts.push(offer.clone());
            self.add(&offer, component);
        }
        self.slots.push(Slot {
            id: slot.0,
            group: slot.1,
            accepts,
            default: id.to_owned(),
            optional: false,
            bundles: BTreeMap::new(),
        });
    }

    fn exhaust(&mut self, bike: &Bike<'_>, options: &BikeOptions) {
        let Some(stock) = bike.piece("EXHAUST") else { return };
        if options.exhaust.is_empty() {
            return;
        }
        let mut offered = Vec::new();
        for item in &options.exhaust {
            let slug = slug(&item.id);
            let (header, muffler) = (format!("exhaust.header.{slug}"), format!("exhaust.muffler.{slug}"));
            // A twin silencer is one part for each can, `exhaust.muffler2.<slug>`, of the solid `<asset>_G2` of the pack.
            let twins: Vec<(String, u8, &Anchor)> = (2u8..)
                .zip(&item.muffler_twins)
                .map(|(number, anchor)| (format!("exhaust.muffler{number}.{slug}"), number, anchor))
                .collect();
            let mut shows = vec![header.clone(), muffler.clone()];
            shows.extend(twins.iter().map(|(id, _, _)| id.clone()));
            offered.push((format!("r6:exhaust.{slug}"), component(display(&item.id), shows)));
            let anchors = [(header, &item.header, &item.header_anchor), (muffler, &item.muffler, &item.muffler_anchor)];
            for (id, asset, anchor) in anchors {
                self.parts.push(bike.fitted(id, (asset, asset), "body", (At::Body, anchor)));
            }
            for (id, number, anchor) in twins {
                let solid = silencer::solid_name(&item.muffler, number);
                self.parts.push(bike.fitted(id, (&item.muffler, &solid), "body", (At::Body, anchor)));
            }
        }
        self.parts.push(bike.stock(stock));
        self.slot(("exhaust", "looks"), ("r6:exhaust.stock", &["exhaust.stock"]), offered);
    }

    /// A slot of front and rear parts: the rims or the brake discs.
    fn wheels(&mut self, bike: &Bike<'_>, kind: Wheels, list: &[Pair]) {
        let (front, rear) = (bike.piece(kind.tags.0), bike.piece(kind.tags.1));
        let (Some(front), Some(rear)) = (front, rear) else { return };
        if list.is_empty() {
            return;
        }
        let prefix = kind.part;
        let mut offered = Vec::new();
        for item in list {
            let slug = slug(&item.asset);
            let (front_id, back_id) = (format!("{prefix}.front.{slug}"), format!("{prefix}.back.{slug}"));
            let name = match kind.slot {
                "rims" => display(&item.id),
                _ => format!("Brake kit level {}", item.level),
            };
            offered
                .push((format!("r6:{}.{slug}", kind.slot), component(name, vec![front_id.clone(), back_id.clone()])));
            let (front_solid, back_solid) =
                (format!("{}_front_{}", item.asset, kind.suffix), format!("{}_back_{}", item.asset, kind.suffix));
            self.parts.push(bike.fitted(
                front_id,
                (&item.asset, &front_solid),
                "wheel_front",
                (At::FrontHub, &item.front),
            ));
            self.parts.push(bike.fitted(back_id, (&item.asset, &back_solid), "wheel_rear", (At::RearHub, &item.back)));
        }
        self.parts.extend([bike.stock(front), bike.stock(rear)]);
        let stock = format!("r6:{}.stock", kind.slot);
        let shows = [format!("{prefix}.front.stock"), format!("{prefix}.back.stock")];
        self.slot((kind.slot, "looks"), (&stock, &[shows[0].as_str(), shows[1].as_str()]), offered);
    }

    fn bar_pad(&mut self, bike: &Bike<'_>, options: &BikeOptions) {
        let Some(stock) = bike.piece("BARPAD") else { return };
        if options.pads.is_empty() {
            return;
        }
        let mut offered = Vec::new();
        for item in &options.pads {
            let slug = slug(&item.id);
            let part_id = format!("barpad.{slug}");
            offered.push((format!("r6:barpad.{slug}"), component(display(&item.id), vec![part_id.clone()])));
            self.parts.push(bike.fitted(part_id, (&item.asset, &item.asset), "steered", (At::FrontHub, &item.anchor)));
        }
        self.parts.push(bike.stock(stock));
        self.slot(("bar_pad", "looks"), ("r6:barpad.stock", &["barpad.stock"]), offered);
    }

    /// The road equipment: one optional slot, group `road`, whose only component shows the side mirrors and the homologation kit the
    /// bike has as solids of their own. Shown by default; `vehicle group road off` and the panel's checkbox take them off.
    fn road(&mut self, bike: &Bike<'_>) {
        let pieces: Vec<&Piece> = [bike.piece("MIRRORS"), bike.piece("HOMOLOGATION")].into_iter().flatten().collect();
        if pieces.is_empty() {
            return;
        }
        // One component for every bike: a part id a bike does not bind is skipped when the car is drawn.
        let shows = [ROAD_MIRRORS, ROAD_HOMOLOGATION].map(str::to_owned).to_vec();
        self.add(ROAD_ON, component("Road equipment".to_owned(), shows));
        for piece in pieces {
            self.parts.push(bike.stock(piece));
        }
        let slot = Slot {
            id: "road_equipment",
            group: "road",
            accepts: vec![ROAD_ON.to_owned()],
            default: ROAD_ON.to_owned(),
            optional: true,
            bundles: BTreeMap::new(),
        };
        self.slots.push(slot);
    }

    /// The disc components of the bike's brake kits by kit level, for the kits whose slot the plan has.
    fn disc_kits(&self, kits: &[Pair]) -> BTreeMap<u32, String> {
        let Some(slot) = self.slots.iter().find(|slot| slot.id == Wheels::DISCS.slot) else { return BTreeMap::new() };
        let mut out = BTreeMap::new();
        for kit in kits {
            let id = format!("r6:{}.{}", Wheels::DISCS.slot, slug(&kit.asset));
            if slot.accepts.contains(&id) {
                out.entry(kit.level).or_insert(id);
            }
        }
        out
    }

    /// The upgrade ladders of the bike's physics asset: a slot for each number its upgrade tables change.
    fn upgrades(&mut self, upgrade: Upgrade, kits: &BTreeMap<u32, String>) {
        let ladders: [(&'static str, &str, f64, u32, &[&'static str]); 3] = [
            ("perf_engine", "engine", upgrade.peak_torque, ENGINE_LEVELS, &["bike.engine.peak_torque_nm"]),
            ("perf_chassis", "chassis", upgrade.mass, CHASSIS_LEVELS, &["bike.chassis.mass_kg"]),
            (
                "perf_brakes",
                "brakes",
                upgrade.brake_force,
                BRAKE_LEVELS,
                &["bike.brakes.front_force_n", "bike.brakes.rear_force_n"],
            ),
        ];
        for (slot, kind, full, levels, keys) in ladders {
            if (full - 1.0).abs() < 1e-9 {
                continue;
            }
            let mut offered = Vec::new();
            let mut bundles = BTreeMap::new();
            for level in 1..=levels {
                let multiplier = step(full, level, levels);
                let name = match kind {
                    "engine" => format!("Engine kit level {level}"),
                    "chassis" => format!("Lightweight chassis level {level}"),
                    _ => format!("Brake kit level {level}"),
                };
                let muls = keys.iter().map(|key| (*key, multiplier)).collect();
                let id = ladder_id(kind, level, multiplier);
                // The brake ladder's level N comes with the discs of the bike's brake kit of level N **[choice]**: the data names
                // both by the same `Brakes-Kit-Level-N` row, and the shop's performance page shows them as one upgrade.
                if let Some(disc) = kits.get(&level).filter(|_| kind == "brakes") {
                    bundles.insert(id.clone(), vec![disc.clone()]);
                }
                offered.push((id, Component { name, tier: level, shows: Vec::new(), muls }));
            }
            self.slot((slot, "performance"), (&format!("r6:{kind}.stock"), &[]), offered);
            if let Some(added) = self.slots.last_mut() {
                added.bundles = bundles;
            }
        }
    }

    /// The plan of the bike `car` with `options`, whose stock parts are the pieces `have` and whose frames are `mounts`.
    pub fn of(car: &str, options: &BikeOptions, have: &[&Piece], mounts: Mounts) -> Self {
        let bike = Bike { car, mounts, have };
        let mut plan = Self::default();
        plan.exhaust(&bike, options);
        plan.wheels(&bike, Wheels::RIMS, &options.rims);
        plan.wheels(&bike, Wheels::DISCS, &options.brakes);
        plan.bar_pad(&bike, options);
        if let Some(upgrade) = options.upgrade {
            let kits = plan.disc_kits(&options.brakes);
            plan.upgrades(upgrade, &kits);
        }
        plan.road(&bike);
        plan
    }

    /// The `[[slot]]` and `[parts]` tables, as manifest text; empty for a bike with no slot.
    pub fn text(&self) -> String {
        let mut t = Toml::new();
        for slot in &self.slots {
            t.table_array("slot");
            t.string("id", slot.id);
            t.string("group", slot.group);
            t.raw(&format!("accepts = [{}]", accepts_text(slot)));
            t.string("default", &slot.default);
            if slot.optional {
                t.raw("optional = true");
            }
        }
        for part in &self.parts {
            t.table(&format!("parts.{}", quote(&part.id)));
            t.string("solid", &part.solid);
            if let Some(pack) = &part.pack {
                t.string("pack", pack);
            }
            t.string("role", part.role);
            if let Some(matrix) = part.matrix {
                t.nums("matrix", &matrix.map(f64::from));
            }
        }
        t.into_string()
    }
}

/// The `accepts` list of a slot: a bare id for a component, an inline table for one that brings others (`bundle`).
fn accepts_text(slot: &Slot) -> String {
    let item = |id: &String| match slot.bundles.get(id) {
        None => quote(id),
        Some(bundle) => {
            let bundled: Vec<String> = bundle.iter().map(|other| quote(other)).collect();
            format!("{{ component = {}, bundle = [{}] }}", quote(id), bundled.join(", "))
        }
    };
    slot.accepts.iter().map(item).collect::<Vec<_>>().join(", ")
}

/// What differs between the rim slot and the brake disc slot.
#[derive(Clone, Copy)]
struct Wheels {
    slot: &'static str,
    /// The tags of the front and rear stock pieces.
    tags: (&'static str, &'static str),
    /// The prefix of the part ids (`rim.front.stock`).
    part: &'static str,
    /// The kind in the part ids of the shared table (`<asset>_front_rim`).
    suffix: &'static str,
}

impl Wheels {
    const RIMS: Self = Self { slot: "rims", tags: ("FRONT_RIM", "REAR_RIM"), part: "rim", suffix: "rim" };
    const DISCS: Self = Self { slot: "brake_discs", tags: ("FRONT_DISC", "REAR_DISC"), part: "disc", suffix: "brake" };
}

/// The catalog file `components/ride6-bikes.toml` of the components of `plans`, each defined once, in id order.
pub fn catalog_text<'a>(plans: impl Iterator<Item = &'a Plan>) -> String {
    let mut all: BTreeMap<&str, &Component> = BTreeMap::new();
    for (id, component) in plans.flat_map(|plan| plan.components.iter()) {
        all.entry(id.as_str()).or_insert(component);
    }
    let mut t = Toml::new();
    t.comment(
        "The components of the RIDE 6 bikes (docs/modding/ride6-bike-upgrades.md). Generated by `ride6-bike-import`.",
    );
    for (id, component) in all {
        t.table_array("component");
        t.string("id", id);
        t.string("name", &component.name);
        if component.tier > 0 {
            t.int("tier", i64::from(component.tier));
        }
        let mut effects = Vec::new();
        if !component.shows.is_empty() {
            let shown: Vec<String> = component.shows.iter().map(|part| quote(part)).collect();
            effects.push(format!("{{ kind = \"part\", show = [{}] }}", shown.join(", ")));
        }
        for (key, value) in &component.muls {
            effects.push(format!("{{ kind = \"param\", key = \"{key}\", op = \"mul\", value = {:?} }}", *value as f32));
        }
        if !effects.is_empty() {
            t.raw(&format!("effects = [{}]", effects.join(", ")));
        }
    }
    t.into_string()
}

#[cfg(test)]
#[path = "options_manifest_tests.rs"]
mod tests;
