//! M3U and M3U8 playlists and the choice of files in a folder. See the README.
//!
//! Nothing here opens a file: the caller reads the playlist bytes and the folder listing.

mod m3u;
mod scan;

pub use m3u::{Entry, decode_text, parse, resolve};
pub use scan::select;
