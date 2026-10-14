use super::super::{CarPhysics, aero, brakes, chassis, engine, induction, nos, tires, transmission};
use super::{
    PerformanceCatalog, PerformanceCategory as Category, PerformancePackage, catalog,
    fields::{PerformanceFields, ResolvedFields},
};
use anyhow::{Context, Result, ensure};
use blackbox_attrib::Database;

pub fn apply(db: &Database, car: &str, package: PerformancePackage, physics: &mut CarPhysics) -> Result<()> {
    ensure!(package.junkman & 0x80 == 0, "invalid Junkman category");
    let options = PerformanceCatalog::read(db, car)?;
    let vehicle = catalog::vehicle(db, car)?;
    let mut next = physics.spec.clone();
    for category in Category::ALL {
        let level = package.get(category);
        let option = options.categories[category as usize];
        ensure!(level <= option.max_level, "{} level {level} exceeds {}", category.label(), option.max_level);
        let records = catalog::records(db, vehicle, category)?;
        let stock = records.first().copied().flatten();
        if stock.is_none() && level == 0 {
            ensure!(!package.has_junkman(category), "{} has no installed physics", category.label());
            continue;
        }
        let stock = stock.or_else(|| records.iter().find_map(|r| *r)).context("category has no physics records")?;
        let mut source = ResolvedFields { low: stock, high: None, fraction: 0.0, modifiers: Vec::new() };
        if level > 0 {
            let low = usize::from(records.len() == 3);
            let high = records[low + 1].context("upper upgrade endpoint is missing")?;
            source.low = high;
            if level != option.max_level {
                source.low = records[low].context("lower upgrade endpoint is missing")?;
                source.high = Some(high);
                source.fraction = level as f32 / option.max_level as f32;
            }
        }
        if package.has_junkman(category) {
            let present = match category {
                Category::Induction => source.f32("LOW_BOOST") > 0.0 || source.f32("HIGH_BOOST") > 0.0,
                Category::Nitrous => source.f32("NOS_CAPACITY") > 0.0 && source.f32("TORQUE_BOOST") > 0.0,
                _ => true,
            };
            ensure!(option.junkman && present, "{} Junkman is unavailable", category.label());
            source.modifiers = catalog::modifiers(vehicle, category)?;
        }
        match category {
            Category::Tires => next.tires = tires(source),
            Category::Brakes => next.brakes = brakes(source),
            Category::Chassis => {
                next.aero = aero(&source);
                next.chassis = chassis(source);
            }
            Category::Transmission => next.transmission = transmission(source),
            Category::Engine => next.engine = engine(source),
            Category::Induction => next.induction = induction(source),
            Category::Nitrous => next.nos = nos(source),
        }
    }
    physics.spec = next;
    Ok(())
}
