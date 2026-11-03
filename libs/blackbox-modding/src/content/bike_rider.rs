//! The `[bike.rider]` table: where the rider sits as the weight, the lean and the speed change, and how his body is held to the
//! bike (`blackbox_vehicle::bike::RiderSpec`, `docs/specs/ride6-rider-body-aero.md`). Every key is optional; a manifest without
//! the table has a rider who sits where `[bike.chassis]` puts him and only shifts along the bike with the weight.

use blackbox_vehicle::bike::RiderSpec;
use serde::Deserialize;

use super::ContentError;
use super::bike_check::{all_ranged, ranged};

/// The rider's placement and body dynamics (`[bike.rider]`). Offsets in m (x right, y up, z forward), speeds in m/s.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Rider {
    /// Added to the rest place whatever the inputs are.
    pub base_offset_m: [f32; 3],
    /// The shift at full lean: x toward the side of the lean (signed with it), y and z for either side.
    pub lateral_offset_m: [f32; 3],
    /// The shifts that replace the moving ones at a standstill, and the speeds over which they give way.
    pub stopped_forward_offset_m: [f32; 3],
    pub stopped_backward_offset_m: [f32; 3],
    pub stopped_lateral_offset_m: [f32; 3],
    pub stopped_speed_range_mps: [f32; 2],
    /// How much the lateral weight input and the normalised target lean ask for the lateral shift.
    pub weight_influence: f32,
    pub lean_influence: f32,
    /// The damper of the rider's translation: its multiplier at the low end of the speed range, 1 at the high end.
    pub damp_speed_range_mps: [f32; 2],
    pub damp_multiplier: f32,
    /// Whether the spring's rest place is raised so the rider does not sag under his weight.
    pub correct_gravity: bool,
    /// The rider's moments of inertia as a multiple of his mass (m^2; x pitch, y yaw, z roll), the angular spring (N m/rad) and
    /// damper (N m s/rad) that hold his body to the bike's attitude, the share of the bike's angular velocity the damper works
    /// against, and the per-axis scale of the reaction torque on the bike. A spring of 0 (the default) leaves him rigid.
    pub inertia_scale: [f32; 3],
    pub angular_k_n_m_per_rad: f32,
    pub angular_damp: f32,
    pub angular_damp_chassis_factor: f32,
    pub torque_scale: [f32; 3],
    /// The air drag of the rider himself, as a coefficient like the bike's (`F = c v^2`): the curve over his longitudinal weight
    /// (`[weight -1..1, coefficient]` points; none: no drag of his own), times the curve over the tuck (`[tuck 0..1, factor]`
    /// points; none: 1), times the scale.
    pub drag_scale: f32,
    pub drag_curve: Vec<[f32; 2]>,
    pub drag_weight_curve: Vec<[f32; 2]>,
}

impl Default for Rider {
    fn default() -> Self {
        let spec = RiderSpec::default();
        Self {
            base_offset_m: spec.base_offset.into(),
            lateral_offset_m: spec.lateral_offset.into(),
            stopped_forward_offset_m: spec.stopped_forward_offset.into(),
            stopped_backward_offset_m: spec.stopped_backward_offset.into(),
            stopped_lateral_offset_m: spec.stopped_lateral_offset.into(),
            stopped_speed_range_mps: spec.stopped_speed_range,
            weight_influence: spec.weight_influence,
            lean_influence: spec.lean_influence,
            damp_speed_range_mps: spec.damp_speed_range,
            damp_multiplier: spec.damp_multiplier,
            correct_gravity: spec.correct_gravity,
            inertia_scale: spec.inertia_scale.into(),
            angular_k_n_m_per_rad: spec.angular_k,
            angular_damp: spec.angular_damp,
            angular_damp_chassis_factor: spec.angular_damp_chassis_factor,
            torque_scale: spec.torque_scale.into(),
            drag_scale: spec.drag_scale,
            drag_curve: Vec::new(),
            drag_weight_curve: Vec::new(),
        }
    }
}

impl Rider {
    /// The table as the dynamics take it.
    pub fn to_spec(&self) -> RiderSpec {
        RiderSpec {
            base_offset: self.base_offset_m.into(),
            lateral_offset: self.lateral_offset_m.into(),
            stopped_forward_offset: self.stopped_forward_offset_m.into(),
            stopped_backward_offset: self.stopped_backward_offset_m.into(),
            stopped_lateral_offset: self.stopped_lateral_offset_m.into(),
            stopped_speed_range: self.stopped_speed_range_mps,
            weight_influence: self.weight_influence,
            lean_influence: self.lean_influence,
            damp_speed_range: self.damp_speed_range_mps,
            damp_multiplier: self.damp_multiplier,
            correct_gravity: self.correct_gravity,
            inertia_scale: self.inertia_scale.into(),
            angular_k: self.angular_k_n_m_per_rad,
            angular_damp: self.angular_damp,
            angular_damp_chassis_factor: self.angular_damp_chassis_factor,
            torque_scale: self.torque_scale.into(),
            drag_scale: self.drag_scale,
            drag_curve: self.drag_curve.iter().map(|p| (p[0], p[1])).collect(),
            drag_weight_curve: self.drag_weight_curve.iter().map(|p| (p[0], p[1])).collect(),
        }
    }

    /// Refuses a number outside the range a typing mistake would leave it.
    pub(super) fn check(&self) -> Result<(), ContentError> {
        all_ranged(&self.base_offset_m, "bike.rider.base_offset_m", -1.0, 1.0)?;
        all_ranged(&self.lateral_offset_m, "bike.rider.lateral_offset_m", -1.0, 1.0)?;
        all_ranged(&self.stopped_forward_offset_m, "bike.rider.stopped_forward_offset_m", -1.0, 1.0)?;
        all_ranged(&self.stopped_backward_offset_m, "bike.rider.stopped_backward_offset_m", -1.0, 1.0)?;
        all_ranged(&self.stopped_lateral_offset_m, "bike.rider.stopped_lateral_offset_m", -1.0, 1.0)?;
        all_ranged(&self.stopped_speed_range_mps, "bike.rider.stopped_speed_range_mps", 0.0, 100.0)?;
        ranged(self.weight_influence, "bike.rider.weight_influence", -10.0, 10.0)?;
        ranged(self.lean_influence, "bike.rider.lean_influence", -10.0, 10.0)?;
        all_ranged(&self.damp_speed_range_mps, "bike.rider.damp_speed_range_mps", 0.0, 200.0)?;
        ranged(self.damp_multiplier, "bike.rider.damp_multiplier", 0.0, 100.0)?;
        all_ranged(&self.inertia_scale, "bike.rider.inertia_scale", 0.001, 10.0)?;
        ranged(self.angular_k_n_m_per_rad, "bike.rider.angular_k_n_m_per_rad", 0.0, 1.0e7)?;
        ranged(self.angular_damp, "bike.rider.angular_damp", 0.0, 1.0e6)?;
        ranged(self.angular_damp_chassis_factor, "bike.rider.angular_damp_chassis_factor", 0.0, 10.0)?;
        all_ranged(&self.torque_scale, "bike.rider.torque_scale", 0.0, 100.0)?;
        ranged(self.drag_scale, "bike.rider.drag_scale", 0.0, 10.0)?;
        for point in &self.drag_curve {
            ranged(point[0], "bike.rider.drag_curve", 0.0, 1.0)?;
            ranged(point[1], "bike.rider.drag_curve", 0.0, 10.0)?;
        }
        for point in &self.drag_weight_curve {
            ranged(point[0], "bike.rider.drag_weight_curve", -1.0, 1.0)?;
            ranged(point[1], "bike.rider.drag_weight_curve", 0.0, 10.0)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_with_nothing_in_it_is_the_default_rider() {
        let table: Rider = toml::from_str("").unwrap();
        assert_eq!(table.to_spec(), RiderSpec::default());
        table.check().unwrap();
    }

    #[test]
    fn the_keys_become_the_spec_and_an_unknown_key_is_refused() {
        let table: Rider = toml::from_str(
            "lateral_offset_m = [-0.12, 0.0, 0.0]\nlean_influence = 1.0\nweight_influence = 0.0\ncorrect_gravity = true\n\
             angular_k_n_m_per_rad = 2500.0\nangular_damp = 250.0\nangular_damp_chassis_factor = 0.0\n\
             drag_scale = 1.25\ndrag_curve = [[0.0, 1.2], [1.0, 0.65]]\ndrag_weight_curve = [[0.0, 0.1]]\nstopped_speed_range_mps = [2.0, 6.0]",
        )
        .unwrap();
        let spec = table.to_spec();
        assert_eq!(spec.lateral_offset.x, -0.12);
        assert_eq!((spec.lean_influence, spec.weight_influence), (1.0, 0.0));
        assert!(spec.correct_gravity);
        assert_eq!((spec.angular_k, spec.angular_damp, spec.angular_damp_chassis_factor), (2500.0, 250.0, 0.0));
        assert_eq!(spec.drag_curve, vec![(0.0, 1.2), (1.0, 0.65)]);
        assert_eq!(spec.drag_weight_curve, vec![(0.0, 0.1)]);
        assert_eq!(spec.stopped_speed_range, [2.0, 6.0]);
        table.check().unwrap();
        assert!(toml::from_str::<Rider>("lateral_offset = [0.0, 0.0, 0.0]").is_err());
    }

    #[test]
    fn a_number_out_of_range_names_its_key() {
        let table: Rider = toml::from_str("lateral_offset_m = [-12.0, 0.0, 0.0]").unwrap();
        assert!(table.check().unwrap_err().to_string().contains("bike.rider.lateral_offset_m"));
        let table: Rider = toml::from_str("drag_curve = [[2.0, 1.0]]").unwrap();
        assert!(table.check().is_err());
        let table: Rider = toml::from_str("inertia_scale = [0.0, 0.2, 0.2]").unwrap();
        assert!(table.check().is_err());
        let table: Rider = toml::from_str("drag_weight_curve = [[3.0, 0.1]]").unwrap();
        assert!(table.check().is_err());
    }
}
