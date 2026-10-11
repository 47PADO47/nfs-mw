//! Refuse game data, binaries and decompiler output.

use crate::git;

/// Extensions of game data, archives and executables. Never committed.
const FORBIDDEN_EXTENSIONS: &[&str] = &[
    // NFS:MW data
    "bun", "bin", "lzc", "bacc", "bak", "vp6", "mus", "mpf", "abk", "gin", "big", "csi", "evt", "fx", "mxb", "dyn",
    "loc", "vlt", "tpk", "fng", "sav", // Executables, debug info, disc images, archives
    "exe", "dll", "asi", "so", "dylib", "pdb", "xex", "elf", "dol", "iso", "7z", "zip", "rar",
    // Extracted media
    "dds", "wav", "png", "jpg", "jpeg", "tga", "bmp", "mp4", "avi",
];

/// File signatures of game data and executables.
const FORBIDDEN_MAGIC: &[&[u8]] = &[
    b"JDLZ", b"HUFF", b"RAWW", b"VPAK", b"MVhd", b"SCHl", b"ABKC", b"Gnsu", b"20CM", b"xDFP", b"MOIR", b"LOCH", b"MZ",
    b"\x7fELF",
];

const MAX_FILE_BYTES: usize = 512 * 1024;
/// The xtask source spells out the patterns it looks for, so its content is not scanned.
const SELF_PATH: &str = "xtask/src/leak.rs";
const ALLOW_MARKER: &str = "leak-check: allow";

/// `FUN_00401850`-style names that Ghidra/IDA give unnamed functions and data.
fn has_decompiler_name(line: &str) -> bool {
    ["FUN_", "DAT_", "LAB_", "PTR_", "sub_", "loc_", "off_", "dword_", "unk_"].iter().any(|prefix| {
        line.match_indices(prefix).any(|(i, _)| {
            let boundary = i == 0 || !matches!(line.as_bytes()[i - 1], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_');
            let hex = line[i + prefix.len()..].bytes().take_while(u8::is_ascii_hexdigit).count();
            boundary && hex >= 6
        })
    })
}

/// A hex literal in speed.exe's mapped address range (image base 0x400000, ends below 0xA00000).
fn has_exe_address(line: &str) -> bool {
    line.match_indices("0x").any(|(i, _)| {
        let digits: String = line[i + 2..].chars().take_while(char::is_ascii_hexdigit).collect();
        let next = line[i + 2 + digits.len()..].chars().next();
        if next.is_some_and(|c| c == '_' || c.is_ascii_alphanumeric()) || !(6..=8).contains(&digits.len()) {
            return false;
        }
        u32::from_str_radix(&digits, 16).is_ok_and(|v| (0x0040_0000..0x00A0_0000).contains(&v))
    })
}

fn check_file(path: &str, bytes: &[u8]) -> Vec<String> {
    // The sole media exception is this reviewed CC0 atlas, pinned byte-for-byte.
    // Updating the asset requires reviewing its source/licence and fingerprint here.
    if path == "assets/input-prompts/kenney-xbox.png" && asset_fingerprint(bytes) == 0xda7a_10ea_8ad2_ea78 {
        return Vec::new();
    }
    let mut problems = Vec::new();
    let name = path.rsplit('/').next().unwrap_or(path);
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    if name == ".env" {
        problems.push("local .env file (only .env.example belongs in the repo)".to_owned());
    }
    if let Some(ext) = ext.as_deref().filter(|e| FORBIDDEN_EXTENSIONS.contains(e)) {
        problems.push(format!("forbidden extension .{ext} (game data, binary or extracted media)"));
    }
    if let Some(magic) = FORBIDDEN_MAGIC.iter().find(|m| bytes.starts_with(m)) {
        problems.push(format!("starts with the signature {:?}", String::from_utf8_lossy(magic)));
    }
    if bytes.len() > MAX_FILE_BYTES && name != "Cargo.lock" {
        problems.push(format!("{} bytes is larger than the {} KiB limit", bytes.len(), MAX_FILE_BYTES / 1024));
    }
    if bytes.iter().take(8192).any(|&b| b == 0) {
        problems.push("binary content (NUL bytes)".to_owned());
        return problems;
    }
    let is_code = matches!(ext.as_deref(), Some("rs" | "wgsl" | "wesl" | "py")) && path != SELF_PATH;
    if is_code {
        for (n, line) in String::from_utf8_lossy(bytes).lines().enumerate() {
            if line.contains(ALLOW_MARKER) {
                continue;
            }
            if has_decompiler_name(line) {
                problems.push(format!("line {}: decompiler-generated name", n + 1));
            }
            if ext.as_deref() == Some("rs") && has_exe_address(line) {
                problems.push(format!(
                    "line {}: hex literal in speed.exe's address range (add `// {ALLOW_MARKER}` if it is not an address)",
                    n + 1
                ));
            }
        }
    }
    problems
}

fn asset_fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, b| (hash ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3))
}

pub fn run(staged: bool) -> Result<bool, String> {
    let files = git::files_to_check(staged)?;
    let mut clean = true;
    for path in &files {
        let Some(bytes) = git::read(path, staged)? else { continue };
        for problem in check_file(path, &bytes) {
            clean = false;
            eprintln!("leak-check: {path}: {problem}");
        }
    }
    if clean {
        eprintln!("leak-check: {} file(s) OK", files.len());
    }
    Ok(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_reviewed_kenney_atlas_is_a_media_exception() {
        let data = include_bytes!("../../assets/input-prompts/kenney-xbox.png");
        assert!(check_file("assets/input-prompts/kenney-xbox.png", data).is_empty());
        assert!(!check_file("assets/input-prompts/capture.png", data).is_empty());
        let mut altered = data.to_vec();
        altered[100] ^= 1;
        assert!(!check_file("assets/input-prompts/kenney-xbox.png", &altered).is_empty());
    }

    #[test]
    fn decompiler_names() {
        assert!(has_decompiler_name("    FUN_00401850();"));
        assert!(has_decompiler_name("x = DAT_008a1234;"));
        assert!(!has_decompiler_name("let fun_name = 1;"));
        assert!(!has_decompiler_name("MY_FUN_00401850")); // not at a word boundary
    }

    #[test]
    fn exe_addresses() {
        assert!(has_exe_address("call(0x401850)"));
        assert!(has_exe_address("const X: u32 = 0x007C4040;"));
        assert!(!has_exe_address("const ID: u32 = 0x8013_4000;"));
        assert!(!has_exe_address("let size = 0x409C;"));
        assert!(!has_exe_address("let big = 0x01000000;"));
    }

    #[test]
    fn game_files_are_refused() {
        assert!(!check_file("CARS/BMWM3GTR/GEOMETRY.BIN", b"\x00\x40\x13\x80").is_empty());
        assert!(!check_file("notes/blob.txt", b"JDLZ\x02\x10").is_empty());
        assert!(!check_file(".env", b"NFSMW_GAME_DIR=D:/x").is_empty());
        assert!(check_file(".env.example", b"NFSMW_GAME_DIR=D:/x").is_empty());
        assert!(check_file("crates/a/src/lib.rs", b"fn main() {}\n").is_empty());
        assert!(!check_file("libs/a/src/shader.wesl", b"fn x() { FUN_00401850(); }\n").is_empty());
    }
}
