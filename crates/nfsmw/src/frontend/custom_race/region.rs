//! The regions the track list filters by (`docs/specs/custom-race.md` section 3).

/// The region of a race, from its data's `Region` text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    College,
    Coastal,
    City,
}

impl Region {
    pub fn of(text: &str) -> Option<Region> {
        match text.to_ascii_lowercase().as_str() {
            "college" => Some(Region::College),
            "coastal" => Some(Region::Coastal),
            "city" => Some(Region::City),
            _ => None,
        }
    }
}

/// The filter of the track list: one region or all of them. The numbers are the original's `RegionFilterBits` (0 College,
/// 1 Coastal, 2 City, 3 All Regions) **[decomp]**; the shoulder buttons step through them with wrap-around.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Filter(u8);

impl Filter {
    pub const ALL: Filter = Filter(3);

    /// The filter of a number; anything past All Regions reads as All Regions.
    #[cfg(test)]
    pub fn from_index(index: u8) -> Filter {
        Filter(index.min(3))
    }

    /// Whether a race of this region is listed.
    pub fn admits(self, region: Option<Region>) -> bool {
        matches!(
            (self.0, region),
            (3, _) | (0, Some(Region::College)) | (1, Some(Region::Coastal)) | (2, Some(Region::City))
        )
    }

    /// The next filter (`forward`) or the previous one, wrapping.
    pub fn step(self, forward: bool) -> Filter {
        Filter(match forward {
            true => (self.0 + 1) % 4,
            false => (self.0 + 3) % 4,
        })
    }

    /// The language label of the filter's name: College, Coastal, City, All Regions **[exe]**.
    pub fn label(self) -> u32 {
        match self.0 {
            0 => 0xA685_0651,
            1 => 0xA5C2_0E7D,
            2 => 0x8663_FAEF,
            _ => 0x632D_D19B,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filter_wraps_and_admits_by_region() {
        assert_eq!(Filter::ALL.step(true), Filter::from_index(0));
        assert_eq!(Filter::from_index(0).step(false), Filter::ALL);
        assert!(Filter::from_index(2).admits(Some(Region::City)));
        assert!(!Filter::from_index(2).admits(Some(Region::College)));
        assert!(!Filter::from_index(1).admits(None));
        assert!(Filter::ALL.admits(None));
        assert_eq!(Filter::from_index(9), Filter::ALL);
    }

    #[test]
    fn the_data_names_map_to_regions() {
        assert_eq!(Region::of("College"), Some(Region::College));
        assert_eq!(Region::of("coastal"), Some(Region::Coastal));
        assert_eq!(Region::of("nowhere"), None);
    }
}
