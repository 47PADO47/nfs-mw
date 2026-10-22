//! The two views of a split-screen match: each player's half of the screen is a frame of the world from their camera.

use blackbox_gfx::{Fog, FrameParams, Instance};

use crate::scenes::world::{WorldScene, visibility};
use crate::viewer::camera::{Camera, Lens};

/// Whose view a frame is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scenes::world) enum Viewer {
    /// The first player (or the only one): their own camera and everything the single-player view shows.
    First,
    /// The second player: their racer's camera; the first player's car is always in the picture.
    Second,
}

impl WorldScene {
    /// The frame of one player's half, drawn at the half's `aspect`. `None` outside a split-screen match.
    pub(in crate::scenes::world) fn split_frame(
        &mut self,
        player: usize,
        aspect: f32,
    ) -> Option<(FrameParams, &[Instance])> {
        if !self.split_on() {
            return None;
        }
        match player {
            0 => Some(self.world_frame(aspect)),
            _ => self.second_frame(aspect),
        }
    }

    /// Whether the screen is split: a split-screen race or a co-op guest in the world.
    pub(in crate::scenes::world) fn split_on(&self) -> bool {
        self.split.is_some() || self.players.has_guests()
    }

    fn second_frame(&mut self, aspect: f32) -> Option<(FrameParams, &[Instance])> {
        // A co-op guest looks through its own chase camera; a split-screen racer through the match's camera.
        let lens = match self.guest_lens(aspect) {
            Some(lens) => lens,
            None => self.split.as_ref()?.camera.lens(aspect),
        };
        let mut visible = std::mem::take(&mut self.second_view.visible);
        self.fill_view(&lens, Viewer::Second, &mut visible);
        let fog_end = self.fog_distance;
        let cut = self.second_view.take_cut();
        let params = FrameParams {
            view: lens.view,
            projection: lens.projection,
            camera_position: lens.position,
            light_dir: self.sun.direction(),
            clear_color: self.fog_colour(),
            fog: Some(Fog { start: fog_end * 0.5, end: fog_end }),
            scattering: self.scattering(),
            surface_light: self.surface_light(),
            camera_cut: cut,
            shadow_tuning: None,
        };
        self.second_view.visible = visible;
        Some((params, &self.second_view.visible))
    }

    /// Culls the world for `lens` into `out`: scenery, props, sky, the traffic and racers, and the first player's car.
    pub(in crate::scenes::world) fn fill_view(&mut self, lens: &Lens, viewer: Viewer, out: &mut Vec<Instance>) {
        let cull = visibility::Camera {
            view_proj: lens.view_proj(),
            position: lens.position.to_array(),
            forward: lens.forward.to_array(),
            fov_y_radians: lens.fov_y_degrees.to_radians(),
            light_dir: self.sun.direction().to_array(),
        };
        let car_view = crate::scenes::world::drive::DetailView {
            view: blackbox_scenery::LodView {
                position: cull.position,
                forward: cull.forward,
                pixel_scale: blackbox_scenery::LodView::pixel_scale_for(cull.fov_y_radians, 480.0),
            },
            settings: self.detail.settings,
        };
        let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
        let original = self.original_shadows;
        let caster_distance = match self.shadow_feed.fed {
            true => 0.0,
            false => self.shadow_quality.sampling().2,
        };
        visibility::collect_main(
            self.residency.placed().filter(|p| original || !p.original_shadow),
            &cull,
            rules,
            self.residency.props(),
            &self.scenery_groups,
            caster_distance,
            &mut self.detail,
            out,
        );
        self.residency.props().fragment_instances(out);
        if self.original_sky {
            self.residency.sky.place(&self.residency.shared, out, lens.position, crate::scenes::world::sky::View::Main);
        }
        if let Some(traffic) = &mut self.traffic {
            let placed = self.residency.placed().filter_map(crate::scenes::world::ai::signal_props::candidate);
            traffic.update_signal_props(self.residency.visual_revision(), placed);
            traffic.instances_with_detail(Some(&car_view), out);
            let lamps = self.lamp_meshes.as_ref().filter(|_| self.traffic_setting.lights);
            traffic.lamp_instances(lamps, lens.position, self.residency.props(), out);
        }
        if let Some(drive) = self.players.lead() {
            let shown = match viewer {
                Viewer::Second => true,
                Viewer::First => self.player_in_picture(drive),
            };
            if shown {
                drive.instances_with_detail(Some(&car_view), out);
            }
            if let (true, Some(meshes)) = (self.markers_on, &self.marker_meshes) {
                drive.marker_instances(meshes, out);
            }
        }
        // Co-op guests are always in the picture: neither view is from inside their car.
        for guest in self.players.guests() {
            guest.drive.instances_with_detail(Some(&car_view), out);
        }
        // A multiplayer client's remote cars: other players, traffic, police (`net_client/remote.rs`).
        #[cfg(feature = "multiplayer")]
        self.remote_car_instances(out);
        // Each half shows its own player's route arrow: the first player's in theirs, a co-op guest's in the second.
        let arrow = match viewer {
            Viewer::First => self.route_arrow(),
            Viewer::Second => self.players.guests().next().and_then(|guest| self.route_arrow_for(guest.id)),
        };
        if let Some(arrow) = arrow {
            out.push(arrow);
        }
        // The NIS scene's characters: placed with the rest, so they sort by mesh with them.
        self.nis.character_instances(out);
        // Every source has appended now: one sort puts the copies of each mesh together, traffic included.
        visibility::sort_by_mesh(out);
    }
}
