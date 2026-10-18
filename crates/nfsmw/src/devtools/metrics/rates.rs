//! Count completed renders and SDK presentations over the same rolling wall-clock interval.
use std::collections::VecDeque;

#[derive(Clone, Debug, Default)]
pub(super) struct Rates {
    samples: VecDeque<(f64, u64, Option<u64>)>,
    generation: bool,
}
impl Rates {
    pub fn push(&mut self, seconds: f64, rendered: u64, presented: Option<u64>, generation: bool) {
        if let Some(&(last, base, output)) = self.samples.back()
            && (seconds <= last
                || rendered < base
                || generation != self.generation
                || output.is_some() != presented.is_some()
                || output.zip(presented).is_some_and(|(before, after)| after < before))
        {
            self.samples.clear();
        }
        self.generation = generation;
        self.samples.push_back((seconds, rendered, presented));
        while self.samples.len() > 2 && self.samples[1].0 < seconds - 1.0 {
            self.samples.pop_front();
        }
        while self.samples.len() > 4096 {
            self.samples.pop_front();
        }
    }
    fn interval(&self) -> Option<(f64, u64, Option<u64>)> {
        let &(start, base, output) = self.samples.front()?;
        let &(end, last_base, last_output) = self.samples.back()?;
        let seconds = end - start;
        if seconds < 0.25 {
            return None;
        }
        Some((seconds, last_base - base, output.zip(last_output).map(|(before, after)| after - before)))
    }
    pub fn rendered(&self) -> Option<f32> {
        let (seconds, frames, _) = self.interval()?;
        Some((frames as f64 / seconds) as f32)
    }
    pub fn presented(&self) -> Option<f32> {
        let (seconds, base, output) = self.interval()?;
        let frames = match (output, self.generation) {
            (Some(value), _) => value,
            (None, false) => base,
            _ => return None,
        };
        Some((frames as f64 / seconds) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdk_counts_are_measured_instead_of_multiplying_a_requested_mode() {
        let mut rates = Rates::default();
        rates.push(0.0, 0, Some(0), true);
        rates.push(0.5, 30, Some(180), true);
        assert_eq!(rates.rendered(), Some(60.0));
        assert_eq!(rates.presented(), Some(360.0));
        // A configured generator may suspend: its real counts, not its setting, decide the FPS.
        rates.push(1.6, 96, Some(246), true);
        rates.push(2.1, 126, Some(276), true);
        assert_eq!(rates.rendered(), Some(60.0));
        assert_eq!(rates.presented(), Some(60.0));
    }
    #[test]
    fn missing_counts_and_counter_resets_never_fabricate_generated_frames() {
        let mut rates = Rates::default();
        rates.push(0.0, 0, None, true);
        rates.push(1.0, 60, None, true);
        assert_eq!(rates.presented(), None);
        rates.push(2.0, 120, None, false);
        assert_eq!(rates.rendered(), None);
        rates.push(3.0, 180, None, false);
        assert_eq!(rates.presented(), Some(60.0));
        rates.push(4.0, 0, Some(0), false);
        assert_eq!(rates.presented(), None);
    }
}
