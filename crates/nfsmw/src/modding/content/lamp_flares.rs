//! A car manifest's `[[lamp_flare]]` and `[[light_bar]]` tables as the lamps the game draws (`nfsmw_data::car::mods::lamp_flares`).

use blackbox_modding::content::{CarManifest, LampFlare, LampLit, LampSide, LampSprite, LightBar, SirenColour};
use nfsmw_data::car::mods::lamp_flares::{Bar, Flare, Lamps, Lit, Look, Side, Siren};

/// The lamps of a manifest, `None` for a car that gives neither table.
pub(super) fn lamps_of(manifest: &CarManifest) -> Option<Lamps> {
    if manifest.lamp_flares.is_empty() && manifest.light_bars.is_empty() {
        return None;
    }
    Some(Lamps {
        flares: manifest.lamp_flares.iter().map(flare).collect(),
        bars: manifest.light_bars.iter().map(bar).collect(),
    })
}

fn flare(flare: &LampFlare) -> Flare {
    let lit = match flare.lit {
        LampLit::Head => Lit::Head,
        LampLit::Tail => Lit::Tail,
        LampLit::Brake => Lit::Brake,
        LampLit::Reverse => Lit::Reverse,
        LampLit::Always => Lit::Always,
    };
    let side = match flare.side {
        LampSide::Left => Side::Left,
        LampSide::Right => Side::Right,
        LampSide::Centre => Side::Centre,
    };
    Flare { lit, side, look: look(&flare.look) }
}

fn bar(bar: &LightBar) -> Bar {
    let siren = match bar.siren {
        SirenColour::Red => Siren::Red,
        SirenColour::Blue => Siren::Blue,
        SirenColour::White => Siren::White,
    };
    Bar {
        siren,
        look: look(&bar.look),
        pattern: bar.pattern.clone(),
        step_seconds: bar.step_seconds,
        phase_seconds: bar.phase_seconds,
    }
}

fn look(look: &LampSprite) -> Look {
    Look {
        sprite: look.sprite.clone(),
        position: look.position,
        direction: look.direction,
        colour: look.colour,
        alpha: look.alpha,
        radius: look.radius,
        range: look.range,
        falloff: look.falloff,
        z_bias: look.z_bias,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_with_lamps_becomes_the_games_lamps_and_one_without_becomes_none() {
        let text = "
[[lamp_flare]]
state = \"brake\"
side = \"right\"
sprite = \"CAR_FLARE0\"
position = [-2.0, -0.6, 0.7]
direction = [-1.0, 0.0, 0.0]
radius = [0.1, 0.4]

[[light_bar]]
siren = \"red\"
sprite = \"CAR_FLARE1\"
position = [0.0, 0.3, 1.4]
direction = [0.0, 0.0, 1.0]
radius = [0.1, 0.4]
pattern = \"909\"
step_seconds = 0.25
";
        let manifest = blackbox_modding::content::parse_car("lampcar", text).unwrap();
        let lamps = lamps_of(&manifest).unwrap();
        assert_eq!((lamps.flares[0].lit, lamps.flares[0].side), (Lit::Brake, Side::Right));
        assert_eq!(lamps.bars[0].siren, Siren::Red);
        assert_eq!(lamps.bars[0].pattern, vec![1.0, 0.0, 1.0]);
        assert_eq!(lamps.flares[0].look.sprite, "CAR_FLARE0");
        let plain = blackbox_modding::content::parse_car("plaincar", "name = \"x\"").unwrap();
        assert!(lamps_of(&plain).is_none());
    }
}
