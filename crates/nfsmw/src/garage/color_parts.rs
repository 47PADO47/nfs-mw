//! Sparse native color identities: TOML has no null array elements.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
struct Entry {
    channel: usize,
    hash: u32,
}

impl super::Selection {
    /// A legacy record-bound RGB extension keeps matching swatches learned from that native record.
    pub(crate) fn inherit_native_color_parts(&mut self, native: &Self) {
        if self.car != native.car {
            return;
        }
        for index in 0..3 {
            if self.parts.vinyl_color_parts[index].is_none()
                && self.parts.vinyl_colors[index].is_some()
                && self.parts.vinyl_colors[index] == native.parts.vinyl_colors[index]
            {
                self.parts.vinyl_color_parts[index] = native.parts.vinyl_color_parts[index];
            }
            if self.gauges.color_parts[index].is_none()
                && self.gauges.colors()[index].is_some()
                && self.gauges.colors()[index] == native.gauges.colors()[index]
            {
                self.gauges.color_parts[index] = native.gauges.color_parts[index];
            }
        }
    }
}

pub(super) fn serialize<S: Serializer>(parts: &[Option<u32>; 3], serializer: S) -> Result<S::Ok, S::Error> {
    let entries: Vec<_> =
        parts.iter().enumerate().filter_map(|(channel, hash)| hash.map(|hash| Entry { channel, hash })).collect();
    entries.serialize(serializer)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[Option<u32>; 3], D::Error> {
    let entries = Vec::<Entry>::deserialize(deserializer)?;
    let mut parts = [None; 3];
    for entry in entries {
        let part =
            parts.get_mut(entry.channel).ok_or_else(|| serde::de::Error::custom("Invalid native color channel"))?;
        if part.is_some() {
            return Err(serde::de::Error::custom("Duplicate native color channel"));
        }
        *part = Some(entry.hash);
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use crate::garage::Selection;

    #[test]
    fn legacy_rgb_extension_inherits_only_matching_native_color_identities() {
        let mut native = Selection { car: "CAR".into(), ..Default::default() };
        native.parts.vinyl_colors = [Some([1, 2, 3]), Some([4, 5, 6]), None];
        native.parts.vinyl_color_parts = [Some(10), Some(11), None];
        native.gauges.backing = Some([7, 8, 9]);
        native.gauges.color_parts[0] = Some(20);
        let mut extension = native.clone();
        extension.parts.vinyl_color_parts = [None; 3];
        extension.parts.vinyl_colors[1] = Some([10, 20, 30]);
        extension.gauges.color_parts[0] = Some(21);
        extension.inherit_native_color_parts(&native);
        assert_eq!(extension.parts.vinyl_color_parts, [Some(10), None, None]);
        assert_eq!(extension.parts.vinyl_colors[1], Some([10, 20, 30]));
        assert_eq!(extension.gauges.color_parts[0], Some(21), "an explicit newer identity wins");
    }
}
