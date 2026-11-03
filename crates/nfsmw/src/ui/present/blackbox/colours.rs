//! Authored vertex tints multiplied through the node/ancestor tint, then premultiplied for UI blending.

use super::vertex_colour;
use blackbox_feng::UiNode;

pub(super) fn for_rect(node: &UiNode, tint: [u8; 4], additive: bool, rect: [f32; 4]) -> [[u8; 4]; 4] {
    let Some(corners) = node.corner_colours else { return [vertex_colour(tint, additive); 4] };
    let colors = corners.map(|corner| {
        vertex_colour(
            std::array::from_fn(|i| ((u32::from(corner[i]) * u32::from(tint[i]) + 127) / 255) as u8),
            additive,
        )
    });
    let [x0, y0, x1, y1] = rect;
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| {
        let weights = match x >= y {
            true => [1.0 - x, x - y, y, 0.0],
            false => [1.0 - y, 0.0, x, y - x],
        };
        std::array::from_fn(|channel| {
            colors.iter().zip(weights).map(|(c, w)| f32::from(c[channel]) * w).sum::<f32>().round().clamp(0.0, 255.0)
                as u8
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_tints_keep_alpha_and_ancestor_opacity_for_normal_and_repeated_images() {
        let node =
            UiNode { corner_colours: Some([[0; 4], [0, 0, 0, 200], [0, 0, 0, 200], [0; 4]]), ..Default::default() };
        assert_eq!(
            for_rect(&node, [255, 255, 255, 230], false, [0.0, 0.0, 1.0, 1.0]),
            [[0; 4], [0, 0, 0, 180], [0, 0, 0, 180], [0; 4]]
        );
        assert_eq!(
            for_rect(&node, [255; 4], false, [0.25, 0.0, 0.75, 1.0]),
            [[0, 0, 0, 50], [0, 0, 0, 150], [0, 0, 0, 150], [0, 0, 0, 50]]
        );
        assert_eq!(
            for_rect(&UiNode::default(), [200, 100, 50, 128], true, [0.0, 0.0, 1.0, 1.0]),
            [[100, 50, 25, 0]; 4]
        );
    }
}
