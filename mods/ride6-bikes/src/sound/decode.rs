//! Decoding Wwise media with `vgmstream-cli` (ISC licence, <https://github.com/vgmstream/vgmstream>), a separate program
//! the user has: this repository neither contains it nor links it. It reads every Wwise codec of RIDE 6's banks and the
//! plugin media of the REV engine models (`-s <n>` picks the n-th recording of a model).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use super::wav::{self, Pcm};

/// The environment variable that names `vgmstream-cli` when `--vgmstream` is not given.
pub const ENV_NAME: &str = "VGMSTREAM_CLI";

/// A way to run `vgmstream-cli`, with a scratch folder for the files it reads and writes.
#[derive(Debug, Clone)]
pub struct Vgmstream {
    exe: PathBuf,
    work: PathBuf,
}

impl Vgmstream {
    /// `explicit`, else `$VGMSTREAM_CLI`, else `vgmstream-cli` on the path. `work` is created and holds only temporary files.
    pub fn new(explicit: Option<&Path>, work: &Path) -> Result<Self> {
        let exe = match explicit {
            Some(path) => path.to_path_buf(),
            None => std::env::var_os(ENV_NAME).map_or_else(|| PathBuf::from("vgmstream-cli"), PathBuf::from),
        };
        fs::create_dir_all(work).with_context(|| format!("creating {}", work.display()))?;
        Ok(Self { exe, work: work.to_path_buf() })
    }

    /// The scratch folder.
    pub fn work_dir(&self) -> &Path {
        &self.work
    }

    /// Decode the file `media` (or its `stream`-th recording, counted from 1) to PCM.
    pub fn decode_file(&self, media: &Path, stream: Option<usize>) -> Result<Pcm> {
        let name = format!("{}-{}.wav", std::process::id(), media.file_stem().and_then(|s| s.to_str()).unwrap_or("x"));
        let out = self.work.join(name);
        let mut command = Command::new(&self.exe);
        command.arg("-i").arg("-o").arg(&out);
        if let Some(stream) = stream {
            command.arg("-s").arg(stream.to_string());
        }
        command.arg(media).stdout(Stdio::null()).stderr(Stdio::piped());
        let output = command
            .output()
            .with_context(|| format!("starting {} (set --vgmstream or {ENV_NAME})", self.exe.display()))?;
        if !output.status.success() {
            bail!("vgmstream-cli failed on {}: {}", media.display(), String::from_utf8_lossy(&output.stderr).trim());
        }
        let bytes = fs::read(&out).with_context(|| format!("reading {}", out.display()))?;
        let _ = fs::remove_file(&out);
        wav::parse(&bytes).with_context(|| format!("decoding the output for {}", media.display()))
    }

    /// Decode media held in memory (written to the scratch folder first). `tag` makes the scratch name unique.
    pub fn decode_bytes(&self, tag: &str, bytes: &[u8], stream: Option<usize>) -> Result<Pcm> {
        let file = self.work.join(format!("{}-{tag}.wem", std::process::id()));
        fs::write(&file, bytes).with_context(|| format!("writing {}", file.display()))?;
        let result = self.decode_file(&file, stream);
        let _ = fs::remove_file(&file);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_program_is_an_error_that_names_the_setting() {
        let work = std::env::temp_dir().join(format!("ride6-bikes-decode-{}", std::process::id()));
        let vgm = Vgmstream::new(Some(Path::new("definitely-not-a-program-ride6")), &work).unwrap();
        let error = vgm.decode_bytes("t", b"RIFF", None).unwrap_err();
        assert!(format!("{error:#}").contains("--vgmstream"));
        fs::remove_dir_all(work).unwrap();
    }
}
