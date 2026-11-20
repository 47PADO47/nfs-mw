//! The numeric keys of a tyre (docs/specs/ac-handling/parameter-mapping.md section 5.1): one table of name, default and range
//! that generates the parameter struct, the TOML table that sets any of them, and the lookups by name that the class
//! multipliers and the car overrides need.

use serde::Deserialize;

/// The range of `axle_angle_bias_deg`, the one key that is not a plain tyre number (it splits the peak angle between the axles).
pub const BIAS_RANGE: (f64, f64) = (0.0, 8.0);

macro_rules! define_keys {
    ($( $name:ident = $default:expr, $lo:expr, $hi:expr; )*) => {
        /// Every tuning number of one tyre, with the default of a missing key.
        #[derive(Debug, Clone, PartialEq)]
        pub struct Keys {
            $(
                #[doc = concat!("`", stringify!($name), "` (range ", stringify!($lo), " to ", stringify!($hi), ").")]
                pub $name: f64,
            )*
        }

        impl Default for Keys {
            fn default() -> Self {
                Keys { $($name: $default,)* }
            }
        }

        impl Keys {
            /// The names of the keys, in the order of the table.
            pub const NAMES: &'static [&'static str] = &[$(stringify!($name)),*];

            /// The value of the key called `key`, or `None` for a name that is not a key.
            pub fn get(&self, key: &str) -> Option<f64> {
                match key {
                    $(stringify!($name) => Some(self.$name),)*
                    _ => None,
                }
            }

            /// Set the key called `key`; false for a name that is not a key.
            pub fn set(&mut self, key: &str, value: f64) -> bool {
                match key {
                    $(stringify!($name) => { self.$name = value; true })*
                    _ => false,
                }
            }

            /// The range the loader accepts for the key called `key`.
            pub fn range(key: &str) -> Option<(f64, f64)> {
                match key {
                    $(stringify!($name) => Some(($lo, $hi)),)*
                    _ => None,
                }
            }
        }

        /// A TOML table of keys: any subset of them, nothing else (an unknown key is an error).
        #[derive(Debug, Clone, Default, PartialEq, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct KeyTable {
            $(
                #[doc = concat!("`", stringify!($name), "`.")]
                pub $name: Option<f64>,
            )*
            /// `axle_angle_bias_deg`: only a compound preset's `[tyre]` table may carry it.
            pub axle_angle_bias_deg: Option<f64>,
        }

        impl KeyTable {
            /// The keys this table sets, as name and value (the axle bias is not among them: read it with the field).
            pub fn entries(&self) -> Vec<(&'static str, f64)> {
                let mut out = Vec::new();
                $(
                    if let Some(value) = self.$name {
                        out.push((stringify!($name), value));
                    }
                )*
                out
            }
        }
    };
}

define_keys! {
    mu_y0 = 1.20, 0.6, 2.2;
    mu_x_ratio = 1.10, 0.8, 1.4;
    exp_y = 0.85, 0.6, 1.0;
    exp_x = 0.88, 0.6, 1.0;
    load_factor_max = 1.6, 1.0, 3.0;
    peak_angle_deg = 8.0, 4.0, 14.0;
    flex = 0.20, 0.0, 0.6;
    cx_ratio = 1.4, 0.8, 3.0;
    fall_y = 0.85, 0.4, 1.0;
    fall_x = 0.82, 0.4, 1.0;
    q_y = 1.2, 0.1, 6.0;
    q_x = 1.4, 0.1, 6.0;
    comb_exp = 2.0, 1.2, 3.0;
    brake_gain = 1.0, 0.8, 1.3;
    speed_sens = 0.0012, 0.0, 0.02;
    camber_gain = 0.10, 0.0, 0.5;
    camber_d1 = 1.15, 0.0, 4.0;
    camber_d2 = 11.0, 0.0, 40.0;
    trail_ratio = 0.12, 0.05, 0.25;
    k_vert_per_width = 1.1e6, 0.5e6, 2.0e6;
    relax_len = 0.34, 0.05, 1.5;
    relax_slide_ratio = 0.5, 0.2, 1.0;
    relax_load_slope = 0.3, 0.0, 1.0;
    relax_tau_max = 0.08, 0.02, 0.3;
    v_low_x = 1.0, 0.2, 3.0;
    v_low_y = 0.4, 0.1, 2.0;
    rr_c0 = 0.012, 0.0, 0.05;
    rr_c2 = 1.5e-6, 0.0, 1.0e-5;
    air_drag_nms = 0.05, 0.0, 1.0;
    stick_cap = 0.5, 0.1, 1.0;
    inertia_scale = 0.3, 0.05, 2.0;
}

impl KeyTable {
    /// Check every number this table sets: finite and inside its range. The error names the key and the reason.
    pub fn validate(&self) -> Result<(), String> {
        for (key, value) in self.entries() {
            check(key, value, Keys::range(key))?;
        }
        let Some(bias) = self.axle_angle_bias_deg else {
            return Ok(());
        };
        check("axle_angle_bias_deg", bias, Some(BIAS_RANGE))
    }
}

/// One number against its range.
pub fn check(key: &str, value: f64, range: Option<(f64, f64)>) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("{key} is not a finite number"));
    }
    let Some((lo, hi)) = range else {
        return Ok(());
    };
    if value < lo || value > hi {
        return Err(format!("{key} = {value} is outside {lo} to {hi}"));
    }
    Ok(())
}
