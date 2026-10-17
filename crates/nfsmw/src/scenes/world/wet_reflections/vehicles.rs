//! Attached vehicle geometry uses its own reflected root, never a common road plane.

use blackbox_gfx::Instance;
use glam::Mat4;

/// Preserve each part's local pose and stable key while changing only its vehicle root.
/// The caller appends attached parts only; detached fragments have independent world poses.
pub(in crate::scenes::world) fn append(root: Mat4, out: &mut Vec<Instance>, draw: impl FnOnce(&mut Vec<Instance>)) {
    if !root.is_finite() || root.determinant().abs() < 1e-6 {
        return;
    }
    let reflected = super::geometry::vehicle_root(root);
    let delta = reflected * root.inverse();
    if !delta.is_finite() {
        return;
    }
    let start = out.len();
    draw(out);
    for instance in &mut out[start..] {
        instance.transform = delta * instance.transform;
    }
}

#[cfg(test)]
mod tests;
