//! The TOP SPEED, HANDLING and ACCELERATION bars that `MyCarsManager.fng`, `Car_Select.fng` and `Quick_Race_Brief.fng`
//! share (`docs/specs/garage-performance-bars.md`). Each axis has a dim base strip that keeps its full authored width
//! (`BASE_<axis>`, 160 units), the lit strip over it (`FILLBAR_<axis>`) and a yellow tick (`HANDLE_<axis>`). A rating of
//! one fills the 160 units from the base's left edge; the tick's left edge marks a second rating, absolute on the same
//! scale.

use blackbox_feng::{PackageId, Runtime, fe_hash_upper};

/// The axes in the ratings' order (`[top speed, handling, acceleration]`), as the objects name them.
const AXES: [&str; 3] = ["TOPSPEED", "HANDLING", "ACCELERATION"];
/// The width a rating of one fills.
const FULL_WIDTH: f32 = 160.0;
/// The strips' units per texture unit: `LED_main.tga` repeats across every 32 units.
const UNITS_PER_UV: f32 = 32.0;

/// Draws the bars of `package`: `filled` lights each axis's strip, `marked` places each tick. `None` hides the lit strips or
/// the ticks; the bases stay. A package without an axis's base leaves that axis alone.
pub(crate) fn apply(rt: &mut Runtime, package: PackageId, filled: Option<[f32; 3]>, marked: Option<[f32; 3]>) {
    for (index, name) in AXES.into_iter().enumerate() {
        let Some(base) = rt.find(package, fe_hash_upper(&format!("BASE_{name}"))) else { continue };
        let Some(position) = rt.position(base) else { continue };
        let Some(size) = rt.size(base) else { continue };
        let left = position.x - size.x / 2.0;
        let top = position.y - size.y / 2.0;
        if let Some(fill) = rt.find(package, fe_hash_upper(&format!("FILLBAR_{name}"))) {
            let fraction = filled.map(|values| values[index].clamp(0.0, 1.0)).unwrap_or(0.0);
            let width = FULL_WIDTH * fraction;
            let height = rt.size(fill).map_or(size.y, |size| size.y);
            rt.set_hidden(fill, filled.is_none() || width <= 0.0);
            rt.set_size_xy(fill, width, height);
            // Native equal-depth siblings put the dim base after the lit strip. Keep the fill in front.
            let depth = rt.position(fill).map_or(position.z - 1.0, |value| value.z.min(position.z - 1.0));
            rt.set_position(fill, glam::Vec3::new(left + width / 2.0, top + height / 2.0, depth));
            rt.set_uv(fill, [0.0, 0.0, width / UNITS_PER_UV, 1.0]);
        }
        let Some(handle) = rt.find(package, fe_hash_upper(&format!("HANDLE_{name}"))) else { continue };
        rt.set_hidden(handle, marked.is_none());
        let Some(values) = marked else { continue };
        let handle_size = rt.size(handle).unwrap_or(glam::Vec3::splat(32.0));
        rt.set_position_xy(
            handle,
            left + FULL_WIDTH * values[index].clamp(0.0, 1.0) + handle_size.x / 2.0,
            top + handle_size.y / 2.0,
        );
    }
}

#[cfg(test)]
#[path = "stat_bars_tests.rs"]
mod tests;
