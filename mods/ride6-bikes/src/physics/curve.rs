//! The tuning curves of the resolved data, as the polylines the manifest takes.
//!
//! RIDE 6 stores a curve as keys (`Points`), the interpolation mode(s) of the curve (`InterpModes`) and, for cubic keys, the
//! arrive and leave tangents (`Tangents`). The manifest's curves are piecewise linear. A linear curve is therefore copied
//! point for point; a cubic curve is sampled along its Hermite spline ([`SAMPLES_PER_SPAN`] points per span), which
//! `blackbox-vehicle` then interpolates linearly; a curve of more points than the manifest allows is simplified by dropping
//! the point whose removal changes the curve least. The dump lists the *set* of modes a curve uses, not the mode of each key,
//! so a curve that mixes modes is read as linear through its points (exact at the keys).

use anyhow::{Result, bail};

use super::node::Node;
use super::vehicle::Vehicle;

/// Points taken from each cubic span.
pub const SAMPLES_PER_SPAN: usize = 8;
const CUBIC: &str = "RCIM_Cubic";

/// A curve as the dump writes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    points: Vec<[f64; 2]>,
    modes: Vec<String>,
    /// `[arrive, leave]` of each key.
    tangents: Vec<[f64; 2]>,
}

impl Curve {
    /// A linear curve through `points`.
    pub fn linear(points: Vec<[f64; 2]>) -> Self {
        Self { points, modes: vec!["RCIM_Linear".to_owned()], tangents: Vec::new() }
    }

    /// Read `{"Points": [[x, y], ..], "InterpModes": [..], "Tangents": [[arrive, leave], ..]}`.
    pub fn parse(node: &Node<'_>) -> Result<Self> {
        let mut points = Vec::new();
        for point in node.req("Points")?.items()? {
            let pair = point.items()?;
            let [x, y] = pair.as_slice() else { bail!("{}: a key is not an [x, y] pair", point.path()) };
            points.push([x.num()?, y.num()?]);
        }
        let mut modes = Vec::new();
        if let Some(list) = node.get("InterpModes") {
            for mode in list.items()? {
                modes.push(mode.text()?.to_owned());
            }
        }
        let mut tangents = Vec::new();
        if let Some(list) = node.get("Tangents") {
            for tangent in list.items()? {
                let pair = tangent.items()?;
                let [arrive, leave] = pair.as_slice() else { bail!("{}: a tangent is not a pair", tangent.path()) };
                tangents.push([arrive.num()?, leave.num()?]);
            }
        }
        Ok(Self { points, modes, tangents })
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Whether every span is a cubic Hermite span the dump gives the tangents of.
    fn cubic(&self) -> bool {
        self.points.len() >= 2
            && self.tangents.len() == self.points.len()
            && !self.modes.is_empty()
            && self.modes.iter().all(|mode| mode == CUBIC)
    }

    /// The curve as a polyline: its keys when it is linear, its samples when it is cubic.
    pub fn polyline(&self) -> Vec<[f64; 2]> {
        if !self.cubic() {
            return self.points.clone();
        }
        let mut out = vec![self.points[0]];
        for (index, pair) in self.points.windows(2).enumerate() {
            let (leave, arrive) = (self.tangents[index][1], self.tangents[index + 1][0]);
            for step in 1..=SAMPLES_PER_SPAN {
                let t = step as f64 / SAMPLES_PER_SPAN as f64;
                out.push([pair[0][0] + (pair[1][0] - pair[0][0]) * t, hermite(pair[0], pair[1], leave, arrive, t)]);
            }
        }
        out
    }
}

/// The value at `t` (0 to 1) of the Hermite spline from `a` to `b` with the leave tangent of `a` and the arrive tangent of
/// `b` (slopes, as Unreal stores them).
fn hermite(a: [f64; 2], b: [f64; 2], leave: f64, arrive: f64, t: f64) -> f64 {
    let dx = b[0] - a[0];
    let (t2, t3) = (t * t, t * t * t);
    (2.0 * t3 - 3.0 * t2 + 1.0) * a[1]
        + (t3 - 2.0 * t2 + t) * dx * leave
        + (-2.0 * t3 + 3.0 * t2) * b[1]
        + (t3 - t2) * dx * arrive
}

/// A `FRuntimeFloatCurve` (`EditorCurveData` and `ExternalCurve`): the **external curve wins** when it is set
/// (`docs/formats/ride6-physics-data.md`, "Inheritance"), else the embedded keys. `None` when neither holds a key.
pub fn runtime_curve(vehicle: &Vehicle, node: &Node<'_>) -> Result<Option<Curve>> {
    if let Some(external) = node.get("ExternalCurve") {
        let target = vehicle.target(&external)?;
        let curve = Curve::parse(&target.req("FloatCurve")?)?;
        return Ok(Some(curve).filter(|curve| !curve.is_empty()));
    }
    let Some(embedded) = node.get("EditorCurveData") else { return Ok(None) };
    Ok(Some(Curve::parse(&embedded)?).filter(|curve| !curve.is_empty()))
}

/// The value of a polyline at `x`, held flat beyond its ends (the library's curves do the same).
pub fn eval(points: &[[f64; 2]], x: f64) -> f64 {
    let Some(first) = points.first() else { return 0.0 };
    if x <= first[0] {
        return first[1];
    }
    for pair in points.windows(2) {
        if x <= pair[1][0] {
            let width = pair[1][0] - pair[0][0];
            if width <= 0.0 {
                return pair[1][1];
            }
            return pair[0][1] + (pair[1][1] - pair[0][1]) * (x - pair[0][0]) / width;
        }
    }
    points.last().map_or(0.0, |last| last[1])
}

/// Reduce a polyline to at most `max` points by dropping, one at a time, the interior point whose removal moves the curve
/// least (its vertical distance from the line between its neighbours). The ends stay.
pub fn simplify(mut points: Vec<[f64; 2]>, max: usize) -> Vec<[f64; 2]> {
    while points.len() > max.max(2) {
        let mut best = (f64::INFINITY, 1);
        for index in 1..points.len() - 1 {
            let (a, b) = (points[index - 1], points[index + 1]);
            let line = eval(&[a, b], points[index][0]);
            let error = (points[index][1] - line).abs();
            if error < best.0 {
                best = (error, index);
            }
        }
        points.remove(best.1);
    }
    points
}

/// A `FClampedRangeTranslator`: the input clamped to `input`, mapped to `output` with an exponent `linearity`, unless it
/// holds a `Curve` of the quantity itself (`docs/specs/ride6-lean-controller.md`, "The range translator used everywhere").
#[derive(Debug, Clone, PartialEq)]
pub struct Translator {
    input: [f64; 2],
    output: [f64; 2],
    linearity: f64,
    curve: Option<Curve>,
}

impl Translator {
    /// Read a translator; an error when it has neither a curve nor a pair of ranges.
    pub fn read(vehicle: &Vehicle, node: &Node<'_>) -> Result<Self> {
        match Self::read_opt(vehicle, node)? {
            Some(translator) => Ok(translator),
            None => bail!("{}: the translator has no input and output range and no curve", node.path()),
        }
    }

    /// Read a translator; `None` when it has neither (a tuning the asset leaves unset, which changes nothing).
    pub fn read_opt(vehicle: &Vehicle, node: &Node<'_>) -> Result<Option<Self>> {
        let curve = match node.get("Curve") {
            Some(runtime) => runtime_curve(vehicle, &runtime)?,
            None => None,
        };
        let (input, output) = match (node.get("InputRange"), node.get("OutputRange")) {
            (Some(input), Some(output)) => (input.xy()?, output.xy()?),
            _ if curve.is_some() => ([0.0, 1.0], [0.0, 1.0]),
            _ => return Ok(None),
        };
        Ok(Some(Self { input, output, linearity: node.f_or("Linearity", 1.0)?, curve }))
    }

    /// The identity translator of the constructor default (`docs/specs/ride6-suspension.md`, "Unset C++ defaults").
    pub fn identity() -> Self {
        Self { input: [0.0, 1.0], output: [0.0, 1.0], linearity: 1.0, curve: None }
    }

    /// The input range.
    pub fn input_range(&self) -> [f64; 2] {
        self.input
    }

    /// The output at the two ends of the input range.
    pub fn output_range(&self) -> [f64; 2] {
        self.output
    }

    /// The translator as a polyline of `(x mapped by fx, y)`, x rising. `fx` converts the input's unit (km/h to m/s, say).
    pub fn points(&self, fx: impl Fn(f64) -> f64) -> Result<Vec<[f64; 2]>> {
        if let Some(curve) = &self.curve {
            return Ok(curve.polyline().into_iter().map(|[x, y]| [fx(x), y]).collect());
        }
        let width = self.input[1] - self.input[0];
        if width == 0.0 {
            bail!("a translator with an empty input range cannot be written as a curve");
        }
        let count = if (self.linearity - 1.0).abs() < 1e-9 { 1 } else { SAMPLES_PER_SPAN };
        let mut out = Vec::new();
        for step in 0..=count {
            let t = step as f64 / count as f64;
            let y = self.output[0] + (self.output[1] - self.output[0]) * t.powf(self.linearity);
            let x = if step == count { self.input[1] } else { self.input[0] + width * t };
            out.push([fx(x), y]);
        }
        if width < 0.0 {
            out.reverse();
        }
        Ok(out)
    }

    /// The output for an input `x`.
    pub fn at(&self, x: f64) -> f64 {
        if let Some(curve) = &self.curve {
            return eval(&curve.polyline(), x);
        }
        let width = self.input[1] - self.input[0];
        let t =
            if width == 0.0 { f64::from(x >= self.input[1]) } else { ((x - self.input[0]) / width).clamp(0.0, 1.0) };
        self.output[0] + (self.output[1] - self.output[0]) * t.powf(self.linearity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn curve(value: &serde_json::Value) -> Curve {
        Curve::parse(&Node::new(value, "c")).unwrap()
    }

    #[test]
    fn a_linear_curve_keeps_its_points_and_a_cubic_one_is_sampled() {
        let linear = curve(&json!({"Points": [[0.0, 1.0], [2.0, 3.0]], "InterpModes": ["RCIM_Linear"]}));
        assert_eq!(linear.polyline(), vec![[0.0, 1.0], [2.0, 3.0]]);
        // A cubic span with zero tangents is the smoothstep: halfway in x it is halfway in y.
        let cubic = curve(&json!({
            "Points": [[0.0, 0.0], [1.0, 1.0]], "InterpModes": ["RCIM_Cubic"], "Tangents": [[0.0, 0.0], [0.0, 0.0]]
        }));
        let line = cubic.polyline();
        assert_eq!(line.len(), 1 + SAMPLES_PER_SPAN);
        assert_eq!(line[SAMPLES_PER_SPAN], [1.0, 1.0]);
        assert!((eval(&line, 0.5) - 0.5).abs() < 1e-12);
        assert!(eval(&line, 0.25) < 0.25);
        // A mixed set of modes is read as linear through the keys.
        let mixed = curve(&json!({
            "Points": [[0.0, 0.0], [1.0, 1.0], [1.5, 2.0]], "InterpModes": ["RCIM_Linear", "RCIM_Cubic"],
            "Tangents": [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]
        }));
        assert_eq!(mixed.polyline().len(), 3);
    }

    #[test]
    fn simplify_drops_the_point_that_matters_least_and_keeps_the_ends() {
        let points = vec![[0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [3.0, 10.0], [4.0, 11.0]];
        let out = simplify(points, 4);
        assert_eq!(out, vec![[0.0, 0.0], [2.0, 2.0], [3.0, 10.0], [4.0, 11.0]]);
        assert_eq!(simplify(vec![[0.0, 0.0], [1.0, 5.0], [2.0, 0.0]], 2), vec![[0.0, 0.0], [2.0, 0.0]]);
    }

    #[test]
    fn a_translator_is_a_clamped_line_and_a_descending_input_comes_out_rising() {
        let value = json!({"InputRange": {"X": 10.0, "Y": 200.0}, "OutputRange": {"X": 1.0, "Y": 0.65}});
        let vehicle = Vehicle::parse(&json!({"Properties": {}}).to_string(), None).unwrap();
        let translator = Translator::read(&vehicle, &Node::new(&value, "t")).unwrap();
        assert_eq!(translator.points(|x| x).unwrap(), vec![[10.0, 1.0], [200.0, 0.65]]);
        assert_eq!(translator.at(5.0), 1.0);
        let down = json!({"InputRange": {"X": -0.3, "Y": -0.5}, "OutputRange": {"X": 0.0, "Y": 1.0}});
        let translator = Translator::read(&vehicle, &Node::new(&down, "t")).unwrap();
        assert_eq!(translator.points(|x| x).unwrap(), vec![[-0.5, 1.0], [-0.3, 0.0]]);
    }
}
