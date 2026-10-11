//! Keep files small: no source or doc file may exceed `MAX_LINES` lines.
//! Split large files into modules (see CONTRIBUTING.md).

use crate::git;

pub const MAX_LINES: usize = 500;

/// Extensions the limit applies to.
const CHECKED: &[&str] = &["rs", "wgsl", "wesl", "py", "md", "toml", "yml"];

/// Generated or vendored files exempt from the limit.
const EXEMPT: &[&str] = &["Cargo.lock", "LICENSE-APACHE"];

fn too_long(path: &str, bytes: &[u8]) -> Option<usize> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase())?;
    if EXEMPT.contains(&name) || !CHECKED.contains(&ext.as_str()) {
        return None;
    }
    let lines = bytes.iter().filter(|&&b| b == b'\n').count();
    (lines > MAX_LINES).then_some(lines)
}

pub fn run(staged: bool) -> Result<bool, String> {
    let files = git::files_to_check(staged)?;
    let mut clean = true;
    for path in &files {
        let Some(bytes) = git::read(path, staged)? else { continue };
        if let Some(lines) = too_long(path, &bytes) {
            clean = false;
            eprintln!("size-check: {path}: {lines} lines (limit {MAX_LINES}); split it into modules");
        }
    }
    if clean {
        eprintln!("size-check: {} file(s) OK", files.len());
    }
    Ok(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits() {
        let long = "x\n".repeat(MAX_LINES + 1);
        assert_eq!(too_long("a/b.rs", long.as_bytes()), Some(MAX_LINES + 1));
        assert_eq!(too_long("a/b.rs", "x\n".repeat(MAX_LINES).as_bytes()), None);
        assert_eq!(too_long("Cargo.lock", long.as_bytes()), None);
        assert_eq!(too_long("image.svg", long.as_bytes()), None);
        assert_eq!(too_long("a/shader.wesl", long.as_bytes()), Some(MAX_LINES + 1));
    }
}
