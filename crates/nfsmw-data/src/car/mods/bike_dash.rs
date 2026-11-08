//! The live screen of a bike's instrument cluster (`[bike.dash]` of its manifest, `docs/modding/ride6-bike-dash.md`): which texture of
//! the car's pack is the screen, which part of it is the display and the colours to paint with, by car folder.

use std::sync::{Arc, PoisonError, RwLock};

use blackbox_hash::bstring_hash;

/// The colours of the painted panel, texel colours `[r, g, b]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    pub background: [u8; 3],
    pub lit: [u8; 3],
    pub dim: [u8; 3],
    pub text: [u8; 3],
    pub label: [u8; 3],
    pub gear: [u8; 3],
}

/// The screen of one bike's cluster.
#[derive(Debug, Clone, PartialEq)]
pub struct Dash {
    /// The texture's name in the car's pack and its hash (`bstring_hash` of the name, as the solids list their textures).
    pub texture: String,
    pub texture_hash: u32,
    /// The display in the texture, `[x, y, width, height]` texels from the top left.
    pub window: [u32; 4],
    pub colors: Colors,
}

impl Dash {
    pub fn new(texture: &str, window: [u32; 4], colors: Colors) -> Self {
        Self { texture: texture.to_owned(), texture_hash: bstring_hash(texture), window, colors }
    }
}

/// The screens of this run's bikes, by folder. Each publication leaks the new list, as the other per-car tables do.
static PUBLISHED: RwLock<&'static [(String, Arc<Dash>)]> = RwLock::new(&[]);

/// Register the screen of the bike in `folder`. A folder already registered keeps its first.
pub fn publish(folder: &str, dash: Dash) {
    let mut published = PUBLISHED.write().unwrap_or_else(PoisonError::into_inner);
    if published.iter().any(|(known, _)| known.eq_ignore_ascii_case(folder)) {
        return;
    }
    let mut all: Vec<(String, Arc<Dash>)> = published.to_vec();
    all.push((folder.to_owned(), Arc::new(dash)));
    *published = Box::leak(all.into_boxed_slice());
}

/// The screen of the bike in `folder`; `None` for a car and for a bike without a live screen.
pub fn of(folder: &str) -> Option<Arc<Dash>> {
    let published = *PUBLISHED.read().unwrap_or_else(PoisonError::into_inner);
    published.iter().find(|(known, _)| known.eq_ignore_ascii_case(folder)).map(|(_, dash)| Arc::clone(dash))
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: Colors = Colors {
        background: [1, 2, 3],
        lit: [4, 5, 6],
        dim: [7, 8, 9],
        text: [10, 11, 12],
        label: [13, 14, 15],
        gear: [16, 17, 18],
    };

    #[test]
    fn a_published_screen_is_found_by_folder_without_regard_to_case_and_the_first_publication_wins() {
        publish("TESTBIKE_DASH", Dash::new("TESTBIKE_DASH_GAUGE1_D", [4, 8, 64, 32], COLORS));
        publish("testbike_dash", Dash::new("OTHER", [0, 0, 8, 8], COLORS));
        let found = of("testbike_Dash").expect("published");
        assert_eq!(found.window, [4, 8, 64, 32]);
        assert_eq!(found.texture_hash, bstring_hash("TESTBIKE_DASH_GAUGE1_D"));
        assert!(of("NOT_A_BIKE").is_none());
    }
}
