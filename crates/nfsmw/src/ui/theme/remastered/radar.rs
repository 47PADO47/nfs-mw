//! The Remastered police detector: a slim dark pill at the top of the screen with a row of LED dashes at each side and a dial
//! in the middle. When a cop is within range and in front of the car, the dashes light outward from the dial in a sweep that
//! runs faster the nearer the cop is, the dial's arrow points at it, and the dashes turn red when it is close. It shows while the
//! player is free or cooling down, and gives way to the pursuit panel while a chase runs.

use blackbox_ui::{Component, DrawList, Image, LayoutCx, NodeKind, Outline, Rect, Ring, Rounded, UiTree};

use super::group::Group;
use super::skin::keys;
use crate::hud::RadarHud;
use crate::ui::theme::custom::DriveFeedCell;
use crate::ui::theme::original::{FengSource, FrameCell};

/// The pill's centre, size and edge, in FEng units.
const CENTRE: (f32, f32) = (0.0, -207.0);
const WIDTH: f32 = 258.0;
const HEIGHT: f32 = 26.0;
const EDGE: f32 = 0.9;
/// The LED dashes: how many a side, their size, the distance between their starts, and the gap between the dial and the first.
const DASHES: usize = 14;
const DASH_WIDTH: f32 = 5.6;
const DASH_HEIGHT: f32 = 4.4;
const DASH_PITCH: f32 = 7.2;
const DASH_START: f32 = 22.0;
/// The dial: its radius, the arc's width and half sweep (the detector's cone), and the arrow's size.
const DIAL: f32 = 13.0;
const DIAL_WIDTH: f32 = 1.1;
const CONE_DEGREES: f32 = 45.0;
const ARROW: f32 = 20.0;
/// The pill is dimmed while the detector sees nothing.
const IDLE_OPACITY: f32 = 0.5;
/// A cop nearer than this share of the range turns the dashes red.
const NEAR: f32 = 0.33;
/// The first reveal of a sweep, and how it grows over the sweep's period: the original's ramp.
const FIRST_REVEAL: f32 = 0.3;
/// The sweep's period per unit of range, and its least range.
const PERIOD: f32 = 1.5;
const LEAST_RANGE: f32 = 0.1;
/// The package's own direction arrow, by object name: the texture the arrow reuses.
const ARROW_OBJECT: u32 = 0x2419_6AD4;
/// The depth of the pill: in front of the world and behind the other HUD pieces.
const DEPTH: f32 = 60.0;

pub struct PoliceRadar {
    frame: FrameCell,
    feed: DriveFeedCell,
    parts: Vec<Box<dyn Component>>,
}

impl PoliceRadar {
    pub fn new(frame: FrameCell, feed: DriveFeedCell) -> Self {
        Self { frame, feed, parts: Vec::new() }
    }
}

impl Component for PoliceRadar {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts.clear();
        let feed = self.feed.snapshot();
        // A chase has its own panel; only a free player, or one who has got away, has the detector.
        if feed.pursuit.is_some_and(|pursuit| !pursuit.cooldown_shown()) {
            return;
        }
        let texture = self.frame.get(FengSource::Hud).and_then(|tree| arrow_texture(&tree));
        let (parts, opacity) = parts(feed.radar, texture, cx);
        self.parts = vec![Box::new(Group::new(parts, opacity))];
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// How much of a side's dashes are lit `time_s` seconds in, for a cop at `range` (0 to 1): the original's reveal, from its first
/// share up to all of them over a sweep that is longer the farther the cop is.
pub(super) fn reveal(range: f32, time_s: f32) -> f32 {
    let period = range.max(LEAST_RANGE) * PERIOD;
    let phase = (time_s.rem_euclid(period) / period).clamp(0.0, 1.0);
    FIRST_REVEAL + (1.0 - FIRST_REVEAL) * phase
}

/// How many dashes a side lights for a reveal of `share`.
pub(super) fn lit(share: f32) -> usize {
    (share.clamp(0.0, 1.0) * DASHES as f32).ceil() as usize
}

fn parts(radar: Option<RadarHud>, texture: Option<u32>, cx: &LayoutCx<'_>) -> (Vec<Box<dyn Component>>, f32) {
    let (x, y) = CENTRE;
    let mut parts: Vec<Box<dyn Component>> = Vec::new();
    let pill = Rect::new(x, y, WIDTH, HEIGHT);
    parts.push(Box::new(Rounded::at_depth(keys::MINIMAP_BEZEL, pill, HEIGHT / 2.0, DEPTH + 10.0, cx)));
    parts.push(Box::new(Outline::new(keys::BAR_BORDER, pill, HEIGHT / 2.0, EDGE, cx)));
    // The dial: the cone the detector sees, and a dot at its foot.
    let foot = (x, y + DIAL * 0.55);
    parts.push(Box::new(Ring::at_depth(
        keys::BAR_BORDER,
        foot,
        DIAL,
        DIAL_WIDTH,
        (-CONE_DEGREES, CONE_DEGREES),
        DEPTH,
        cx,
    )));
    let Some(radar) = radar else { return (parts_with_dashes(parts, 0, keys::MINIMAP_HEAT_DIM, cx), IDLE_OPACITY) };
    let range = radar.normalized_range();
    let colour = if range < NEAR { keys::PURSUIT_BUSTED } else { keys::MINIMAP_HEAT };
    let lit = lit(reveal(range, cx.time_s));
    let mut parts = parts_with_dashes(parts, lit, colour, cx);
    if let Some(texture) = texture {
        // The package's arrow points up and turns clockwise on the screen; the feed's angle is positive to the left.
        let mut arrow = Image::raw(texture).with_tint(cx.skin.colour(colour).unwrap_or([255; 4])).with_z(DEPTH - 10.0);
        arrow = arrow.with_rotation(-radar.angle);
        arrow.layout(cx, Rect::new(foot.0, foot.1 - DIAL * 0.5, ARROW, ARROW));
        parts.push(Box::new(arrow));
    }
    (parts, 1.0)
}

/// Adds both rows of dashes, `lit` of each side in `colour` from the dial outward and the rest dim.
fn parts_with_dashes(
    mut parts: Vec<Box<dyn Component>>,
    lit: usize,
    colour: &'static str,
    cx: &LayoutCx<'_>,
) -> Vec<Box<dyn Component>> {
    for side in [-1.0_f32, 1.0] {
        for i in 0..DASHES {
            let key = if i < lit { colour } else { keys::MINIMAP_HEAT_DIM };
            let centre = CENTRE.0 + side * (DASH_START + DASH_PITCH * i as f32 + DASH_WIDTH / 2.0);
            let rect = Rect::new(centre, CENTRE.1, DASH_WIDTH, DASH_HEIGHT);
            parts.push(Box::new(Rounded::at_depth(key, rect, 1.0, DEPTH, cx)));
        }
    }
    parts
}

/// The direction arrow's texture: the one the package's own arrow draws.
fn arrow_texture(tree: &UiTree) -> Option<u32> {
    let node = tree.nodes.iter().find(|node| node.name_hash == ARROW_OBJECT)?;
    match node.kind {
        NodeKind::Image { texture, .. } => Some(texture),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use blackbox_ui::{BuildCx, Theme, UiNode};
    use glam::Vec3;

    use super::*;
    use crate::hud::{HudState, PursuitHud};
    use crate::ui::present::Screen;
    use crate::ui::theme::custom::palette;
    use crate::ui::theme::remastered::RemasteredTheme;
    use crate::ui::theme::remastered::skin::{palette_skin, skin};
    use crate::ui::theme::{ElementId, MenuListCell};

    const SCREEN: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };

    fn arrow_tree() -> UiTree {
        let mut node = blackbox_ui::UiNode { index: 0, name_hash: ARROW_OBJECT, visible: true, ..Default::default() };
        node.kind = NodeKind::Image {
            texture: 77,
            uv: [0.0, 0.0, 1.0, 1.0],
            mask: None,
            mask_rotation: [0.5, 0.5, 0.0],
            mask_uv: [0.0; 4],
        };
        let mut list = DrawList::new();
        list.push(node);
        list.into_tree(|_| true)
    }

    fn built(state: &HudState, time_s: f32) -> Vec<UiNode> {
        let frame = FrameCell::new();
        frame.set(FengSource::Hud, arrow_tree());
        let feed = DriveFeedCell::new();
        feed.set(state, SCREEN);
        let skin = palette::remastered().skin().merged(skin()).merged(palette_skin(&palette::remastered()));
        let theme = RemasteredTheme::new(frame, feed, Default::default(), MenuListCell::new(), Default::default());
        let mut radar =
            theme.element(ElementId::PoliceRadar.slot(), &BuildCx::new(&skin)).expect("Remastered fills it");
        radar.layout(&LayoutCx::new(&skin, time_s), Rect::SCREEN);
        let mut out = DrawList::new();
        radar.build(&mut out);
        out.iter().cloned().collect()
    }

    /// The dashes lit in `key`'s colour on each side of the dial: the index of each dash, from the dial outward.
    fn lit_dashes(nodes: &[UiNode], key: &str) -> [Vec<usize>; 2] {
        let colour =
            palette::remastered().skin().merged(skin()).merged(palette_skin(&palette::remastered())).colour(key);
        let mut sides = [Vec::new(), Vec::new()];
        for node in nodes.iter().filter(|n| Some(n.colour) == colour && n.world.x_axis.x.abs() < DASH_WIDTH + 0.1) {
            let from_dial = node.world.w_axis.x - CENTRE.0;
            let index = ((from_dial.abs() - DASH_START) / DASH_PITCH).floor() as usize;
            let side = &mut sides[usize::from(from_dial > 0.0)];
            if !side.contains(&index) {
                side.push(index);
            }
        }
        sides
    }

    #[test]
    fn the_reveal_starts_at_its_first_share_and_grows_over_a_period_that_follows_the_range() {
        assert!((reveal(0.5, 0.0) - FIRST_REVEAL).abs() < 1e-6);
        let period = 0.5 * PERIOD;
        assert!((reveal(0.5, period * 0.5) - (FIRST_REVEAL + (1.0 - FIRST_REVEAL) * 0.5)).abs() < 1e-5);
        assert!((reveal(0.5, period) - FIRST_REVEAL).abs() < 1e-5, "the sweep starts over");
        assert!(reveal(0.05, 0.05) > reveal(1.0, 0.05), "a near cop sweeps faster");
        assert_eq!((lit(0.0), lit(0.3), lit(1.0), lit(2.0)), (0, 5, DASHES, DASHES));
    }

    #[test]
    fn with_no_cop_the_pill_is_dim_and_lights_nothing_and_a_cop_lights_both_rows_and_the_arrow() {
        let free = built(&HudState::default(), 0.2);
        assert!(!free.is_empty());
        assert!(free.iter().all(|n| n.colour[3] <= 128), "the idle pill is dimmed");
        assert!(!free.iter().any(|n| matches!(n.kind, NodeKind::Image { texture: 77, .. })), "no arrow without a cop");
        let state = HudState { radar: Some(RadarHud { angle: 0.4, distance: 200.0 }), ..Default::default() };
        let seen = built(&state, 0.0);
        let arrow = seen.iter().find(|n| matches!(n.kind, NodeKind::Image { texture: 77, .. })).expect("the arrow");
        // Turned clockwise on the screen for a cop at the left would be negative: the feed's left-positive angle is negated.
        let turn = arrow.world.transform_vector3(Vec3::X);
        assert!(turn.y.abs() > 0.0 && turn.x > 0.0, "the arrow is turned: {turn:?}");
        let [left, right] = lit_dashes(&seen, keys::MINIMAP_HEAT);
        assert_eq!(left.len(), lit(reveal(200.0 / 300.0, 0.0)));
        assert_eq!(left.len(), right.len(), "the same dashes light on both sides");
        assert_eq!(left.iter().max(), Some(&(left.len() - 1)), "they light outward from the dial with no gap");
    }

    #[test]
    fn a_near_cop_turns_the_dashes_red_and_a_chase_hides_the_detector() {
        let near = HudState { radar: Some(RadarHud { angle: 0.0, distance: 30.0 }), ..Default::default() };
        let seen = built(&near, 0.0);
        assert!(!lit_dashes(&seen, keys::PURSUIT_BUSTED)[0].is_empty());
        assert!(lit_dashes(&seen, keys::MINIMAP_HEAT)[0].is_empty());
        let chase = HudState { pursuit: Some(PursuitHud { secs: 3.0, ..Default::default() }), ..near.clone() };
        assert!(built(&chase, 0.0).is_empty(), "the pursuit panel has the screen while a chase runs");
        let cooling = PursuitHud { secs: 3.0, evade: 0.2, cooldown: true, ..Default::default() };
        assert!(
            !built(&HudState { pursuit: Some(cooling), ..near }, 0.0).is_empty(),
            "the detector is back in the cool-down"
        );
    }
}
