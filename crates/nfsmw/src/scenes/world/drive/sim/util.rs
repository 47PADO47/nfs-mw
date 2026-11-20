//! Two small helpers of the car simulation.

use glam::Vec3;

/// A vector from physics space to render space axes: `(x, y, z) -> (z, -x, y)`.
pub(super) fn to_model_axes(v: Vec3) -> Vec3 {
    Vec3::new(v.z, -v.x, v.y)
}

/// A sound identity no other source uses: the top bit marks a car's, the rest counts up.
pub(super) fn next_sound_identity() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    (1 << 63) | NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
