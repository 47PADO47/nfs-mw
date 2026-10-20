use super::*;

impl Instance {
    pub(super) fn stop(&mut self) {
        self.stopping = true;
        for (_, emitter) in &mut self.emitters {
            emitter.set_enabled(false);
        }
    }

    pub(super) fn step(&mut self, props: &super::super::props::PropWorld, dt: f32) {
        if !self.stopping && self.source.effect.instance != 0 {
            if self.source.owner.upgrade().is_none() {
                self.stop();
            }
            if !self.stopping {
                self.refresh_frame(props.effect_frame(self.source.id, self.source.effect.position));
            }
        }
        let mut frame = self.source.frame;
        frame.intensity = self.parameters.intensity(self.source.effect.intensity);
        frame.inherit_velocity *= self.parameters.inherit_velocity;
        for (_, emitter) in &mut self.emitters {
            emitter.step(dt, &frame);
        }
    }

    pub(super) fn refresh_frame(&mut self, frame: Option<Frame>) {
        match frame {
            Some(frame) => {
                self.source.frame.intensity = self.source.effect.intensity;
                self.source.frame.inherit_velocity = frame.inherit_velocity;
                if self.source.effect.tracking {
                    self.source.frame.to_world = frame.to_world;
                }
            }
            None => self.stop(),
        }
    }
}
