//! Reading the user's extracted Wwise files: banks and media under a folder that holds `WwiseAudio/` (the pak reader's
//! output for the paths `ride6/Content/WwiseAudio/...`, see `docs/modding/ride6-bikes-sound.md`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result, bail};

use super::bnk;
use super::data::Event;
use super::hirc::{self, Hirc, SoundRef};

/// Banks kept in memory at once, in bytes; the cache is dropped when it grows past this.
const CACHE_LIMIT: usize = 192 * 1024 * 1024;

/// The extracted files.
#[derive(Debug)]
pub struct Wwise {
    root: PathBuf,
    banks: HashMap<String, Rc<Vec<u8>>>,
    cached: usize,
}

impl Wwise {
    /// `root` is the folder that contains `WwiseAudio/` (for the pak reader's default output, `<out>/ride6/Content`).
    pub fn new(root: &Path) -> Result<Self> {
        if !root.join("WwiseAudio").is_dir() {
            bail!(
                "{} has no WwiseAudio folder: extract the banks first (docs/modding/ride6-bikes-sound.md)",
                root.display()
            );
        }
        Ok(Self { root: root.to_path_buf(), banks: HashMap::new(), cached: 0 })
    }

    /// A bank by its packaged path (`WwiseAudio/Event/Backfire_X.bnk`).
    pub fn bank(&mut self, file: &str) -> Result<Rc<Vec<u8>>> {
        if let Some(bank) = self.banks.get(file) {
            return Ok(Rc::clone(bank));
        }
        let path = self.root.join(file);
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        if self.cached + bytes.len() > CACHE_LIMIT {
            self.banks.clear();
            self.cached = 0;
        }
        self.cached += bytes.len();
        let bank = Rc::new(bytes);
        self.banks.insert(file.to_owned(), Rc::clone(&bank));
        Ok(bank)
    }

    /// The bytes of media `id`: embedded in `bank_file`, else the loose file `WwiseAudio/Media/<id>.wem`.
    pub fn media(&mut self, bank_file: &str, id: u32) -> Result<Vec<u8>> {
        let bank = self.bank(bank_file)?;
        if let Some(bytes) = bnk::media_by_id(&bank, id)? {
            return Ok(bytes.to_vec());
        }
        let loose = self.root.join("WwiseAudio").join("Media").join(format!("{id}.wem"));
        fs::read(&loose).with_context(|| format!("media {id} is neither in {bank_file} nor at {}", loose.display()))
    }

    /// The sounds an event plays, looked up in the banks the event names (the first that holds the event). Returns the
    /// bank file that held it too, where the sounds' media are looked up first.
    pub fn event_sounds(&mut self, event: &Event) -> Result<(String, Vec<SoundRef>)> {
        let mut last = None;
        for bank in &event.banks {
            let bytes = self.bank(&bank.file)?;
            let Some(chunk) = bnk::find_chunk(&bytes, b"HIRC")? else { continue };
            let hirc = Hirc::parse(chunk).with_context(|| format!("the object list of {}", bank.file))?;
            match hirc.event_sounds(event.event_id) {
                Ok(sounds) => return Ok((bank.file.clone(), sounds)),
                Err(error) => last = Some(error),
            }
        }
        match last {
            Some(error) => Err(error.context(format!("event {} ({})", event.name, event.event_id))),
            None => bail!("event {} names no bank with an object list", event.name),
        }
    }

    /// The media ids of the codec sounds in `sounds`, each once, in order.
    pub fn codec_media(sounds: &[SoundRef]) -> Vec<u32> {
        let mut out = Vec::new();
        for sound in sounds.iter().filter(|sound| hirc::is_codec(sound.plugin)) {
            if !out.contains(&sound.source) {
                out.push(sound.source);
            }
        }
        out
    }

    /// The media file of the first source plugin of `plugin_id` among `sounds` (the REV model): the plugin object is read
    /// from `bank_file`.
    pub fn plugin_media(&mut self, bank_file: &str, sounds: &[SoundRef], plugin_id: u32) -> Result<u32> {
        let Some(sound) = sounds.iter().find(|sound| sound.plugin == plugin_id) else {
            bail!("the event plays no sound of plugin {plugin_id:#x}")
        };
        let bytes = self.bank(bank_file)?;
        let Some(chunk) = bnk::find_chunk(&bytes, b"HIRC")? else { bail!("{bank_file} has no object list") };
        let object = Hirc::parse(chunk)?.plugin_object(sound.source)?;
        object.media.first().copied().with_context(|| format!("plugin object {} lists no media", sound.source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-wwise-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("WwiseAudio/Media")).unwrap();
        dir
    }

    fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn media_come_from_the_bank_first_then_the_loose_folder() {
        let dir = temp("media");
        let mut index = Vec::new();
        for value in [5u32, 0, 3] {
            index.extend_from_slice(&value.to_le_bytes());
        }
        let mut bank = chunk(b"BKHD", &[154, 0, 0, 0, 1, 0, 0, 0]);
        bank.extend(chunk(b"DIDX", &index));
        bank.extend(chunk(b"DATA", b"abc"));
        fs::write(dir.join("WwiseAudio/B.bnk"), bank).unwrap();
        fs::write(dir.join("WwiseAudio/Media/9.wem"), b"loose").unwrap();
        let mut wwise = Wwise::new(&dir).unwrap();
        assert_eq!(wwise.media("WwiseAudio/B.bnk", 5).unwrap(), b"abc");
        assert_eq!(wwise.media("WwiseAudio/B.bnk", 9).unwrap(), b"loose");
        assert!(wwise.media("WwiseAudio/B.bnk", 10).is_err());
        assert!(wwise.media("WwiseAudio/Missing.bnk", 5).is_err());
        assert!(Wwise::new(&dir.join("nowhere")).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn codec_media_are_listed_once_in_order() {
        let sound = |id, plugin, source| SoundRef { id, plugin, source };
        let sounds = [
            sound(1, 0x0004_0001, 30),
            sound(2, 0x01A0_1052, 31),
            sound(3, 0x0002_0001, 20),
            sound(4, 0x0004_0001, 30),
        ];
        assert_eq!(Wwise::codec_media(&sounds), [30, 20]);
    }
}
