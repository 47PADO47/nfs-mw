//! Reading the rider's physics asset and the ragdoll component of his blueprint (the JSON of the package reader).

use anyhow::{Result, bail};
use glam::{Mat3, Quat, Vec3};
use serde_json::Value;

use crate::rider::convert;

/// Unreal's default limit of a limited angular motion, degrees.
const DEFAULT_LIMIT: f32 = 45.0;
/// The limit that means "free" in the file.
const FREE: f32 = 180.0;

/// A capsule on a bone (the physics asset's `SkeletalBodySetup`).
#[derive(Debug, Clone)]
pub struct Body {
    pub bone: String,
    pub mass: f32,
    pub radius: f32,
    pub half_length: f32,
    pub centre: Vec3,
    pub axis: Vec3,
}

/// A joint (the asset's `PhysicsConstraintTemplate`), limits in degrees.
#[derive(Debug, Clone)]
pub struct Joint {
    pub child: String,
    pub parent: String,
    pub frame_child: Quat,
    pub frame_parent: Quat,
    pub swing_z: f32,
    pub swing_y: f32,
    pub twist: f32,
}

/// One kind of fall (the ragdoll component's `FallConfiguration`), velocities in km/h.
#[derive(Debug, Clone, Copy, Default)]
pub struct Fall {
    pub additional: Vec3,
    pub scale: Vec3,
    pub world: Vec3,
    pub angular: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Falls {
    pub high_side: Fall,
    pub low_side: Fall,
}

fn number(value: &Value, key: &str) -> Option<f32> {
    value.get(key)?.as_f64().map(|n| n as f32)
}

/// A `{X, Y, Z}` vector.
fn vector(value: &Value, key: &str) -> Option<[f32; 3]> {
    let v = value.get(key)?;
    Some([number(v, "X")?, number(v, "Y")?, number(v, "Z")?])
}

/// The exports of a given type.
fn exports<'a>(file: &'a Value, kind: &'a str) -> impl Iterator<Item = &'a Value> {
    file.as_array().into_iter().flatten().filter(move |e| e["Type"] == kind)
}

/// The first shape of a kind in an asset's geometry.
fn first<'a>(geometry: &'a Value, kind: &str) -> Option<&'a Value> {
    geometry[kind].as_array()?.first()
}

/// A box (`FKBoxElem`: the full sizes `X`, `Y`, `Z`, cm) as the capsule that stands for it: along the box's longest side, with the mean
/// half-size of the other two as its radius and the rest of the longest side as its straight part. A modelling choice: the physics
/// solver of the game has capsules only; the rider's `_CB` asset gives the hands boxes. Returns the axis in the shape's frame, the
/// radius and the straight length (cm).
fn box_capsule(boxed: &Value) -> (Vec3, f32, f32) {
    let sizes = [
        (Vec3::X, number(boxed, "X").unwrap_or(0.0)),
        (Vec3::Y, number(boxed, "Y").unwrap_or(0.0)),
        (Vec3::Z, number(boxed, "Z").unwrap_or(0.0)),
    ];
    let longest = sizes.iter().copied().fold(sizes[0], |best, size| if size.1 > best.1 { size } else { best });
    let others: Vec<f32> = sizes.iter().filter(|size| size.0 != longest.0).map(|size| size.1).collect();
    let radius = others.iter().sum::<f32>() / 4.0;
    (longest.0, radius, (longest.1 - 2.0 * radius).max(0.0))
}

/// The capsule bodies of the asset: mass (`MassInKgOverride`) and the first capsule of the shapes, in the rider's frame and metres.
pub fn bodies(asset: &Value) -> Result<Vec<Body>> {
    let mut out = Vec::new();
    for setup in exports(asset, "SkeletalBodySetup") {
        let props = &setup["Properties"];
        let Some(bone) = props["BoneName"].as_str() else { continue };
        let Some(mass) = number(&props["DefaultInstance"], "MassInKgOverride") else {
            bail!("the body of {bone} has no mass (MassInKgOverride)");
        };
        let geometry = &props["AggGeom"];
        let (shape, along, radius, length) = match first(geometry, "SphylElems") {
            Some(capsule) => {
                // A capsule's axis is the Z axis of its shape.
                let (radius, length) =
                    (number(capsule, "Radius").unwrap_or(0.0), number(capsule, "Length").unwrap_or(0.0));
                (capsule, Vec3::Z, radius, length)
            }
            None => {
                let Some(boxed) = first(geometry, "BoxElems") else {
                    bail!("the body of {bone} has no capsule or box")
                };
                let (along, radius, length) = box_capsule(boxed);
                (boxed, along, radius, length)
            }
        };
        let centre = vector(shape, "Center").unwrap_or([0.0; 3]);
        let rotation = &shape["Rotation"];
        let q = convert::rotator_quat(
            number(rotation, "Pitch").unwrap_or(0.0),
            number(rotation, "Yaw").unwrap_or(0.0),
            number(rotation, "Roll").unwrap_or(0.0),
        );
        // The axis, in the bone's frame.
        let axis = Quat::from_xyzw(q[0], q[1], q[2], q[3]) * along;
        out.push(Body {
            bone: bone.to_owned(),
            mass,
            radius: radius * 0.01,
            half_length: length * 0.01 / 2.0,
            centre: convert::position(centre),
            axis: convert::direction(axis.to_array()).normalize(),
        });
    }
    if out.is_empty() {
        bail!("the physics asset has no bodies");
    }
    Ok(out)
}

/// The frame of a constraint from its primary and secondary axes: the twist axis X, the secondary axis Y, Z their cross product
/// (Unreal's `MakeFromXY`), as a rotation in the car's frame.
fn frame(primary: Option<[f32; 3]>, secondary: Option<[f32; 3]>) -> Quat {
    let x = Vec3::from_array(primary.unwrap_or([1.0, 0.0, 0.0])).normalize_or(Vec3::X);
    let y = Vec3::from_array(secondary.unwrap_or([0.0, 1.0, 0.0]));
    let z = x.cross(y).normalize_or(Vec3::Z);
    let y = z.cross(x);
    let q = Quat::from_mat3(&Mat3::from_cols(x, y, z));
    convert::rotation([q.x, q.y, q.z, q.w])
}

/// A limit in degrees from its motion and value: locked is 0, free is 180, limited is the value (45 by default).
fn limit(motion: Option<&str>, degrees: Option<f32>) -> f32 {
    match motion {
        Some(m) if m.ends_with("ACM_Locked") => 0.0,
        Some(m) if m.ends_with("ACM_Limited") => degrees.unwrap_or(DEFAULT_LIMIT).clamp(0.0, FREE),
        _ => FREE,
    }
}

/// The joints of the asset between the bodies (`bodies`): the child is `ConstraintBone1`, the parent `ConstraintBone2`. The cone's
/// first swing (`Swing1`) is the swing about the constraint frame's Z axis, the second about its Y axis.
pub fn joints(asset: &Value, bodies: &[Body]) -> Result<Vec<Joint>> {
    let has = |bone: &str| bodies.iter().any(|b| b.bone == bone);
    let mut out = Vec::new();
    for template in exports(asset, "PhysicsConstraintTemplate") {
        let instance = &template["Properties"]["DefaultInstance"];
        let (Some(child), Some(parent)) = (instance["ConstraintBone1"].as_str(), instance["ConstraintBone2"].as_str())
        else {
            continue;
        };
        if !has(child) || !has(parent) {
            bail!("the joint {parent} > {child} joins bones that have no body");
        }
        let profile = &instance["ProfileInstance"];
        let (cone, twist) = (&profile["ConeLimit"], &profile["TwistLimit"]);
        out.push(Joint {
            child: child.to_owned(),
            parent: parent.to_owned(),
            frame_child: frame(vector(instance, "PriAxis1"), vector(instance, "SecAxis1")),
            frame_parent: frame(vector(instance, "PriAxis2"), vector(instance, "SecAxis2")),
            swing_z: limit(cone["Swing1Motion"].as_str(), number(cone, "Swing1LimitDegrees")),
            swing_y: limit(cone["Swing2Motion"].as_str(), number(cone, "Swing2LimitDegrees")),
            twist: limit(twist["TwistMotion"].as_str(), number(twist, "TwistLimitDegrees")),
        });
    }
    Ok(out)
}

/// One `FallConfiguration`, its vectors turned to the rider's frame (x forward, y left, z up): `FallAdditionalVelocity`,
/// `FallVelocityMultiplierAdditional`, `FallAdditionalVelocityWorld` and `FallAngularVelocityFactor`.
fn fall(config: &Value) -> Fall {
    let mirrored = |key: &str| Vec3::from_array(vector(config, key).unwrap_or([0.0; 3]));
    let flip = |v: Vec3| Vec3::new(v.x, -v.y, v.z);
    Fall {
        additional: flip(mirrored("FallAdditionalVelocity")),
        // A factor per axis does not change sign when the axis does.
        scale: mirrored("FallVelocityMultiplierAdditional"),
        world: flip(mirrored("FallAdditionalVelocityWorld")),
        angular: number(config, "FallAngularVelocityFactor").unwrap_or(0.0),
    }
}

/// The configurations of the two kinds of fall the rider component tells apart (`StartFallConfigurations`: the keys
/// `ERiderFallTypeEnum::VE_HighSide` and `VE_LowSide`).
pub fn falls(blueprint: &Value) -> Result<Falls> {
    let Some(component) = exports(blueprint, "IgnitionRagdollComponent").next() else {
        bail!("the rider blueprint has no ragdoll component");
    };
    let Some(configs) = component["Properties"]["StartFallConfigurations"].as_array() else {
        bail!("the ragdoll component has no StartFallConfigurations");
    };
    let mut out = Falls::default();
    for entry in configs {
        let key = entry["Key"].as_str().unwrap_or_default();
        match () {
            () if key.ends_with("VE_HighSide") => out.high_side = fall(&entry["Value"]),
            () if key.ends_with("VE_LowSide") => out.low_side = fall(&entry["Value"]),
            () => {}
        }
    }
    Ok(out)
}
