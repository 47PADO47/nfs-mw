//! The helicopter's height sheet of a track: every `HeliSheet` chunk of `STREAM<track>.BUN`, one per drivable section
//! (`docs/formats/pursuit-data.md` section 1). The stream file is hundreds of megabytes, so the chunks are found by
//! walking the top-level headers with small reads, and only the sheet payloads are read.

use std::io::{self, BufReader, Read};

use blackbox_chunk::ids::HELI_SHEET;
use blackbox_roads::HeliSheet;
use game_install::GameDir;

/// The chunk header: id and size, eight bytes.
const HEADER: u64 = 8;
/// The read buffer. Most top-level chunks are a few hundred bytes to a few kilobytes, so one fill serves many headers
/// and the walk skips inside the buffer instead of seeking the file chunk by chunk.
const BUFFER: usize = 256 * 1024;
/// The id that marks a compressed blob in place of a header; the walk does not follow those.
const BARE_JDLZ: u32 = blackbox_chunk::BARE_JDLZ_ID;

/// [`read`] once per session: later calls while the file is unchanged (same resolved path, size and modification
/// time, see [`crate::memo`]) get a copy of the first read instead of walking the stream file again.
pub fn cached(dir: &GameDir, stream_file: &str) -> anyhow::Result<HeliSheet> {
    crate::memo(dir, &[stream_file], || Ok(Memo(read(dir, stream_file)?))).map(|hit| hit.0.clone())
}

/// The memo entry type of [`shared`], private so no other memo of a `HeliSheet` shares its entries.
struct Memo(HeliSheet);

/// Reads the sheet of the stream file `stream_file` (relative to the install). Sections whose sheet does not parse are
/// skipped and counted in the log.
pub fn read(dir: &GameDir, stream_file: &str) -> io::Result<HeliSheet> {
    let file = dir.open_file(stream_file)?;
    let len = file.metadata()?.len();
    let mut file = BufReader::with_capacity(BUFFER, file);
    let mut at = 0u64;
    let mut triangles = Vec::new();
    let mut skipped = 0usize;
    while at + HEADER <= len {
        let mut header = [0u8; 8];
        file.read_exact(&mut header)?;
        let id = u32::from_le_bytes(header[0..4].try_into().expect("four bytes"));
        let size = u64::from(u32::from_le_bytes(header[4..8].try_into().expect("four bytes")));
        if id == BARE_JDLZ {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "compressed chunks at the top level are not walked",
            ));
        }
        if at + HEADER + size > len {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("chunk at {at} overruns the file")));
        }
        if id != HELI_SHEET {
            // Skipped inside the buffer when the payload is short enough, else the file is advanced.
            file.seek_relative(i64::try_from(size).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?)?;
            at += HEADER + size;
            continue;
        }
        let mut payload = vec![0u8; size as usize];
        file.read_exact(&mut payload)?;
        match HeliSheet::parse_section(&payload) {
            Some((_, sheet)) => triangles.extend(sheet),
            None => skipped += 1,
        }
        at += HEADER + size;
    }
    if skipped > 0 {
        log::warn!("helicopter sheet: {skipped} section sheets did not parse");
    }
    Ok(HeliSheet::new(triangles))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stream file of one unrelated chunk and one `HeliSheet` chunk holding a single triangle.
    fn stream() -> Vec<u8> {
        let mut sheet = vec![0x11; 4];
        sheet.extend_from_slice(&[11, 0, 0, 0, 11, 0, 0, 0]);
        sheet.extend_from_slice(&7u32.to_le_bytes());
        sheet.extend_from_slice(&1u32.to_le_bytes());
        sheet.extend_from_slice(&[0; 8]);
        for word in [0i16, 160, 0, 0, 0, 160, 160, 160, 320] {
            sheet.extend_from_slice(&word.to_le_bytes());
        }
        sheet.extend_from_slice(&[0, 0]);
        let mut out = Vec::new();
        out.extend_from_slice(&0x1234_5678u32.to_le_bytes());
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&[1, 2, 3, 4]);
        out.extend_from_slice(&HELI_SHEET.to_le_bytes());
        out.extend_from_slice(&(sheet.len() as u32).to_le_bytes());
        out.extend_from_slice(&sheet);
        out
    }

    #[test]
    fn cached_sheet_equals_a_fresh_read_on_every_call() {
        let root = std::env::temp_dir().join(format!("nfsmw-data-heli-sheet-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("TRACKS")).unwrap();
        std::fs::write(root.join("TRACKS/STREAML2RA.BUN"), stream()).unwrap();
        let dir = GameDir::open(&root).unwrap();
        let fresh = read(&dir, "TRACKS/STREAML2RA.BUN").unwrap();
        assert_eq!(fresh.len(), 1);
        assert_eq!(cached(&dir, "TRACKS/STREAML2RA.BUN").unwrap(), fresh);
        assert_eq!(cached(&dir, "TRACKS/STREAML2RA.BUN").unwrap(), fresh, "a memo hit is the same sheet");
        std::fs::remove_dir_all(root).unwrap();
    }
}
