//! The stat bars of the car card of `Car_Select.fng` (the car lot, the career's car select and the custom race's): the
//! original front end's ratings of the shown car (`docs/specs/garage-performance-bars.md`), drawn as the garage draws them
//! (`frontend::stat_bars`).

use nfsmw_career::stable::Stable;
use nfsmw_data::car::physics::PerformancePackage;

use crate::frontend::logic::Cx;
use crate::frontend::quick_play::pool::{CarPool, PoolCar};

/// The ratings of a pool car built with `package`, `[top speed, handling, acceleration]`. `None` for a car the rating book
/// does not know.
pub(crate) fn ratings(pool: &CarPool, car: &PoolCar, package: PerformancePackage) -> Option<[f32; 3]> {
    Some(pool.ratings_of(car)?.rating(package))
}

/// The performance package installed on the stable's car with this handle: its saved upgrade levels and Junkman parts, or
/// none for a car with no customization record.
pub(crate) fn installed_package(stable: &Stable, handle: u32) -> PerformancePackage {
    let Some(record) = stable.customization_for_handle(handle) else { return PerformancePackage::default() };
    PerformancePackage {
        levels: record.performance().map(|level| level.clamp(0, i32::from(u8::MAX)) as u8),
        junkman: (record.junkman() & 0x7f) as u8,
    }
}

/// Draws the three bars of the shown car. The tick marks the same rating as the lit strip: the card shows one car, which
/// the original's car selects compare with itself (its quick race car select puts the tick at the end of the lit strip).
/// `None` (no car, or one with no ratings) leaves only the dim bases.
pub(crate) fn set_bars(cx: &mut Cx, ratings: Option<[f32; 3]>) {
    crate::frontend::stat_bars::apply(cx.rt, cx.package, ratings, ratings);
}
