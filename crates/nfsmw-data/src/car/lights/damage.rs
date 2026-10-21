//! Original named VehicleFX groups, separate from model-marker identities.
use blackbox_attrib::vlt_hash;

/// Resolve an EDamageLights name; unknown names never imply all lights.
pub fn damage_mask(name: u32) -> Option<u32> {
    [
        ("LIGHT_LHEAD", 1),
        ("LIGHT_RHEAD", 2),
        ("LIGHT_CHEAD", 4),
        ("LIGHT_LBRAKE", 8),
        ("LIGHT_RBRAKE", 16),
        ("LIGHT_CBRAKE", 32),
        ("LIGHT_LREVERSE", 64),
        ("LIGHT_RREVERSE", 128),
        ("LIGHT_LRSIGNAL", 256),
        ("LIGHT_RRSIGNAL", 512),
        ("LIGHT_LFSIGNAL", 1024),
        ("LIGHT_RFSIGNAL", 2048),
        ("LIGHT_COPRED", 4096),
        ("LIGHT_COPBLUE", 8192),
        ("LIGHT_COPWHITE", 16384),
        ("LIGHT_COPS", 28672),
        ("LIGHT_LSIGNAL", 1280),
        ("LIGHT_RSIGNAL", 2560),
        ("LIGHT_HEADLIGHTS", 7),
        ("LIGHT_REVERSE", 192),
        ("LIGHT_BRAKELIGHTS", 56),
    ]
    .into_iter()
    .find_map(|(text, mask)| (vlt_hash(text) == name).then_some(mask))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn individual_and_combined_names_keep_distinct_bits_and_unknowns_have_none() {
        assert_eq!(damage_mask(vlt_hash("LIGHT_LHEAD")), Some(1));
        assert_eq!(damage_mask(vlt_hash("LIGHT_RBRAKE")), Some(16));
        assert_eq!(damage_mask(vlt_hash("LIGHT_COPWHITE")), Some(16384));
        assert_eq!(damage_mask(vlt_hash("LIGHT_COPS")), Some(4096 | 8192 | 16384));
        assert_eq!(damage_mask(vlt_hash("LIGHT_LSIGNAL")), Some(256 | 1024));
        assert_eq!(damage_mask(vlt_hash("LIGHT_BRAKELIGHTS")), Some(8 | 16 | 32));
        assert_eq!(damage_mask(0x5B66_7470), Some(28672));
        assert_eq!(damage_mask(vlt_hash("")), None);
        assert_eq!(damage_mask(vlt_hash("LEFT_HEADLIGHT")), None);
    }
}
