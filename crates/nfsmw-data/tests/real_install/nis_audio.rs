//! The NIS audio streams: the take of each scene's stream is found in the install and decodes to PCM.

use ea_audio::ReadAt;
use ea_audio::speech::{Sentences, SpeechIndex};
use game_install::GameDir;
use nfsmw_data::sound::nis_take;

use crate::install;

const INDEX: &str = "SOUND/STREAMS/NISAudio.idx";
const EVENTS: &str = "SOUND/STREAMS/NISAudio.evt";
const STREAM: &str = "SOUND/STREAMS/NISAudio.big";

/// `NISAudio.big`, read on demand: the file is about 184 MB.
struct Stream<'a> {
    dir: &'a GameDir,
    len: u64,
}

impl ReadAt for Stream<'_> {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> usize {
        let bytes = self.dir.read_range(STREAM, offset, buf.len()).unwrap_or_default();
        buf[..bytes.len()].copy_from_slice(&bytes);
        bytes.len()
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_nis_scenes_find_and_decode_their_audio_takes() {
    let Some(dir) = install() else { return };
    let index = SpeechIndex::parse(&dir.read(INDEX).expect("NISAudio.idx")).expect("NISAudio.idx parses");
    let events = Sentences::parse(&dir.read(EVENTS).expect("NISAudio.evt")).expect("NISAudio.evt parses");
    let len = std::fs::metadata(dir.resolve(STREAM).expect("NISAudio.big")).expect("NISAudio.big metadata").len();
    let big = Stream { dir: &dir, len };
    for scene in ["IntroNis05", "IntroNisBL02", "EndingNis04", "ArrestF02"] {
        let take = nis_take(&index, &events, scene).unwrap_or_else(|| panic!("{scene}: no take"));
        let pcm = index.banks()[take.bank].decode(&big, take.take).unwrap_or_else(|e| panic!("{scene}: {e:?}"));
        assert!(pcm.frames() > 0 && !pcm.samples.is_empty(), "{scene}: empty take");
        println!(
            "{scene}: bank {} take {} (exact {}), {:.2} s, {} Hz, {} ch",
            take.bank,
            take.take,
            take.exact,
            pcm.duration_secs(),
            pcm.sample_rate,
            pcm.channels
        );
    }
}
