//! Native color-part identity resolution, independent of RGB equality.

use anyhow::{Context, Result, ensure};
use nfsmw_career::stable::CustomizationRecord;

pub(super) struct Swatch {
    pub hash: u32,
    pub index: u16,
    pub rgb: [u8; 3],
}

pub(super) fn read(
    record: &CustomizationRecord,
    slot: usize,
    stock: Option<u32>,
    swatches: &[Swatch],
) -> Result<(Option<[u8; 3]>, Option<u32>)> {
    let Some(index) = record.part(slot) else { return Ok((None, None)) };
    let swatch =
        swatches.iter().find(|swatch| swatch.index == index).context("Saved color is incompatible with its channel")?;
    if Some(swatch.hash) == stock {
        return Ok((None, None));
    }
    Ok((Some(swatch.rgb), Some(swatch.hash)))
}

pub(super) fn write(
    record: &mut CustomizationRecord,
    slot: usize,
    stock: Option<u32>,
    rgb: Option<[u8; 3]>,
    identity: Option<u32>,
    swatches: &[Swatch],
) -> Result<()> {
    if let Some(hash) = identity {
        let swatch = swatches
            .iter()
            .find(|swatch| swatch.hash == hash)
            .context("Native color part is incompatible with its channel")?;
        ensure!(rgb == Some(swatch.rgb), "Native color identity does not match its RGB value");
        ensure!(record.set_part(slot, Some(swatch.index)), "Native color index cannot be stored");
        return Ok(());
    }
    if let Some(rgb) = rgb {
        if let Some(previous) = record.part(slot).and_then(|index| swatches.iter().find(|swatch| swatch.index == index))
            && previous.rgb == rgb
        {
            return Ok(());
        }
        if let Some(swatch) = swatches.iter().find(|swatch| swatch.rgb == rgb) {
            ensure!(record.set_part(slot, Some(swatch.index)), "Native color index cannot be stored");
        }
        // Arbitrary RGB remains in the record-bound extension and keeps its previous native swatch.
        return Ok(());
    }
    let index = stock
        .map(|hash| {
            swatches
                .iter()
                .find(|swatch| swatch.hash == hash)
                .map(|swatch| swatch.index)
                .context("Stock color part is unavailable")
        })
        .transpose()?;
    ensure!(record.set_part(slot, index), "Native color index cannot be stored");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_rgb_parts_keep_the_chosen_identity_through_native_commit_and_reload() {
        let palette =
            [Swatch { hash: 10, index: 3, rgb: [20, 40, 60] }, Swatch { hash: 11, index: 7, rgb: [20, 40, 60] }];
        let mut record = CustomizationRecord::default();
        for slot in [79, 133] {
            write(&mut record, slot, None, Some([20, 40, 60]), Some(11), &palette).unwrap();
        }
        let persisted = CustomizationRecord::parse(record.as_bytes()).unwrap();
        for slot in [79, 133] {
            assert_eq!(persisted.part(slot), Some(7));
            let (rgb, identity) = read(&persisted, slot, None, &palette).unwrap();
            assert_eq!((rgb, identity), (Some([20, 40, 60]), Some(11)));
            let mut legacy = persisted.clone();
            write(&mut legacy, slot, None, rgb, None, &palette).unwrap();
            assert_eq!(legacy.part(slot), Some(7), "legacy RGB preserves an existing matching second swatch");
        }
    }

    #[test]
    fn native_color_identity_requires_compatible_rgb() {
        let palette = [Swatch { hash: 10, index: 3, rgb: [20, 40, 60] }];
        let mut record = CustomizationRecord::default();
        assert!(write(&mut record, 79, None, Some([1, 2, 3]), Some(10), &palette).is_err());
        assert!(write(&mut record, 79, None, Some([20, 40, 60]), Some(99), &palette).is_err());
        assert!(record.part(79).is_none());
    }
}
