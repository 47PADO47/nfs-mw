//! PC plain-pack texture UV scrolling; see `docs/specs/world-texture-scroll.md`.

use blackbox_chunk::ids;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureScroll {
    pub name_hash: u32,
    pub mode: u8,
    pub step: f32,
    pub speed: [f32; 2],
}

impl TextureScroll {
    pub fn offset(self, seconds: f32) -> [f32; 2] {
        if !seconds.is_finite() || (self.mode == 2 && self.step <= 0.0) {
            return [0.0; 2];
        }
        let t = if self.mode == 2 { (seconds / self.step).trunc() } else { seconds };
        self.speed.map(|speed| (t * speed).fract())
    }
}

pub fn read_texture_scrolls(data: &[u8]) -> Vec<TextureScroll> {
    let mut out = Vec::new();
    for infos in blackbox_chunk::find_all(data, ids::TEXTURE_PACK_INFO_TEXTURES) {
        for raw in infos.payload.as_chunks::<0x7c>().0 {
            let mode = raw[0x52];
            if !(1..=2).contains(&mode) {
                continue;
            }
            let word = |offset| i16::from_le_bytes([raw[offset], raw[offset + 1]]);
            out.push(TextureScroll {
                name_hash: u32::from_le_bytes(raw[0x24..0x28].try_into().unwrap()),
                mode,
                step: f32::from(word(0x58)) / 256.0,
                speed: [f32::from(word(0x5a)) / 1024.0, f32::from(word(0x5c)) / 1024.0],
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_and_snap_preserve_signed_wrap_and_speed() {
        let mut scroll = TextureScroll { name_hash: 1, mode: 1, step: 1.0, speed: [0.25, -0.9375] };
        assert_eq!(scroll.offset(2.0), [0.5, -0.875]);
        scroll.mode = 2;
        assert_eq!(scroll.offset(2.5), [0.5, -0.875]);
        scroll.step = 0.0;
        assert_eq!(scroll.offset(2.5), [0.0; 2]);
    }

    #[test]
    fn reads_plain_texture_scroll_fields() {
        let mut info = [0u8; 0x7c];
        info[0x24..0x28].copy_from_slice(&123u32.to_le_bytes());
        info[0x52] = 1;
        info[0x58..0x5a].copy_from_slice(&256i16.to_le_bytes());
        info[0x5c..0x5e].copy_from_slice(&960i16.to_le_bytes());
        let mut chunk = ids::TEXTURE_PACK_INFO_TEXTURES.to_le_bytes().to_vec();
        chunk.extend_from_slice(&(info.len() as u32).to_le_bytes());
        chunk.extend_from_slice(&info);
        let scroll = read_texture_scrolls(&chunk);
        assert_eq!(scroll.len(), 1);
        assert_eq!(scroll[0].name_hash, 123);
        assert_eq!(scroll[0].offset(1.0), [0.0, 0.9375]);
    }
}
