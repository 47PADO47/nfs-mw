//! Installed cutscene engine curves. Layout: `docs/specs/nis-events.md` section 7.7.

/// One point of a scene's engine curve. Torque is the authored 0–100 sound control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NisRev {
    pub time: f32,
    pub rpm: f32,
    pub torque: f32,
}

/// The sixteen car channels. Extra bytes after the last channel are permitted by the format.
#[derive(Clone, Debug, PartialEq)]
pub struct NisRevs([Vec<NisRev>; 16]);

impl NisRevs {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        let mut cursor = 0;
        let mut channels: [Vec<NisRev>; 16] = std::array::from_fn(|_| Vec::new());
        for points in &mut channels {
            let count = word(bytes, &mut cursor)? as usize;
            if count > bytes.len().saturating_sub(cursor) / 12 {
                return Err("truncated NIS rev channel");
            }
            for _ in 0..count {
                let time = f32::from_bits(word(bytes, &mut cursor)?);
                let rpm = word(bytes, &mut cursor)? as i32 as f32;
                let torque = word(bytes, &mut cursor)? as i32 as f32;
                if !time.is_finite() || time < 0.0 || points.last().is_some_and(|last: &NisRev| last.time > time) {
                    return Err("invalid NIS rev timeline");
                }
                points.push(NisRev { time, rpm, torque });
            }
        }
        Ok(Self(channels))
    }

    /// Linear interpolation while the channel has a curve; no fabricated looping after its end.
    pub fn sample(&self, channel: usize, time: f32) -> Option<NisRev> {
        if !time.is_finite() {
            return None;
        }
        let points = self.0.get(channel)?;
        let first = *points.first()?;
        let last = *points.last()?;
        if points.len() < 2 || time < first.time || time > last.time {
            return None;
        }
        let right = points.partition_point(|point| point.time <= time).min(points.len() - 1);
        let left = right.saturating_sub(1);
        let (a, b) = (points[left], points[right]);
        let fraction = if b.time > a.time { (time - a.time) / (b.time - a.time) } else { 1.0 };
        Some(NisRev {
            time,
            rpm: (a.rpm + fraction * (b.rpm - a.rpm)).clamp(1000.0, 10000.0),
            torque: (a.torque + fraction * (b.torque - a.torque)).clamp(0.0, 100.0),
        })
    }
}

fn word(bytes: &[u8], cursor: &mut usize) -> Result<u32, &'static str> {
    let word = bytes.get(*cursor..*cursor + 4).ok_or("truncated NIS rev file")?;
    *cursor += 4;
    Ok(u32::from_le_bytes(word.try_into().map_err(|_| "truncated NIS rev word")?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut words = vec![2, 0_f32.to_bits(), 1000, 0, 2_f32.to_bits(), 9000, 100];
        words.extend([0; 15]);
        words.into_iter().flat_map(u32::to_le_bytes).collect()
    }

    #[test]
    fn curves_interpolate_by_channel_and_end_without_looping() {
        let mut bytes = fixture();
        bytes.extend([0xFF; 7]);
        let curves = NisRevs::parse(&bytes).unwrap();
        assert_eq!(curves.sample(0, 1.0), Some(NisRev { time: 1.0, rpm: 5000.0, torque: 50.0 }));
        assert_eq!(curves.sample(0, 2.0).unwrap().rpm, 9000.0);
        assert_eq!(curves.sample(0, 2.1), None);
        assert_eq!(curves.sample(1, 1.0), None);
        assert_eq!(curves.sample(16, 1.0), None);
    }

    #[test]
    fn bad_counts_and_timelines_are_rejected() {
        let bytes = fixture();
        assert!(NisRevs::parse(&bytes[..bytes.len() - 1]).is_err());
        let mut invalid = bytes.clone();
        invalid[4..8].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(NisRevs::parse(&invalid).is_err());
        let mut reversed = bytes;
        reversed[4..8].copy_from_slice(&3_f32.to_le_bytes());
        assert!(NisRevs::parse(&reversed).is_err());
    }
}
