//! The helicopter's height sheet of a track: every `HeliSheet` chunk of `STREAM<track>.BUN`, one per drivable section
//! (`docs/formats/pursuit-data.md` section 1). The stream file is hundreds of megabytes, so the chunks are found by
//! walking the top-level headers with small reads, and only the sheet payloads are read.

use std::io::{self, Read, Seek, SeekFrom};

use blackbox_chunk::ids::HELI_SHEET;
use blackbox_roads::HeliSheet;
use game_install::GameDir;

/// The chunk header: id and size, eight bytes.
const HEADER: u64 = 8;
/// The id that marks a compressed blob in place of a header; the walk does not follow those.
const BARE_JDLZ: u32 = blackbox_chunk::BARE_JDLZ_ID;

/// Reads the sheet of the stream file `stream_file` (relative to the install). Sections whose sheet does not parse are
/// skipped and counted in the log.
pub fn read(dir: &GameDir, stream_file: &str) -> io::Result<HeliSheet> {
    let mut file = dir.open_file(stream_file)?;
    let len = file.metadata()?.len();
    let mut at = 0u64;
    let mut triangles = Vec::new();
    let mut skipped = 0usize;
    while at + HEADER <= len {
        file.seek(SeekFrom::Start(at))?;
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
        if id == HELI_SHEET {
            let mut payload = vec![0u8; size as usize];
            file.read_exact(&mut payload)?;
            match HeliSheet::parse_section(&payload) {
                Some((_, sheet)) => triangles.extend(sheet),
                None => skipped += 1,
            }
        }
        at += HEADER + size;
    }
    if skipped > 0 {
        log::warn!("helicopter sheet: {skipped} section sheets did not parse");
    }
    Ok(HeliSheet::new(triangles))
}
