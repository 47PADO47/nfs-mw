//! An impact's stream sweeteners: the `aud_moment_strm` recordings an `audioimpact` adds to a heavy or scripted
//! hit (`StreamSweetner[]`). Spec: `docs/specs/engine-sound-effects.md` §8 ("Stream sweetener").

use blackbox_attrib::vlt_hash;

/// One `CollisionStream` record: the moment collection to stream and the lowest impact intensity (0 to 127) that
/// selects it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamSweetener {
    /// The `aud_moment_strm` collection key.
    pub moment: u32,
    pub threshold: u32,
}

impl StreamSweetener {
    /// A 16-byte `CollisionStream`: the moment's `RefSpec` (class, collection, unresolved pointer) and the
    /// threshold. Records that name another class or no collection are not sweeteners.
    pub(super) fn read(bytes: &[u8]) -> Option<Self> {
        let word = |at: usize| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
        let (class, moment, threshold) = (word(0)?, word(4)?, word(12)?);
        if class != vlt_hash("aud_moment_strm") || moment == 0 {
            return None;
        }
        Some(Self { moment, threshold })
    }

    /// The sweetener an impact of `intensity` (0 to 127) starts: the first in list order whose threshold the
    /// intensity reaches.
    pub fn select(sweeteners: &[Self], intensity: i32) -> Option<Self> {
        sweeteners.iter().copied().find(|s| i64::from(s.threshold) <= i64::from(intensity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(class: u32, moment: u32, threshold: u32) -> Vec<u8> {
        [class, moment, 0, threshold].into_iter().flat_map(u32::to_le_bytes).collect()
    }

    #[test]
    fn records_name_a_moment_collection_and_its_threshold() {
        let moment = vlt_hash("aud_moment_strm");
        assert_eq!(
            StreamSweetener::read(&record(moment, 0x5AEF_C6A8, 0)),
            Some(StreamSweetener { moment: 0x5AEF_C6A8, threshold: 0 })
        );
        assert_eq!(StreamSweetener::read(&record(vlt_hash("audioimpact"), 7, 0)), None, "another class");
        assert_eq!(StreamSweetener::read(&record(moment, 0, 0)), None, "no collection");
        assert_eq!(StreamSweetener::read(&record(moment, 7, 0)[..12]), None, "truncated");
    }

    #[test]
    fn the_first_reached_threshold_in_list_order_wins() {
        let list = [
            StreamSweetener { moment: 1, threshold: 100 },
            StreamSweetener { moment: 2, threshold: 40 },
            StreamSweetener { moment: 3, threshold: 0 },
        ];
        assert_eq!(StreamSweetener::select(&list, 127).map(|s| s.moment), Some(1));
        assert_eq!(StreamSweetener::select(&list, 60).map(|s| s.moment), Some(2));
        assert_eq!(StreamSweetener::select(&list, 0).map(|s| s.moment), Some(3));
        assert_eq!(StreamSweetener::select(&list[..2], 10), None);
        assert_eq!(StreamSweetener::select(&[], 127), None);
    }
}
