use chameleon_bundle::{Bundle, Import, ResourceKind};

use crate::error::{Error, Result};

/// One wheel of the graphics spec: where it sits and the models drawn there.
#[derive(Debug, Clone, PartialEq)]
pub struct WheelSpec {
    /// `FrontLeft`, `FrontRight`, `RearLeft` or `RearRight`.
    pub name: String,
    /// The hub, metres from the car's origin, in engine space (x left, y up, z forward).
    pub position: [f32; 3],
    /// Model resources: the tyre, the brake disc, the rim and the caliper, as far as the car has them.
    pub models: Vec<u64>,
}

/// The `VehicleGraphicsSpec` resource: the body model and the wheels.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicsSpec {
    pub body_model: u64,
    pub wheels: Vec<WheelSpec>,
}

const WHEEL_STRIDE: usize = 0x90;
const MODELS_PER_WHEEL: u32 = 4;

fn u32_at(block: &[u8], offset: usize) -> Result<u32> {
    let bytes = block.get(offset..offset + 4).ok_or_else(|| Error::Spec(format!("block ends before 0x{offset:X}")))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn f32_at(block: &[u8], offset: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(block, offset)?))
}

fn import_at(imports: &[Import], offset: u32) -> Option<u64> {
    imports.iter().find(|import| import.offset == offset).map(|import| import.id)
}

impl GraphicsSpec {
    pub fn parse(main: &[u8], imports: &[Import]) -> Result<Self> {
        let body_slot = u32_at(main, 0)?;
        let body_model = import_at(imports, body_slot).ok_or_else(|| Error::Spec("no body model import".into()))?;
        let wheel_count = usize::from(*main.get(0x13).ok_or_else(|| Error::Spec("block too short".into()))?);
        let table = u32_at(main, 0xC)? as usize;
        let mut wheels = Vec::with_capacity(wheel_count);
        for n in 0..wheel_count {
            let at = table + n * WHEEL_STRIDE;
            let models_at = u32_at(main, at + 0x78)?;
            let name = main
                .get(at + 0x82..)
                .map(|tail| tail.iter().take_while(|&&byte| byte != 0).map(|&byte| char::from(byte)).collect())
                .unwrap_or_default();
            wheels.push(WheelSpec {
                name,
                position: [f32_at(main, at)?, f32_at(main, at + 4)?, f32_at(main, at + 8)?],
                models: (0..MODELS_PER_WHEEL).filter_map(|slot| import_at(imports, models_at + 4 * slot)).collect(),
            });
        }
        Ok(Self { body_model, wheels })
    }
}

/// The id of the vehicle graphics spec of a vehicle bundle.
pub fn find_spec(bundle: &Bundle) -> Option<u64> {
    let kind = ResourceKind::VehicleGraphicsSpec.id();
    bundle.entries().iter().find(|entry| entry.kind == kind).map(|entry| entry.id)
}
