//! The lean damping multiplier (`LeanComponent.LeanDampMultiplier`).
//!
//! The lean spring-damper of the rider has the damping ratio `|LeanDamp|` times a multiplier that the input handler works
//! out each tick from the size of the bike. Its structure is read from the handler's tick (function `0x1450e0480`, build
//! `5.6.1-1697004`); the meaning of two operands is inferred, as `docs/specs/vehicle-two-wheel-parameters.md` (section 1)
//! says.
//!
//! ```text
//! J     = roll inertia reference of the bike   = I_x_scale (M + m_r) + m_r z_r^2      [kg m^2]   (inferred operands)
//! a     = clamp01((J - W0 T0) / (W1 T1 - W0 T0))              weight range W, tensor-x range T of the component
//! ratio = h_com / wheelbase                                    both in the same unit
//! b     = clamp01((ratio - H0 / L0) / (H1 / L1 - H0 / L0))    height range H, wheelbase range L
//! multiplier = D0 + (D1 - D0) a b                              multiplier range D
//! ```
//!
//! `h_com` is the height of the combined centre of gravity at rest. The library keeps one number for the whole ride, so the
//! importer evaluates the multiplier at rest.

use anyhow::Result;

use super::node::Node;
use super::vehicle::Vehicle;

/// `x` mapped from `[from, to]` to `[0, 1]`, held at the ends; a range of no width is a step at `to`.
fn unit_ramp(x: f64, from: f64, to: f64) -> f64 {
    let width = to - from;
    if width.abs() <= f64::EPSILON {
        return f64::from(x >= to);
    }
    ((x - from) / width).clamp(0.0, 1.0)
}

pub fn damping_multiplier(vehicle: &Vehicle, handler: &Node<'_>, wheelbase_m: f64) -> Result<f64> {
    let component = handler.req("LeanComponent")?;
    let weight = component.xy_of("ReferenceWeightRange")?;
    let tensor = component.xy_of("ReferenceTensorXRange")?;
    let height = component.xy_of("ReferenceHeightRange")?;
    let base = component.xy_of("ReferenceWheelbaseRange")?;
    let range = component.xy_of("LeanDampMultiplierRange")?;

    let props = vehicle.props();
    let chassis = props.req("CHASSIS")?;
    let rider = handler.req("Rider")?;
    let (mass, rider_mass) = (chassis.f("Mass")?, rider.f("Mass")?);
    let scale_x = chassis.req("InertiaTensorScale")?.f("X")?;
    // Heights in centimetres, as the component's reference ranges are; the rider's arm in metres for the inertia.
    let (cg_cm, rider_cm) = (chassis.req("CGPosition")?.f("Z")?, rider.req("COMOffset")?.f("Z")?);
    let inertia = scale_x * (mass + rider_mass) + rider_mass * (rider_cm / 100.0).powi(2);
    let total = mass + rider_mass;
    let com_cm = (mass * cg_cm + rider_mass * (cg_cm + rider_cm)) / total;

    let a = unit_ramp(inertia, weight[0] * tensor[0], weight[1] * tensor[1]);
    let b = unit_ramp(com_cm / (wheelbase_m * 100.0), height[0] / base[0], height[1] / base[1]);
    Ok(range[0] + (range[1] - range[0]) * a * b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_light_bike_has_the_lower_end_of_the_range_and_a_heavy_long_low_one_the_upper_end() {
        let handler = |scale_x: f64, cg: f64| {
            json!({
                "Properties": {"CHASSIS": {"Mass": 113.0, "InertiaTensorScale": {"X": scale_x}, "CGPosition": {"Z": cg}}},
                "Handler": {
                    "Rider": {"Mass": 70.0, "COMOffset": {"Z": 40.0}},
                    "LeanComponent": {
                        "ReferenceWeightRange": {"X": 180.0, "Y": 352.0}, "ReferenceTensorXRange": {"X": 0.2, "Y": 0.3},
                        "ReferenceHeightRange": {"X": 80.0, "Y": 51.0}, "ReferenceWheelbaseRange": {"X": 131.0, "Y": 167.0},
                        "LeanDampMultiplierRange": {"X": 1.0, "Y": 1.5}
                    }
                }
            })
        };
        let multiplier = |scale_x: f64, cg: f64, wheelbase: f64| {
            let value = handler(scale_x, cg);
            let vehicle = Vehicle::parse(&json!({"Properties": value["Properties"]}).to_string(), None).unwrap();
            damping_multiplier(&vehicle, &Node::new(&value["Handler"], "h"), wheelbase).unwrap()
        };
        // Inertia under the lowest reference: the range starts, whatever the geometry.
        assert_eq!(multiplier(0.05, 40.0, 1.67), 1.0);
        // Inertia over the highest reference and a long, low bike (the height over the wheelbase under the last ratio): the
        // range ends.
        let top = multiplier(1.0, 30.0, 1.67);
        assert!((top - 1.5).abs() < 1e-12, "{top}");
        let middle = multiplier(0.2, 67.0, 1.45);
        assert!(middle > 1.0 && middle < 1.1, "{middle}");
    }
}
