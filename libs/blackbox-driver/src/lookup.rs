//! Lookup helpers: evenly spaced tables, piecewise-linear graphs and the ramp function.
//! Spec: `docs/specs/ai-driver-control-pid.md` (§1.1).

/// `clamp((v - a) / (b - a), 0, 1)`.
pub fn ramp(v: f32, a: f32, b: f32) -> f32 {
    ((v - a) / (b - a)).clamp(0.0, 1.0)
}

/// Samples spread evenly over `[min, max]`, linear between neighbours and clamped outside.
#[derive(Debug, Clone, Copy)]
pub struct Table<const N: usize> {
    pub data: [f32; N],
    pub min: f32,
    pub max: f32,
}

impl<const N: usize> Table<N> {
    pub fn lookup(&self, x: f32) -> f32 {
        let position = ((x - self.min) * (N - 1) as f32 / (self.max - self.min)).clamp(0.0, (N - 1) as f32);
        let low = (position.floor() as usize).min(N - 1);
        let high = (low + 1).min(N - 1);
        let frac = position - low as f32;
        self.data[low] + (self.data[high] - self.data[low]) * frac
    }
}

/// Piecewise-linear over `(x, y)` points with increasing `x`; clamps to the end values.
#[derive(Debug, Clone, Copy)]
pub struct Graph<const N: usize> {
    pub points: [(f32, f32); N],
}

impl<const N: usize> Graph<N> {
    pub fn lookup(&self, x: f32) -> f32 {
        let (first, last) = (self.points[0], self.points[N - 1]);
        if x <= first.0 {
            return first.1;
        }
        if x >= last.0 {
            return last.1;
        }
        let i = self.points.iter().position(|p| p.0 > x).unwrap_or(N - 1);
        let ((x0, y0), (x1, y1)) = (self.points[i - 1], self.points[i]);
        y0 + (y1 - y0) * (x - x0) / (x1 - x0)
    }
}
