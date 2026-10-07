//! Animated textures (`TextureAnimPack`, `0xB0300100`): a texture whose image
//! cycles through other textures (water, flashing lights, signals).
//! Spec: `docs/formats/textures.md` ("Animated textures").

use blackbox_chunk::ids;

/// One animation. Drawing code references the animation by `name_hash` (the
/// first frame's hash) and shows `frames[(time * fps) % frames.len()]`.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureAnim {
    pub name: String,
    pub name_hash: u32,
    pub frames_per_second: u32,
    /// Texture name hashes, in order.
    pub frames: Vec<u32>,
}

impl TextureAnim {
    /// The frame to show `seconds` after the animation started.
    pub fn frame_at(&self, seconds: f32) -> Option<u32> {
        if self.frames.is_empty() {
            return None;
        }
        let frame = (seconds.max(0.0) * self.frames_per_second as f32) as usize % self.frames.len();
        Some(self.frames[frame])
    }
}

/// Size of a `TextureAnim` record and of a frame entry (NFS: Most Wanted, PC).
const ANIM_LEN: usize = 0x34;
const ENTRY_LEN: usize = 0x10;

/// Every animation in every `TextureAnimPack` of `data`.
pub fn read_texture_anims(data: &[u8]) -> Vec<TextureAnim> {
    let mut out = Vec::new();
    for pack in blackbox_chunk::find_all(data, ids::TEXTURE_ANIM_PACK) {
        let (Some(anims), Some(entries)) =
            (pack.child(ids::TEXTURE_ANIM_PACK_ANIMS), pack.child(ids::TEXTURE_ANIM_PACK_FRAMES))
        else {
            continue;
        };
        // Frames of all animations are stored back to back, in animation order.
        let mut frames = entries.payload.chunks_exact(ENTRY_LEN).map(|e| u32_at(e, 0));
        for a in anims.payload.chunks_exact(ANIM_LEN) {
            let count = u32_at(a, 0x1C) as usize;
            let name_len = a[..24].iter().position(|&b| b == 0).unwrap_or(24);
            out.push(TextureAnim {
                name: String::from_utf8_lossy(&a[..name_len]).into_owned(),
                name_hash: u32_at(a, 0x18),
                frames_per_second: u32_at(a, 0x20),
                frames: frames.by_ref().take(count).collect(),
            });
        }
    }
    out
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = id.to_le_bytes().to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn reads_frames_in_order() {
        let mut anim = vec![0u8; ANIM_LEN];
        anim[..6].copy_from_slice(b"WATER_");
        anim[0x18..0x1C].copy_from_slice(&0xAAu32.to_le_bytes());
        anim[0x1C..0x20].copy_from_slice(&3u32.to_le_bytes());
        anim[0x20..0x24].copy_from_slice(&10u32.to_le_bytes());
        let entries: Vec<u8> =
            [0xAAu32, 0xBB, 0xCC].iter().flat_map(|h| [h.to_le_bytes(), [0; 4], [0; 4], [0; 4]].concat()).collect();
        let body = [
            chunk(ids::TEXTURE_ANIM_PACK_HEADER, &[1, 0, 0, 0]),
            chunk(ids::TEXTURE_ANIM_PACK_ANIMS, &anim),
            chunk(ids::TEXTURE_ANIM_PACK_FRAMES, &entries),
        ]
        .concat();
        let anims = read_texture_anims(&chunk(ids::TEXTURE_ANIM_PACK, &body));
        assert_eq!(anims.len(), 1);
        let a = &anims[0];
        assert_eq!((a.name.as_str(), a.name_hash, a.frames_per_second), ("WATER_", 0xAA, 10));
        assert_eq!(a.frames, [0xAA, 0xBB, 0xCC]);
        assert_eq!(a.frame_at(0.0), Some(0xAA));
        assert_eq!(a.frame_at(0.25), Some(0xCC));
        assert_eq!(a.frame_at(0.35), Some(0xAA));
    }
}
