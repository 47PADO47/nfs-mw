//! The parameters of one axle's tyre: the keys of [`crate::keys::Keys`] and the two numbers derived from them and the car.

use std::ops::Deref;

use crate::keys::Keys;

/// One axle's tyre, ready for [`crate::tyre_force::tyre_force`] and [`crate::wheel::wheel_tick`]. It derefs to its [`Keys`].
#[derive(Debug, Clone, PartialEq)]
pub struct TyreParams {
    keys: Keys,
    /// Vertical stiffness of the tyre, N/m: `k_vert_per_width` times the tyre width.
    pub k_vert: f64,
}

impl TyreParams {
    /// A tyre from its keys and its section width (m). A width that is not a usable number counts as 0.225 m.
    pub fn new(keys: Keys, tyre_width_m: f64) -> Self {
        let width = if tyre_width_m.is_finite() && tyre_width_m >= 0.05 { tyre_width_m } else { 0.225 };
        let k_vert = keys.k_vert_per_width * width;
        TyreParams { keys, k_vert }
    }

    /// Peak longitudinal friction at the reference load: `mu_y0 * mu_x_ratio`.
    pub fn mu_x0(&self) -> f64 {
        self.keys.mu_y0 * self.keys.mu_x_ratio
    }
}

impl Deref for TyreParams {
    type Target = Keys;

    fn deref(&self) -> &Keys {
        &self.keys
    }
}

impl Default for TyreParams {
    /// The defaults of the spec with a tyre 0.225 m wide: the `sport` tyre of the test vectors.
    fn default() -> Self {
        TyreParams::new(Keys::default(), 0.225)
    }
}
