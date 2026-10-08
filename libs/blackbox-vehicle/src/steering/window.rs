/// A moving-average window over the last `N` samples (a fixed ring, no allocation).
#[derive(Clone, Copy, Debug)]
pub struct Window<const N: usize> {
    samples: [f32; N],
    next: usize,
    len: usize,
}

impl<const N: usize> Default for Window<N> {
    fn default() -> Self {
        Self { samples: [0.0; N], next: 0, len: 0 }
    }
}

impl<const N: usize> Window<N> {
    pub fn push(&mut self, v: f32) {
        self.samples[self.next] = v;
        self.next = (self.next + 1) % N;
        self.len = (self.len + 1).min(N);
    }

    /// Mean of the samples seen so far (0 when empty).
    pub fn mean(&self) -> f32 {
        if self.len == 0 { 0.0 } else { self.samples[..self.len.min(N)].iter().sum::<f32>() / self.len as f32 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_of_the_last_n() {
        let mut w = Window::<3>::default();
        assert_eq!(w.mean(), 0.0);
        w.push(3.0);
        assert_eq!(w.mean(), 3.0);
        w.push(6.0);
        w.push(9.0);
        w.push(12.0);
        assert_eq!(w.mean(), 9.0);
    }
}
