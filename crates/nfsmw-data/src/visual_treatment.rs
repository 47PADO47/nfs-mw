use blackbox_attrib::{Database, Value};
use blackbox_gfx::{ColourTreatment, RgbaImage};
use blackbox_tpk::Texture;
pub mod speedbreaker;

/// The ordinary free-roam look. Missing or malformed data leaves grading disabled.
pub fn midday(db: &Database) -> Option<ColourTreatment> {
    let look = db.collection("visuallook", "default")?;
    let tint = look.get("ColourBloomTint")?.as_vector4()?;
    let profile = ColourTreatment {
        tint: [tint[0], tint[1], tint[2]],
        luminance: [0.6125, 0.5154, 0.0721],
        desaturation: look.get("Desaturation")?.as_f32()?,
        black_intensity: look.get("BlackBloomIntensity")?.as_f32()?,
        colour_intensity: look.get("ColourBloomIntensity")?.as_f32()?,
        black_curve: ordinates(look.get("BlackBloomCurve")?)?,
        colour_curve: ordinates(look.get("ColourBloomCurve")?)?,
    };
    if !profile.is_valid() {
        return None;
    }
    Some(profile)
}

/// A curve is a Matrix4: the shader evaluates its four row-y values as Bezier ordinates.
fn ordinates(value: &Value) -> Option<[f32; 4]> {
    let Value::Raw { bytes, .. } = value else { return None };
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0.0; 4];
    for (row, v) in out.iter_mut().enumerate() {
        let at = row * 16 + 4;
        *v = f32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
        if !v.is_finite() {
            return None;
        }
    }
    Some(out)
}

/// Use the installed screen gain map, without shipping a replacement asset.
pub fn gain_map(textures: &[Texture]) -> Option<RgbaImage> {
    for hash in [0x9687_75E8, 0x0CC0_87FE] {
        let Some(t) = textures.iter().find(|t| t.name_hash == hash) else { continue };
        let Some(rgba) = blackbox_tpk::decode_rgba8(t) else { continue };
        if let Ok(image) = RgbaImage::new(t.width, t.height, rgba) {
            return Some(image);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs the base-game install selected by NFSMW_GAME_DIR"]
    fn base_game_midday_and_gain_map_load() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        assert!(midday(&db).expect("installed midday look").is_valid());
        let globals = crate::world::load_global_textures(&dir).unwrap();
        let gain = gain_map(&globals.textures).expect("installed gain map");
        assert!(gain.width > 1 && gain.height > 1);
        assert!(gain.pixel(gain.width / 2, gain.height / 2).unwrap()[2] > gain.pixel(0, 0).unwrap()[2]);
    }

    #[test]
    fn matrix_rows_supply_y_not_x_and_non_finite_values_are_rejected() {
        let mut bytes = Vec::new();
        for row in 0..4 {
            for f in [9.0, row as f32 * 0.1, 8.0, 7.0] {
                bytes.extend(f.to_le_bytes());
            }
        }
        let value = Value::Raw { type_key: 0, bytes: bytes.clone() };
        assert_eq!(ordinates(&value), Some([0.0, 0.1, 0.2, 0.3]));
        bytes[20..24].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(ordinates(&Value::Raw { type_key: 0, bytes }).is_none());
        assert!(ordinates(&Value::Float(1.0)).is_none());
    }
}
