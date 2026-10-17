//! The cop health bars' quads (`docs/specs/cop-health-bars.md` sections 11, 13.2 step 5 and 13.4). The scene keeps the
//! bar state here. Each simulated step the traffic gives the bars; each frame they become one alpha-blended textured
//! effect of flat-colour quads, drawn with a 1 x 1 white texture.

use blackbox_gfx::{
    BlendMode, EffectLayer, EffectVertex, PixelFormat, RenderBackend, TextureDesc, TextureHandle, TexturedEffect,
};
use blackbox_pursuit::health_bar::{Bar, BarParams, CopBarStates, Frame, segment_quads};

use crate::scenes::world::ai::TrafficWorld;

/// One opaque white texel: the quads carry their colour in the vertices.
const WHITE: [u8; 4] = [255; 4];

/// The bar state, the bars of the last step, and the white texture the quads are drawn with.
#[derive(Default)]
pub struct CopHealthBars {
    states: CopBarStates,
    bars: Vec<Bar>,
    texture: Option<TextureHandle>,
}

impl CopHealthBars {
    /// Steps the bars by `dt` simulated seconds: the health, lag and death timers of every cop, and the bars to draw.
    /// Without traffic there are no cops, so no state is kept.
    pub fn step(&mut self, traffic: Option<&TrafficWorld>, frame: Frame, dt: f32) {
        let Some(traffic) = traffic else {
            self.states = CopBarStates::default();
            self.bars.clear();
            return;
        };
        self.bars = traffic.cop_health_bars(&mut self.states, frame, dt, &BarParams::default());
    }

    /// Appends the last step's bars to `out` as one alpha-blended textured effect, when the setting `on` is set. The
    /// state is kept either way, so turning the bars on mid-chase shows the current health. Nothing is appended when no
    /// bar is drawn. The white texture is created on first use.
    pub fn emit(&mut self, renderer: &mut dyn RenderBackend, on: bool, out: &mut Vec<TexturedEffect>) {
        if !on || self.bars.is_empty() {
            return;
        }
        let texture = *self.texture.get_or_insert_with(|| white_texture(renderer));
        let vertices = vertices(&self.bars, &BarParams::default());
        out.push(TexturedEffect { texture, blend: BlendMode::AlphaBlend, vertices });
    }
}

/// The vertices of the bars' quads, in the bars' order (back to front) and each bar's draw order (border, lag, fill).
fn vertices(bars: &[Bar], params: &BarParams) -> Vec<EffectVertex> {
    let mut out = Vec::new();
    for bar in bars {
        for quad in segment_quads(bar, params) {
            let [r, g, b, a] = quad.rgba;
            let colour = [encode(r), encode(g), encode(b), byte(a)];
            EffectLayer::particle_quad(&mut out, quad.corners, colour, [0.0; 2]);
        }
    }
    out
}

fn white_texture(renderer: &mut dyn RenderBackend) -> TextureHandle {
    renderer.create_texture(&TextureDesc {
        label: "cop health bar",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&WHITE[..]],
    })
}

/// A linear colour channel as the sRGB byte the effect shaders take: they decode colour from the sRGB curve on an
/// sRGB target, so the vertex colour is encoded here (the inverse of `srgb_to_linear`).
fn encode(linear: f32) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let srgb = match c <= 0.003_130_8 {
        true => 12.92 * c,
        false => 1.055 * c.powf(1.0 / 2.4) - 0.055,
    };
    byte(srgb)
}

/// A 0 to 1 value as a byte. Alpha is not encoded.
fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_pursuit::health_bar::{Gates, Sample, build_bars, srgb_to_linear};
    use glam::Vec3;

    fn frame(gates: Gates) -> Frame {
        Frame { camera_pos: Vec3::ZERO, camera_forward: Vec3::X, gates }
    }

    fn open() -> Gates {
        Gates { pursuing: true, shot_running: false, hud_visible: true }
    }

    fn bars_of(sample: Sample, gates: Gates) -> Vec<Bar> {
        let mut states = CopBarStates::default();
        build_bars(&[sample], &frame(gates), &mut states, 0.0, &BarParams::default())
    }

    #[test]
    fn every_byte_survives_the_trip_to_linear_and_back() {
        for channel in 0u8..=255 {
            assert_eq!(encode(srgb_to_linear(channel)), channel);
        }
    }

    #[test]
    fn a_hurt_cop_draws_a_border_and_a_fill_and_a_dead_one_only_its_border() {
        let hurt = Sample { key: 1, position: Vec3::new(10.0, 0.0, 0.0), damage: 0.4, destroyed: false };
        // Border and fill, six vertices each: the lag is empty because it equals the health on the first step.
        assert_eq!(vertices(&bars_of(hurt, open()), &BarParams::default()).len(), 12);
        let dead = Sample { destroyed: true, ..hurt };
        // A destroyed cop has zero health and no lag, so the border is the only quad.
        assert_eq!(vertices(&bars_of(dead, open()), &BarParams::default()).len(), 6);
    }

    #[test]
    fn a_shut_gate_gives_no_vertices() {
        let hurt = Sample { key: 1, position: Vec3::new(10.0, 0.0, 0.0), damage: 0.4, destroyed: false };
        let shot = Gates { shot_running: true, ..open() };
        let hidden = Gates { hud_visible: false, ..open() };
        assert!(vertices(&bars_of(hurt, shot), &BarParams::default()).is_empty());
        assert!(vertices(&bars_of(hurt, hidden), &BarParams::default()).is_empty());
    }
}
