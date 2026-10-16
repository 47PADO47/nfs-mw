//! The parsed custom fonts, by tier. Rule: `docs/specs/custom-fonts.md`, sections 4 and 5.

use std::collections::HashMap;

use blackbox_chunk::find_all;
use blackbox_feng::Font;
use blackbox_tpk::{Texture, read_texture_packs};

use super::folder::FontFile;
use super::tier::Tier;

/// The chunk tag of an `FEngFont` (`0x00030201`).
const FENG_FONT: u32 = 0x0003_0201;

/// The fonts and font textures one tier's files supply.
#[derive(Default)]
struct TierFonts {
    fonts: HashMap<u32, Font>,
    textures: HashMap<u32, Texture>,
}

/// Every usable custom font, grouped by the tier of its file. A font is keyed by its name hash, the key the stock
/// loader uses, so a custom font of the same name replaces the stock one.
#[derive(Default)]
pub struct CustomFontSet {
    tiers: [TierFonts; 3],
}

impl CustomFontSet {
    /// Reads and parses each file. A file that cannot be read or parsed is skipped with a warning, so the stock font
    /// stays in use for its name.
    pub fn load(files: &[FontFile]) -> Self {
        let mut set = Self::default();
        for file in files {
            let origin = file.path.display().to_string();
            match std::fs::read(&file.path) {
                Ok(data) => set.add(file.tier, &origin, &data),
                Err(e) => log::warn!("custom font {origin}: cannot read it: {e}"),
            }
        }
        if !set.is_empty() {
            log::info!("custom fonts: {} files", set.len());
        }
        set
    }

    /// Adds the font of one file's bytes to `tier`. A file with no font, a font that does not parse, or a name the tier
    /// already has (the first file in the folder's order wins) adds nothing.
    pub fn add(&mut self, tier: Tier, origin: &str, data: &[u8]) {
        let Some(font) = parse_font(origin, data) else { return };
        let slot = &mut self.tiers[tier.index()];
        if slot.fonts.contains_key(&font.hash) {
            log::warn!(
                "custom font {origin}: {} is already in the {} tier; this file is ignored",
                font.name,
                tier.name()
            );
            return;
        }
        if let Some(texture) = texture_named(origin, data, font.texture_hash) {
            slot.textures.entry(font.texture_hash).or_insert(texture);
        }
        slot.fonts.insert(font.hash, font);
    }

    /// The custom font for a name hash at `tier`, with the tier its file is for. The files are tried in the order of
    /// [`Tier::chain`]: this tier first, then each lower one. `None` means the stock font is used.
    pub fn font(&self, hash: u32, tier: Tier) -> Option<(Tier, &Font)> {
        tier.chain().find_map(|t| self.tiers[t.index()].fonts.get(&hash).map(|f| (t, f)))
    }

    /// The custom texture for a texture name hash, searched in the same order as [`CustomFontSet::font`]. `None` means
    /// the stock texture is used.
    pub fn texture(&self, hash: u32, tier: Tier) -> Option<&Texture> {
        tier.chain().find_map(|t| self.tiers[t.index()].textures.get(&hash))
    }

    /// The number of fonts in all the tiers.
    pub fn len(&self) -> usize {
        self.tiers.iter().map(|t| t.fonts.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The first font chunk of a file, parsed. A file without one, or with one that does not parse, gives `None` with a
/// warning.
fn parse_font(origin: &str, data: &[u8]) -> Option<Font> {
    let Some(chunk) = find_all(data, FENG_FONT).into_iter().next() else {
        log::warn!("custom font {origin}: no font chunk in the file");
        return None;
    };
    match Font::parse(chunk.payload) {
        Ok(font) => Some(font),
        Err(e) => {
            log::warn!("custom font {origin}: the font does not parse: {e}");
            None
        }
    }
}

/// The texture of a file's texture packs with the given name hash. A file whose packs do not read gives `None`.
fn texture_named(origin: &str, data: &[u8], name_hash: u32) -> Option<Texture> {
    let packs = match read_texture_packs(data) {
        Ok(packs) => packs,
        Err(e) => {
            log::warn!("custom font {origin}: its texture packs do not read: {e}");
            return None;
        }
    };
    packs.into_iter().flat_map(|p| p.textures).find(|t| t.name_hash == name_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bChunk file holding one `FEngFont` chunk: the name in the first 0x100 bytes, one glyph, no kerning.
    fn font_file(name: &str) -> Vec<u8> {
        let mut payload = vec![0u8; 0x200];
        payload[..name.len()].copy_from_slice(name.as_bytes());
        payload[0x100..0x100 + name.len()].copy_from_slice(name.as_bytes());
        let mut f = vec![0u8; 0x94];
        f[..4].copy_from_slice(b"FNTF");
        f[8..10].copy_from_slice(&414u16.to_le_bytes());
        f[10..12].copy_from_slice(&1u16.to_le_bytes());
        f[12..16].copy_from_slice(&0x40000i32.to_le_bytes());
        f[18] = 14;
        f[19] = 4;
        f[20..24].copy_from_slice(&0x80u32.to_le_bytes());
        f[24..28].copy_from_slice(&0x90u32.to_le_bytes());
        f[0x80..0x82].copy_from_slice(&65u16.to_le_bytes());
        f[0x82] = 10;
        f[0x83] = 14;
        f[0x8e..0x90].copy_from_slice(&12i16.to_le_bytes());
        payload.extend(f);
        chunk(FENG_FONT, &payload)
    }

    fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut out = id.to_le_bytes().to_vec();
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn hash(name: &str) -> u32 {
        blackbox_feng::fe_hash_upper(name)
    }

    #[test]
    fn a_custom_font_is_found_by_its_name_hash() {
        let mut set = CustomFontSet::default();
        set.add(Tier::Mid, "a.ffn", &font_file("font_mw_body"));
        let (tier, font) = set.font(hash("font_mw_body"), Tier::Mid).unwrap();
        assert_eq!((tier, font.name.as_str()), (Tier::Mid, "font_mw_body"));
        assert!(set.font(hash("font_other"), Tier::Mid).is_none());
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn the_tier_order_falls_back_to_lower_files_only() {
        let mut set = CustomFontSet::default();
        set.add(Tier::High, "high.ffn", &font_file("font_high_only"));
        set.add(Tier::Low, "low.ffn", &font_file("font_both"));
        set.add(Tier::High, "high-both.ffn", &font_file("font_both"));
        assert_eq!(set.font(hash("font_both"), Tier::High).map(|(t, _)| t), Some(Tier::High));
        assert_eq!(set.font(hash("font_both"), Tier::Mid).map(|(t, _)| t), Some(Tier::Low));
        assert_eq!(set.font(hash("font_both"), Tier::Low).map(|(t, _)| t), Some(Tier::Low));
        assert!(set.font(hash("font_high_only"), Tier::Mid).is_none(), "a High file is never used at Mid");
        assert!(set.font(hash("font_high_only"), Tier::Low).is_none());
    }

    #[test]
    fn the_first_file_of_a_name_in_one_tier_wins() {
        let mut set = CustomFontSet::default();
        set.add(Tier::Mid, "first.ffn", &font_file("font_dup"));
        set.add(Tier::Mid, "second.ffn", &font_file("font_dup"));
        assert_eq!(set.len(), 1);
        assert_eq!(set.font(hash("font_dup"), Tier::Mid).map(|(_, f)| f.name.clone()), Some("font_dup".to_owned()));
    }

    #[test]
    fn a_file_that_does_not_parse_adds_nothing() {
        let mut set = CustomFontSet::default();
        set.add(Tier::Mid, "garbage.ffn", &chunk(FENG_FONT, b"not a font at all"));
        set.add(Tier::Mid, "empty.ffn", &chunk(0x0003_4600, b"abcd"));
        set.add(Tier::Mid, "not-bchunk.ffn", &[1, 2]);
        assert!(set.is_empty());
    }
}
