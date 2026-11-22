//! The numeric keys of the model (docs/specs/ac-handling/parameter-mapping.md section 5.1): one table of name, default and range
//! per key set that generates the parameter struct, the TOML table that sets any of them, and the lookups by name that the class
//! multipliers and the per-car overrides need. Three sets use it: the tyre ([`Keys`]), the suspension of one axle
//! ([`SuspensionKeys`], suspension.md section 5) and the steering ([`SteeringKeys`], steering.md section 5).

use serde::Deserialize;

/// The range of `axle_angle_bias_deg`, the one key that is not a plain tyre number (it splits the peak angle between the axles).
pub const BIAS_RANGE: (f64, f64) = (0.0, 8.0);

/// Define a key set: the struct of `f64` values, its defaults, the by-name lookups, and the TOML table of optional values.
/// `extra` names fields of the table that are not keys of the struct (the axle bias).
macro_rules! define_keys {
    (
        $keys:ident, $table:ident, [$($extra:ident),*];
        $( $name:ident = $default:expr, $lo:expr, $hi:expr; )*
    ) => {
        /// Every tuning number of one set, with the default of a missing key.
        #[derive(Debug, Clone, PartialEq)]
        pub struct $keys {
            $(
                #[doc = concat!("`", stringify!($name), "` (range ", stringify!($lo), " to ", stringify!($hi), ").")]
                pub $name: f64,
            )*
        }

        impl Default for $keys {
            fn default() -> Self {
                $keys { $($name: $default,)* }
            }
        }

        impl $keys {
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

            /// Keep every key inside its range.
            pub fn clamp_ranges(&mut self) {
                $( self.$name = self.$name.clamp($lo, $hi); )*
            }

            /// Set every key a table names.
            pub fn apply(&mut self, table: &$table) {
                $( if let Some(value) = table.$name { self.$name = value; } )*
            }
        }

        /// A TOML table of keys: any subset of them, nothing else (an unknown key is an error).
        #[derive(Debug, Clone, Default, PartialEq, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $table {
            $(
                #[doc = concat!("`", stringify!($name), "`.")]
                pub $name: Option<f64>,
            )*
            $(
                #[doc = concat!("`", stringify!($extra), "`: not a key of the set; the table that allows it says so.")]
                pub $extra: Option<f64>,
            )*
        }

        impl $table {
            /// The keys this table sets, as name and value (the extra fields are not among them: read them directly).
            pub fn entries(&self) -> Vec<(&'static str, f64)> {
                let mut out = Vec::new();
                $(
                    if let Some(value) = self.$name {
                        out.push((stringify!($name), value));
                    }
                )*
                out
            }

            /// Check every key this table sets: finite and inside its range. The error names the key and the reason.
            pub fn validate_keys(&self) -> Result<(), String> {
                for (key, value) in self.entries() {
                    check(key, value, $keys::range(key))?;
                }
                Ok(())
            }
        }
    };
}

define_keys! {
    Keys, KeyTable, [axle_angle_bias_deg];
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
    inertia_scale = 1.0, 0.05, 2.0;
}

// The suspension of one axle: the front defaults of suspension.md section 5 (the rear differs in `SuspensionKeys::rear`).
define_keys! {
    SuspensionKeys, SuspensionTable, [];
    m_u = 38.0, 15.0, 60.0;
    k_w = 30000.0, 10000.0, 200000.0;
    k2 = 0.0, 0.0, 5.0e6;
    k_t = 250000.0, 150000.0, 350000.0;
    c_t = 200.0, 0.0, 800.0;
    c_bs = 1900.0, 200.0, 12000.0;
    c_bf = 700.0, 200.0, 12000.0;
    c_rs = 3800.0, 200.0, 16000.0;
    c_rf = 1400.0, 200.0, 16000.0;
    v_kb = 0.15, 0.05, 0.5;
    v_kr = 0.20, 0.05, 0.5;
    x_up = 0.060, 0.01, 0.15;
    x_dn = 0.070, 0.01, 0.15;
    k_bs = 500000.0, 1.0e5, 2.0e6;
    k_arb = 20000.0, 0.0, 80000.0;
    g_t = 0.40, 0.0, 1.0;
    gamma0 = -0.035, -0.10, 0.05;
    kappa = 0.10, 0.0, 0.3;
    h_rc = 0.06, -0.1, 0.4;
}

impl SuspensionKeys {
    /// The rear axle's defaults (suspension.md section 5).
    pub fn rear() -> SuspensionKeys {
        SuspensionKeys {
            k_w: 28000.0,
            c_bs: 1600.0,
            c_bf: 600.0,
            c_rs: 3200.0,
            c_rf: 1200.0,
            k_arb: 12000.0,
            g_t: 0.30,
            gamma0: -0.026,
            kappa: 0.0,
            h_rc: 0.12,
            ..SuspensionKeys::default()
        }
    }
}

// The steering (steering.md section 5): angles in degrees here, the model converts.
define_keys! {
    SteeringKeys, SteeringTable, [];
    theta_max_deg = 450.0, 180.0, 720.0;
    delta_max_deg = 30.0, 20.0, 45.0;
    s_sens = 1.0, 0.0, 1.0;
    f_min = 0.20, 0.1, 0.5;
    v0 = 22.0, 10.0, 40.0;
    gamma = 1.3, 1.0, 2.0;
    r_hi = 3.0, 0.5, 6.0;
    r_lo = 0.9, 0.5, 6.0;
    v_r = 15.0, 5.0, 40.0;
    rho_ret = 1.6, 1.0, 3.0;
    c_cs = 1.0, 0.0, 2.0;
    tau_f = 0.0, 0.0, 0.1;
    tau_ref = 12.0, 3.0, 40.0;
    tau_ff = 0.02, 0.005, 0.1;
    g_gyro = 0.2, 0.0, 1.0;
    j_w = 1.6, 0.5, 4.0;
    v_ls = 2.8, 0.0, 8.0;
    m_ls = 0.2, 0.0, 1.0;
    g_ffb = 1.0, 0.0, 3.0;
    p_curve = 1.0, 0.4, 1.5;
}

impl KeyTable {
    /// Check every number this table sets: finite and inside its range, the axle bias included.
    pub fn validate(&self) -> Result<(), String> {
        self.validate_keys()?;
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
