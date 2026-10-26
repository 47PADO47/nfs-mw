//! A car's base stats: the numbers of its handling definition. Spec: `docs/formats/chameleon-vehicle-stats.md`.
//!
//! Every value is read from the Genesys objects of the car's own bundle at a fixed offset of a named structure. The
//! meaning of each offset was inferred from the data (field names are hashed), so the doc tags each one verified or
//! unconfirmed. Units are the file's: metres, kilograms, rpm, newton metres, miles per hour. Axes are the file's:
//! x left, y up, z forward.

use chameleon_bundle::Bundle;

use crate::error::{Error, Result};
use crate::genesys::{Object, TypeNames, find_object};

/// A knot of the engine's torque curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorqueKnot {
    pub rpm: f32,
    pub torque_nm: f32,
    /// Slope (N m per rpm) the curve leaves the knot with, from its out tangent.
    pub out_slope: f32,
    /// Slope the curve arrives with, from its in tangent.
    pub in_slope: f32,
}

/// One wheel: where its rigid body sits and the cylinder that stands for the tyre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelStats {
    pub position: [f32; 3],
    pub radius: f32,
    pub half_width: f32,
    pub mass_kg: f32,
}

/// The base stats of one car.
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleStats {
    /// Mass of the body rigid body, without the wheels.
    pub body_mass_kg: f32,
    /// Where the body's centre of gravity is, relative to the car's origin.
    pub centre_of_mass: [f32; 3],
    /// Half sizes of the body box (x, y, z) and its centre.
    pub box_half_extents: [f32; 3],
    pub box_centre: [f32; 3],
    /// z of the front and the rear axle, from the vehicle info.
    pub axle_z: [f32; 2],
    /// The wheels, front pair first (left before right inside a pair).
    pub wheels: [WheelStats; 4],
    pub torque_curve: Vec<TorqueKnot>,
    /// Stock gearbox: reverse ratio (a magnitude), the forward ratios from first, the final drive.
    pub reverse_ratio: f32,
    pub gear_ratios: Vec<f32>,
    pub final_drive: f32,
    /// The four numbers of the differential structure.
    pub differentials: [f32; 4],
    /// Brake torque per wheel in N m: front, rear.
    pub brake_torque_nm: [f32; 2],
    /// Peak longitudinal grip of the front and rear tyre (a friction coefficient).
    pub tire_grip: [f32; 2],
    /// The car's design top speed and its top speed with nitrous, in mph.
    pub top_speed_mph: f32,
    pub boosted_top_speed_mph: f32,
    /// Nitrous strength: the torque multiplier while it burns.
    pub nitrous_strength: f32,
}

impl VehicleStats {
    /// Engine speed of the lowest knot: the lowest speed the curve defines.
    pub fn lowest_rpm(&self) -> f32 {
        self.torque_curve.first().map_or(0.0, |knot| knot.rpm)
    }

    /// Engine speed of the highest knot: where the engine stops turning.
    pub fn redline_rpm(&self) -> f32 {
        self.torque_curve.last().map_or(0.0, |knot| knot.rpm)
    }

    /// Torque in N m at `rpm`: a cubic Hermite spline through the knots, flat outside them.
    pub fn torque_at(&self, rpm: f32) -> f32 {
        let knots = &self.torque_curve;
        let (Some(first), Some(last)) = (knots.first(), knots.last()) else { return 0.0 };
        if rpm <= first.rpm {
            return first.torque_nm;
        }
        if rpm >= last.rpm {
            return last.torque_nm;
        }
        let Some(pair) = knots.windows(2).find(|pair| rpm <= pair[1].rpm) else { return last.torque_nm };
        let (a, b) = (pair[0], pair[1]);
        let width = b.rpm - a.rpm;
        if width <= f32::EPSILON {
            return b.torque_nm;
        }
        let t = (rpm - a.rpm) / width;
        let (t2, t3) = (t * t, t * t * t);
        let (m0, m1) = (a.out_slope * width, b.in_slope * width);
        (2.0 * t3 - 3.0 * t2 + 1.0) * a.torque_nm
            + (t3 - 2.0 * t2 + t) * m0
            + (-2.0 * t3 + 3.0 * t2) * b.torque_nm
            + (t3 - t2) * m1
    }
}

/// Read the stats out of a car bundle (`VEH_<n>_HI.BNDL`).
pub fn read_stats(bundle: &Bundle) -> Result<VehicleStats> {
    let types = TypeNames::read(bundle)?;
    let handling = find_object(bundle, &types, "Genesys.Gen.HandlingBehaviour")?;
    let physical = find_object(bundle, &types, "Genesys.Gen.RaceCarPhysicalDefinition")?;
    let info = find_object(bundle, &types, "Genesys.Gen.VehicleInfo")?;
    let (body, wheels) = rigid_bodies(&physical)?;
    let definition = at(&physical, "Genesys.Gen.RaceCarPhysicalDefinition.PhysicalDefinition", 0)?;
    let (gear_ratios, reverse_ratio, final_drive) = gearbox(&handling)?;
    let aero = at(&handling, "Genesys.Gen.AerodynamicBehaviour", 0)?;
    let tyres: Vec<usize> = handling.all("Genesys.Gen.Tyre").collect();
    let grips: Vec<usize> = handling.all("Genesys.Gen.Tyre.Long_Grip_Curve").collect();
    let tire_grip = [tire_grip(&handling, &tyres, &grips, 0)?, tire_grip(&handling, &tyres, &grips, 1)?];
    let brakes = at(&handling, "Genesys.Gen.BrakeBehaviour", 0)?;
    let differentials = handling.floats::<4>(at(&handling, "Genesys.Gen.Engine.Differentials", 0)? + 0xC)?;
    let stats = VehicleStats {
        body_mass_kg: physical.f32(body + 0xA8)?,
        centre_of_mass: physical.floats::<3>(body + 0x40)?,
        box_half_extents: physical.floats::<3>(definition + 0x20)?,
        box_centre: physical.floats::<3>(definition + 0x10)?,
        axle_z: axles(&info)?,
        wheels: sorted_wheels(&physical, &wheels)?,
        torque_curve: torque_curve(&handling)?,
        reverse_ratio,
        gear_ratios,
        final_drive,
        differentials,
        brake_torque_nm: [handling.f32(brakes + 0x10)?, handling.f32(brakes + 0x14)?],
        tire_grip,
        top_speed_mph: handling.f32(aero + 0x60)?,
        boosted_top_speed_mph: handling.f32(aero + 0x44)?,
        nitrous_strength: handling.f32(at(&handling, "Genesys.Gen.NitrousStrength", 0)? + 0x3C)?,
    };
    check(&stats)?;
    Ok(stats)
}

fn at(object: &Object, name: &str, nth: usize) -> Result<usize> {
    object.at(name, nth).ok_or_else(|| Error::Spec(format!("no {name} in the object")))
}

/// The body rigid body (the heaviest) and the four wheel bodies in file order.
fn rigid_bodies(physical: &Object) -> Result<(usize, Vec<usize>)> {
    let name = "Genesys.Gen.RaceCarPhysicalDefinition.PhysicalDefinition.RigidBody";
    let bodies: Vec<usize> = physical.all(name).collect();
    let mut heaviest = None;
    for &body in &bodies {
        let mass = physical.f32(body + 0xA8)?;
        if heaviest.is_none_or(|(_, best)| mass > best) {
            heaviest = Some((body, mass));
        }
    }
    let (body, _) = heaviest.ok_or_else(|| Error::Spec("no rigid body".into()))?;
    let wheels: Vec<usize> = bodies.into_iter().filter(|&other| other != body).collect();
    if wheels.len() != 4 {
        return Err(Error::Spec(format!("{} wheel rigid bodies, expected 4", wheels.len())));
    }
    Ok((body, wheels))
}

/// The wheels with the radius of their cylinder, the front pair (larger z) first and left (positive x) first.
fn sorted_wheels(physical: &Object, bodies: &[usize]) -> Result<[WheelStats; 4]> {
    let cylinders: Vec<usize> =
        physical.all("Genesys.Gen.RaceCarPhysicalDefinition.PhysicalDefinition.RigidBody.CylinderVolume").collect();
    if cylinders.len() != 4 {
        return Err(Error::Spec(format!("{} wheel cylinders, expected 4", cylinders.len())));
    }
    let mut wheels = Vec::with_capacity(4);
    for (&body, &cylinder) in bodies.iter().zip(&cylinders) {
        wheels.push(WheelStats {
            position: physical.floats::<3>(body + 0x40)?,
            radius: physical.f32(cylinder + 0x58)?,
            half_width: physical.f32(cylinder + 0x54)?,
            mass_kg: physical.f32(body + 0xA8)?,
        });
    }
    wheels.sort_by(|a, b| b.position[2].total_cmp(&a.position[2]));
    let (front, rear) = wheels.split_at_mut(2);
    front.sort_by(|a, b| b.position[0].total_cmp(&a.position[0]));
    rear.sort_by(|a, b| b.position[0].total_cmp(&a.position[0]));
    wheels.try_into().map_err(|_| Error::Spec("wheels".into()))
}

/// z of the front and rear axle: the two axles of the vehicle info, the larger z first.
fn axles(info: &Object) -> Result<[f32; 2]> {
    let zs: Vec<f32> =
        info.all("Genesys.Gen.VehicleInfo.Axle").map(|axle| info.f32(axle + 0x20)).collect::<Result<_>>()?;
    let [a, b] = zs[..] else { return Err(Error::Spec(format!("{} axles, expected 2", zs.len()))) };
    Ok([a.max(b), a.min(b)])
}

/// The knots of the power curve, through the pointer table the curve structure holds.
fn torque_curve(handling: &Object) -> Result<Vec<TorqueKnot>> {
    let curve = at(handling, "Genesys.Gen.Engine.Power_curve", 0)?;
    let table = handling.u32(curve + 0x4C)? as usize;
    let count = usize::from(*handling.main.get(curve + 0x50).ok_or_else(|| Error::Spec("power curve".into()))?);
    let slope = |tangent: usize| -> Result<f32> {
        let [dx, dy] = handling.floats::<2>(tangent + 0xC)?;
        Ok(if dx.abs() > f32::EPSILON { dy / dx } else { 0.0 })
    };
    (0..count)
        .map(|n| {
            let knot = handling.u32(table + 4 * n)? as usize;
            let [rpm, torque_nm] = handling.floats::<2>(knot + 0xC)?;
            Ok(TorqueKnot { rpm, torque_nm, in_slope: slope(knot + 0x18)?, out_slope: slope(knot + 0x30)? })
        })
        .collect()
}

/// The stock gearbox (the one not flagged as the upgrade): forward ratios, reverse ratio, final drive.
fn gearbox(handling: &Object) -> Result<(Vec<f32>, f32, f32)> {
    let boxes: Vec<usize> = handling.all("Genesys.Gen.Gearbox").collect();
    let stock = boxes
        .iter()
        .copied()
        .find(|&gearbox| handling.main.get(gearbox + 0x52).copied() == Some(0))
        .or_else(|| boxes.first().copied())
        .ok_or_else(|| Error::Spec("no gearbox".into()))?;
    let array = handling.u32(stock + 0x4C)? as usize;
    let others = usize::from(handling.u16(stock + 0x50)?);
    // The first gear (reverse) sits inline at 0xC; the rest are an array of 0x1C-byte gears.
    let reverse = handling.f32(stock + 0xC + 0x10)?.abs();
    let mut ratios = Vec::with_capacity(others);
    for gear in 0..others {
        ratios.push(handling.f32(array + 0x1C * gear + 0x10)?.abs());
    }
    // The array starts with neutral (ratio 0).
    let forward: Vec<f32> = ratios.into_iter().skip(1).collect();
    Ok((forward, reverse, handling.f32(stock + 0x38)?))
}

/// The peak longitudinal grip of the tyre `n`: the first long-grip curve inside its structure.
fn tire_grip(handling: &Object, tyres: &[usize], grips: &[usize], n: usize) -> Result<f32> {
    let tyre = *tyres.get(n).ok_or_else(|| Error::Spec("tyre structure".into()))?;
    let curve = grips
        .iter()
        .copied()
        .find(|&curve| curve > tyre && tyres.get(n + 1).is_none_or(|&next| curve < next))
        .ok_or_else(|| Error::Spec("no long grip curve in a tyre".into()))?;
    handling.f32(curve + 0x10)
}

/// Refuse a car whose numbers cannot be real, so a misread layout is caught here and not in the game.
fn check(stats: &VehicleStats) -> Result<()> {
    let bad = |what: &str| Err(Error::Spec(format!("implausible stats: {what}")));
    if !(100.0..=40_000.0).contains(&stats.body_mass_kg) {
        return bad("body mass");
    }
    if stats.torque_curve.len() < 2 || stats.torque_curve.windows(2).any(|pair| pair[1].rpm <= pair[0].rpm) {
        return bad("torque curve");
    }
    if stats.gear_ratios.is_empty() || stats.gear_ratios.iter().any(|ratio| !(0.2..=10.0).contains(ratio)) {
        return bad("gear ratios");
    }
    if !(0.5..=10.0).contains(&stats.final_drive) || stats.axle_z[0] <= stats.axle_z[1] {
        return bad("final drive or axles");
    }
    Ok(())
}
