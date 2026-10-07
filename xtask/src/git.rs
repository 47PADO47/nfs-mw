//! Running git and listing the files a check should look at.

use std::process::Command;

pub fn git(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("git").args(args).output().map_err(|e| format!("running git: {e}"))?;
    if !out.status.success() {
        return Err(format!("git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(out.stdout)
}

pub fn lines(bytes: Vec<u8>) -> Vec<String> {
    String::from_utf8_lossy(&bytes).lines().filter(|l| !l.is_empty()).map(str::to_owned).collect()
}

/// Files to check: what is staged, or everything `git add .` could pick up.
pub fn files_to_check(staged: bool) -> Result<Vec<String>, String> {
    if staged {
        return Ok(lines(git(&["diff", "--cached", "--name-only", "--diff-filter=ACMR"])?));
    }
    let mut files = lines(git(&["ls-files"])?);
    files.extend(lines(git(&["ls-files", "--others", "--exclude-standard"])?));
    Ok(files)
}

/// The content to check: from the index when staged, else from the working tree.
pub fn read(path: &str, staged: bool) -> Result<Option<Vec<u8>>, String> {
    if staged {
        return git(&["show", &format!(":{path}")]).map(Some);
    }
    Ok(std::fs::read(path).ok()) // None: deleted in the working tree
}
