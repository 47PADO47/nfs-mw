use super::{PerformanceCategory as Category, PerformancePackage};
use anyhow::{Context, Result, ensure};
use blackbox_attrib::{CollectionRef, Database, Value, vlt_hash};

#[derive(Clone, Copy, Debug, Default)]
pub struct CategoryOptions {
    pub max_level: u8,
    pub missing_endpoints: bool,
    pub junkman: bool,
    /// Induction/nitrous must be present before their Junkman checkbox can be enabled.
    pub stock_present: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PerformanceCatalog {
    pub categories: [CategoryOptions; 7],
}

impl PerformanceCatalog {
    pub fn read(db: &Database, car: &str) -> Result<Self> {
        let vehicle = vehicle(db, car)?;
        let mut categories = [CategoryOptions::default(); 7];
        for category in Category::ALL {
            let field = category.field();
            let max = vehicle.get_i32(&format!("{field}_upgrades")).unwrap_or(0);
            ensure!((0..=u8::MAX as i32).contains(&max), "invalid {field} upgrade count");
            let records = records(db, vehicle, category)?;
            let stock = records.first().copied().flatten();
            let lower = usize::from(records.len() == 3);
            let endpoints =
                records.get(lower).is_some_and(Option::is_some) && records.get(lower + 1).is_some_and(Option::is_some);
            let stock_present = match category {
                Category::Induction => stock.is_some_and(|c| {
                    c.get_f32("LOW_BOOST").unwrap_or(0.0) > 0.0 || c.get_f32("HIGH_BOOST").unwrap_or(0.0) > 0.0
                }),
                Category::Nitrous => stock.is_some_and(|c| {
                    c.get_f32("NOS_CAPACITY").unwrap_or(0.0) > 0.0 && c.get_f32("TORQUE_BOOST").unwrap_or(0.0) > 0.0
                }),
                _ => stock.is_some(),
            };
            categories[category as usize] = CategoryOptions {
                max_level: match endpoints {
                    true => max as u8,
                    false => 0,
                },
                missing_endpoints: max > 0 && !endpoints,
                junkman: !modifiers(vehicle, category)?.is_empty() && records.iter().any(Option::is_some),
                stock_present,
            };
        }
        Ok(Self { categories })
    }

    pub fn maximum(&self) -> PerformancePackage {
        PerformancePackage { levels: self.categories.map(|c| c.max_level), junkman: 0 }
    }

    pub fn junkman_available(&self, category: Category, package: PerformancePackage) -> bool {
        let option = self.categories[category as usize];
        option.junkman && (option.stock_present || package.get(category) > 0)
    }
}

pub(super) fn vehicle<'a>(db: &'a Database, car: &str) -> Result<CollectionRef<'a>> {
    db.collection("pvehicle", &car.to_ascii_lowercase()).with_context(|| format!("no vehicle record for {car}"))
}

pub(super) fn records<'a>(
    db: &'a Database,
    vehicle: CollectionRef<'a>,
    category: Category,
) -> Result<Vec<Option<CollectionRef<'a>>>> {
    let Some(value) = vehicle.get(category.field()) else { return Ok(Vec::new()) };
    let values = match value {
        Value::Array(items) => items.as_slice(),
        one => std::slice::from_ref(one),
    };
    Ok(values.iter().map(|v| v.as_ref_spec().and_then(|r| db.resolve(r))).collect())
}

pub(super) fn modifiers(vehicle: CollectionRef<'_>, category: Category) -> Result<Vec<(u32, f32)>> {
    let Some(junkman) = vehicle.follow("junkman") else { return Ok(Vec::new()) };
    let Some(value) = junkman.get(&format!("{}_package", category.field())) else { return Ok(Vec::new()) };
    let values = match value {
        Value::Array(items) => items.as_slice(),
        one => std::slice::from_ref(one),
    };
    values
        .iter()
        .map(|value| {
            let Value::Raw { type_key, bytes } = value else { anyhow::bail!("invalid Junkman modifier type") };
            ensure!(*type_key == vlt_hash("JunkmanMod") && bytes.len() == 12, "invalid Junkman modifier layout");
            let field = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
            let scale = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
            ensure!(scale.is_finite(), "nonfinite Junkman scale");
            Ok((field, scale))
        })
        .collect()
}
