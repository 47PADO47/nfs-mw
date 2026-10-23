//! Entity views to replicate records and back.

use blackbox_net_core::replicate::{KindId, MAX_FIELDS};
use nfsmw_sim::{EntityId, EntityKind, EntityView, PlayerId};

use super::layout::{self, OWNER_BITS, car, flat, posed};
use super::quant::{self, clamped, pack, pack_quat, pack_yaw, unpack, unpack_quat, unpack_yaw, value};
use super::{GameSchema, SchemaError, WireKind};

/// One quantised entity: its kind and field values, in a fixed array so encoding never allocates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    kind: KindId,
    values: [u32; MAX_FIELDS],
    len: usize,
}

impl Default for Record {
    fn default() -> Self {
        Self { kind: KindId(0), values: [0; MAX_FIELDS], len: 0 }
    }
}

impl Record {
    /// The replicate kind id.
    pub fn kind(&self) -> KindId {
        self.kind
    }

    /// The field values, as `Frame::push` takes them.
    pub fn values(&self) -> &[u32] {
        self.values.get(..self.len).unwrap_or(&[])
    }

    fn start(&mut self, kind: WireKind) {
        self.kind = kind.kind_id();
        self.len = kind.fields().len();
        self.values = [0; MAX_FIELDS];
    }

    fn set(&mut self, index: usize, value: u32) {
        if let Some(slot) = self.values.get_mut(index) {
            *slot = value;
        }
    }
}

/// The car state a full player record carries beyond the view: wheels, engine and the inputs for the remote look.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CarDetail {
    /// Suspension compression per wheel, 0..1 (front left, front right, rear left, rear right).
    pub wheel_compression: [f32; 4],
    pub wheel_on_ground: [bool; 4],
    /// Front-wheel steer, -1..1 of full lock.
    pub wheel_steer: f32,
    /// Skid and smoke, 2 bits per wheel.
    pub skid: u8,
    pub rpm: f32,
    /// 0..15; higher gears are sent as 15.
    pub gear: u8,
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
}

/// Which tier a record was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    Full,
    /// A [`WireKind::PlayerCarReduced`] record.
    Reduced,
}

/// A decoded record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decoded {
    pub view: EntityView,
    /// The wheels, engine and inputs of a full player car.
    pub car: Option<CarDetail>,
    pub tier: Tier,
}

impl GameSchema {
    /// Quantises `view` into `out` at the full tier of its kind. `detail` feeds a player car's wheel, engine and input
    /// fields (zero when `None`) and is ignored for other kinds.
    pub fn encode(&self, view: &EntityView, detail: Option<&CarDetail>, out: &mut Record) -> Result<(), SchemaError> {
        let kind = WireKind::of(view.kind);
        out.start(kind);
        self.put_position(view.position, out)?;
        match kind {
            WireKind::PlayerCar => self.put_car(view, &detail.copied().unwrap_or_default(), out),
            WireKind::TrafficCar => put_flat(view, layout::COARSE_YAW_BITS, &quant::TRAFFIC_SPEED, out, view.type_id),
            WireKind::PoliceCar => put_flat(view, layout::POLICE_YAW_BITS, &quant::POLICE_SPEED, out, view.type_id),
            WireKind::Roadblock => {
                out.set(flat::YAW, pack_yaw(view.orientation, layout::COARSE_YAW_BITS)?);
                out.set(4, flags(view.flags, layout::ROADBLOCK_FLAG_BITS));
                out.set(5, u32::from(view.type_id));
                Ok(())
            }
            WireKind::Helicopter | WireKind::MovingProp => put_posed(view, kind, out),
            // `WireKind::of` never gives the reduced tier; `encode_reduced` writes it.
            WireKind::PlayerCarReduced => Err(SchemaError::BadRecord),
        }
    }

    /// Quantises a player car into `out` at the reduced tier: position, yaw, forward speed, the low
    /// [`layout::REDUCED_FLAG_BITS`] flag bits and the owner.
    pub fn encode_reduced(&self, view: &EntityView, out: &mut Record) -> Result<(), SchemaError> {
        out.start(WireKind::PlayerCarReduced);
        self.put_position(view.position, out)?;
        put_flat(view, layout::COARSE_YAW_BITS, &quant::REDUCED_SPEED, out, 0)?;
        out.set(5, flags(view.flags, layout::REDUCED_FLAG_BITS));
        out.set(6, owner_code(view.owner)?);
        Ok(())
    }

    /// The view of record `values` of replicate kind `kind` for entity `id`.
    pub fn decode(&self, id: EntityId, kind: KindId, values: &[u32]) -> Result<Decoded, SchemaError> {
        let wire = WireKind::from_kind_id(kind).ok_or(SchemaError::BadRecord)?;
        if values.len() != wire.fields().len() {
            return Err(SchemaError::BadRecord);
        }
        let field = |index: usize| values.get(index).copied().ok_or(SchemaError::BadRecord);
        let mut view = EntityView {
            id,
            kind: wire.entity_kind(),
            owner: None,
            position: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0],
            linear_velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
            type_id: 0,
            flags: 0,
        };
        for (axis, slot) in view.position.iter_mut().enumerate() {
            *slot = value(&self.position, field(axis)?)?;
        }
        let mut car_detail = None;
        let mut tier = Tier::Full;
        match wire {
            WireKind::PlayerCar => car_detail = Some(take_car(&mut view, values)?),
            WireKind::PlayerCarReduced => {
                tier = Tier::Reduced;
                take_flat(&mut view, field(3)?, layout::COARSE_YAW_BITS, &quant::REDUCED_SPEED, field(4)?)?;
                view.flags = field(5)?;
                view.owner = owner_of(field(6)?);
            }
            WireKind::TrafficCar => {
                take_flat(&mut view, field(3)?, layout::COARSE_YAW_BITS, &quant::TRAFFIC_SPEED, field(4)?)?;
                view.flags = field(5)?;
                view.type_id = type_id(field(6)?)?;
            }
            WireKind::PoliceCar => {
                take_flat(&mut view, field(3)?, layout::POLICE_YAW_BITS, &quant::POLICE_SPEED, field(4)?)?;
                view.flags = field(5)?;
                view.type_id = type_id(field(6)?)?;
            }
            WireKind::Roadblock => {
                view.orientation = quant::quat_of_yaw(unpack_yaw(field(3)?, layout::COARSE_YAW_BITS));
                view.flags = field(4)?;
                view.type_id = type_id(field(5)?)?;
            }
            WireKind::Helicopter | WireKind::MovingProp => {
                view.orientation = unpack_quat(field(posed::ORIENT)?)?;
                view.flags = field(posed::FLAGS)?;
                view.type_id = type_id(field(posed::TYPE_ID)?)?;
            }
        }
        Ok(Decoded { view, car: car_detail, tier })
    }

    fn put_position(&self, position: [f32; 3], out: &mut Record) -> Result<(), SchemaError> {
        for (axis, x) in position.iter().enumerate() {
            if !x.is_finite() {
                return Err(SchemaError::NotFinite);
            }
            if x.abs() > self.half_extent {
                return Err(SchemaError::OutOfBounds);
            }
            out.set(axis, self.position.encode(*x));
        }
        Ok(())
    }

    fn put_car(&self, view: &EntityView, detail: &CarDetail, out: &mut Record) -> Result<(), SchemaError> {
        out.set(car::ORIENT, pack_quat(view.orientation)?);
        for axis in 0..3 {
            out.set(car::VEL + axis, clamped(&quant::LINEAR_VELOCITY, view.linear_velocity[axis])?);
            out.set(car::ANG + axis, clamped(&quant::ANGULAR_VELOCITY, view.angular_velocity[axis])?);
        }
        let mut compression = [0u32; 4];
        for (code, x) in compression.iter_mut().zip(detail.wheel_compression) {
            *code = clamped(&quant::WHEEL_COMPRESSION, x)?;
        }
        out.set(car::WHEEL_COMPRESSION, pack(&compression, 6));
        out.set(car::WHEEL_GROUND, pack(&detail.wheel_on_ground.map(u32::from), 1));
        out.set(car::WHEEL_STEER, clamped(&quant::WHEEL_STEER, detail.wheel_steer)?);
        out.set(car::SKID, u32::from(detail.skid));
        out.set(car::RPM, clamped(&quant::RPM, detail.rpm)?);
        out.set(car::GEAR, u32::from(detail.gear.min(15)));
        out.set(car::FLAGS, flags(view.flags, car::FLAG_BITS));
        out.set(car::TYPE_ID, u32::from(view.type_id));
        out.set(car::OWNER, owner_code(view.owner)?);
        let inputs = [
            clamped(&quant::PEDAL_LOOK, detail.throttle)?,
            clamped(&quant::PEDAL_LOOK, detail.brake)?,
            clamped(&quant::STEER_LOOK, detail.steer)?,
        ];
        out.set(car::INPUTS, (inputs[0] << 14) | (inputs[1] << 8) | inputs[2]);
        Ok(())
    }
}

/// Yaw, forward speed, flags and type of a traffic or police car (or the yaw and speed of a reduced player car).
fn put_flat(
    view: &EntityView,
    yaw_bits: u32,
    speed: &blackbox_net_core::bits::Quant,
    out: &mut Record,
    type_id: u16,
) -> Result<(), SchemaError> {
    let yaw = quant::yaw_of(view.orientation);
    out.set(flat::YAW, pack_yaw(view.orientation, yaw_bits)?);
    out.set(flat::SPEED, clamped(speed, quant::forward_speed(view.linear_velocity, yaw))?);
    let flag_bits = match view.kind {
        EntityKind::PoliceCar => layout::POLICE_FLAG_BITS,
        _ => layout::TRAFFIC_FLAG_BITS,
    };
    out.set(5, flags(view.flags, flag_bits));
    out.set(6, u32::from(type_id));
    Ok(())
}

/// Orientation, flags and type of a helicopter or moving prop.
fn put_posed(view: &EntityView, kind: WireKind, out: &mut Record) -> Result<(), SchemaError> {
    let flag_bits = match kind {
        WireKind::Helicopter => layout::HELICOPTER_FLAG_BITS,
        _ => layout::PROP_FLAG_BITS,
    };
    out.set(posed::ORIENT, pack_quat(view.orientation)?);
    out.set(posed::FLAGS, flags(view.flags, flag_bits));
    out.set(posed::TYPE_ID, u32::from(view.type_id));
    Ok(())
}

fn take_car(view: &mut EntityView, values: &[u32]) -> Result<CarDetail, SchemaError> {
    let field = |index: usize| values.get(index).copied().ok_or(SchemaError::BadRecord);
    view.orientation = unpack_quat(field(car::ORIENT)?)?;
    for axis in 0..3 {
        view.linear_velocity[axis] = value(&quant::LINEAR_VELOCITY, field(car::VEL + axis)?)?;
        view.angular_velocity[axis] = value(&quant::ANGULAR_VELOCITY, field(car::ANG + axis)?)?;
    }
    view.flags = field(car::FLAGS)?;
    view.type_id = type_id(field(car::TYPE_ID)?)?;
    view.owner = owner_of(field(car::OWNER)?);
    let mut detail = CarDetail::default();
    let compression: [u32; 4] = unpack(field(car::WHEEL_COMPRESSION)?, 6);
    for (slot, code) in detail.wheel_compression.iter_mut().zip(compression) {
        *slot = value(&quant::WHEEL_COMPRESSION, code)?;
    }
    let ground: [u32; 4] = unpack(field(car::WHEEL_GROUND)?, 1);
    detail.wheel_on_ground = ground.map(|bit| bit != 0);
    detail.wheel_steer = value(&quant::WHEEL_STEER, field(car::WHEEL_STEER)?)?;
    detail.skid = u8::try_from(field(car::SKID)?).map_err(|_| SchemaError::BadValue)?;
    detail.rpm = value(&quant::RPM, field(car::RPM)?)?;
    detail.gear = u8::try_from(field(car::GEAR)?).map_err(|_| SchemaError::BadValue)?;
    let inputs = field(car::INPUTS)?;
    detail.throttle = value(&quant::PEDAL_LOOK, (inputs >> 14) & 0x3F)?;
    detail.brake = value(&quant::PEDAL_LOOK, (inputs >> 8) & 0x3F)?;
    detail.steer = value(&quant::STEER_LOOK, inputs & 0xFF)?;
    Ok(detail)
}

fn take_flat(
    view: &mut EntityView,
    yaw_code: u32,
    yaw_bits: u32,
    speed: &blackbox_net_core::bits::Quant,
    speed_code: u32,
) -> Result<(), SchemaError> {
    let yaw = unpack_yaw(yaw_code, yaw_bits);
    view.orientation = quant::quat_of_yaw(yaw);
    view.linear_velocity = quant::velocity_of(value(speed, speed_code)?, yaw);
    Ok(())
}

/// The low `bits` bits of `flags`.
fn flags(flags: u32, bits: u32) -> u32 {
    match bits {
        32.. => flags,
        _ => flags & ((1u32 << bits) - 1),
    }
}

/// The owner field: 0 for none, else the player id plus one.
fn owner_code(owner: Option<PlayerId>) -> Result<u32, SchemaError> {
    let max = (1u32 << OWNER_BITS) - 2;
    match owner {
        None => Ok(0),
        Some(PlayerId(id)) if u32::from(id) <= max => Ok(u32::from(id) + 1),
        Some(_) => Err(SchemaError::BadOwner),
    }
}

fn owner_of(code: u32) -> Option<PlayerId> {
    let id = code.checked_sub(1)?;
    u8::try_from(id).ok().map(PlayerId)
}

fn type_id(code: u32) -> Result<u16, SchemaError> {
    u16::try_from(code).map_err(|_| SchemaError::BadValue)
}
